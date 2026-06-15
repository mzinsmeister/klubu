use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, Request, Response, StatusCode},
    response::{IntoResponse, Response as AxumResponse},
    routing::{get, post},
    Router,
};
use tower_http::services::ServeDir;
use tower_http::cors::{Any, CorsLayer};
use sqlx::postgres::PgPool;
use std::net::SocketAddr;

async fn handle_server_fns(
    State(pool): State<PgPool>,
    req: Request<Body>,
) -> impl IntoResponse {
    leptos_axum::handle_server_fns_with_context(
        move || {
            leptos::provide_context(pool.clone());
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
    State(pool): State<PgPool>,
) -> impl IntoResponse {
    match fetch_invoice_db(&pool, id).await {
        Ok(invoice) => {
            if invoice.committed_timestamp.is_none() {
                return Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Body::from("Can only export committed invoices"))
                    .unwrap();
            }
            let typst_code = app::generate_invoice_typst(&invoice);
            match app::pdf::compiler::compile_typst(typst_code) {
                Ok(pdf_bytes) => Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "application/pdf")
                    .header(
                        header::CONTENT_DISPOSITION,
                        format!("attachment; filename=\"invoice_{}.pdf\"", id),
                    )
                    .body(Body::from(pdf_bytes))
                    .unwrap(),
                Err(e) => Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from(format!("PDF compilation failed: {}", e)))
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
    State(pool): State<PgPool>,
) -> impl IntoResponse {
    match fetch_offer_db(&pool, id).await {
        Ok(offer) => {
            if offer.committed_timestamp.is_none() {
                return Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Body::from("Can only export committed offers"))
                    .unwrap();
            }
            let typst_code = app::generate_offer_typst(&offer);
            match app::pdf::compiler::compile_typst(typst_code) {
                Ok(pdf_bytes) => Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "application/pdf")
                    .header(
                        header::CONTENT_DISPOSITION,
                        format!("attachment; filename=\"offer_{}.pdf\"", id),
                    )
                    .body(Body::from(pdf_bytes))
                    .unwrap(),
                Err(e) => Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from(format!("PDF compilation failed: {}", e)))
                    .unwrap(),
            }
        }
        Err(e) => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from(format!("Offer not found: {}", e)))
            .unwrap(),
    }
}

#[tokio::main]
async fn main() {
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://klubu:klubu-test@localhost:5433/klubu".to_string());
    println!("Connecting to database: {}", db_url);
    
    let pool = PgPool::connect(&db_url)
        .await
        .expect("Failed to connect to PostgreSQL");
        
    println!("Running database migrations...");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");
        
    seed_database(&pool).await.expect("Failed to seed database");
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
        .with_state(pool);
        
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("Listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn seed_database(pool: &PgPool) -> Result<(), sqlx::Error> {
    let count = sqlx::query_scalar!("SELECT COUNT(*) FROM receipt_item_category_type")
        .fetch_one(pool)
        .await?
        .unwrap_or(0);
        
    if count == 0 {
        println!("Seeding default receipt item category types and categories...");
        
        let types = vec![
            ("Einnahmen", vec!["Mitgliedsbeiträge", "Spenden", "Sponsoring", "Sonstige Einnahmen"]),
            ("Ausgaben", vec!["Miete", "Bürobedarf", "Marketing", "Reisekosten", "Sonstige Ausgaben"]),
            ("Investitionen", vec!["Hardware", "Software", "Anschaffungen"]),
        ];
        
        for (type_name, categories) in types {
            let type_id = sqlx::query!(
                "INSERT INTO receipt_item_category_type (name) VALUES ($1) RETURNING id",
                type_name
            )
            .fetch_one(pool)
            .await?
            .id;
            
            for cat_name in categories {
                sqlx::query!(
                    "INSERT INTO receipt_item_category (name, category_type_id) VALUES ($1, $2)",
                    cat_name,
                    type_id
                )
                .execute(pool)
                .await?;
            }
        }
    }
    Ok(())
}

pub async fn fetch_invoice_db(pool: &PgPool, id: i64) -> Result<shared::Invoice, String> {
    let id_i32 = id as i32;
    let i = sqlx::query!(
        "SELECT * FROM invoice WHERE id = $1", id_i32
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "Invoice not found".to_string())?;
    
    let items_rows = sqlx::query!(
        "SELECT * FROM invoice_item WHERE invoice_id = $1 ORDER BY position_number", id_i32
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;
    
    let items = items_rows.into_iter().map(|r| shared::Item {
        item: r.item,
        quantity: r.quantity,
        unit: r.unit,
        price: shared::Money::new(r.price as i64),
    }).collect();
    
    let payments_rows = sqlx::query!(
        "SELECT * FROM invoice_payment WHERE invoice_id = $1", id_i32
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;
    
    let payments = payments_rows.into_iter().map(|r| shared::Payment {
        date: chrono::NaiveDate::parse_from_str(&r.payment_date, "%Y-%m-%d").unwrap_or_default(),
        amount_cents: r.amount as i64,
    }).collect();
    
    let contact = if let Some(ccid) = i.customer_contact_id {
        let row = sqlx::query!(
            "SELECT * FROM contact WHERE id = $1", ccid
        )
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?;
        row.map(|row| shared::Contact {
            id: Some(row.id as i64),
            form_of_address: row.form_of_address,
            title: row.title,
            name: row.name,
            first_name: row.first_name,
            street: row.street,
            zip_code: row.zip_code,
            city: row.city,
            house_number: row.house_number,
            country: row.country,
            phone: row.phone,
            is_person: row.is_person != 0,
        })
    } else {
        None
    };

    let doc = i.document_id.map(|did| shared::Document {
        id: did as i64,
        media_type: "application/pdf".to_string(),
        extension: "pdf".to_string(),
        storage_key_prefix: format!("invoice_{}", id),
    });
    
    Ok(shared::Invoice {
        id: Some(i.id as i64),
        items,
        created_timestamp: i.created_timestamp.as_ref().and_then(|s| s.parse::<i64>().ok()).and_then(|t| chrono::DateTime::from_timestamp(t, 0)),
        committed_timestamp: i.committed_timestamp.as_ref().and_then(|s| s.parse::<i64>().ok()).and_then(|t| chrono::DateTime::from_timestamp(t, 0)),
        invoice_number: i.invoice_number.map(|n| n as i64),
        payments,
        invoice_date: chrono::NaiveDate::parse_from_str(&i.invoice_date.unwrap_or_default(), "%Y-%m-%d").ok(),
        is_canceled: i.is_canceled != 0,
        is_cancelation: i.is_cancelation != 0,
        corrected_invoice_id: i.corrected_invoice_id.map(|n| n as i64),
        customer_contact: contact,
        document: doc,
        recipient: Some(shared::Recipient {
            form_of_address: i.recipient_form_of_address,
            title: i.recipient_title,
            name: i.recipient_name,
            first_name: i.recipient_first_name,
            street: i.street,
            zip_code: i.zip_code,
            city: i.city,
            house_number: i.house_number,
            country: i.country,
        }),
        header_html: i.header_html,
        footer_html: i.footer_html,
        title: i.title,
        subject: i.subject,
    })
}

pub async fn fetch_offer_db(pool: &PgPool, id: i64) -> Result<shared::Offer, String> {
    let id_i32 = id as i32;
    let o = sqlx::query!(
        "SELECT * FROM offer WHERE id = $1", id_i32
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "Offer not found".to_string())?;
    
    let items_rows = sqlx::query!(
        "SELECT * FROM offer_item WHERE offer_id = $1 AND offer_revision = $2 ORDER BY position_number", id_i32, o.revision
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;
    
    let items = items_rows.into_iter().map(|r| shared::Item {
        item: r.item,
        quantity: r.quantity,
        unit: r.unit,
        price: shared::Money::new(r.price as i64),
    }).collect();
    
    let contact = if let Some(ccid) = o.customer_contact_id {
        let row = sqlx::query!("SELECT * FROM contact WHERE id = $1", ccid)
            .fetch_optional(pool)
            .await
            .map_err(|e| e.to_string())?;
        row.map(|row| shared::Contact {
            id: Some(row.id as i64),
            form_of_address: row.form_of_address,
            title: row.title,
            name: row.name,
            first_name: row.first_name,
            street: row.street,
            zip_code: row.zip_code,
            city: row.city,
            house_number: row.house_number,
            country: row.country,
            phone: row.phone,
            is_person: row.is_person != 0,
        })
    } else {
        None
    };

    let doc = o.document_id.map(|did| shared::Document {
        id: did as i64,
        media_type: "application/pdf".to_string(),
        extension: "pdf".to_string(),
        storage_key_prefix: format!("offer_{}", id),
    });
    
    Ok(shared::Offer {
        id: Some(o.id as i64),
        revision: Some(o.revision as i64),
        title: o.title,
        customer_contact: contact,
        offer_date: chrono::NaiveDate::parse_from_str(&o.offer_date.unwrap_or_default(), "%Y-%m-%d").ok(),
        valid_until_date: None,
        recipient: Some(shared::Recipient {
            form_of_address: o.recipient_form_of_address,
            title: o.recipient_title,
            name: o.recipient_name,
            first_name: o.recipient_first_name,
            street: o.street,
            zip_code: o.zip_code,
            city: o.city,
            house_number: o.house_number,
            country: o.country,
        }),
        items,
        created_timestamp: o.created_timestamp.as_ref().and_then(|s| s.parse::<i64>().ok()).and_then(|t| chrono::DateTime::from_timestamp(t, 0)),
        committed_timestamp: o.committed_timestamp.as_ref().and_then(|s| s.parse::<i64>().ok()).and_then(|t| chrono::DateTime::from_timestamp(t, 0)),
        subject: o.subject,
        header_html: o.header_html,
        footer_html: o.footer_html,
        document: doc,
    })
}
