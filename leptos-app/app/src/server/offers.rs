use leptos::*;
use chrono::{NaiveDate, Utc};
use shared::*;


#[server(name = GetOffers, prefix = "/api", endpoint = "get_offers")]
pub async fn get_offers() -> Result<Vec<OfferListItem>, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
        
    let rows = sqlx::query!(
        r#"
        SELECT o.id, o.revision, o.title, o.created_timestamp, o.committed_timestamp, o.offer_number,
               c.id as "contact_id?", c.name as "contact_name?", c.first_name as "contact_first_name?"
        FROM offer o
        LEFT JOIN contact c ON o.customer_contact_id = c.id
        INNER JOIN (
            SELECT COALESCE(group_id, id) as gid, MAX(revision) as max_rev
            FROM offer
            GROUP BY COALESCE(group_id, id)
        ) latest ON COALESCE(o.group_id, o.id) = latest.gid AND o.revision = latest.max_rev
        ORDER BY o.id DESC
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
        
        OfferListItem {
            id: r.id as i64,
            revision: r.revision as i64,
            offer_number: r.offer_number.map(|num| num as i64),
            title: r.title,
            created_timestamp: chrono::DateTime::from_timestamp(r.created_timestamp.unwrap_or_default().parse::<i64>().unwrap_or_default(), 0).unwrap_or(chrono::DateTime::<Utc>::MIN_UTC),
            customer_contact: contact,
            committed: r.committed_timestamp.is_some(),
        }
    }).collect();
    
    Ok(items)
}

#[server(name = GetOffer, prefix = "/api", endpoint = "get_offer")]
pub async fn get_offer(id: i64) -> Result<Offer, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
        
    let id_i32 = id as i32;
    let o = sqlx::query!(
        "SELECT * FROM offer WHERE id = $1", id_i32
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?
    .ok_or_else(|| ServerFnError::new("Offer not found"))?;
    
    let items_rows = sqlx::query!(
        "SELECT * FROM offer_item WHERE offer_id = $1 AND offer_revision = $2 ORDER BY position_number", id_i32, o.revision
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
    
    let contact = if let Some(ccid) = o.customer_contact_id {
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

    let doc = if let Some(did) = o.document_id {
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
    
    Ok(Offer {
        id: Some(o.id as i64),
        revision: Some(o.revision as i64),
        offer_number: o.offer_number.map(|num| num as i64),
        title: o.title,
        customer_contact: contact,
        offer_date: o.offer_date.as_deref().and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()),
        valid_until_date: None,
        recipient: Some(Recipient {
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

#[server(name = SaveOffer, prefix = "/api", endpoint = "save_offer")]
pub async fn save_offer(offer: Offer) -> Result<Offer, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
        
    let recipient = offer.recipient.clone().unwrap_or(Recipient {
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
    
    let offer_date_str = offer.offer_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| Utc::now().naive_utc().date().format("%Y-%m-%d").to_string());
    
    let customer_contact_id = offer.customer_contact.as_ref().and_then(|c| c.id);
    let customer_contact_id_i32 = customer_contact_id.map(|id| id as i32);
    
    let final_offer = if let Some(id) = offer.id {
        let id_i32 = id as i32;
        
        // Check if already committed
        let committed_check = sqlx::query!(
            "SELECT committed_timestamp FROM offer WHERE id = $1", id_i32
        )
        .fetch_optional(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        
        if let Some(row) = committed_check {
            if row.committed_timestamp.is_some() {
                return Err(ServerFnError::new("Cannot modify a finalized offer"));
            }
        }
        
        sqlx::query!(
            "UPDATE offer SET offer_date = $1, subject = $2, title = $3, header_html = $4, footer_html = $5, recipient_name = $6, recipient_first_name = $7, recipient_title = $8, recipient_form_of_address = $9, street = $10, house_number = $11, zip_code = $12, city = $13, country = $14, customer_contact_id = $15 WHERE id = $16",
            offer_date_str,
            offer.subject,
            offer.title,
            offer.header_html,
            offer.footer_html,
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
        
        sqlx::query!("DELETE FROM offer_item WHERE offer_id = $1", id_i32)
            .execute(&pool)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
            
        id
    } else {
        let created_ts_str = Utc::now().timestamp().to_string();
        
        let row = sqlx::query!(
            "INSERT INTO offer (revision, offer_number, offer_date, subject, title, header_html, footer_html, recipient_name, recipient_first_name, recipient_title, recipient_form_of_address, street, house_number, zip_code, city, country, customer_contact_id, created_timestamp, committed_timestamp) VALUES (1, NULL, $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, NULL) RETURNING id",
            offer_date_str,
            offer.subject,
            offer.title,
            offer.header_html,
            offer.footer_html,
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
    
    let final_offer_i32 = final_offer as i32;
    // Fetch revision
    let revision = sqlx::query_scalar!("SELECT revision FROM offer WHERE id = $1", final_offer_i32)
        .fetch_one(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        
    let revision_i32 = revision as i32;
    
    // Insert offer items
    for (i, item) in offer.items.iter().enumerate() {
        let total = (item.quantity * item.price.amount_cents as f64) as i64;
        let pos_num = (i + 1) as i64;
        let item_price = item.price.amount_cents;
        
        let pos_num_i32 = pos_num as i32;
        let item_price_i32 = item_price as i32;
        let total_i32 = total as i32;
        
        sqlx::query!(
            "INSERT INTO offer_item (offer_id, offer_revision, position_number, item, quantity, unit, price, total) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            final_offer_i32,
            revision_i32,
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
    
    get_offer(final_offer).await
}

#[server(name = CommitOffer, prefix = "/api", endpoint = "commit_offer")]
pub async fn commit_offer(id: i64) -> Result<(), ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
    let id_i32 = id as i32;
    
    let row = sqlx::query!(
        "SELECT committed_timestamp, customer_contact_id FROM offer WHERE id = $1", id_i32
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?
    .ok_or_else(|| ServerFnError::new("Offer not found"))?;
    
    if row.committed_timestamp.is_some() {
        return Err(ServerFnError::new("Offer is already finalized"));
    }
    
    if row.customer_contact_id.is_none() {
        return Err(ServerFnError::new("Cannot finalize offer without an assigned customer contact"));
    }
    
    let next_number = sqlx::query_scalar!("SELECT COALESCE(MAX(offer_number), 0) FROM offer")
        .fetch_one(&pool)
        .await
        .unwrap_or(Some(0))
        .unwrap_or(0) + 1;
        
    let next_number_i32 = next_number as i32;
    let committed_ts = Utc::now().timestamp().to_string();
    
    sqlx::query!(
        "UPDATE offer SET offer_number = $1, committed_timestamp = $2 WHERE id = $3",
        next_number_i32,
        committed_ts,
        id_i32
    )
    .execute(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    
    Ok(())
}

#[server(name = DeleteOffer, prefix = "/api", endpoint = "delete_offer")]
pub async fn delete_offer(id: i64) -> Result<(), ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
    let id_i32 = id as i32;
    
    let row = sqlx::query!(
        "SELECT committed_timestamp FROM offer WHERE id = $1", id_i32
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?
    .ok_or_else(|| ServerFnError::new("Offer not found"))?;
    
    if row.committed_timestamp.is_some() {
        return Err(ServerFnError::new("Cannot delete a finalized offer"));
    }
    
    sqlx::query!("DELETE FROM offer_item WHERE offer_id = $1", id_i32)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        
    sqlx::query!("DELETE FROM offer WHERE id = $1", id_i32)
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        
    Ok(())
}

#[server(name = GetOfferRevisions, prefix = "/api", endpoint = "get_offer_revisions")]
pub async fn get_offer_revisions(offer_id: i64) -> Result<Vec<shared::OfferRevision>, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
        
    let id_i32 = offer_id as i32;
    let rows = sqlx::query!(
        r#"
        SELECT id, revision, created_timestamp
        FROM offer
        WHERE COALESCE(group_id, id) = (
            SELECT COALESCE(group_id, id) FROM offer WHERE id = $1
        )
        ORDER BY revision DESC
        "#,
        id_i32
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    
    let revisions = rows.into_iter().map(|r| {
        let ts_sec = r.created_timestamp.as_ref().and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
        shared::OfferRevision {
            id: r.id as i64,
            revision_number: r.revision as i64,
            creation_date: chrono::DateTime::from_timestamp(ts_sec, 0).unwrap_or(chrono::DateTime::<Utc>::MIN_UTC),
        }
    }).collect();
    
    Ok(revisions)
}

#[server(name = CreateOfferRevision, prefix = "/api", endpoint = "create_offer_revision")]
pub async fn create_offer_revision(offer_id: i64) -> Result<Offer, ServerFnError> {
    let pool = use_context::<sqlx::PgPool>()
        .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
        
    let id_i32 = offer_id as i32;
    let offer = get_offer(offer_id).await?;
    
    let parent_row = sqlx::query!(
        "SELECT group_id, revision FROM offer WHERE id = $1",
        id_i32
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?;
    
    let group_id = parent_row.group_id.unwrap_or(id_i32);
    
    let max_rev = sqlx::query_scalar!(
        "SELECT COALESCE(MAX(revision), 0) FROM offer WHERE id = $1 OR group_id = $1",
        group_id
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| ServerFnError::new(e.to_string()))?
    .unwrap_or(0);
    
    let new_revision = max_rev + 1;
    let created_ts_str = Utc::now().timestamp().to_string();
    
    let recipient = offer.recipient.clone().unwrap_or(Recipient {
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
    
    let offer_date_str = offer.offer_date.map(|d| d.format("%Y-%m-%d").to_string());
    let customer_contact_id_i32 = offer.customer_contact.as_ref().and_then(|c| c.id).map(|id| id as i32);
    
    let new_row = sqlx::query!(
        "INSERT INTO offer (group_id, revision, offer_number, offer_date, subject, title, header_html, footer_html, recipient_name, recipient_first_name, recipient_title, recipient_form_of_address, street, house_number, zip_code, city, country, customer_contact_id, created_timestamp, committed_timestamp) VALUES ($1, $2, NULL, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, NULL) RETURNING id",
        group_id,
        new_revision,
        offer_date_str,
        offer.subject,
        offer.title,
        offer.header_html,
        offer.footer_html,
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
    
    let new_id = new_row.id;
    
    for (i, item) in offer.items.iter().enumerate() {
        let total = (item.quantity * item.price.amount_cents as f64) as i64;
        let pos_num = (i + 1) as i64;
        let item_price = item.price.amount_cents;
        
        sqlx::query!(
            "INSERT INTO offer_item (offer_id, offer_revision, position_number, item, quantity, unit, price, total) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            new_id,
            new_revision,
            pos_num as i32,
            item.item,
            item.quantity,
            item.unit,
            item_price as i32,
            total as i32
        )
        .execute(&pool)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    }
    
    get_offer(new_id as i64).await
}
