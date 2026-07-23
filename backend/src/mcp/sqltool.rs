//! Read-only SQL access for the assistant tools.
//!
//! Defense in depth, because "read only" must hold even against a creative
//! model: the statement is filtered to SELECT/WITH/EXPLAIN, it runs inside a
//! transaction that is always rolled back, and on PostgreSQL the transaction
//! is additionally `SET TRANSACTION READ ONLY` (which also blocks
//! data-modifying CTEs the textual filter cannot see).

use app::db::{dialect, DbPool};
use futures_util::TryStreamExt;
use serde_json::{json, Value};
use sqlx::{Column, Row, ValueRef};

const MAX_ROWS: usize = 200;
const MAX_CELL_CHARS: usize = 2000;

pub async fn run_read_only_query(pool: &DbPool, query: String) -> Result<Value, String> {
    let statement = validate(&query)?;

    let mut tx = pool
        .begin()
        .await
        .map_err(|error| format!("Could not open a read-only transaction: {error}"))?;
    if dialect(pool) == "postgres" {
        sqlx::query("SET TRANSACTION READ ONLY")
            .execute(&mut *tx)
            .await
            .map_err(|error| format!("Could not enforce a read-only transaction: {error}"))?;
    }

    let mut columns: Vec<String> = Vec::new();
    let mut rows: Vec<Value> = Vec::new();
    let mut truncated = false;
    {
        let mut stream = sqlx::query(statement).fetch(&mut *tx);
        while let Some(row) = stream.try_next().await.map_err(|error| {
            // The Any driver decodes rows eagerly and cannot represent every
            // native type (e.g. Postgres `name`, `numeric`). A cast in the
            // query fixes that, so say so instead of just failing.
            format!(
                "SQL error: {error}. Hint: the portable driver cannot decode some native column types; cast them in the query (e.g. table_name::text, amount::bigint)."
            )
        })? {
            if columns.is_empty() {
                columns = row
                    .columns()
                    .iter()
                    .map(|column| column.name().to_string())
                    .collect();
            }
            if rows.len() >= MAX_ROWS {
                truncated = true;
                break;
            }
            rows.push(Value::Array(
                (0..row.columns().len())
                    .map(|index| cell_value(&row, index))
                    .collect(),
            ));
        }
    }
    // Nothing this tool ran may persist, whatever it was.
    let _ = tx.rollback().await;

    Ok(json!({
        "dialect": dialect(pool),
        "columns": columns,
        "rows": rows,
        "row_count": rows.len(),
        "truncated": truncated,
    }))
}

/// Accepts a single read-only statement. Over-accepting a non-statement is
/// harmless (the database rejects it inside the rolled-back transaction);
/// under-rejecting a write is what this function must never do.
fn validate(query: &str) -> Result<&str, String> {
    let statement = query.trim().trim_end_matches(';').trim();
    if statement.is_empty() {
        return Err("The SQL query is empty".to_string());
    }
    if statement.contains(';') {
        return Err("Exactly one SQL statement is allowed per call".to_string());
    }
    let lowered = statement.to_ascii_lowercase();
    if ["select", "with", "explain"]
        .iter()
        .any(|keyword| lowered.starts_with(keyword))
    {
        Ok(statement)
    } else {
        Err(
            "Only read-only queries are allowed: the statement must start with SELECT, WITH, or EXPLAIN (comments included)"
                .to_string(),
        )
    }
}

/// Decodes one cell into JSON without knowing the column type up front — the
/// `Any` driver only reveals types at runtime. Undecodable values degrade to a
/// marker string instead of failing the whole query.
fn cell_value(row: &sqlx::any::AnyRow, index: usize) -> Value {
    match row.try_get_raw(index) {
        Ok(raw) if raw.is_null() => return Value::Null,
        Ok(_) => {}
        Err(_) => return json!("<unreadable>"),
    }
    if let Ok(value) = row.try_get::<i64, _>(index) {
        return json!(value);
    }
    if let Ok(value) = row.try_get::<f64, _>(index) {
        return json!(value);
    }
    if let Ok(value) = row.try_get::<String, _>(index) {
        if value.chars().count() > MAX_CELL_CHARS {
            let cut: String = value.chars().take(MAX_CELL_CHARS).collect();
            return json!(format!("{cut} …[truncated]"));
        }
        return json!(value);
    }
    if let Ok(value) = row.try_get::<bool, _>(index) {
        return json!(value);
    }
    if let Ok(value) = row.try_get::<Vec<u8>, _>(index) {
        return json!(format!("<{} bytes binary>", value.len()));
    }
    json!("<undecodable>")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_single_read_statements() {
        assert!(validate("SELECT * FROM invoices").is_ok());
        assert!(validate("  with x as (select 1) select * from x ; ").is_ok());
        assert!(validate("EXPLAIN SELECT 1").is_ok());

        assert!(validate("").is_err());
        assert!(validate("DELETE FROM invoices").is_err());
        assert!(validate("UPDATE invoices SET id = 1").is_err());
        assert!(validate("INSERT INTO x VALUES (1)").is_err());
        assert!(validate("PRAGMA journal_mode = DELETE").is_err());
        assert!(validate("ATTACH DATABASE 'x' AS y").is_err());
        assert!(validate("SELECT 1; DROP TABLE invoices").is_err());
        assert!(validate("-- comment\nDROP TABLE invoices").is_err());
    }
}
