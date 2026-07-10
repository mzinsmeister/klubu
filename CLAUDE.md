# Klubu

The Rust/Leptos workspace at the repository root is the only application in
this repository. It contains the shared models, Leptos UI, Axum server, database
migrations, Typst templates, and MCP server.

## Database rules

The server compiles both database drivers and selects SQLite or PostgreSQL at
runtime from the `DATABASE_URL` scheme. There are no `sqlx::query!` macros — SQL
is checked when it runs.

On every schema change:

1. Add the migration to both `backend/migrations-postgres/` and
   `backend/migrations-sqlite/` with the same filename.
2. Exercise the affected flow once against SQLite and once against PostgreSQL.
3. Test the `migrate-db` SQLite → PostgreSQL → SQLite roundtrip on scratch
   databases and verify that the dumps remain byte-identical.

Never point `migrate-db` at a non-empty target; it refuses when `audit_log` has
rows.

## Language boundary

Rust identifiers, module and file names, shared types, server functions, SQL
identifiers, migration names, and internal errors use English. German is used
for frontend labels, help text, and generated business documents. The internal
name for the German “Auftrag” feature is `engagement`.

## Data integrity

- SQLite runs in WAL mode with `synchronous = FULL`; shutdown must go through
  `dbcopy.rs::shutdown_pool`.
- Committed documents are immutable. Corrections use storno or new revisions,
  and every write appends to `audit_log` in the same transaction.
- Contacts are archived, never hard-deleted: their ids remain resolvable for
  committed invoices.
