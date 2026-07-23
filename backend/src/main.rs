use app::db::KlubuRepository;
use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, Request, Response, StatusCode},
    middleware::Next,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

mod dbcopy;
use dbcopy::{connect_pool, migrator_for, run_db_migration, shutdown_pool};

mod mail;
mod mcp;

const DEFAULT_DATABASE_URL: &str = "sqlite://klubu.db?mode=rwc";

/// Endpoints reachable without a session. Everything else under `/api` needs one.
///
/// Matched exactly, not by prefix: `login` and `initialize_admin` are the only
/// ways in, and `check_setup_required` / `get_current_user` are what the SPA asks
/// before it knows whether it has a session at all.
const PUBLIC_API_PATHS: &[&str] = &[
    "/api/check_setup_required",
    "/api/initialize_admin",
    "/api/login",
    "/api/get_current_user",
];

/// Authenticates every `/api` request and attaches the resolved identity to the
/// request extensions, where `handle_server_fns` picks it up.
///
/// The identity has to travel in the extensions rather than a task-local:
/// `leptos_axum` runs each server function on a separate task (`spawn_pinned`),
/// which a task-local does not survive.
async fn auth_middleware(
    State(repo): State<app::db::ActiveRepository>,
    mut req: Request<Body>,
    next: Next,
) -> Response<Body> {
    let path = req.uri().path();

    if !path.starts_with("/api") || PUBLIC_API_PATHS.contains(&path) {
        return next.run(req).await;
    }

    let username = match req
        .headers()
        .get(header::COOKIE)
        .and_then(|c| c.to_str().ok())
        .and_then(app::server::auth::session_token_from_cookie_header)
    {
        Some(token) => app::server::auth::lookup_session(repo.pool(), &token)
            .await
            .unwrap_or(None),
        None => None,
    };

    let Some(username) = username else {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Body::from("Unauthorized"))
            .unwrap();
    };

    req.extensions_mut()
        .insert(app::server::auth::CurrentUser(username));
    next.run(req).await
}

async fn handle_server_fns(
    State(repo): State<app::db::ActiveRepository>,
    req: Request<Body>,
) -> impl IntoResponse {
    // Read the identity out here, on the Axum task, and hand it to the closure —
    // which runs inside the spawned server-fn task, where the audit log reads it
    // back via `use_context`.
    let current_user = req
        .extensions()
        .get::<app::server::auth::CurrentUser>()
        .cloned();

    leptos_axum::handle_server_fns_with_context(
        move || {
            leptos::provide_context(repo.clone());
            // The chat assistant's tool table; shared with /mcp but available
            // regardless of whether that endpoint is enabled.
            leptos::provide_context(mcp::chat_tool_backend(repo.clone()));
            if let Some(user) = current_user.clone() {
                leptos::provide_context(user);
            }
        },
        req,
    )
    .await
}

fn get_dist_dir() -> String {
    if std::path::Path::new("frontend/dist").exists() {
        "frontend/dist".to_string()
    } else if std::path::Path::new("../frontend/dist").exists() {
        "../frontend/dist".to_string()
    } else if std::path::Path::new("dist").exists() {
        "dist".to_string()
    } else {
        "./dist".to_string()
    }
}

async fn download_invoice_pdf(
    Path(id): Path<i64>,
    State(repo): State<app::db::ActiveRepository>,
) -> impl IntoResponse {
    match repo.get_invoice(id).await {
        Ok(invoice) => {
            // Committed invoices download as ZUGFeRD (PDF/A-3b with the CII XML
            // embedded); drafts stay the plain watermarked preview.
            match app::einvoice::render_invoice_pdf(&invoice) {
                Ok(pdf_bytes) => Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "application/pdf")
                    .header(
                        header::CONTENT_DISPOSITION,
                        format!("inline; filename=\"invoice_{}.pdf\"", id),
                    )
                    .body(Body::from(pdf_bytes))
                    .unwrap(),
                Err(e) => Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from(format!("Failed to compile typst: {}", e)))
                    .unwrap(),
            }
        }
        Err(e) => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from(format!("Invoice not found: {}", e)))
            .unwrap(),
    }
}

async fn download_offer_pdf(
    Path(id): Path<i64>,
    State(repo): State<app::db::ActiveRepository>,
) -> impl IntoResponse {
    match repo.get_offer(id).await {
        Ok(offer) => {
            let typst_code = app::generate_offer_typst(&offer);
            match app::pdf::compiler::compile_typst_pdfa(typst_code) {
                Ok(pdf_bytes) => Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "application/pdf")
                    .header(
                        header::CONTENT_DISPOSITION,
                        format!("inline; filename=\"offer_{}.pdf\"", id),
                    )
                    .body(Body::from(pdf_bytes))
                    .unwrap(),
                Err(e) => Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from(format!("Failed to compile typst: {}", e)))
                    .unwrap(),
            }
        }
        Err(e) => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from(format!("Offer not found: {}", e)))
            .unwrap(),
    }
}

/// Media types a browser may render in place (chat previews open this
/// endpoint in a same-origin iframe that carries the session cookie).
/// Everything else — HTML, SVG, and anything unknown — is forced to download
/// so an uploaded file can never run script in the app's origin.
fn renders_inline(media_type: &str) -> bool {
    matches!(
        media_type,
        "application/pdf" | "image/png" | "image/jpeg" | "image/webp" | "image/gif" | "text/plain"
    )
}

async fn download_document(
    Path(id): Path<i64>,
    State(repo): State<app::db::ActiveRepository>,
) -> impl IntoResponse {
    let doc_id = id as i32;

    let doc = match repo.get_document_meta(doc_id).await {
        Ok(Some(d)) => d,
        Ok(None) => {
            return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("Document not found"))
                .unwrap()
        }
        Err(e) => {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from(e.to_string()))
                .unwrap()
        }
    };

    let version = match repo.get_latest_document_version(doc_id).await {
        Ok(Some(v)) => v,
        Ok(None) => {
            return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("Document version not found"))
                .unwrap()
        }
        Err(e) => {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from(e.to_string()))
                .unwrap()
        }
    };

    if version.1 != 0 {
        return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Document was deleted"))
            .unwrap();
    }

    let storage_dir = std::env::var("KLUBU_DOCUMENT_STORAGE_PATH")
        .unwrap_or_else(|_| "./document_storage".to_string());

    let file_name = format!("{}_{}.{}", doc.2, version.0, doc.0);
    let file_path = std::path::Path::new(&storage_dir).join(&file_name);

    match tokio::fs::read(&file_path).await {
        Ok(bytes) => Response::builder()
            .status(StatusCode::OK)
            .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
            .header(
                header::CONTENT_DISPOSITION,
                format!(
                    "{}; filename=\"document_{}.{}\"",
                    if renders_inline(&doc.1) {
                        "inline"
                    } else {
                        "attachment"
                    },
                    doc_id,
                    doc.0
                ),
            )
            .header(header::CONTENT_TYPE, doc.1)
            .body(Body::from(bytes))
            .unwrap(),
        Err(e) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from(format!("Failed to read file: {}", e)))
            .unwrap(),
    }
}

#[tokio::main]
async fn main() {
    let props = app::typst_gen::load_props();
    let db_url = std::env::var("DATABASE_URL")
        .ok()
        .or_else(|| props.get("klubu.database.url").cloned())
        .unwrap_or_else(|| DEFAULT_DATABASE_URL.to_string());

    // `migrate-db --to <target-url>` copies this instance's data into another
    // database (typically the other dialect) and exits. See `dbcopy.rs`.
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("migrate-db") {
        let target_url = match args.get(2).map(String::as_str) {
            Some("--to") => args.get(3).cloned(),
            Some(url) => Some(url.to_string()),
            None => None,
        }
        .unwrap_or_else(|| {
            eprintln!("Usage: klubu-backend migrate-db --to <target-database-url>");
            std::process::exit(2);
        });
        if let Err(error) = run_db_migration(&db_url, &target_url).await {
            eprintln!("Migration fehlgeschlagen: {error}");
            std::process::exit(1);
        }
        return;
    }

    println!("Connecting to database: {}", db_url);
    let pool = connect_pool(&db_url)
        .await
        .expect("Failed to connect to database");

    println!("Running database migrations...");
    migrator_for(&db_url)
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    // The pool is reference-counted; this clone is for the teardown after
    // `axum::serve` returns, once `repo` has been moved into the router state.
    let repo: app::db::ActiveRepository = Arc::new(app::db::SqlRepository::new(pool.clone()));

    repo.seed_database().await.expect("Failed to seed database");

    // No users yet? Mint a one-shot setup token so the first admin can be created.
    // A failure to read `users` must not be mistaken for "no users" — that would
    // print a setup link for a database we cannot actually talk to.
    let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(repo.pool())
        .await
        .expect("Failed to query users table");

    if user_count == 0 {
        let token = app::server::auth::generate_random_token();
        {
            let mut lock = app::server::auth::get_setup_token_lock().lock().unwrap();
            *lock = Some(token.clone());
        }
        println!("========================================================================");
        println!("[SETUP] NO USERS FOUND IN DATABASE.");
        println!("To initialize the admin account, please open the following link:");
        println!("http://localhost:8080/setup?token={}", token);
        println!("========================================================================");
    }

    app::init_templates();
    app::register_server_fns();
    let mail_tasks = mail::spawn(repo.clone());

    let dist_dir = get_dist_dir();
    println!("Serving static files from: {}", dist_dir);

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let mut app = Router::new()
        .route("/api/*fn_name", post(handle_server_fns))
        .route("/api/pdf/invoice/:id", get(download_invoice_pdf))
        .route("/api/pdf/offer/:id", get(download_offer_pdf))
        .route("/api/documents/:id", get(download_document))
        .layer(cors);

    // The MCP endpoint is nested after the CORS layer on purpose: it enforces
    // its own Origin allowlist and must not be relaxed by the permissive
    // web-app CORS policy. `auth_middleware` only guards `/api` paths, so
    // `/mcp` relies solely on its bearer token.
    match mcp::router(repo.clone()) {
        Ok(Some(mcp_router)) => {
            println!("MCP endpoint enabled at /mcp");
            app = app.nest_service("/mcp", mcp_router);
        }
        Ok(None) => println!("MCP endpoint disabled (set KLUBU_MCP_TOKEN to enable)"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }

    let app = app
        .fallback_service(
            ServeDir::new(&dist_dir).not_found_service(tower::service_fn(|_req| async {
                let dist_dir = get_dist_dir();
                let index_path = std::path::Path::new(&dist_dir).join("index.html");
                match tokio::fs::read_to_string(&index_path).await {
                    Ok(content) => Ok(Response::builder()
                        .status(StatusCode::OK)
                        .header(header::CONTENT_TYPE, "text/html")
                        .body(Body::from(content))
                        .unwrap()),
                    Err(_) => Ok(Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .body(Body::from("SPA index.html not found"))
                        .unwrap()),
                }
            })),
        )
        .layer(axum::middleware::from_fn_with_state(
            repo.clone(),
            auth_middleware,
        ))
        .with_state(repo);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("Listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    for task in mail_tasks {
        task.abort();
    }

    // In-flight requests have drained; checkpoint the WAL back into the main
    // .db file and close cleanly, so the -wal/-shm sidecars are empty and the
    // database is a single self-contained file for backup or deletion.
    println!("Shutting down, checkpointing database...");
    shutdown_pool(&pool, &db_url).await;
}

/// Resolves on Ctrl+C or SIGTERM (what systemd and `docker stop` send). Either
/// one stops the listener; the teardown after `axum::serve` then runs instead
/// of the process dying mid-write.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scriptable_uploads_are_forced_to_download() {
        assert!(renders_inline("application/pdf"));
        assert!(renders_inline("image/png"));
        assert!(!renders_inline("text/html"));
        assert!(!renders_inline("image/svg+xml"));
        assert!(!renders_inline("application/octet-stream"));
    }
}
