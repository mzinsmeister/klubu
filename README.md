# Klubu

Klubu is an invoicing and bookkeeping application for German small businesses.
The Rust/Leptos workspace in [`leptos-app`](./leptos-app) is the only application
in this repository.

It provides contact management, offers with revisions, invoices, receipts,
reports, document storage, optional e-mail and local-AI receipt prefill. PDFs
are rendered directly from Typst templates, and the server supports both SQLite
and PostgreSQL.

## Development

Install Rust, the WebAssembly target, and Trunk:

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk
```

Start the backend with the zero-setup SQLite default:

```bash
cd leptos-app
cargo run --package backend
```

In a second terminal, serve the WebAssembly frontend and proxy its API calls:

```bash
cd leptos-app/frontend
trunk serve --port 8081 --proxy-backend=http://localhost:8080/api
```

Open <http://localhost:8081>. SQLite is the default and stores data in
`leptos-app/klubu.db`. PostgreSQL is supported when selected explicitly with a
`postgres://` or `postgresql://` `DATABASE_URL`; the matching migrations are
kept in `leptos-app/backend/migrations-postgres/` and
`leptos-app/backend/migrations-sqlite/`.

The complete feature and deployment documentation is in
[`leptos-app/README.md`](./leptos-app/README.md).

## Production build

Build the frontend and server from the Leptos workspace:

```bash
cd leptos-app/frontend
trunk build --release
cd ..
cargo build --release --package backend
```

For a containerized deployment from the repository root:

```bash
docker compose up --build
```

The application listens on <http://localhost:8080>. The root Compose setup uses
persistent SQLite by default. To use PostgreSQL, start its profile and point the
application at the Compose database:

```bash
DATABASE_URL=postgres://klubu:klubu-test@db:5432/klubu \
  docker compose --profile postgres up --build
```

## Traefik

Klubu can run behind Traefik as a normal HTTP service. Create a shared Docker
network and make the `app` service join it alongside Traefik:

```bash
docker network create traefik
```

Add the following to the root `docker-compose.yml` (replace the hostname and
certificate resolver with your own values):

```yaml
services:
  app:
    networks:
      - default
      - traefik
    labels:
      - traefik.enable=true
      - traefik.docker.network=traefik
      - traefik.http.routers.klubu.rule=Host(`klubu.example.com`)
      - traefik.http.routers.klubu.entrypoints=websecure
      - traefik.http.routers.klubu.tls=true
      - traefik.http.routers.klubu.tls.certresolver=letsencrypt
      - traefik.http.services.klubu.loadbalancer.server.port=8080

networks:
  traefik:
    external: true
    name: traefik
```

Set `KLUBU_SECURE_COOKIES=true` for an HTTPS deployment. Traefik should route
the application unchanged: keep `/api` intact and do not rewrite paths, so
server functions, SPA fallback routing, and authentication cookies continue to
work. Once Traefik is the only entry point, the host mapping `8080:8080` can be
removed from the app service.

The HTTP router above does not proxy the optional SMTP and IMAP relay ports
(`2525` and `2143`). Keep those services private unless you deliberately add
Traefik TCP routers with appropriate TLS and authentication controls.

## License

Klubu is available under the GNU Affero General Public License (AGPL).
