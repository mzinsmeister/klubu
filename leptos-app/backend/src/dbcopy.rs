//! `migrate-db`: copies this instance's database into another one, typically of
//! the other dialect (SQLite ⇄ Postgres). Run with the server stopped.
//!
//! The copy is **catalog-driven**: tables, columns, column types, primary keys
//! and foreign-key ordering are all read from the database's own metadata
//! (`information_schema` / `PRAGMA`), never from a hand-maintained list. A
//! schema change that lands in the migrations is picked up here automatically —
//! the tool cannot drift.
//!
//! It moves **raw rows**, deliberately not the domain-level repository methods:
//! replaying documents through `save_invoice`/`commit_invoice` would re-number
//! ids, re-stamp `committed_timestamp` and write fresh audit entries — a
//! falsified archive. GoBD requires the migrated data to be the *same* records,
//! so the journal, the timestamps and every id survive byte-for-byte.

use sqlx::any::AnyRow;
use sqlx::{AnyPool, Executor, Row};
use std::collections::{BTreeMap, BTreeSet};

pub fn is_sqlite_url(db_url: &str) -> bool {
    db_url.trim_start().starts_with("sqlite:")
}

/// SQLite is configured for durability rather than throughput: a bookkeeping
/// ledger may not lose a committed transaction, and §147 AO expects the archive
/// to still be readable in ten years.
///
/// sqlx leaves `journal_mode` at SQLite's built-in default (`DELETE`), where any
/// writer blocks every reader. WAL fixes that. WAL alone is only crash-safe up to
/// the last checkpoint, so `synchronous = FULL` is set as well: it fsyncs the WAL
/// on every commit, which is what makes a committed invoice survive power loss.
/// `foreign_keys` defaults to OFF outside sqlx's own options path, and the
/// `ON DELETE SET NULL` clauses depend on it.
///
/// All of these are connection-scoped (journal_mode is the exception: it sticks
/// to the database file), so they run on every pooled connection via the
/// `after_connect` hook — `AnyConnectOptions` itself only carries a URL. The
/// pool stays small so a connection that is already inside a transaction can
/// never starve a nested acquire.
pub async fn connect_pool(db_url: &str) -> Result<AnyPool, sqlx::Error> {
    install_drivers_once();

    let sqlite = is_sqlite_url(db_url);
    sqlx::pool::PoolOptions::<sqlx::Any>::new()
        .max_connections(if sqlite { 4 } else { 10 })
        .after_connect(move |conn, _meta| {
            Box::pin(async move {
                if sqlite {
                    conn.execute(
                        "PRAGMA journal_mode = WAL; \
                         PRAGMA synchronous = FULL; \
                         PRAGMA foreign_keys = ON; \
                         PRAGMA busy_timeout = 30000;",
                    )
                    .await?;
                }
                Ok(())
            })
        })
        .connect(db_url)
        .await
}

/// `install_default_drivers` panics when the driver table is already set, and
/// `migrate-db` needs two pools in one process.
fn install_drivers_once() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(sqlx::any::install_default_drivers);
}

/// Clean pool teardown. For SQLite this first checkpoints the WAL with
/// `TRUNCATE`, so all committed data is back in the main database file and the
/// `-wal` / `-shm` sidecars are empty — safe to delete, and the `.db` file
/// alone is a complete copy for backups. SQLite would checkpoint when the last
/// connection closes cleanly anyway; doing it explicitly makes teardown
/// deterministic and lets us notice when it could not complete.
pub async fn shutdown_pool(pool: &AnyPool, db_url: &str) {
    if is_sqlite_url(db_url) {
        // Returns one row: (busy, wal_pages, checkpointed_pages).
        match sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
            .fetch_optional(pool)
            .await
        {
            Ok(row) => {
                let busy = row
                    .and_then(|r| r.try_get::<i64, _>(0).ok())
                    .unwrap_or_default();
                if busy != 0 {
                    eprintln!(
                        "Warnung: WAL-Checkpoint unvollständig (Datenbank war belegt); \
                         die -wal-Datei enthält noch Daten und darf nicht gelöscht werden"
                    );
                }
            }
            Err(error) => eprintln!("Warnung: WAL-Checkpoint fehlgeschlagen: {error}"),
        }
    }
    pool.close().await;
}

/// The dialect-matching migration set. Both directories are compiled into the
/// binary and kept in lockstep (same file names, same versions).
pub fn migrator_for(db_url: &str) -> sqlx::migrate::Migrator {
    if is_sqlite_url(db_url) {
        sqlx::migrate!("./migrations-sqlite")
    } else {
        sqlx::migrate!("./migrations")
    }
}

type CopyError = String;

fn db_err(context: &str) -> impl Fn(sqlx::Error) -> CopyError + '_ {
    move |error| format!("{context}: {error}")
}

/// How a column travels through the copy. Derived from the catalog's declared
/// type per *column*, not per value: a NULL must be bound with the type of its
/// column, or Postgres refuses the insert (there is no `int8 → varchar` cast,
/// not even for NULL).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Int,
    Float,
    Text,
    Blob,
}

/// Maps a declared column type to a transport kind. This is the only mapping in
/// the tool, and it enumerates the *dialects'* type vocabulary — a closed set —
/// not the schema's columns.
fn kind_of(declared: &str) -> Result<Kind, CopyError> {
    let lower = declared.to_ascii_lowercase();
    let base = lower.split('(').next().unwrap_or("").trim();
    Ok(match base {
        "integer" | "int" | "int2" | "int4" | "int8" | "smallint" | "bigint" | "serial"
        | "bigserial" | "boolean" | "bool" => Kind::Int,
        "real" | "double precision" | "float" | "float4" | "float8" | "numeric" | "decimal" => {
            Kind::Float
        }
        "text" | "varchar" | "character varying" | "character" | "char" | "clob" => Kind::Text,
        "bytea" | "blob" => Kind::Blob,
        other => {
            return Err(format!(
                "Unbekannter Spaltentyp '{other}' — bitte `kind_of` in dbcopy.rs erweitern"
            ))
        }
    })
}

struct TableMeta {
    name: String,
    /// (column name, transport kind), in catalog order.
    columns: Vec<(String, Kind)>,
    /// Primary-key columns; rows are copied in ascending PK order so that
    /// self-referencing foreign keys (storno → invoice, offer revision → group
    /// head) always point at an already-copied row.
    pk: Vec<String>,
    /// Tables this one references (self-references excluded).
    refs: BTreeSet<String>,
}

async fn source_tables(pool: &AnyPool, sqlite: bool) -> Result<Vec<TableMeta>, CopyError> {
    let names: Vec<String> = if sqlite {
        sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type = 'table' \
             AND name NOT LIKE 'sqlite_%' AND name NOT LIKE '_sqlx_%' ORDER BY name",
        )
        .fetch_all(pool)
        .await
        .map_err(db_err("Tabellenliste (sqlite_master) konnte nicht gelesen werden"))?
    } else {
        sqlx::query_scalar(
            "SELECT tablename::text FROM pg_catalog.pg_tables WHERE schemaname = 'public' \
             AND tablename NOT LIKE '\\_sqlx\\_%' ORDER BY tablename",
        )
        .fetch_all(pool)
        .await
        .map_err(db_err("Tabellenliste (pg_tables) konnte nicht gelesen werden"))?
    };

    let mut tables = Vec::new();
    for name in names {
        tables.push(if sqlite {
            sqlite_table_meta(pool, &name).await?
        } else {
            postgres_table_meta(pool, &name).await?
        });
    }
    Ok(tables)
}

async fn sqlite_table_meta(pool: &AnyPool, name: &str) -> Result<TableMeta, CopyError> {
    // PRAGMA arguments cannot be bound; `name` comes from sqlite_master, not
    // from user input.
    let info = sqlx::query(&format!("PRAGMA table_info({name})"))
        .fetch_all(pool)
        .await
        .map_err(db_err("PRAGMA table_info fehlgeschlagen"))?;

    let mut columns = Vec::new();
    let mut pk_ranked: Vec<(i64, String)> = Vec::new();
    for row in &info {
        let column: String = row.try_get("name").map_err(|e| e.to_string())?;
        let declared: String = row.try_get("type").map_err(|e| e.to_string())?;
        let pk_rank: i64 = row.try_get("pk").map_err(|e| e.to_string())?;
        columns.push((column.clone(), kind_of(&declared)?));
        if pk_rank > 0 {
            pk_ranked.push((pk_rank, column));
        }
    }
    pk_ranked.sort();

    let fk_rows = sqlx::query(&format!("PRAGMA foreign_key_list({name})"))
        .fetch_all(pool)
        .await
        .map_err(db_err("PRAGMA foreign_key_list fehlgeschlagen"))?;
    let mut refs = BTreeSet::new();
    for row in &fk_rows {
        let to: String = row.try_get("table").map_err(|e| e.to_string())?;
        if to != name {
            refs.insert(to);
        }
    }

    Ok(TableMeta {
        name: name.to_string(),
        columns,
        pk: pk_ranked.into_iter().map(|(_, c)| c).collect(),
        refs,
    })
}

async fn postgres_table_meta(pool: &AnyPool, name: &str) -> Result<TableMeta, CopyError> {
    let column_rows = sqlx::query(
        "SELECT column_name::text AS column_name, data_type::text AS data_type FROM information_schema.columns \
         WHERE table_schema = 'public' AND table_name = $1 ORDER BY ordinal_position",
    )
    .bind(name)
    .fetch_all(pool)
    .await
    .map_err(db_err("information_schema.columns konnte nicht gelesen werden"))?;
    let mut columns = Vec::new();
    for row in &column_rows {
        let column: String = row.try_get("column_name").map_err(|e| e.to_string())?;
        let declared: String = row.try_get("data_type").map_err(|e| e.to_string())?;
        columns.push((column, kind_of(&declared)?));
    }

    let pk: Vec<String> = sqlx::query_scalar(
        "SELECT kcu.column_name::text \
         FROM information_schema.table_constraints tc \
         JOIN information_schema.key_column_usage kcu \
           ON kcu.constraint_name = tc.constraint_name \
          AND kcu.table_schema = tc.table_schema \
         WHERE tc.constraint_type = 'PRIMARY KEY' \
           AND tc.table_schema = 'public' AND tc.table_name = $1 \
         ORDER BY kcu.ordinal_position",
    )
    .bind(name)
    .fetch_all(pool)
    .await
    .map_err(db_err("Primärschlüssel konnte nicht gelesen werden"))?;

    let ref_rows: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT ccu.table_name::text \
         FROM information_schema.table_constraints tc \
         JOIN information_schema.constraint_column_usage ccu \
           ON ccu.constraint_name = tc.constraint_name \
          AND ccu.table_schema = tc.table_schema \
         WHERE tc.constraint_type = 'FOREIGN KEY' \
           AND tc.table_schema = 'public' AND tc.table_name = $1",
    )
    .bind(name)
    .fetch_all(pool)
    .await
    .map_err(db_err("Fremdschlüssel konnten nicht gelesen werden"))?;
    let refs = ref_rows.into_iter().filter(|to| to != name).collect();

    Ok(TableMeta {
        name: name.to_string(),
        columns,
        pk,
        refs,
    })
}

/// Referenced tables first. Self-references are already excluded from `refs`;
/// within a table, ascending PK order covers them.
fn topo_sort(mut tables: Vec<TableMeta>) -> Result<Vec<TableMeta>, CopyError> {
    let mut sorted: Vec<TableMeta> = Vec::new();
    let mut done: BTreeSet<String> = BTreeSet::new();
    while !tables.is_empty() {
        let ready: Vec<usize> = tables
            .iter()
            .enumerate()
            .filter(|(_, t)| t.refs.iter().all(|r| done.contains(r)))
            .map(|(i, _)| i)
            .collect();
        if ready.is_empty() {
            let stuck: Vec<&str> = tables.iter().map(|t| t.name.as_str()).collect();
            return Err(format!(
                "Zyklische Fremdschlüssel zwischen Tabellen: {}",
                stuck.join(", ")
            ));
        }
        // Everything in one batch depends only on earlier batches, so the order
        // within a batch is free; removal runs back-to-front to keep the
        // collected indices valid, and a sort restores alphabetical order.
        let mut batch: Vec<TableMeta> = ready.into_iter().rev().map(|i| tables.remove(i)).collect();
        batch.sort_by(|a, b| a.name.cmp(&b.name));
        for t in batch {
            done.insert(t.name.clone());
            sorted.push(t);
        }
    }
    Ok(sorted)
}

/// One in-flight cell. `None` on the inner option is SQL NULL, carried with its
/// column's kind so the bind is correctly typed on the target.
enum Cell {
    Int(Option<i64>),
    Float(Option<f64>),
    Text(Option<String>),
    Blob(Option<Vec<u8>>),
}

fn read_cell(row: &AnyRow, index: usize, kind: Kind) -> Result<Cell, CopyError> {
    Ok(match kind {
        Kind::Int => Cell::Int(row.try_get(index).map_err(|e| e.to_string())?),
        Kind::Float => Cell::Float(read_float(row, index)?),
        Kind::Text => Cell::Text(row.try_get(index).map_err(|e| e.to_string())?),
        Kind::Blob => Cell::Blob(row.try_get(index).map_err(|e| e.to_string())?),
    })
}

/// SQLite stores a REAL column's value as INTEGER when it happens to be whole
/// (type affinity), so a float column may decode as an integer.
fn read_float(row: &AnyRow, index: usize) -> Result<Option<f64>, CopyError> {
    match row.try_get::<Option<f64>, _>(index) {
        Ok(value) => Ok(value),
        Err(_) => Ok(row
            .try_get::<Option<i64>, _>(index)
            .map_err(|e| e.to_string())?
            .map(|v| v as f64)),
    }
}

/// Placeholder list `$1, $2, …` — both dialects accept `$N` natively.
fn placeholders(n: usize) -> String {
    (1..=n).map(|i| format!("${i}")).collect::<Vec<_>>().join(", ")
}

pub async fn run_db_migration(source_url: &str, target_url: &str) -> Result<(), CopyError> {
    if source_url == target_url {
        return Err("Quelle und Ziel sind dieselbe Datenbank".to_string());
    }

    println!("Quelle: {source_url}");
    println!("Ziel:   {target_url}");

    let source = connect_pool(source_url)
        .await
        .map_err(db_err("Verbindung zur Quelldatenbank fehlgeschlagen"))?;
    let target = connect_pool(target_url)
        .await
        .map_err(db_err("Verbindung zur Zieldatenbank fehlgeschlagen"))?;

    println!("Wende Migrationen auf das Ziel an...");
    migrator_for(target_url)
        .run(&target)
        .await
        .map_err(|e| format!("Migrationen auf dem Ziel fehlgeschlagen: {e}"))?;

    // Same schema version on both sides, or the column sets could differ. The
    // checksums differ by design (dialect-specific SQL); the versions must not.
    let versions = |pool: &AnyPool| {
        let pool = pool.clone();
        async move {
            sqlx::query_scalar::<_, i64>("SELECT version FROM _sqlx_migrations ORDER BY version")
                .fetch_all(&pool)
                .await
                .map_err(db_err("Migrationsstand konnte nicht gelesen werden"))
        }
    };
    let source_versions = versions(&source).await?;
    let target_versions = versions(&target).await?;
    if source_versions != target_versions {
        return Err(format!(
            "Migrationsstände unterscheiden sich (Quelle: {source_versions:?}, Ziel: {target_versions:?}). \
             Die Quelle einmal mit dem Server starten, damit ihre Migrationen laufen."
        ));
    }

    let tables = topo_sort(source_tables(&source, is_sqlite_url(source_url)).await?)?;

    // The journal is append-only on both sides (enforced by triggers), so a
    // non-empty target cannot be cleared — and mixing two histories would make
    // both unglaubwürdig. Everything else on a fresh target is either empty or
    // migration-seeded (document_counter) and is replaced by the copy.
    let audit_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_log")
        .fetch_one(&target)
        .await
        .map_err(db_err("audit_log im Ziel konnte nicht geprüft werden"))?;
    if audit_count != 0 {
        return Err(
            "Das Ziel enthält bereits Journaleinträge (audit_log). Migration nur in eine leere \
             Datenbank."
                .to_string(),
        );
    }

    let mut tx = target
        .begin()
        .await
        .map_err(db_err("Transaktion auf dem Ziel konnte nicht gestartet werden"))?;

    // Clear migration-seeded rows, dependents first. audit_log is empty (checked
    // above) and its triggers forbid DELETE, so skip it.
    for table in tables.iter().rev() {
        if table.name == "audit_log" {
            continue;
        }
        sqlx::query(&format!("DELETE FROM {}", table.name))
            .execute(&mut *tx)
            .await
            .map_err(db_err(&format!("Leeren von {} fehlgeschlagen", table.name)))?;
    }

    let mut summary: BTreeMap<String, usize> = BTreeMap::new();
    for table in &tables {
        let column_list = table
            .columns
            .iter()
            .map(|(c, _)| c.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let order_by = if table.pk.is_empty() {
            String::new()
        } else {
            format!(" ORDER BY {}", table.pk.join(", "))
        };

        // A one-person bookkeeping database fits in memory comfortably; a
        // streaming copy would only complicate the transaction handling.
        let rows = sqlx::query(&format!(
            "SELECT {column_list} FROM {}{order_by}",
            table.name
        ))
        .fetch_all(&source)
        .await
        .map_err(db_err(&format!("Lesen aus {} fehlgeschlagen", table.name)))?;

        let insert_sql = format!(
            "INSERT INTO {} ({column_list}) VALUES ({})",
            table.name,
            placeholders(table.columns.len())
        );

        for row in &rows {
            let mut insert = sqlx::query(&insert_sql);
            for (index, (_, kind)) in table.columns.iter().enumerate() {
                insert = match read_cell(row, index, *kind)? {
                    Cell::Int(v) => insert.bind(v),
                    Cell::Float(v) => insert.bind(v),
                    Cell::Text(v) => insert.bind(v),
                    Cell::Blob(v) => insert.bind(v),
                };
            }
            insert
                .execute(&mut *tx)
                .await
                .map_err(db_err(&format!("Einfügen in {} fehlgeschlagen", table.name)))?;
        }
        summary.insert(table.name.clone(), rows.len());
    }

    tx.commit()
        .await
        .map_err(db_err("Übernahme auf dem Ziel fehlgeschlagen"))?;

    // Postgres sequences never saw the copied ids; advance each one past the
    // copied maximum. SQLite needs no equivalent — AUTOINCREMENT tracks the
    // largest rowid ever inserted on its own.
    if !is_sqlite_url(target_url) {
        for table in &tables {
            let serial_columns: Vec<String> = sqlx::query_scalar(
                "SELECT column_name::text FROM information_schema.columns \
                 WHERE table_schema = 'public' AND table_name = $1 \
                   AND column_default LIKE 'nextval(%'",
            )
            .bind(&table.name)
            .fetch_all(&target)
            .await
            .map_err(db_err("Sequenzspalten konnten nicht ermittelt werden"))?;
            for column in serial_columns {
                sqlx::query(&format!(
                    "SELECT setval(pg_get_serial_sequence('{t}', '{column}'), \
                     COALESCE((SELECT MAX({column}) FROM {t}), 0) + 1, false)",
                    t = table.name
                ))
                .execute(&target)
                .await
                .map_err(db_err(&format!(
                    "Sequenz für {}.{column} konnte nicht gesetzt werden",
                    table.name
                )))?;
            }
        }
    }

    // Row counts on both ends are the cheapest end-to-end verification that
    // nothing was silently skipped.
    for (table, copied) in &summary {
        let target_count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
            .fetch_one(&target)
            .await
            .map_err(db_err("Zielzählung fehlgeschlagen"))?;
        if target_count as usize != *copied {
            return Err(format!(
                "Zeilenzahl in {table} stimmt nicht: {copied} kopiert, {target_count} im Ziel"
            ));
        }
        println!("  {table}: {copied} Zeilen");
    }

    // Leaves SQLite files self-contained: the copy is checkpointed into the
    // main .db file and the WAL sidecars are emptied on both ends.
    shutdown_pool(&source, source_url).await;
    shutdown_pool(&target, target_url).await;

    println!("Migration abgeschlossen. Den Server jetzt mit DATABASE_URL={target_url} starten.");
    println!("Hinweis: das Dokumentenarchiv (document_storage/) liegt im Dateisystem und wandert unverändert mit.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declared_types_of_both_dialects_map_to_a_transport_kind() {
        // The union of what the two migration sets actually declare.
        for declared in [
            "INTEGER", "integer", "INT", "SERIAL", "BIGINT", "bigint", "boolean",
            "DOUBLE PRECISION", "double precision", "REAL",
            "TEXT", "text", "VARCHAR(255)", "character varying", "VARCHAR(50)", "VARCHAR(8)",
            "BYTEA", "BLOB",
        ] {
            assert!(kind_of(declared).is_ok(), "unmapped type: {declared}");
        }
        assert!(kind_of("json").is_err());
    }

    #[test]
    fn topo_sort_puts_referenced_tables_first_and_reports_cycles() {
        let t = |name: &str, refs: &[&str]| TableMeta {
            name: name.into(),
            columns: vec![("id".into(), Kind::Int)],
            pk: vec!["id".into()],
            refs: refs.iter().map(|s| s.to_string()).collect(),
        };
        let sorted = topo_sort(vec![
            t("invoice_item", &["invoice"]),
            t("invoice", &["contact", "document"]),
            t("contact", &[]),
            t("document", &[]),
        ])
        .expect("no cycle");
        let pos = |n: &str| sorted.iter().position(|x| x.name == n).unwrap();
        assert!(pos("contact") < pos("invoice"));
        assert!(pos("document") < pos("invoice"));
        assert!(pos("invoice") < pos("invoice_item"));

        assert!(topo_sort(vec![t("a", &["b"]), t("b", &["a"])]).is_err());
    }

    #[test]
    fn placeholders_are_one_based_dollar_numbers() {
        assert_eq!(placeholders(3), "$1, $2, $3");
    }
}
