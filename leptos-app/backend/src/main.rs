use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, Request, Response, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use tower_http::services::ServeDir;
use tower_http::cors::{Any, CorsLayer};
use app::db::{DbPool, KlubuRepository};
use std::sync::Arc;
use std::net::SocketAddr;

async fn handle_server_fns(
    State(repo): State<app::db::ActiveRepository>,
    req: Request<Body>,
) -> impl IntoResponse {
    leptos_axum::handle_server_fns_with_context(
        move || {
            leptos::provide_context(repo.clone());
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
            let typst_code = app::generate_invoice_typst(&invoice);
            match app::pdf::compiler::compile_typst(typst_code) {
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
            match app::pdf::compiler::compile_typst(typst_code) {
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

async fn download_document(
    Path(id): Path<i64>,
    State(repo): State<app::db::ActiveRepository>,
) -> impl IntoResponse {
    let doc_id = id as i32;
    
    let doc = match repo.get_document_meta(doc_id).await {
        Ok(Some(d)) => d,
        Ok(None) => return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Document not found"))
            .unwrap(),
        Err(e) => return Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from(e.to_string()))
            .unwrap(),
    };
    
    let version = match repo.get_latest_document_version(doc_id).await {
        Ok(Some(v)) => v,
        Ok(None) => return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Document version not found"))
            .unwrap(),
        Err(e) => return Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from(e.to_string()))
            .unwrap(),
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
            .header(header::CONTENT_TYPE, doc.1)
            .header(
                header::CONTENT_DISPOSITION,
                format!("inline; filename=\"document_{}.{}\"", doc_id, doc.0),
            )
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
    #[cfg(feature = "sqlite")]
    let default_db_url = "sqlite://klubu.db?mode=rwc";
    #[cfg(feature = "postgres")]
    let default_db_url = "postgres://klubu:klubu-test@localhost:5433/klubu";

    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| default_db_url.to_string());
    println!("Connecting to database: {}", db_url);
    
    let pool = DbPool::connect(&db_url)
        .await
        .expect("Failed to connect to database");
        
    println!("Running database migrations...");
    #[cfg(feature = "postgres")]
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    #[cfg(feature = "sqlite")]
    sqlx::migrate!("./migrations-sqlite")
        .run(&pool)
        .await
        .expect("Failed to run migrations");
        
    let repo: app::db::ActiveRepository = Arc::new(app::db::SqlRepository::new(pool));


    repo.seed_database().await.expect("Failed to seed database");
    app::init_templates();
    app::register_server_fns();
    
    let dist_dir = get_dist_dir();
    println!("Serving static files from: {}", dist_dir);
    
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/api/*fn_name", post(handle_server_fns))
        .route("/api/pdf/invoice/:id", get(download_invoice_pdf))
        .route("/api/pdf/offer/:id", get(download_offer_pdf))
        .route("/api/documents/:id", get(download_document))
        .layer(cors)
        .fallback_service(
            ServeDir::new(&dist_dir)
                .not_found_service(tower::service_fn(|_req| async {
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
                }))
        )
        .with_state(repo);
        
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("Listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
