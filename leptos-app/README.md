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

- **Dashboard**: Quick overview of revenues, pending invoices, and activities.
- **Contacts Management**: Create, edit, and delete client or supplier contacts.
- **Invoices**: Create invoices, record payments, finalize (commit) them, and export to PDF.
- **Offers**: Manage offers with revisions and export to PDF.
- **Receipts**: Bookkeep receipts and categorize items (e.g., Miete, Bürobedarf) for tax reports.
- **Typst-Based PDF Rendering**: Beautiful, pixel-perfect PDF rendering compiled directly in-memory from Typst templates (no PDFBox or Apache FOP needed).

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
