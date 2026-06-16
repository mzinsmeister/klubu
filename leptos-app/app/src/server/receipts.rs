#[cfg(feature = "ssr")]
use super::documents::{store_new_version, delete_document};
use leptos::*;
use chrono::{NaiveDate, Utc};
use shared::*;


#[server(name = GetReceipts, prefix = "/api", endpoint = "get_receipts")]
pub async fn get_receipts() -> Result<Vec<ReceiptListItem>, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
        
    let rows = sqlx::query!(
        r#"
        SELECT r.id, r.created_timestamp, r.receipt_number, r.receipt_date,
               c.id as "contact_id?", c.name as "contact_name?", c.first_name as "contact_first_name?"
        FROM receipt r
        LEFT JOIN contact c ON r.customer_contact_id = c.id
        ORDER BY r.id DESC
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
        
        ReceiptListItem {
            id: r.id as i64,
            created_timestamp: chrono::DateTime::from_timestamp(r.created_timestamp.unwrap_or_default().parse::<i64>().unwrap_or_default(), 0).unwrap_or(chrono::DateTime::<Utc>::MIN_UTC),
            supplier_contact: contact,
            paid_date: None,
            due_date: None,
            receipt_date: NaiveDate::parse_from_str(r.receipt_date.as_deref().unwrap_or(""), "%Y-%m-%d").ok(),
            receipt_number: r.receipt_number,
        }
    }).collect();
    
    Ok(items)
}

#[server(name = GetReceipt, prefix = "/api", endpoint = "get_receipt")]
pub async fn get_receipt(id: i64) -> Result<Receipt, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
        
    let id_i32 = id as i32;
    let r = sqlx::query!(
        "SELECT * FROM receipt WHERE id = $1", id_i32
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?
    .ok_or_else(|| ServerFnError::new("Receipt not found"))?;
    
    let items_rows = sqlx::query!(
        r#"
        SELECT ri.*, c.name as "category_name?", t.id as "type_id?", t.name as "type_name?"
        FROM receipt_item ri
        LEFT JOIN receipt_item_category c ON ri.category_id = c.id
        LEFT JOIN receipt_item_category_type t ON c.category_type_id = t.id
        WHERE ri.receipt_id = $1
        ORDER BY ri.position_number
        "#, id_i32
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    
    let items = items_rows.into_iter().map(|row| ReceiptItem {
        item: row.item,
        price: Money::new(row.price as i64),
        category: row.category_id.map(|cid| ReceiptItemCategory {
            id: cid as i64,
            name: row.category_name.clone().unwrap_or_default(),
            category_type: ReceiptItemCategoryType {
                id: row.type_id.unwrap_or_default() as i64,
                name: row.type_name.clone().unwrap_or_default(),
            },
        }),
    }).collect();
    
    let payments_rows = sqlx::query!(
        "SELECT * FROM receipt_payment WHERE receipt_id = $1", id_i32
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    
    let payments = payments_rows.into_iter().map(|row| Payment {
        date: NaiveDate::parse_from_str(&row.payment_date, "%Y-%m-%d").unwrap_or_default(),
        amount_cents: row.amount as i64,
    }).collect();
    
    let supplier = if let Some(ccid) = r.customer_contact_id {
        let c = sqlx::query!("SELECT * FROM contact WHERE id = $1", ccid)
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

    let doc = if let Some(did) = r.document_id {
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
    
    Ok(Receipt {
        id: Some(r.id as i64),
        items,
        created_timestamp: None,
        committed_timestamp: None,
        receipt_number: r.receipt_number.unwrap_or_default(),
        payments,
        receipt_date: NaiveDate::parse_from_str(r.receipt_date.as_deref().unwrap_or(""), "%Y-%m-%d").ok(),
        due_date: None,
        supplier_contact: supplier,
        document: doc,
        document_data: None,
    })
}

#[server(name = SaveReceipt, prefix = "/api", endpoint = "save_receipt")]
pub async fn save_receipt(receipt: Receipt) -> Result<Receipt, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
        
    let supplier_contact_id = receipt.supplier_contact.as_ref().and_then(|c| c.id).map(|id| id as i32);
    let receipt_date_str = receipt.receipt_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| Utc::now().naive_utc().date().format("%Y-%m-%d").to_string());
    
    let final_receipt = if let Some(id) = receipt.id {
        let id_i32 = id as i32;
        sqlx::query!(
            "UPDATE receipt SET receipt_number = $1, receipt_date = $2, customer_contact_id = $3 WHERE id = $4",
            receipt.receipt_number,
            receipt_date_str,
            supplier_contact_id,
            id_i32
        )
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        
        sqlx::query!("DELETE FROM receipt_item WHERE receipt_id = $1", id_i32)
            .execute(&pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
            
        id
    } else {
        let created_ts_str = Utc::now().timestamp().to_string();
        
        let row = sqlx::query!(
            "INSERT INTO receipt (receipt_number, receipt_date, customer_contact_id, created_timestamp, subject, recipient_name, street, house_number, zip_code, city, is_canceled) VALUES ($1, $2, $3, $4, 'Beleg', 'Supplier', '', '', '', '', 0) RETURNING id",
            receipt.receipt_number,
            receipt_date_str,
            supplier_contact_id,
            created_ts_str
        )
        .fetch_one(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        
        row.id as i64
    };
    
    // Insert items
    for (i, item) in receipt.items.iter().enumerate() {
        let pos_num = (i + 1) as i32;
        let item_price = item.price.amount_cents as i32;
        let item_category_id = item.category.as_ref().map(|c| c.id as i32);
        let final_receipt_i32 = final_receipt as i32;
        sqlx::query!(
            "INSERT INTO receipt_item (receipt_id, position_number, item, quantity, unit, price, total, category_id) VALUES ($1, $2, $3, 1.0, 'Stk', $4, $5, $6)",
            final_receipt_i32,
            pos_num,
            item.item,
            item_price,
            item_price,
            item_category_id
        )
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    }
    
    // Handle document upload and versioning
    let mut updated_doc_id = receipt.document.as_ref().map(|d| d.id as i32);
    if let Some(doc_data) = &receipt.document_data {
        if let Ok(bytes) = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &doc_data.data) {
            let prefix = format!("receipts/{}", final_receipt);
            let doc = store_new_version(&pool, updated_doc_id, &doc_data.extension, &doc_data.media_type, &prefix, &bytes).await?;
            updated_doc_id = Some(doc.id as i32);
        }
    } else if receipt.document.is_none() {
        if let Some(did) = updated_doc_id {
            delete_document(&pool, did).await?;
            updated_doc_id = None;
        }
    }

    sqlx::query!("UPDATE receipt SET document_id = $1 WHERE id = $2", updated_doc_id, final_receipt as i32)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    
    get_receipt(final_receipt).await
}

#[server(name = GetCategories, prefix = "/api", endpoint = "get_categories")]
pub async fn get_categories() -> Result<Vec<ReceiptItemCategory>, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
        
    let rows = sqlx::query!(
        r#"
        SELECT c.id, c.name, t.id as "type_id", t.name as "type_name"
        FROM receipt_item_category c
        JOIN receipt_item_category_type t ON c.category_type_id = t.id
        ORDER BY c.name
        "#
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    
    let list = rows.into_iter().map(|r| ReceiptItemCategory {
        id: r.id as i64,
        name: r.name,
        category_type: ReceiptItemCategoryType {
            id: r.type_id as i64,
            name: r.type_name,
        },
    }).collect();
    
    Ok(list)
}

#[server(name = AddReceiptPayment, prefix = "/api", endpoint = "add_receipt_payment")]
pub async fn add_receipt_payment(receipt_id: i64, amount_cents: i64, date: NaiveDate) -> Result<(), ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
    let date_str = date.format("%Y-%m-%d").to_string();
    let receipt_id_i32 = receipt_id as i32;
    let amount_cents_i32 = amount_cents as i32;
    sqlx::query!(
        "INSERT INTO receipt_payment (receipt_id, amount, payment_date) VALUES ($1, $2, $3)",
        receipt_id_i32,
        amount_cents_i32,
        date_str
    )
    .execute(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}

#[server(name = DeleteReceiptPayment, prefix = "/api", endpoint = "delete_receipt_payment")]
pub async fn delete_receipt_payment(id: i64) -> Result<(), ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
    let id_i32 = id as i32;
    sqlx::query!("DELETE FROM receipt_payment WHERE id = $1", id_i32)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    Ok(())
}
