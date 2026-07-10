# Klubu

Two apps live here: the legacy Kotlin/Spring + Vue app (`backend/`, `frontend/`) and its
replacement, a Rust/Leptos full-stack app in `leptos-app/` (workspace: `app`, `backend`,
`frontend`, `shared`). Active development happens in `leptos-app/`.

## Database rules (leptos-app)

The server compiles both database drivers and picks SQLite or Postgres at runtime from
the `DATABASE_URL` scheme (sqlx `Any` driver). There are **no** `sqlx::query!` macros —
all SQL is runtime-checked only, so type or column mistakes surface when the query
*runs*, not at compile time.

**On every schema change:**

1. Add the migration to **both** directories with the **same file name**:
   `leptos-app/backend/migrations-postgres/` (Postgres) and
   `leptos-app/backend/migrations-sqlite/` (SQLite). `migrate-db` refuses to run when
   the version lists differ.
2. **Test against both DBMSes**, not just one: start the server and exercise the
   affected flows once with a SQLite URL and once with Postgres
   (`postgres://klubu:klubu-test@localhost:5433/klubu` in dev). The dialects decode
   differently (e.g. Postgres `information_schema` identifiers, INT4 vs INT8), and
   runtime-checked SQL means only execution finds the mismatch.
3. **Test the migration tool**: run a roundtrip
   `migrate-db` SQLite → Postgres → SQLite on scratch databases and diff the dumps —
   it must be byte-identical. The copier (`leptos-app/backend/src/dbcopy.rs`) is
   catalog-driven and picks up new tables/columns automatically, but a column with a
   *new declared type* needs a mapping in `kind_of()` (it fails loudly, not silently).

Never point `migrate-db` at a non-empty target; it refuses when `audit_log` has rows.

## Language boundary

- Rust identifiers, module/file names, shared types, server function names, SQL
  identifiers, migration names, and internal error/log messages use English.
- German is allowed in frontend labels, help text, and generated business
  document copy. The internal name for the frontend's German “Auftrag” feature
  is `engagement`.

- SQLite runs in WAL mode with `synchronous = FULL`; teardown checkpoints and truncates
  the WAL (see `dbcopy.rs::shutdown_pool`). Keep any new exit path going through it.
- Committed (festgeschriebene) documents are immutable; corrections go through storno /
  new revisions, and every write appends to the `audit_log` in the same transaction.
  Never add a code path that mutates or deletes committed rows.
- Contacts are archived (`archived_timestamp`), never hard-deleted: the contact id is
  the Kundennummer printed on committed invoices and must stay resolvable.
