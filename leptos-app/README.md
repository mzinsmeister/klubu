# KlubU (Leptos Version)

A modern, fast, and lightweight port of the **Klubu** invoicing tool, written entirely in Rust using the **Leptos** web framework.

Compared to the Kotlin/Spring Boot version, this version has a significantly lighter footprint:
- **No Headless Chromium/Selenium dependency**: PDFs are compiled directly in-memory from Typst templates using the `typst` and `typst-pdf` crates.
- **Fast and lightweight**: Single-binary deployment for the backend, and a compiled WebAssembly client.

---

## Project Structure

This is a Cargo workspace consisting of four crates:

- **[`shared`](./shared)**: Contains shared data models (e.g. `Invoice`, `Offer`, `Contact`, `Receipt`, `Payment`) and helpers used by both client and server.
- **[`app`](./app)**: The core UI logic and router built with Leptos. It defines the components and routing for all views (Dashboard, Contacts, Invoices, Offers, Receipts) and contains Typst templates for PDFs.
- **[`frontend`](./frontend)**: The client-side entry point that compiles to WebAssembly using Trunk.
- **[`backend`](./backend)**: The server-side application built with Axum and `leptos_axum`. It serves the compiled WebAssembly frontend, runs database migrations, handles server functions (endpoints queryable directly from the frontend), and serves PDF downloads.

---

## Features

- **Dashboard**: Real figures for the current business year — revenue, expenses, net result (Einnahmenüberschussrechnung), open and draft invoices.
- **Contacts Management**: Create, edit, and delete client or supplier contacts.
- **Invoices**: Create invoices, record payments, finalize (commit) them, and export to PDF.
- **Offers**: Manage offers with revisions and export to PDF.
- **Receipts**: Bookkeep receipts and categorize items (e.g., Miete, Bürobedarf) for tax reports.
- **Local-AI receipt prefill** (optional, off by default): read a receipt PDF with a small language model running on your own server, and prefill supplier, date, number and positions. See [Local AI receipt prefill](#local-ai-receipt-prefill).
- **Typst-Based PDF Rendering**: Beautiful, pixel-perfect PDF rendering compiled directly in-memory from Typst templates (no PDFBox or Apache FOP needed).

Amount fields accept German and plain notation interchangeably (`3,4`, `4.5`, `1.234,56`, `12 €`) and normalise to `1.234,56` when the field loses focus. Amounts are held as integer cents throughout, so no rounding drift creeps in.

---

## Local AI receipt prefill

Uploading a receipt PDF on the **Belege** page can prefill the form: supplier, receipt number, date and the individual positions with a category guess.

Everything runs on the machine that runs the server — the receipt never leaves it. The text layer of the PDF is extracted in-process, and only that text is sent to a local [Ollama](https://ollama.com) instance.

**The feature is off by default.** While it is off, the button is not rendered, nothing contacts a model, and **no model needs to exist on disk**.

To switch it on:

```bash
ollama pull qwen2.5:3b                       # ~1.9 GB
KLUBU_AI_ENABLED=true cargo run --package backend
```

### Configuration

Each setting can come from an environment variable or from `config/application.properties` (the env var wins). These are the defaults:

| Property | Environment variable | Default | Meaning |
| --- | --- | --- | --- |
| `klubu.ai.enabled` | `KLUBU_AI_ENABLED` | `false` | Master switch. Accepts `true`/`1`/`yes`/`on`. |
| `klubu.ai.model` | `KLUBU_AI_MODEL` | `qwen2.5:3b` | Ollama model tag. |
| `klubu.ai.url` | `KLUBU_AI_URL` | `http://localhost:11434` | Ollama base URL. |
| `klubu.ai.timeoutSeconds` | `KLUBU_AI_TIMEOUT_SECONDS` | `120` | Request timeout. |

### Notes and limits

- **CPU is fine.** A 3B model answers in roughly 10–25 s on a modern CPU with no GPU. A smaller tag such as `qwen2.5:1.5b` is faster and usually still good enough; a larger one is rarely worth it for this task.
- **The reply is schema-constrained.** Ollama is given a JSON schema, so the model cannot return prose or malformed JSON.
- **PDFs with a text layer only.** There is no OCR stage, so scans and photos are rejected with an explanatory message rather than silently producing nonsense.
- **The result is a suggestion.** Categories and suppliers are matched against existing rows; anything unmatched is reported as a warning and left for you to fix. Nothing is saved until you press *Speichern*.
- If the model is missing or Ollama is not running, the UI shows exactly that instead of failing silently.

---

## Prerequisites

To run and build this application, you need:

1. **Rust**: Install via [rustup](https://rustup.rs/).
2. **Trunk**: The WebAssembly bundler. Install it via Cargo:
   ```bash
   cargo install trunk
   ```
3. **WebAssembly Target**: Add the WASM target to Rust:
   ```bash
   rustup target add wasm32-unknown-unknown
   ```
4. **PostgreSQL**: A running PostgreSQL database.
   - Default connection URL: `postgres://klubu:klubu-test@localhost:5433/klubu`
5. **Ollama** — only if you want the [local-AI receipt prefill](#local-ai-receipt-prefill). Not needed otherwise.

**You do not need a database to compile.** See [Compiling without a database](#compiling-without-a-database-sqlx-offline-mode).

---

## Compiling without a database (sqlx offline mode)

`sqlx::query!` verifies every SQL statement against a real schema *at compile time*.
Left to its own devices that means you cannot build without a running, already-migrated
Postgres — which breaks fresh clones, CI and `docker build`, and is circular besides,
since the migrations only run when the server starts.

The standard sqlx answer is to commit the query metadata instead:

- `.sqlx/` holds one JSON file per query, describing its parameters and result columns.
- `.cargo/config.toml` sets `SQLX_OFFLINE = "true"`, so builds read that metadata and
  never open a connection.

So `cargo build` works on a fresh clone with no database and no `DATABASE_URL`.

Migrations are applied at **startup** by `sqlx::migrate!()` in `backend/src/main.rs`.
That is the right place for them: a compiler should not be mutating a database, and
production databases should not be migrated as a side effect of a build.

### After you change an SQL query

The build will fail with a clear message (`no cached data for this query, run cargo sqlx prepare`).
Refresh the metadata against a running database and commit the result:

```bash
cargo install sqlx-cli --no-default-features --features postgres,rustls
sqlx migrate run --source backend/migrations   # if the schema changed
cargo sqlx prepare --workspace -- --package backend
```

In CI, assert that nobody forgot:

```bash
cargo sqlx prepare --check --workspace -- --package backend
```

> Why not run the migrations from `build.rs`? Because Cargo caches build-script runs
> (so they would silently not re-run), `rust-analyzer` executes build scripts (so your
> editor would quietly mutate your dev database), and it would *still* require a live
> database at build time — solving nothing for CI or Docker.

---

## How to Run (Development)

### 1. Set Up the Database

Ensure PostgreSQL is running and you have created a database. If you use Docker, you can spin one up quickly:
```bash
docker run --name klubu-db -e POSTGRES_USER=klubu -e POSTGRES_PASSWORD=klubu-test -e POSTGRES_DB=klubu -p 5433:5432 -d postgres:latest
```

### 2. Start the Backend

The backend listens on `http://localhost:8080`, so start it first in one terminal from the `leptos-app` root directory:

```bash
DATABASE_URL=postgres://klubu:klubu-test@localhost:5433/klubu cargo run --package backend
```
*Note: The backend automatically runs database migrations from `backend/migrations` and seeds default receipt categories on startup.*

### 3. Start the Frontend Dev Server

The frontend utilizes Trunk to watch files, compile to WebAssembly, and serve them locally. Because Trunk defaults to port `8080`, start it on a different port and proxy `/api` requests to the backend without rewriting the path:

```bash
cd frontend
trunk serve --port 8081 --proxy-backend=http://localhost:8080/api
```
Open the app at `http://localhost:8081`.

---

## How to Build (Production)

To compile the application for production deployment:

### 1. Build the Frontend Assets
Compile the WASM and bundle resources using Trunk:
```bash
cd frontend
trunk build --release
```
This will output the compiled assets in `frontend/dist`.

### 2. Build the Backend Binary
Compile the server in release mode:
```bash
cargo build --release --package backend
```
This creates the production binary at `target/release/backend`.

### 3. Deploy
Run the backend binary in production:
```bash
DATABASE_URL=postgres://your_prod_db_url /path/to/target/release/backend
```
The backend binary automatically detects and serves the compiled frontend assets from `frontend/dist`.
