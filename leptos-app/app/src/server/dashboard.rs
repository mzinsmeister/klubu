//! Real figures for the overview page.
//!
//! Dates are stored as `YYYY-MM-DD` strings, so the year filter is a prefix
//! comparison rather than a date function.

use leptos::*;
use shared::*;

#[server(name = GetDashboardStats, prefix = "/api", endpoint = "get_dashboard_stats")]
pub async fn get_dashboard_stats() -> Result<DashboardStats, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;

    let year = chrono::Utc::now().naive_utc().date().format("%Y").to_string();
    let year_prefix = format!("{year}-%");

    // Revenue: finalized invoices that were not canceled and are not themselves
    // a cancelation document.
    let revenue_cents = sqlx::query_scalar!(
        r#"
        SELECT COALESCE(SUM(ii.total), 0) as "sum!"
        FROM invoice i
        JOIN invoice_item ii ON ii.invoice_id = i.id
        WHERE i.committed_timestamp IS NOT NULL
          AND i.is_canceled = 0
          AND i.is_cancelation = 0
          AND i.invoice_date LIKE $1
        "#,
        year_prefix
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    // Expenses: receipt positions booked into a category of type "Ausgaben".
    let expenses_cents = sqlx::query_scalar!(
        r#"
        SELECT COALESCE(SUM(ri.total), 0) as "sum!"
        FROM receipt r
        JOIN receipt_item ri ON ri.receipt_id = r.id
        JOIN receipt_item_category c ON ri.category_id = c.id
        JOIN receipt_item_category_type t ON c.category_type_id = t.id
        WHERE t.name = 'Ausgaben'
          AND r.receipt_date LIKE $1
        "#,
        year_prefix
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    // Open: finalized, not canceled, and nothing paid against it yet.
    let open = sqlx::query!(
        r#"
        -- SUM() over a bigint yields numeric in Postgres, so cast back explicitly.
        SELECT COUNT(*) as "count!", COALESCE(SUM(totals.total), 0)::bigint as "sum!"
        FROM (
            SELECT i.id, COALESCE(SUM(ii.total), 0) as total
            FROM invoice i
            LEFT JOIN invoice_item ii ON ii.invoice_id = i.id
            WHERE i.committed_timestamp IS NOT NULL
              AND i.is_canceled = 0
              AND i.is_cancelation = 0
              AND NOT EXISTS (SELECT 1 FROM invoice_payment p WHERE p.invoice_id = i.id)
            GROUP BY i.id
        ) totals
        "#
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    let draft_invoice_count = sqlx::query_scalar!(
        r#"SELECT COUNT(*) as "count!" FROM invoice WHERE committed_timestamp IS NULL"#
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    let receipt_count = sqlx::query_scalar!(
        r#"SELECT COUNT(*) as "count!" FROM receipt WHERE receipt_date LIKE $1"#,
        year_prefix
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    let contact_count = sqlx::query_scalar!(r#"SELECT COUNT(*) as "count!" FROM contact"#)
        .fetch_one(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(DashboardStats {
        year: year.parse().unwrap_or_default(),
        revenue_cents,
        expenses_cents,
        open_invoice_count: open.count,
        open_invoice_cents: open.sum,
        draft_invoice_count,
        receipt_count,
        contact_count,
    })
}
