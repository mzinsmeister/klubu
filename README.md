# KlubU (Leptos Version)

A modern, fast, and lightweight invoicing tool, written entirely in Rust using
the **Leptos** web framework.

The application has a lightweight footprint:
- **No Headless Chromium/Selenium dependency**: PDFs are compiled directly in-memory from Typst templates using the `typst` and `typst-pdf` crates.
- **Fast and lightweight**: Single-binary deployment for the backend, and a compiled WebAssembly client.

---

## Project Structure

This is a Cargo workspace consisting of five crates:

- **[`shared`](./shared)**: Contains shared data models (e.g. `Invoice`, `Offer`, `Contact`, `Receipt`, `Payment`) and helpers used by both client and server.
- **[`app`](./app)**: The core UI logic and router built with Leptos. It defines the components and routing for all views (Dashboard, Contacts, Invoices, Offers, Receipts) and contains Typst templates for PDFs.
- **[`frontend`](./frontend)**: The client-side entry point that compiles to WebAssembly using Trunk.
- **[`backend`](./backend)**: The server-side application built with Axum and `leptos_axum`. It serves the compiled WebAssembly frontend, runs database migrations, handles server functions (endpoints queryable directly from the frontend), and serves PDF downloads.
- **[`mcp`](./mcp)**: A local stdio MCP server that exposes the application's audited business operations to LLM agents.

---

## Features

- **Dashboard**: Real figures for the current business year — revenue, expenses, net result (Einnahmenüberschussrechnung), open and draft invoices.
- **Contacts Management**: Create, edit, and delete client or supplier contacts.
- **Invoices**: Create invoices, record payments, finalize (commit) them, and export to PDF.
- **Offers**: Manage offers with revisions and export to PDF.
- **Receipts**: Bookkeep receipts and categorize items (e.g., Miete, Bürobedarf) for tax reports.
- **Receipt prefill** (optional, off by default): read PDFs or scanned receipt images locally and prefill supplier, date, number and positions without sending the document anywhere.
- **E-Mail client and relay**: read and compose mail in the browser, or use a normal mail client through the local SMTP/IMAP relay. Every accepted message is retained as an integrity-checked, content-addressed `.eml` file and indexed in the append-only audit trail.
- **Typst-Based PDF Rendering**: Beautiful, pixel-perfect PDF rendering compiled directly in-memory from Typst templates (no PDFBox or Apache FOP needed).

Amount fields accept German and plain notation interchangeably (`3,4`, `4.5`, `1.234,56`, `12 €`) and normalise to `1.234,56` when the field loses focus. Amounts are held as integer cents throughout, so no rounding drift creeps in.

---

## Receipt prefill

Uploading a receipt PDF on the **Belege** page can prefill the form: supplier, receipt number, date and the individual positions with a category guess.

Everything runs on the machine that runs the server — the receipt never leaves it. Native PDF text is extracted in-process. Scans and image uploads are rendered/read with Tesseract OCR, and the resulting text is then sent to the local Qwen model.

The default `auto` mode sends both native text and OCR text to Qwen when the
configured model is available, for better supplier, line-item, and category
recall. If Ollama is unavailable, it uses the deterministic parser instead.
Electronic invoices remain on the exact structured parser because their fields
are authoritative rather than inferred. The model request is capped at ten
seconds, with a five-second default.

**The feature is off by default.** While it is off, the button is not rendered, nothing contacts a model, and **no model needs to exist on disk**.

To switch on prefill:

```bash
ollama pull qwen2.5:0.5b-instruct            # about 398 MB in Ollama
KLUBU_AI_ENABLED=true cargo run --package backend
```

If latency matters more than extraction quality, the explicit `fast` mode skips
Qwen and uses only deterministic field/amount parsing:

```bash
KLUBU_AI_ENABLED=true KLUBU_AI_MODE=fast cargo run --package backend
```

The Compose file can start Ollama behind the opt-in `ai` profile as well.

### Configuration

Each setting can come from an environment variable or from `config/application.toml` (the env var wins). These are the defaults:

| Property | Environment variable | Default | Meaning |
| --- | --- | --- | --- |
| `klubu.ai.enabled` | `KLUBU_AI_ENABLED` | `false` | Master switch. Accepts `true`/`1`/`yes`/`on`. |
| `klubu.ai.mode` | `KLUBU_AI_MODE` | `auto` | `auto` uses Qwen when available and otherwise falls back to deterministic parsing; `llm` requires Qwen; `fast` always skips it. |
| `klubu.ai.model` | `KLUBU_AI_MODEL` | `qwen2.5:0.5b-instruct` | Ollama model tag used only in `llm` mode. |
| `klubu.ai.url` | `KLUBU_AI_URL` | `http://localhost:11434` | Ollama base URL. |
| `klubu.ai.timeoutSeconds` | `KLUBU_AI_TIMEOUT_SECONDS` | `5` | Maximum wait for the optional LLM request. |

### Notes and limits

- **Qwen2.5-0.5B-Instruct is the speed/quality compromise.** Gemma 3 270M is smaller but has much less room for reliable German structured extraction; Gemma 4 E2B is multimodal but is several gigabytes larger and not a CPU-five-second model.
- **OCR is bounded.** At most two PDF pages are rendered at 160 DPI and OCR has a four-second preprocessing budget; native text PDFs skip this cost.
- **LLM replies are schema-constrained.** Ollama is given a JSON schema, so the reply is parseable even when the model is used.
- **OCR requires system packages.** The container installs `poppler-utils`, `tesseract-ocr`, and German/English Tesseract data. A host install needs the same tools available on `PATH`.
- **The result is a suggestion.** Categories and suppliers are matched against existing rows; anything unmatched is reported as a warning and left for you to fix. Nothing is saved until you press *Speichern*.
- If prefill is enabled and the model or OCR tools are missing, the UI shows exactly that instead of failing silently.

## E-Mail client and relay

The **E-Mail** page provides an inbox, Sent folder, plain-text compose form and
an export of the original `.eml`. The same mailbox is available to ordinary
mail programs through the relay started by the backend:

| Service | Default | Client setting |
| --- | ---: | --- |
| SMTP submission/inbound | `127.0.0.1:2525` | SMTP, authentication enabled for external relay |
| IMAP | `127.0.0.1:2143` | IMAP4rev1, username/password login |

The local account username is also the local-part of its address. For example,
user `anna` receives mail at `anna@localhost` by default. Configure the domain
and ports with:

```bash
KLUBU_MAIL_DOMAIN=example.org
KLUBU_MAIL_SMTP_PORT=2525
KLUBU_MAIL_IMAP_PORT=2143
KLUBU_MAIL_STORAGE_PATH=./mail_storage
```

External delivery is optional. Set `KLUBU_MAIL_SMTP_UPSTREAM=host:port` and,
when required, `KLUBU_MAIL_SMTP_USER` / `KLUBU_MAIL_SMTP_PASSWORD`. The relay
does not provide TLS itself and binds to localhost by default; expose it only
through a TLS-capable mail proxy or a private network. Set
`KLUBU_MAIL_RELAY_ENABLED=false` to disable both listeners.

### Mail archive and GoBD handling

The exact message received by SMTP, submitted by the web client, or appended
over IMAP is written once to a content-addressed `.eml` file. The SHA-256 hash,
sender/recipient index, message-id, timestamps, source and transport status are
stored in `mail_message`, and the archive action is journalled in `audit_log` in
the same database transaction. The database prevents rewriting the archive
identity or deleting a message row. IMAP flags and transport status remain
changeable operational metadata, and each change is journalled. IMAP `EXPUNGE`
creates a tombstone while retaining the original `.eml` bytes.

This is an implementation of technical GoBD controls, not a legal or tax
certification. A production installation still needs its own retention,
backup, access-control and TLS procedures documented in the business's
Verfahrensdokumentation.

MIME attachments are indexed as metadata without extracting a second copy. If
an attachment hash matches a document already stored in the DMS, the mail view
links to that document and directly to its invoice, offer, or receipt.

## Aufträge, Angebote und Rechnungen

An **engagement** (shown as **Auftrag** in the German UI) is a separate, auditable grouping record. It can link mail
archive entries, offer revisions and invoices without copying or changing the
underlying records. The Auftrag page lets you create a group and attach
existing records; composing mail or sending an offer/invoice can link the new
message to an engagement as well. Multiple offers and invoices may be linked to
the same engagement, and creating an invoice from a linked offer carries the
engagement links forward.

Contacts use a normalized email-address table rather than a JSON list. Incoming
mail is matched to the contact automatically, and the contact CRM view provides
notes, recent mail activity, and direct links to all related engagements,
offers, invoices, and receipts.

Finalized offers have two related actions: **Revision erstellen** creates a new
immutable offer revision, and **Rechnung aus Angebot** copies the finalized
revision into a new editable invoice draft. A finalized offer or invoice can
be sent from its detail page as a MIME mail with the generated PDF attached;
the exact outgoing message is then archived like every other mail.

## MCP server for autonomous operation

The `klubu-mcp` workspace binary is a
[Model Context Protocol](https://modelcontextprotocol.io/) server with local
stdio and authenticated remote Streamable HTTP transports.
It exposes 60 typed tools covering the dashboard, contacts and CRM notes,
invoices and payments, offer revisions, receipts and e-invoice/AI extraction,
engagements, email, managed documents, and reports. It also exposes an operating
guide, the current session, and the live dashboard as MCP resources.

The MCP server opens the same SQLite or PostgreSQL database and calls Klubu's
existing server functions. It does not provide raw SQL or a validation bypass:
draft/finalization rules, immutable records, document integrity checks, and the
audit journal therefore behave exactly as they do in the web app. Every write
is attributed to an existing Klubu user.

Build it with:

```bash
cargo build --release --package klubu-mcp
```

For local stdio use, add the binary to an MCP host. The shape below is accepted
by hosts that use the common `mcpServers` JSON configuration:

```json
{
  "mcpServers": {
    "klubu": {
      "command": "/absolute/path/to/target/release/klubu-mcp",
      "env": {
        "KLUBU_MCP_WORKDIR": "/absolute/path/to",
        "DATABASE_URL": "sqlite:///absolute/path/to/klubu.db?mode=rwc",
        "KLUBU_MCP_USER": "anna"
      }
    }
  }
}
```

`KLUBU_MCP_WORKDIR` makes relative template, document archive, mail archive,
and configuration paths resolve as they do for the backend. A binary built in
this workspace auto-detects the workspace in its normal `target` location, but
setting the variable explicitly is recommended for deployment. The usual
`KLUBU_DOCUMENT_STORAGE_PATH`, `KLUBU_MAIL_STORAGE_PATH`,
`KLUBU_EXPORT_TEMPLATES_PATH`, mail, and AI environment variables are honored.

`KLUBU_MCP_USER` must name a user already initialized in Klubu. It may be
omitted only when the database contains exactly one user, in which case that
identity is selected automatically. In stdio mode no password is accepted:
process launch is the authorization boundary, so only configure it in a trusted
MCP host. Finalization, cancellation, deletion, append-only linking, and
email-sending tools carry explicit MCP safety annotations.

### Remote MCP over HTTPS

Remote mode serves Streamable HTTP at `/mcp`. It requires a static bearer token
of at least 32 random characters and binds to loopback by default:

```bash
# Generate this once and keep it in a secret manager or protected environment file.
openssl rand -hex 32

KLUBU_MCP_TRANSPORT=http \
KLUBU_MCP_BIND=127.0.0.1:8090 \
KLUBU_MCP_TOKEN='replace-with-the-generated-token' \
KLUBU_MCP_USER=anna \
KLUBU_MCP_WORKDIR=/srv/klubu/app \
DATABASE_URL='sqlite:///srv/klubu/data/klubu.db?mode=rwc' \
/srv/klubu/app/target/release/klubu-mcp
```

Do not publish port 8090 directly. Terminate TLS in a reverse proxy. A minimal
Caddy configuration is:

```caddyfile
mcp.example.com {
    request_body {
        max_size 75MB
    }
    reverse_proxy 127.0.0.1:8090
}
```

Configure a remote-capable MCP client with the URL and authorization header;
the exact secret interpolation syntax depends on the client:

```json
{
  "mcpServers": {
    "klubu": {
      "url": "https://mcp.example.com/mcp",
      "headers": {
        "Authorization": "Bearer ${KLUBU_MCP_TOKEN}"
      }
    }
  }
}
```

The token is bound server-side to `KLUBU_MCP_USER`; a caller cannot select or
impersonate a different user in a request. Run a separate instance, port, URL,
and token for each Klubu identity when multiple users need remote access. Token
comparison is constant-time, every request is authenticated, unsupported MCP
protocol versions are rejected, and browser `Origin` headers are denied unless
listed in the comma-separated `KLUBU_MCP_ALLOWED_ORIGINS` setting.

Remote mode deliberately refuses a non-loopback bind unless
`KLUBU_MCP_ALLOW_NON_LOOPBACK=true` is set. That override is intended only for
a protected container/private network where an HTTPS proxy cannot reach the
process through loopback. Because this mode uses a configured bearer token
rather than interactive OAuth discovery, the MCP client must support static
HTTP headers or an equivalent secret setting.

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
4. **A database** — chosen at runtime by the `DATABASE_URL` scheme; no database is needed to compile:
   - **SQLite** (default, zero setup): `sqlite://klubu.db?mode=rwc` — the file is created on first start, in WAL mode with `synchronous = FULL`.
   - **PostgreSQL**: e.g. `postgres://klubu:klubu-test@localhost:5433/klubu`
5. **Ollama** — only if you want the default LLM-backed receipt prefill. The explicit `fast` mode does not need it.

---

## Choosing and switching the database

Both drivers are compiled into the one server binary (via sqlx's `Any` driver); the
`DATABASE_URL` scheme picks the backend at runtime. Migrations are applied at
**startup** by `sqlx::migrate!()` in `backend/src/main.rs` from the dialect-matching
directory (`backend/migrations-postgres` / `backend/migrations-sqlite` — same file names, same
versions, kept in lockstep).

To move an existing installation between the two (either direction), stop the server
and run:

```bash
# SQLite development database -> PostgreSQL
DATABASE_URL=sqlite://klubu.db?mode=rwc cargo run --package backend -- migrate-db --to postgres://klubu:klubu-test@localhost:5433/klubu

# PostgreSQL -> SQLite development database
DATABASE_URL=postgres://klubu:klubu-test@localhost:5433/klubu cargo run --package backend -- migrate-db --to sqlite://klubu-copy.db?mode=rwc
```

The copy is catalog-driven (tables, columns, foreign-key order all come from the
database's own metadata, so it never drifts from the migrations) and moves raw rows —
ids, `committed_timestamp`s and the append-only audit journal survive unchanged, as
GoBD requires. The target must be empty; the migration refuses anything else. The
document archive (`document_storage/`) lives in the filesystem and simply stays where
it is.

---

## How to Run (Development)

### 1. Set Up the Database

SQLite is the default development database and needs no setup: `klubu.db` is
created on first start. PostgreSQL is optional and is behind the `postgres`
profile in the workspace Compose file:
```bash
docker compose --profile postgres up -d klubu-postgres-dev
DATABASE_URL=postgres://klubu:klubu-test@localhost:5433/klubu cargo run --package backend
```

### 2. Start the Backend

The backend listens on `http://localhost:8080`, so start it first in one terminal from the repository root:

```bash
cargo run --package backend
# or, when PostgreSQL is explicitly selected:
DATABASE_URL=postgres://klubu:klubu-test@localhost:5433/klubu cargo run --package backend
```
*Note: The backend automatically runs the dialect-matching database migrations and seeds default receipt categories on startup.*

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

### Installable frontend (PWA)

The frontend is an installable, online-first progressive web app. `trunk build
--release` copies the web manifest, service worker, and platform icons into
`frontend/dist`. Browsers on iPhone/iPad, Android, and desktop can then install
Klubu from the browser's install/share menu. The service worker caches only the
app shell; authenticated API calls and archived mail remain online-only.

### 2. Build the Backend Binary
Compile the server in release mode:
```bash
cargo build --release --package backend
```
This creates the production binary at `target/release/backend`.

### 3. Deploy
Run the backend binary in production:
```bash
# Single-node deployment with SQLite:
DATABASE_URL=sqlite:///var/lib/klubu/klubu.db?mode=rwc /path/to/target/release/backend

# Or explicitly select PostgreSQL:
DATABASE_URL=postgres://your_prod_db_url /path/to/target/release/backend
```
The backend binary automatically detects and serves the compiled frontend assets from `frontend/dist`.

## Containerized deployment

Build and start the production container from the repository root:

```bash
docker compose up --build
```

The application listens on `http://localhost:8080` and uses persistent SQLite
storage by default. To use PostgreSQL, start the database profile and point the
application at the Compose database:

```bash
DATABASE_URL=postgres://klubu:klubu-test@db:5432/klubu \
  docker compose --profile postgres up --build
```

The optional `ai` profile starts Ollama and pulls the small LLM used by the
prefill. The SMTP and IMAP relay ports are exposed on `2525` and
`2143`.

For HTTPS deployments behind Traefik, route the application unchanged and keep
`/api` intact so server functions, SPA fallback routing, and authentication
cookies continue to work. Set `KLUBU_SECURE_COOKIES=true` when HTTPS is the
only entry point. Keep the mail relay ports private unless they are deliberately
protected with suitable TCP/TLS routers.
