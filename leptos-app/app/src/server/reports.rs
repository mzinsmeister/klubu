//! File-based reports.
//!
//! A report is a single `.report` file under `templates/reports/`. It has a tiny
//! key/value header followed by named verbatim blocks, so the SQL and the Typst
//! template keep their natural formatting — no nesting, no escaping:
//!
//! ```text
//! title: Anlage EÜR
//! param year: int = 2025
//!
//! --- description ---
//! Einnahmenüberschussrechnung nach § 4 Abs. 3 EStG.
//!
//! --- query dialect=postgres ---
//! SELECT ... WHERE substring(d FROM 1 FOR 4) = $1::text
//!
//! --- query dialect=sqlite ---
//! SELECT ... WHERE substr(d, 1, 4) = CAST(?1 AS TEXT)
//!
//! --- template ---
//! = Anlage EÜR #params.year
//! #table( .. )
//! ```
//!
//! Header lines are `key: value` (`title`), or
//! `param <name> ["<label>"]: <kind> [options=<opts>] [= <default>]`
//! where kind is `int` | `date` | `text`. A block starts with a line
//! `--- <name> ---` and runs verbatim to the next such line. Recognised blocks:
//! `description`, `template`, one or more query blocks, and `options` blocks.
//!
//! ## Parameters
//!
//! Both a default and a dropdown may come from SQL rather than being written out:
//!
//! ```text
//! param year "Jahr": int options=query(jahre) = query(vorjahr)
//! param art "Belegart": text options=[Einnahme, Ausgabe] = Ausgabe
//!
//! --- options name=vorjahr dialect=postgres ---
//! SELECT to_char(now() - interval '1 year', 'YYYY');
//!
//! --- options name=jahre dialect=postgres ---
//! SELECT DISTINCT substring(payment_date FROM 1 FOR 4) FROM invoice_payment ORDER BY 1 DESC;
//! ```
//!
//! An `options` block is a lookup: it runs *before* the parameters exist, so it
//! takes no bind values, and its rows never reach the template. Its first column
//! is the value bound into the query; a second column, if present, is the label
//! shown to the user. `= query(name)` takes that block's first row — so a list
//! sorted newest-first defaults to the newest entry.
//!
//! `options=` makes the field a dropdown and a closed set: a value outside the
//! list is rejected before binding. Note the consequence — a `query(…)` default
//! must be *in* its own dropdown, or the report cannot run. Where the default is
//! computed rather than observed (e.g. "last year", which may have no rows yet),
//! the options query has to include it explicitly.
//!
//! ## Queries
//!
//! A report may declare several named queries; each result set reaches the
//! template as `data.<name>`. A query block takes optional `name` and `dialect`
//! attributes (`dialect` is `postgres` or `sqlite`):
//!
//! ```text
//! --- query ---                          # the query "main"       -> data.main
//! --- query dialect=postgres ---         # main, PostgreSQL variant
//! --- query name=income ---              # query "income"          -> data.income
//! --- query name=income dialect=sqlite --- # SQLite variant of "income"
//! ```
//!
//! Omitting `dialect` makes a query apply to every backend; a dialect-specific
//! block wins over it when present. Every query is bound the same parameters in
//! the same order. `#` starts a comment in the header only — never inside a
//! block, where `#` is Typst.
//!
//! Nothing about any particular report is compiled in. Adding a report — or a
//! new tax year of an existing one — is dropping in one file, no rebuild.
//!
//! The engine does three things and knows nothing else:
//!  1. runs the query for the active dialect, **read-only**, with bound params;
//!  2. hands the resulting rows to the report's template as `data`;
//!  3. compiles that to HTML (for the on-screen view) or PDF (for download).
//!
//! ## Read-only
//!
//! Report SQL never runs with write capability. On PostgreSQL each query runs
//! inside a `READ ONLY` transaction that is always rolled back, with a statement
//! timeout; on SQLite the connection is put in `PRAGMA query_only` for the
//! duration. A report that tries to `INSERT`/`UPDATE`/`DELETE`/`DROP` fails at
//! the database, not on trust. Parameters are always bound, never interpolated.

use leptos::*;

use shared::{ReportDownload, ReportInfo, ReportRender};

// ---------------------------------------------------------------------------
// Discovery & metadata (server-only)
// ---------------------------------------------------------------------------

/// The conventional name of the single unnamed query, and the key it appears
/// under in the template's `data`.
#[cfg(feature = "ssr")]
const MAIN_QUERY: &str = "main";

#[cfg(feature = "ssr")]
#[derive(Debug, Clone, Default)]
struct ReportManifest {
    title: String,
    description: Option<String>,
    params: Vec<ParamManifest>,
    /// One entry per named query. The template sees each result set under
    /// `data.<name>`; a report with a single unnamed query gets `data.main`.
    queries: std::collections::BTreeMap<String, QuerySpec>,
    /// Lookup queries that feed parameter defaults and dropdowns. They run
    /// *before* the parameters exist, so unlike `queries` they take no bind
    /// values, and their rows never reach the template.
    options: std::collections::BTreeMap<String, QuerySpec>,
    template: String,
}

/// A single query in each SQL dialect. Report SQL is allowed to be
/// dialect-specific (`substr` vs `substring`, `?1` vs `$1`, …); `default` is a
/// fallback for queries that happen to be portable.
#[cfg(feature = "ssr")]
#[derive(Debug, Clone, Default)]
struct QuerySpec {
    postgres: Option<String>,
    sqlite: Option<String>,
    default: Option<String>,
}

#[cfg(feature = "ssr")]
impl QuerySpec {
    fn for_dialect(&self, dialect: &str) -> Option<&str> {
        let specific = match dialect {
            "postgres" => self.postgres.as_deref(),
            "sqlite" => self.sqlite.as_deref(),
            _ => None,
        };
        specific.or(self.default.as_deref())
    }
}

/// Where a parameter's default comes from: written literally, or looked up.
#[cfg(feature = "ssr")]
#[derive(Debug, Clone, PartialEq)]
enum ValueSpec {
    Literal(String),
    /// Name of an `--- options ---` block; the first row's first column wins.
    Query(String),
}

/// Where a parameter's dropdown choices come from, if it has any.
#[cfg(feature = "ssr")]
#[derive(Debug, Clone, Default, PartialEq)]
enum OptionsSpec {
    /// A free-text/number/date input.
    #[default]
    Free,
    Fixed(Vec<String>),
    /// Name of an `--- options ---` block. First column is the bound value; a
    /// second column, if present, is the label shown to the user.
    Query(String),
}

#[cfg(feature = "ssr")]
#[derive(Debug, Clone)]
struct ParamManifest {
    name: String,
    label: String,
    /// "int" | "date" | "text"
    kind: String,
    default: Option<ValueSpec>,
    options: OptionsSpec,
}

/// The kinds of block a `.report` file can contain.
#[cfg(feature = "ssr")]
enum BlockKind {
    Description,
    Template,
    Query,
    Options,
}

/// A parsed block header, e.g. `--- query name=income dialect=postgres ---`.
#[cfg(feature = "ssr")]
struct BlockHeader {
    kind: BlockKind,
    /// Query name; irrelevant for description/template.
    name: String,
    /// `Some` restricts a query to one dialect; `None` applies to all.
    dialect: Option<String>,
}

/// Recognises and parses a block header line. Returns `None` for ordinary
/// content lines, so a stray `---` inside SQL or Typst is left untouched.
#[cfg(feature = "ssr")]
fn parse_block_header(line: &str, origin: &str) -> Option<Result<BlockHeader, String>> {
    let inner = line.trim().strip_prefix("---")?.strip_suffix("---")?.trim();
    let mut tokens = inner.split_whitespace();
    let kind = match tokens.next()? {
        "description" => BlockKind::Description,
        "template" => BlockKind::Template,
        "query" => BlockKind::Query,
        "options" => BlockKind::Options,
        // Not a block keyword — treat the line as content.
        _ => return None,
    };

    let mut name = MAIN_QUERY.to_string();
    let mut dialect = None;
    for tok in tokens {
        let Some((key, value)) = tok.split_once('=') else {
            return Some(Err(format!("{origin}: block attribute '{tok}' must be key=value")));
        };
        match (&kind, key) {
            (BlockKind::Query | BlockKind::Options, "name") => name = value.to_string(),
            (BlockKind::Query | BlockKind::Options, "dialect") => {
                if !matches!(value, "postgres" | "sqlite") {
                    return Some(Err(format!("{origin}: unknown dialect '{value}'")));
                }
                dialect = Some(value.to_string());
            }
            _ => return Some(Err(format!("{origin}: '{key}' is not valid on this block"))),
        }
    }
    // An options block is always referenced by name; defaulting it to `main`
    // would silently collide with the report's own query.
    if matches!(kind, BlockKind::Options) && name == MAIN_QUERY {
        return Some(Err(format!("{origin}: an options block needs an explicit name=…")));
    }
    Some(Ok(BlockHeader { kind, name, dialect }))
}

/// Parses the `.report` format described in the module docs.
///
/// `origin` names the file for error messages.
#[cfg(feature = "ssr")]
fn parse_report(text: &str, origin: &str) -> Result<ReportManifest, String> {
    let mut manifest = ReportManifest::default();
    let mut header_lines: Vec<&str> = Vec::new();
    let mut current: Option<BlockHeader> = None;
    let mut buf = String::new();

    // Assigns a finished block's content to the manifest, rejecting duplicates.
    let mut commit = |header: BlockHeader, content: String| -> Result<(), String> {
        match header.kind {
            BlockKind::Description => {
                if manifest.description.is_some() {
                    return Err(format!("{origin}: duplicate description block"));
                }
                manifest.description = Some(content).filter(|s| !s.trim().is_empty());
            }
            BlockKind::Template => {
                if !manifest.template.is_empty() {
                    return Err(format!("{origin}: duplicate template block"));
                }
                manifest.template = content;
            }
            BlockKind::Query | BlockKind::Options => {
                let (bucket, what) = match header.kind {
                    BlockKind::Options => (&mut manifest.options, "options"),
                    _ => (&mut manifest.queries, "query"),
                };
                let spec = bucket.entry(header.name.clone()).or_default();
                let slot = match header.dialect.as_deref() {
                    Some("postgres") => &mut spec.postgres,
                    Some("sqlite") => &mut spec.sqlite,
                    _ => &mut spec.default,
                };
                if slot.is_some() {
                    return Err(format!(
                        "{origin}: duplicate {what} '{}'{}",
                        header.name,
                        header.dialect.as_deref().map(|d| format!(" ({d})")).unwrap_or_default()
                    ));
                }
                *slot = Some(content);
            }
        }
        Ok(())
    };

    for line in text.lines() {
        if let Some(parsed) = parse_block_header(line, origin) {
            let header = parsed?;
            if let Some(prev) = current.take() {
                commit(prev, buf.trim_matches('\n').to_string())?;
                buf.clear();
            }
            current = Some(header);
        } else if current.is_some() {
            buf.push_str(line);
            buf.push('\n');
        } else {
            header_lines.push(line);
        }
    }
    if let Some(prev) = current.take() {
        commit(prev, buf.trim_matches('\n').to_string())?;
    }

    for raw in header_lines {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix("param ") {
            manifest.params.push(parse_param(rest, origin)?);
        } else if let Some((key, value)) = line.split_once(':') {
            match key.trim() {
                "title" => manifest.title = value.trim().to_string(),
                other => return Err(format!("{origin}: unknown header key '{other}'")),
            }
        } else {
            return Err(format!("{origin}: cannot parse header line '{line}'"));
        }
    }

    if manifest.title.is_empty() {
        return Err(format!("{origin}: missing 'title:' in header"));
    }
    if manifest.template.is_empty() {
        return Err(format!("{origin}: missing --- template --- block"));
    }
    if manifest.queries.is_empty() {
        return Err(format!("{origin}: no --- query --- block"));
    }

    // A typo in `query(…)` should fail when the report is loaded, not silently
    // yield an empty dropdown at the moment someone tries to run it.
    for p in &manifest.params {
        let referenced = [
            match &p.default {
                Some(ValueSpec::Query(q)) => Some(q),
                _ => None,
            },
            match &p.options {
                OptionsSpec::Query(q) => Some(q),
                _ => None,
            },
        ];
        for q in referenced.into_iter().flatten() {
            if !manifest.options.contains_key(q) {
                return Err(format!(
                    "{origin}: param '{}' refers to query({q}), but there is no \
                     '--- options name={q} ---' block",
                    p.name
                ));
            }
        }
    }
    Ok(manifest)
}

/// Parses `value` or `query(name)` on the right of a `=`.
#[cfg(feature = "ssr")]
fn parse_value_spec(raw: &str) -> ValueSpec {
    match raw
        .strip_prefix("query(")
        .and_then(|r| r.strip_suffix(')'))
    {
        Some(q) => ValueSpec::Query(q.trim().to_string()),
        None => ValueSpec::Literal(raw.to_string()),
    }
}

/// Parses one `param` header line:
///
/// ```text
/// param year: int = 2025
/// param year "Veranlagungsjahr": int = 2025
/// param year "Veranlagungsjahr": int options=query(jahre) = query(jahre)
/// param art "Belegart": text options=[Einnahme, Ausgabe] = Einnahme
/// ```
///
/// The optional quoted label is what the UI shows; without it the name is used.
/// `options=` makes the field a dropdown, either from a fixed list or from an
/// `--- options name=… ---` block. A default may likewise be looked up with
/// `query(name)`, which takes that block's first row and first column — so a
/// dropdown of years sorted newest-first defaults to the newest year.
#[cfg(feature = "ssr")]
fn parse_param(rest: &str, origin: &str) -> Result<ParamManifest, String> {
    // The label is quoted and may itself contain a colon, so find the separator
    // outside quotes rather than taking the first one.
    let sep = {
        let mut in_quotes = false;
        rest.char_indices()
            .find(|&(_, c)| {
                if c == '"' {
                    in_quotes = !in_quotes;
                }
                c == ':' && !in_quotes
            })
            .map(|(i, _)| i)
            .ok_or_else(|| format!("{origin}: param needs 'name: kind' (got '{}')", rest.trim()))?
    };
    let name_part = rest[..sep].trim();
    let mut tail = rest[sep + 1..].trim();

    let (name, label) = if let Some(q1) = name_part.find('"') {
        let after = &name_part[q1 + 1..];
        let q2 = after
            .find('"')
            .ok_or_else(|| format!("{origin}: param label is missing its closing quote"))?;
        (name_part[..q1].trim().to_string(), after[..q2].to_string())
    } else {
        (name_part.to_string(), name_part.to_string())
    };
    if name.is_empty() {
        return Err(format!("{origin}: param has empty name"));
    }

    let kind_end = tail.find(char::is_whitespace).unwrap_or(tail.len());
    let kind = tail[..kind_end].trim().to_string();
    tail = tail[kind_end..].trim();
    if !matches!(kind.as_str(), "int" | "date" | "text") {
        return Err(format!("{origin}: param '{name}' has unknown kind '{kind}'"));
    }

    let mut options = OptionsSpec::Free;
    if let Some(after) = tail.strip_prefix("options=") {
        let (spec, remainder) = if let Some(inner) = after.strip_prefix('[') {
            let close = inner
                .find(']')
                .ok_or_else(|| format!("{origin}: param '{name}': options list is missing its ']'"))?;
            let values: Vec<String> = inner[..close]
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if values.is_empty() {
                return Err(format!("{origin}: param '{name}': empty options list"));
            }
            (OptionsSpec::Fixed(values), &inner[close + 1..])
        } else {
            let end = after.find(char::is_whitespace).unwrap_or(after.len());
            let token = &after[..end];
            match parse_value_spec(token) {
                ValueSpec::Query(q) => (OptionsSpec::Query(q), &after[end..]),
                ValueSpec::Literal(_) => {
                    return Err(format!(
                        "{origin}: param '{name}': options must be '[a, b]' or 'query(name)'"
                    ))
                }
            }
        };
        options = spec;
        tail = remainder.trim();
    }

    let default = match tail.strip_prefix('=') {
        Some(raw) => Some(parse_value_spec(raw.trim())),
        None if tail.is_empty() => None,
        None => {
            return Err(format!(
                "{origin}: param '{name}': cannot parse '{tail}' (expected 'options=…' then '= default')"
            ))
        }
    };

    Ok(ParamManifest {
        name,
        label,
        kind,
        default,
        options,
    })
}

#[cfg(feature = "ssr")]
fn reports_dir() -> std::path::PathBuf {
    let dir = std::env::var("KLUBU_EXPORT_TEMPLATES_PATH")
        .unwrap_or_else(|_| "./templates".to_string());
    std::path::Path::new(&dir).join("reports")
}

/// Report directory names are used to build file paths, so keep them to a safe
/// charset. This is what stops `../../etc` from being a report name.
#[cfg(feature = "ssr")]
fn is_safe_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

#[cfg(feature = "ssr")]
fn load_manifest(name: &str) -> Result<ReportManifest, ServerFnError> {
    if !is_safe_name(name) {
        return Err(ServerFnError::new(format!("Invalid report name: {name}")));
    }
    let path = reports_dir().join(format!("{name}.report"));
    let content = std::fs::read_to_string(&path)
        .map_err(|e| ServerFnError::new(format!("Report '{name}' not found: {e}")))?;
    parse_report(&content, name).map_err(ServerFnError::new)
}

#[cfg(feature = "ssr")]
fn manifest_to_info(name: String, m: ReportManifest, params: Vec<shared::ReportParamInfo>) -> ReportInfo {
    ReportInfo {
        name,
        title: m.title,
        description: m.description,
        params,
    }
}

// ---------------------------------------------------------------------------
// Query execution (server-only)
// ---------------------------------------------------------------------------

/// A parameter value bound into a report query. String-typed on the wire; parsed
/// into one of these before binding so the database sees a real int/date, not
/// text it has to coerce.
#[cfg(feature = "ssr")]
enum Bound {
    Int(i64),
    Date(chrono::NaiveDate),
    Text(String),
}

#[cfg(feature = "ssr")]
fn bind_values(
    params: &[shared::ReportParamInfo],
    supplied: &[(String, String)],
) -> Result<Vec<Bound>, ServerFnError> {
    params
        .iter()
        .map(|p| {
            let raw = supplied
                .iter()
                .find(|(k, _)| k == &p.name)
                .map(|(_, v)| v.as_str())
                .or(p.default.as_deref())
                .ok_or_else(|| {
                    ServerFnError::new(format!("Missing value for parameter '{}'", p.name))
                })?;
            // A dropdown is a closed set. Values are bound, never interpolated,
            // so this is not an injection guard — it stops a stale or hand-made
            // request from quietly reporting on something that is not offered.
            if !p.options.is_empty() && !p.options.iter().any(|o| o.value == raw) {
                return Err(ServerFnError::new(format!(
                    "Parameter '{}' must be one of the offered options",
                    p.name
                )));
            }
            match p.kind.as_str() {
                "int" => raw
                    .trim()
                    .parse::<i64>()
                    .map(Bound::Int)
                    .map_err(|_| ServerFnError::new(format!("Parameter '{}' must be an integer", p.name))),
                "date" => chrono::NaiveDate::parse_from_str(raw.trim(), "%Y-%m-%d")
                    .map(Bound::Date)
                    .map_err(|_| ServerFnError::new(format!("Parameter '{}' must be a date (YYYY-MM-DD)", p.name))),
                _ => Ok(Bound::Text(raw.to_string())),
            }
        })
        .collect()
}

/// Decodes one column of a result row into JSON, trying the handful of types the
/// schema actually uses. Money and counts are integers, quantities are floats,
/// everything else (including dates, which are stored as text) is a string.
#[cfg(feature = "ssr")]
fn value_at(row: &super::db::DbRow, idx: usize) -> serde_json::Value {
    use serde_json::Value;
    use sqlx::Row;

    // Order matters: Postgres distinguishes INT4/INT8, so try the wider integer
    // first and fall back. A NULL of a compatible type decodes as Ok(None).
    if let Ok(v) = row.try_get::<Option<i64>, _>(idx) {
        return v.map_or(Value::Null, |n| Value::from(n));
    }
    if let Ok(v) = row.try_get::<Option<i32>, _>(idx) {
        return v.map_or(Value::Null, |n| Value::from(n));
    }
    if let Ok(v) = row.try_get::<Option<f64>, _>(idx) {
        return v.map_or(Value::Null, |n| Value::from(n));
    }
    if let Ok(v) = row.try_get::<Option<bool>, _>(idx) {
        return v.map_or(Value::Null, Value::from);
    }
    if let Ok(v) = row.try_get::<Option<String>, _>(idx) {
        return v.map_or(Value::Null, Value::from);
    }
    Value::Null
}

#[cfg(feature = "ssr")]
fn rows_to_json(rows: &[super::db::DbRow]) -> serde_json::Value {
    use sqlx::{Column, Row};
    let arr = rows
        .iter()
        .map(|row| {
            let mut obj = serde_json::Map::new();
            for col in row.columns() {
                obj.insert(col.name().to_string(), value_at(row, col.ordinal()));
            }
            serde_json::Value::Object(obj)
        })
        .collect();
    serde_json::Value::Array(arr)
}

/// Runs a report query read-only and returns the decoded rows. The write barrier
/// is enforced by the database (see the module docs), not by inspecting the SQL —
/// which mechanism depends on the dialect the pool is talking to at runtime.
///
/// Dates bind as their ISO string: both dialects store document dates as text,
/// and the `Any` driver has no date type of its own.
#[cfg(feature = "ssr")]
async fn fetch_rows(
    pool: &super::db::DbPool,
    sql: &str,
    bound: &[Bound],
) -> Result<Vec<super::db::DbRow>, ServerFnError> {
    let err = |e: sqlx::Error| ServerFnError::new(format!("Report query failed: {e}"));

    fn bind_all<'q>(
        mut q: sqlx::query::Query<'q, sqlx::Any, sqlx::any::AnyArguments<'q>>,
        bound: &'q [Bound],
    ) -> sqlx::query::Query<'q, sqlx::Any, sqlx::any::AnyArguments<'q>> {
        for b in bound {
            q = match b {
                Bound::Int(i) => q.bind(*i),
                Bound::Date(d) => q.bind(d.format("%Y-%m-%d").to_string()),
                Bound::Text(s) => q.bind(s),
            };
        }
        q
    }

    if super::db::dialect(pool) == "postgres" {
        let mut tx = pool.begin().await.map_err(err)?;
        sqlx::query("SET TRANSACTION READ ONLY").execute(&mut *tx).await.map_err(err)?;
        sqlx::query("SET LOCAL statement_timeout = '15s'").execute(&mut *tx).await.map_err(err)?;

        let rows = bind_all(sqlx::query(sql), bound).fetch_all(&mut *tx).await.map_err(err)?;
        // Report queries only read; rolling back keeps that a hard guarantee even
        // if a function-based side effect slipped through.
        let _ = tx.rollback().await;
        Ok(rows)
    } else {
        // `query_only` is connection-scoped, so pin one connection, arm it, run,
        // and disarm before it returns to the pool.
        let mut conn = pool.acquire().await.map_err(err)?;
        sqlx::query("PRAGMA query_only = ON").execute(&mut *conn).await.map_err(err)?;

        let result = bind_all(sqlx::query(sql), bound).fetch_all(&mut *conn).await;
        let _ = sqlx::query("PRAGMA query_only = OFF").execute(&mut *conn).await;
        result.map_err(err)
    }
}

/// Renders a decoded cell as the plain string a parameter binds/displays.
#[cfg(feature = "ssr")]
fn cell_to_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => String::new(),
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Runs one `--- options ---` block. These take no bind values, because they
/// exist to produce the parameters in the first place.
#[cfg(feature = "ssr")]
async fn fetch_options(
    manifest: &ReportManifest,
    qname: &str,
    pool: &super::db::DbPool,
) -> Result<Vec<shared::ReportParamOption>, ServerFnError> {
    use sqlx::Row;
    let dialect = super::db::dialect(pool);
    let spec = manifest
        .options
        .get(qname)
        .ok_or_else(|| ServerFnError::new(format!("Unknown options query '{qname}'")))?;
    let sql = spec.for_dialect(dialect).ok_or_else(|| {
        ServerFnError::new(format!("Options query '{qname}' has no SQL for dialect '{dialect}'"))
    })?;

    let rows = fetch_rows(pool, sql, &[]).await?;
    Ok(rows
        .iter()
        .map(|row| {
            let value = cell_to_string(&value_at(row, 0));
            // A second column, when present, is the human-readable label.
            let label = if row.columns().len() > 1 {
                cell_to_string(&value_at(row, 1))
            } else {
                value.clone()
            };
            shared::ReportParamOption { value, label }
        })
        .collect())
}

/// Turns the declared parameters into what the form needs: dropdown choices
/// resolved, `query(…)` defaults executed.
///
/// A lookup referenced by both `options=` and `=` runs once.
#[cfg(feature = "ssr")]
async fn resolve_params(
    manifest: &ReportManifest,
    pool: &super::db::DbPool,
) -> Result<Vec<shared::ReportParamInfo>, ServerFnError> {
    let mut cache: std::collections::HashMap<String, Vec<shared::ReportParamOption>> =
        std::collections::HashMap::new();
    let mut out = Vec::with_capacity(manifest.params.len());

    for p in &manifest.params {
        for qname in [
            match &p.options {
                OptionsSpec::Query(q) => Some(q),
                _ => None,
            },
            match &p.default {
                Some(ValueSpec::Query(q)) => Some(q),
                _ => None,
            },
        ]
        .into_iter()
        .flatten()
        {
            if !cache.contains_key(qname) {
                let opts = fetch_options(manifest, qname, pool).await?;
                cache.insert(qname.clone(), opts);
            }
        }

        let options = match &p.options {
            OptionsSpec::Free => Vec::new(),
            OptionsSpec::Fixed(values) => values
                .iter()
                .map(|v| shared::ReportParamOption { value: v.clone(), label: v.clone() })
                .collect(),
            OptionsSpec::Query(q) => cache.get(q).cloned().unwrap_or_default(),
        };

        let default = match &p.default {
            None => None,
            Some(ValueSpec::Literal(s)) => Some(s.clone()),
            // An empty lookup yields no default rather than an error: the report
            // is simply not runnable until there is data, and the form says so.
            Some(ValueSpec::Query(q)) => cache.get(q).and_then(|o| o.first()).map(|o| o.value.clone()),
        };

        out.push(shared::ReportParamInfo {
            name: p.name.clone(),
            label: p.label.clone(),
            kind: p.kind.clone(),
            default,
            options,
        });
    }
    Ok(out)
}

/// Quotes one CSV field per RFC 4180: wrap in quotes when the value contains a
/// separator, a quote, a newline, or edge whitespace; double any inner quote.
#[cfg(feature = "ssr")]
fn csv_field(v: &serde_json::Value) -> String {
    let raw = match v {
        serde_json::Value::Null => String::new(),
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    };
    let needs_quotes = raw.contains([',', '"', '\n', '\r'])
        || raw.starts_with(' ')
        || raw.ends_with(' ');
    if needs_quotes {
        format!("\"{}\"", raw.replace('"', "\"\""))
    } else {
        raw
    }
}

/// Serialises rows as RFC 4180 CSV, preserving the column order of the SQL —
/// unlike the JSON handed to Typst, whose object keys sort alphabetically.
///
/// Prefixed with a UTF-8 BOM: the audience is an auditor opening the file in
/// Excel, which otherwise mangles the umlauts in names and journal entries.
/// Returns an empty string for an empty result set — with no row there is no
/// column list to write a header from.
#[cfg(feature = "ssr")]
fn rows_to_csv(rows: &[super::db::DbRow]) -> String {
    use sqlx::{Column, Row};
    let Some(first) = rows.first() else {
        return String::new();
    };

    let mut out = String::from("\u{feff}");
    let columns = first.columns();
    let header: Vec<String> = columns
        .iter()
        .map(|c| csv_field(&serde_json::Value::from(c.name())))
        .collect();
    out.push_str(&header.join(","));
    out.push_str("\r\n");

    for row in rows {
        let fields: Vec<String> = (0..row.columns().len())
            .map(|i| csv_field(&value_at(row, i)))
            .collect();
        out.push_str(&fields.join(","));
        out.push_str("\r\n");
    }
    out
}

/// Runs a report and returns one query's rows as CSV.
///
/// A report with several named queries has no single tabular shape, so the one
/// named `main` is exported; that is also the name a single unnamed query gets.
#[cfg(feature = "ssr")]
pub async fn render_csv(
    name: &str,
    supplied: &[(String, String)],
    pool: &super::db::DbPool,
) -> Result<String, ServerFnError> {
    let manifest = load_manifest(name)?;
    let resolved = resolve_params(&manifest, pool).await?;
    let bound = bind_values(&resolved, supplied)?;
    let dialect = super::db::dialect(pool);

    let spec = manifest.queries.get(MAIN_QUERY).ok_or_else(|| {
        let names: Vec<_> = manifest.queries.keys().cloned().collect();
        ServerFnError::new(format!(
            "Report '{name}' has no '{MAIN_QUERY}' query to export as CSV (has: {})",
            names.join(", ")
        ))
    })?;
    let sql = spec.for_dialect(dialect).ok_or_else(|| {
        ServerFnError::new(format!(
            "Report '{name}': query '{MAIN_QUERY}' has no SQL for dialect '{dialect}'"
        ))
    })?;

    Ok(rows_to_csv(&fetch_rows(pool, sql, &bound).await?))
}

/// Builds the Typst markup: the report's template with `data` (query rows) and
/// `params` (the values it was run with) prepended as bindings.
#[cfg(feature = "ssr")]
fn assemble_markup(
    template: &str,
    data: &serde_json::Value,
    supplied: &[(String, String)],
) -> String {
    let params_json = serde_json::Value::Object(
        supplied
            .iter()
            .map(|(k, v)| (k.clone(), serde_json::Value::from(v.clone())))
            .collect(),
    );

    format!(
        "#let data = {}\n#let params = {}\n{}",
        crate::typst_gen::json_to_typst(data),
        crate::typst_gen::json_to_typst(&params_json),
        template
    )
}

/// Runs a report against `pool` and returns the Typst markup to compile. Split
/// from the server functions so it can be driven with a plain pool in tests.
#[cfg(feature = "ssr")]
pub async fn render_markup(
    name: &str,
    supplied: &[(String, String)],
    pool: &super::db::DbPool,
) -> Result<String, ServerFnError> {
    let manifest = load_manifest(name)?;
    let resolved = resolve_params(&manifest, pool).await?;
    let bound = bind_values(&resolved, supplied)?;
    let dialect = super::db::dialect(pool);

    // Each named query becomes `data.<name>` in the template.
    let mut data = serde_json::Map::new();
    for (qname, spec) in &manifest.queries {
        let sql = spec.for_dialect(dialect).ok_or_else(|| {
            ServerFnError::new(format!(
                "Report '{name}': query '{qname}' has no SQL for dialect '{dialect}'"
            ))
        })?;
        data.insert(qname.clone(), rows_to_json(&fetch_rows(pool, sql, &bound).await?));
    }

    Ok(assemble_markup(
        &manifest.template,
        &serde_json::Value::Object(data),
        &effective_params(&resolved, supplied),
    ))
}

/// The values the report actually ran with, defaults included. The template sees
/// these as `params`, so a heading can print the year even when nobody typed it.
#[cfg(feature = "ssr")]
fn effective_params(
    resolved: &[shared::ReportParamInfo],
    supplied: &[(String, String)],
) -> Vec<(String, String)> {
    resolved
        .iter()
        .filter_map(|p| {
            supplied
                .iter()
                .find(|(k, _)| k == &p.name)
                .map(|(_, v)| v.clone())
                .or_else(|| p.default.clone())
                .map(|v| (p.name.clone(), v))
        })
        .collect()
}

#[cfg(feature = "ssr")]
async fn render(
    name: &str,
    supplied: &[(String, String)],
) -> Result<String, ServerFnError> {
    let repo = use_context::<super::db::ActiveRepository>()
        .ok_or_else(|| ServerFnError::new("Repository not found"))?;
    render_markup(name, supplied, repo.pool()).await
}

// ---------------------------------------------------------------------------
// Server functions
// ---------------------------------------------------------------------------

#[server(name = ListReports, prefix = "/api", endpoint = "list_reports")]
pub async fn list_reports() -> Result<Vec<ReportInfo>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        let mut reports = Vec::new();
        if let Ok(entries) = std::fs::read_dir(reports_dir()) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(true, |e| e != "report") {
                    continue;
                }
                let Some(name) = path.file_stem().and_then(|s| s.to_str()).map(str::to_string)
                else {
                    continue;
                };
                if !is_safe_name(&name) {
                    continue;
                }
                // A report whose lookup query is broken is skipped with a log
                // line, exactly like one that fails to parse — the others still list.
                match load_manifest(&name) {
                    Ok(m) => match resolve_params(&m, repo.pool()).await {
                        Ok(params) => reports.push(manifest_to_info(name, m, params)),
                        Err(e) => logging::log!("Skipping report '{name}': {e:?}"),
                    },
                    Err(e) => logging::log!("Skipping report '{name}': {e}"),
                }
            }
        }
        reports.sort_by(|a, b| a.title.cmp(&b.title));
        Ok(reports)
    }
    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::new("server only"))
    }
}

#[server(name = RunReport, prefix = "/api", endpoint = "run_report")]
pub async fn run_report(
    name: String,
    params: Vec<(String, String)>,
) -> Result<ReportRender, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let res = async {
            let markup = render(&name, &params).await?;
            let html = crate::pdf::compiler::compile_typst_html(markup)
                .map_err(|e| ServerFnError::new(format!("Report render failed: {e}")))?;
            Ok::<_, ServerFnError>(ReportRender { html })
        }
        .await;
        if let Err(ref e) = res {
            logging::log!("run_report({name}): {e:?}");
        }
        res
    }
    #[cfg(not(feature = "ssr"))]
    {
        _ = (name, params);
        Err(ServerFnError::new("server only"))
    }
}

/// The machine-readable counterpart to the PDF: the report's underlying rows,
/// not its layout. A Betriebsprüfer asking for Datenzugriff nach Z3 (GoBD
/// Rz. 165 ff.) wants exactly this — data they can import, sort and re-total.
#[server(name = ExportReportCsv, prefix = "/api", endpoint = "export_report_csv")]
pub async fn export_report_csv(
    name: String,
    params: Vec<(String, String)>,
) -> Result<ReportDownload, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use base64::Engine;
        let res = async {
            let repo = use_context::<super::db::ActiveRepository>()
                .ok_or_else(|| ServerFnError::new("Repository not found"))?;
            let csv = render_csv(&name, &params, repo.pool()).await?;
            Ok::<_, ServerFnError>(ReportDownload {
                filename: format!("{name}.csv"),
                media_type: "text/csv;charset=utf-8".to_string(),
                base64: base64::engine::general_purpose::STANDARD.encode(csv.as_bytes()),
            })
        }
        .await;
        if let Err(ref e) = res {
            logging::log!("export_report_csv({name}): {e:?}");
        }
        res
    }
    #[cfg(not(feature = "ssr"))]
    {
        _ = (name, params);
        Err(ServerFnError::new("server only"))
    }
}

#[server(name = ExportReportPdf, prefix = "/api", endpoint = "export_report_pdf")]
pub async fn export_report_pdf(
    name: String,
    params: Vec<(String, String)>,
) -> Result<ReportDownload, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use base64::Engine;
        let res = async {
            let markup = render(&name, &params).await?;
            let bytes = crate::pdf::compiler::compile_typst(markup)
                .map_err(|e| ServerFnError::new(format!("Report PDF failed: {e}")))?;
            Ok::<_, ServerFnError>(ReportDownload {
                filename: format!("{name}.pdf"),
                media_type: "application/pdf".to_string(),
                base64: base64::engine::general_purpose::STANDARD.encode(&bytes),
            })
        }
        .await;
        if let Err(ref e) = res {
            logging::log!("export_report_pdf({name}): {e:?}");
        }
        res
    }
    #[cfg(not(feature = "ssr"))]
    {
        _ = (name, params);
        Err(ServerFnError::new("server only"))
    }
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
title: Umsatz je Kunde
param year \"Jahr\": int = 2025
param from: date

--- description ---
Wer hat wie viel gezahlt.

--- query dialect=postgres ---
SELECT 1 WHERE $1::text = '2025';

--- template ---
= Report #params.year
";

    #[test]
    fn parses_header_blocks_and_params() {
        let m = parse_report(SAMPLE, "sample").unwrap();
        assert_eq!(m.title, "Umsatz je Kunde");
        assert_eq!(m.description.as_deref(), Some("Wer hat wie viel gezahlt."));
        assert_eq!(m.params.len(), 2);

        let year = &m.params[0];
        assert_eq!(year.name, "year");
        assert_eq!(year.label, "Jahr"); // quoted label wins over the name
        assert_eq!(year.kind, "int");
        assert_eq!(year.default, Some(ValueSpec::Literal("2025".into())));
        assert_eq!(year.options, OptionsSpec::Free);

        let from = &m.params[1];
        assert_eq!(from.name, "from");
        assert_eq!(from.label, "from"); // no label -> name
        assert_eq!(from.kind, "date");
        assert_eq!(from.default, None);

        let main = m.queries.get(MAIN_QUERY).expect("main query");
        assert!(main.postgres.as_deref().unwrap().contains("SELECT 1"));
        assert_eq!(main.sqlite, None);
        assert!(m.template.starts_with("= Report"));
    }

    #[test]
    fn multiple_named_queries_are_collected() {
        let src = "\
title: T
--- query name=income dialect=postgres ---
SELECT 1;
--- query name=income dialect=sqlite ---
SELECT 2;
--- query name=expenses ---
SELECT 3;
--- template ---
#data.income #data.expenses
";
        let m = parse_report(src, "t").unwrap();
        let mut names: Vec<&String> = m.queries.keys().collect();
        names.sort();
        assert_eq!(names, ["expenses", "income"]);
        assert_eq!(m.queries["income"].for_dialect("postgres"), Some("SELECT 1;"));
        assert_eq!(m.queries["income"].for_dialect("sqlite"), Some("SELECT 2;"));
        // "expenses" is portable: same SQL for both dialects.
        assert_eq!(m.queries["expenses"].for_dialect("postgres"), Some("SELECT 3;"));
        assert_eq!(m.queries["expenses"].for_dialect("sqlite"), Some("SELECT 3;"));
    }

    #[test]
    fn unknown_dialect_is_an_error() {
        let src = "title: T\n--- query dialect=mysql ---\nSELECT 1;\n--- template ---\nx\n";
        let err = parse_report(src, "t").unwrap_err();
        assert!(err.contains("unknown dialect"), "{err}");
    }

    #[test]
    fn duplicate_query_dialect_is_an_error() {
        let src = "title: T\n--- query dialect=postgres ---\nA\n--- query dialect=postgres ---\nB\n--- template ---\nx\n";
        assert!(parse_report(src, "t").is_err());
    }

    #[test]
    fn blocks_are_verbatim_including_hashes() {
        // `#` inside a block is Typst, never a comment: it must survive intact.
        let src = "title: T\n--- query ---\nSELECT 1;\n--- template ---\n#let x = 1\n// note\n";
        let m = parse_report(src, "t").unwrap();
        assert_eq!(m.template, "#let x = 1\n// note");
    }

    #[test]
    fn dialect_selection_prefers_specific_then_default() {
        let q = QuerySpec {
            postgres: Some("pg".into()),
            sqlite: None,
            default: Some("any".into()),
        };
        assert_eq!(q.for_dialect("postgres"), Some("pg"));
        assert_eq!(q.for_dialect("sqlite"), Some("any")); // falls back to default
    }

    #[test]
    fn missing_template_is_an_error() {
        assert!(parse_report("title: T\n", "t").is_err());
    }

    #[test]
    fn unknown_header_key_is_an_error() {
        let err = parse_report("bogus: x\n--- template ---\nhi\n", "t").unwrap_err();
        assert!(err.contains("unknown header key"), "{err}");
    }

    #[test]
    fn unknown_param_kind_is_an_error() {
        let err = parse_report("param x: money\n--- template ---\nhi\n", "t").unwrap_err();
        assert!(err.contains("unknown kind"), "{err}");
    }

    #[test]
    fn duplicate_block_is_an_error() {
        let src = "title: T\n--- template ---\na\n--- template ---\nb\n";
        assert!(parse_report(src, "t").is_err());
    }

    const WITH_OPTIONS: &str = "\
title: T
param year \"Jahr\": int options=query(jahre) = query(vorjahr)
param art \"Belegart\": text options=[Einnahme, Ausgabe] = Ausgabe
param frei: text

--- options name=jahre ---
SELECT '2026' UNION SELECT '2025';

--- options name=vorjahr ---
SELECT '2025';

--- query ---
SELECT 1;

--- template ---
x
";

    #[test]
    fn parses_dropdowns_and_looked_up_defaults() {
        let m = parse_report(WITH_OPTIONS, "t").unwrap();
        assert_eq!(m.options.len(), 2);

        let year = &m.params[0];
        assert_eq!(year.options, OptionsSpec::Query("jahre".into()));
        assert_eq!(year.default, Some(ValueSpec::Query("vorjahr".into())));

        let art = &m.params[1];
        assert_eq!(
            art.options,
            OptionsSpec::Fixed(vec!["Einnahme".into(), "Ausgabe".into()])
        );
        assert_eq!(art.default, Some(ValueSpec::Literal("Ausgabe".into())));

        // A plain param keeps working: no options, no default.
        let frei = &m.params[2];
        assert_eq!(frei.options, OptionsSpec::Free);
        assert_eq!(frei.default, None);
    }

    /// The lookup runs before the parameters exist, so it cannot be one of the
    /// report's own queries; a name that resolves to neither is a load error.
    #[test]
    fn a_param_referring_to_a_missing_options_block_is_an_error() {
        let src = "title: T\nparam y: int = query(nope)\n--- query ---\nSELECT 1;\n--- template ---\nx\n";
        let err = parse_report(src, "t").unwrap_err();
        assert!(err.contains("options name=nope"), "{err}");
    }

    #[test]
    fn an_options_block_needs_a_name() {
        let src = "title: T\n--- options ---\nSELECT 1;\n--- query ---\nSELECT 1;\n--- template ---\nx\n";
        let err = parse_report(src, "t").unwrap_err();
        assert!(err.contains("explicit name"), "{err}");
    }

    /// A colon inside the quoted label must not be mistaken for the `: kind`
    /// separator.
    #[test]
    fn a_label_may_contain_a_colon() {
        let src = "title: T\nparam y \"Jahr: welches?\": int = 2025\n--- query ---\nSELECT 1;\n--- template ---\nx\n";
        let m = parse_report(src, "t").unwrap();
        assert_eq!(m.params[0].label, "Jahr: welches?");
        assert_eq!(m.params[0].kind, "int");
    }

    #[test]
    fn malformed_options_are_an_error() {
        for src in [
            "title: T\nparam y: int options=[2025\n--- query ---\nSELECT 1;\n--- template ---\nx\n",
            "title: T\nparam y: int options=2025\n--- query ---\nSELECT 1;\n--- template ---\nx\n",
            "title: T\nparam y: int options=[]\n--- query ---\nSELECT 1;\n--- template ---\nx\n",
        ] {
            assert!(parse_report(src, "t").is_err(), "should reject: {src}");
        }
    }

    #[test]
    fn csv_quotes_only_what_rfc4180_requires() {
        use serde_json::Value;
        assert_eq!(csv_field(&Value::from("plain")), "plain");
        assert_eq!(csv_field(&Value::from("a,b")), "\"a,b\"");
        assert_eq!(csv_field(&Value::from("say \"hi\"")), "\"say \"\"hi\"\"\"");
        assert_eq!(csv_field(&Value::from("two\nlines")), "\"two\nlines\"");
        assert_eq!(csv_field(&Value::from(" padded ")), "\" padded \"");
        assert_eq!(csv_field(&Value::Null), "");
        assert_eq!(csv_field(&Value::from(42)), "42");
    }
}
