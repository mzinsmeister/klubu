use leptos::*;
use chrono::{NaiveDate, Utc};
use shared::*;

#[server(name = GetInvoices, prefix = "/api", endpoint = "get_invoices")]
pub async fn get_invoices() -> Result<Vec<InvoiceListItem>, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
    
    // We select invoices and join with contact if present
    let rows = sqlx::query!(
        r#"
        SELECT i.id, i.created_timestamp, i.invoice_number, i.is_canceled, i.is_cancelation, i.committed_timestamp, i.subject,
               c.id as "contact_id?", c.name as "contact_name?", c.first_name as "contact_first_name?"
        FROM invoice i
        LEFT JOIN contact c ON i.customer_contact_id = c.id
        ORDER BY i.invoice_number DESC NULLS LAST, i.id DESC
        "#
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    
    let items = rows.into_iter().map(|r| {
        let contact = r.contact_id.map(|cid| Contact {
            id: Some(cid as i64),
            name: r.contact_name.unwrap_or_default(),
            first_name: r.contact_first_name,
            form_of_address: None,
            title: None,
            street: None,
            zip_code: None,
            city: None,
            house_number: None,
            country: None,
            phone: None,
            is_person: false,
        });
        
        InvoiceListItem {
            id: r.id as i64,
            created_timestamp: chrono::DateTime::from_timestamp(r.created_timestamp.unwrap_or_default().parse::<i64>().unwrap_or_default(), 0).unwrap_or(chrono::DateTime::<Utc>::MIN_UTC),
            customer_contact: contact,
            paid_date: None, // Simplified
            committed: r.committed_timestamp.is_some(),
            invoice_number: r.invoice_number.map(|n| n as i64),
            is_canceled: r.is_canceled != 0,
            is_cancelation: r.is_cancelation != 0,
            subject: r.subject,
        }
    }).collect();
    
    Ok(items)
}

#[server(name = GetInvoice, prefix = "/api", endpoint = "get_invoice")]
pub async fn get_invoice(id: i64) -> Result<Invoice, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
        
    let id_i32 = id as i32;
    let i = sqlx::query!(
        "SELECT * FROM invoice WHERE id = $1", id_i32
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?
    .ok_or_else(|| ServerFnError::new("Invoice not found"))?;
    
    let items_rows = sqlx::query!(
        "SELECT * FROM invoice_item WHERE invoice_id = $1 ORDER BY position_number", id_i32
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    
    let items = items_rows.into_iter().map(|r| Item {
        item: r.item,
        quantity: r.quantity,
        unit: r.unit,
        price: Money::new(r.price as i64),
    }).collect();
    
    let payments_rows = sqlx::query!(
        "SELECT * FROM invoice_payment WHERE invoice_id = $1", id_i32
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    
    let payments = payments_rows.into_iter().map(|r| Payment {
        date: NaiveDate::parse_from_str(&r.payment_date, "%Y-%m-%d").unwrap_or_default(),
        amount_cents: r.amount as i64,
    }).collect();
    
    let contact = if let Some(ccid) = i.customer_contact_id {
        let c = sqlx::query!(
            "SELECT * FROM contact WHERE id = $1", ccid
        )
        .fetch_optional(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        c.map(|row| Contact {
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

    let doc = if let Some(did) = i.document_id {
        let d_row = sqlx::query!(
            "SELECT * FROM document WHERE id = $1", did
        )
        .fetch_optional(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        d_row.map(|row| Document {
            id: row.id as i64,
            media_type: row.media_type,
            extension: row.extension,
            storage_key_prefix: row.storage_key_prefix,
        })
    } else {
        None
    };
    
    Ok(Invoice {
        id: Some(i.id as i64),
        items,
        created_timestamp: i.created_timestamp.as_ref().and_then(|s| s.parse::<i64>().ok()).and_then(|t| chrono::DateTime::from_timestamp(t, 0)),
        committed_timestamp: i.committed_timestamp.as_ref().and_then(|s| s.parse::<i64>().ok()).and_then(|t| chrono::DateTime::from_timestamp(t, 0)),
        invoice_number: i.invoice_number.map(|n| n as i64),
        payments,
        invoice_date: i.invoice_date.as_deref().and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()),
        is_canceled: i.is_canceled != 0,
        is_cancelation: i.is_cancelation != 0,
        corrected_invoice_id: i.corrected_invoice_id.map(|n| n as i64),
        customer_contact: contact,
        document: doc,
        recipient: Some(Recipient {
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

#[server(name = SaveInvoice, prefix = "/api", endpoint = "save_invoice")]
pub async fn save_invoice(invoice: Invoice) -> Result<Invoice, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
        
    let recipient = invoice.recipient.clone().unwrap_or(Recipient {
        form_of_address: None,
        title: None,
        name: "Name".to_string(),
        first_name: None,
        street: None,
        zip_code: None,
        city: None,
        house_number: None,
        country: None,
    });
    
    let invoice_date_str = invoice.invoice_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| Utc::now().naive_utc().date().format("%Y-%m-%d").to_string());
    
    let customer_contact_id = invoice.customer_contact.as_ref().and_then(|c| c.id);
    let customer_contact_id_i32 = customer_contact_id.map(|id| id as i32);
    
    let final_invoice = if let Some(id) = invoice.id {
        let id_i32 = id as i32;
        
        // Check if already committed
        let committed_check = sqlx::query!(
            "SELECT committed_timestamp FROM invoice WHERE id = $1", id_i32
        )
        .fetch_optional(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        
        if let Some(row) = committed_check {
            if row.committed_timestamp.is_some() {
                return Err(ServerFnError::new("Cannot modify a finalized invoice"));
            }
        }
        
        sqlx::query!(
            "UPDATE invoice SET invoice_date = $1, subject = $2, title = $3, header_html = $4, footer_html = $5, recipient_name = $6, recipient_first_name = $7, recipient_title = $8, recipient_form_of_address = $9, street = $10, house_number = $11, zip_code = $12, city = $13, country = $14, customer_contact_id = $15 WHERE id = $16",
            invoice_date_str,
            invoice.subject,
            invoice.title,
            invoice.header_html,
            invoice.footer_html,
            recipient.name,
            recipient.first_name,
            recipient.title,
            recipient.form_of_address,
            recipient.street,
            recipient.house_number,
            recipient.zip_code,
            recipient.city,
            recipient.country,
            customer_contact_id_i32,
            id_i32
        )
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        
        // Remove old items
        sqlx::query!("DELETE FROM invoice_item WHERE invoice_id = $1", id_i32)
            .execute(&pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
            
        id
    } else {
        let created_ts_str = Utc::now().timestamp().to_string();
        
        let row = sqlx::query!(
            "INSERT INTO invoice (invoice_number, invoice_date, subject, title, header_html, footer_html, recipient_name, recipient_first_name, recipient_title, recipient_form_of_address, street, house_number, zip_code, city, country, customer_contact_id, created_timestamp, committed_timestamp, is_canceled, is_cancelation) VALUES (NULL, $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, NULL, 0, 0) RETURNING id",
            invoice_date_str,
            invoice.subject,
            invoice.title,
            invoice.header_html,
            invoice.footer_html,
            recipient.name,
            recipient.first_name,
            recipient.title,
            recipient.form_of_address,
            recipient.street,
            recipient.house_number,
            recipient.zip_code,
            recipient.city,
            recipient.country,
            customer_contact_id_i32,
            created_ts_str
        )
        .fetch_one(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        
        row.id as i64
    };
    
    // Insert new items
    for (i, item) in invoice.items.iter().enumerate() {
        let total = (item.quantity * item.price.amount_cents as f64) as i64;
        let pos_num = (i + 1) as i64;
        let item_price = item.price.amount_cents;
        
        let final_invoice_i32 = final_invoice as i32;
        let pos_num_i32 = pos_num as i32;
        let item_price_i32 = item_price as i32;
        let total_i32 = total as i32;
        
        sqlx::query!(
            "INSERT INTO invoice_item (invoice_id, position_number, item, quantity, unit, price, total) VALUES ($1, $2, $3, $4, $5, $6, $7)",
            final_invoice_i32,
            pos_num_i32,
            item.item,
            item.quantity,
            item.unit,
            item_price_i32,
            total_i32
        )
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    }
    
    get_invoice(final_invoice).await
}

#[server(name = CancelInvoice, prefix = "/api", endpoint = "cancel_invoice")]
pub async fn cancel_invoice(id: i64) -> Result<(), ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
    let id_i32 = id as i32;
    sqlx::query!("UPDATE invoice SET is_canceled = 1 WHERE id = $1", id_i32)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

#[server(name = AddInvoicePayment, prefix = "/api", endpoint = "add_invoice_payment")]
pub async fn add_invoice_payment(invoice_id: i64, amount_cents: i64, date: NaiveDate) -> Result<(), ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
    let date_str = date.format("%Y-%m-%d").to_string();
    let invoice_id_i32 = invoice_id as i32;
    let amount_cents_i32 = amount_cents as i32;
    sqlx::query!(
        "INSERT INTO invoice_payment (invoice_id, amount, payment_date) VALUES ($1, $2, $3)",
        invoice_id_i32,
        amount_cents_i32,
        date_str
    )
    .execute(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

#[server(name = DeleteInvoicePayment, prefix = "/api", endpoint = "delete_invoice_payment")]
pub async fn delete_invoice_payment(id: i64) -> Result<(), ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
    let id_i32 = id as i32;
    sqlx::query!("DELETE FROM invoice_payment WHERE id = $1", id_i32)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

#[server(name = CommitInvoice, prefix = "/api", endpoint = "commit_invoice")]
pub async fn commit_invoice(id: i64) -> Result<(), ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
    let id_i32 = id as i32;
    
    let row = sqlx::query!(
        "SELECT committed_timestamp, customer_contact_id FROM invoice WHERE id = $1", id_i32
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?
    .ok_or_else(|| ServerFnError::new("Invoice not found"))?;
    
    if row.committed_timestamp.is_some() {
        return Err(ServerFnError::new("Invoice is already finalized"));
    }
    
    if row.customer_contact_id.is_none() {
        return Err(ServerFnError::new("Cannot finalize invoice without an assigned customer contact"));
    }
    
    let next_number = sqlx::query_scalar!("SELECT COALESCE(MAX(invoice_number), 0) FROM invoice")
        .fetch_one(&pool)
        .await
        .unwrap_or(Some(0))
        .unwrap_or(0) + 1;
        
    let next_number_i32 = next_number as i32;
    let committed_ts = Utc::now().timestamp().to_string();
    
    sqlx::query!(
        "UPDATE invoice SET invoice_number = $1, committed_timestamp = $2 WHERE id = $3",
        next_number_i32,
        committed_ts,
        id_i32
    )
    .execute(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    
    Ok(())
}

#[server(name = DeleteInvoice, prefix = "/api", endpoint = "delete_invoice")]
pub async fn delete_invoice(id: i64) -> Result<(), ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
    let id_i32 = id as i32;
    
    let row = sqlx::query!(
        "SELECT committed_timestamp FROM invoice WHERE id = $1", id_i32
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?
    .ok_or_else(|| ServerFnError::new("Invoice not found"))?;
    
    if row.committed_timestamp.is_some() {
        return Err(ServerFnError::new("Cannot delete a finalized invoice"));
    }
    
    sqlx::query!("DELETE FROM invoice_item WHERE invoice_id = $1", id_i32)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        
    sqlx::query!("DELETE FROM invoice_payment WHERE invoice_id = $1", id_i32)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        
    sqlx::query!("DELETE FROM invoice WHERE id = $1", id_i32)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        
    Ok(())
}

