use leptos::*;
use shared::{Engagement, EngagementInput, EngagementLinkKind, EngagementListItem, Page};

#[cfg(feature = "ssr")]
use chrono::{DateTime, Utc};
#[cfg(feature = "ssr")]
use shared::{Contact, EngagementLink};

#[cfg(feature = "ssr")]
use super::db::{repository::load_contact_emails, ActiveRepository};

#[cfg(feature = "ssr")]
fn db_error(context: &'static str) -> impl FnOnce(sqlx::Error) -> ServerFnError {
    move |error| ServerFnError::new(format!("{context}: {error}"))
}

#[cfg(feature = "ssr")]
fn current_user() -> Result<String, ServerFnError> {
    use_context::<super::db::CurrentUser>()
        .map(|user| user.0)
        .ok_or_else(|| ServerFnError::new("No authenticated user"))
}

#[cfg(feature = "ssr")]
fn timestamp(raw: &str) -> Result<DateTime<Utc>, ServerFnError> {
    raw.parse::<i64>()
        .ok()
        .and_then(|value| DateTime::from_timestamp(value, 0))
        .ok_or_else(|| ServerFnError::new(format!("Invalid engagement timestamp: {raw}")))
}

#[cfg(feature = "ssr")]
fn contact_from_row(row: &sqlx::any::AnyRow) -> Result<Contact, ServerFnError> {
    use sqlx::Row;
    let archived = row
        .try_get::<Option<String>, _>("archived_timestamp")
        .map_err(|error| ServerFnError::new(error.to_string()))?
        .and_then(|value| value.parse::<i64>().ok())
        .and_then(|value| DateTime::from_timestamp(value, 0));
    Ok(Contact {
        id: Some(
            row.try_get::<i64, _>("id")
                .map_err(|error| ServerFnError::new(error.to_string()))?,
        ),
        form_of_address: row
            .try_get("form_of_address")
            .map_err(|error| ServerFnError::new(error.to_string()))?,
        title: row
            .try_get("title")
            .map_err(|error| ServerFnError::new(error.to_string()))?,
        name: row
            .try_get("name")
            .map_err(|error| ServerFnError::new(error.to_string()))?,
        first_name: row
            .try_get("first_name")
            .map_err(|error| ServerFnError::new(error.to_string()))?,
        street: row
            .try_get("street")
            .map_err(|error| ServerFnError::new(error.to_string()))?,
        zip_code: row
            .try_get("zip_code")
            .map_err(|error| ServerFnError::new(error.to_string()))?,
        city: row
            .try_get("city")
            .map_err(|error| ServerFnError::new(error.to_string()))?,
        house_number: row
            .try_get("house_number")
            .map_err(|error| ServerFnError::new(error.to_string()))?,
        country: row
            .try_get("country")
            .map_err(|error| ServerFnError::new(error.to_string()))?,
        phones: row
            .try_get::<Option<String>, _>("phone")
            .map_err(|error| ServerFnError::new(error.to_string()))?
            .map(|val| vec![val])
            .unwrap_or_default(),
        is_person: row
            .try_get::<i64, _>("is_person")
            .map_err(|error| ServerFnError::new(error.to_string()))?
            != 0,
        archived_timestamp: archived,
        emails: Vec::new(),
    })
}

#[cfg(feature = "ssr")]
async fn load_engagement(
    repo: &ActiveRepository,
    actor: &str,
    id: i64,
) -> Result<Engagement, ServerFnError> {
    use sqlx::Row;
    let row = sqlx::query(
        "SELECT id, title, description, customer_contact_id, created_timestamp FROM engagement WHERE id = $1 AND created_by = $2",
    )
    .bind(id)
    .bind(actor)
    .fetch_optional(repo.pool())
    .await
    .map_err(db_error("Could not load engagement"))?
    .ok_or_else(|| ServerFnError::new("Engagement not found"))?;

    let mut customer_contact = match row
        .try_get::<Option<i64>, _>("customer_contact_id")
        .map_err(|error| ServerFnError::new(error.to_string()))?
    {
        Some(contact_id) => sqlx::query("SELECT * FROM contact WHERE id = $1")
            .bind(contact_id)
            .fetch_optional(repo.pool())
            .await
            .map_err(db_error("Could not load engagement contact"))?
            .map(|value| contact_from_row(&value))
            .transpose()?,
        None => None,
    };
    if let Some(contact) = &mut customer_contact {
        if let Some(contact_id) = contact.id {
            contact.emails = load_contact_emails(repo.pool(), contact_id).await?;
        }
    }

    let mut links = Vec::new();
    let offers = sqlx::query(
        "SELECT o.id, o.offer_number, o.revision, o.subject, o.committed_timestamp FROM engagement_offer link JOIN offer o ON o.id = link.offer_id WHERE link.engagement_id = $1 ORDER BY link.created_timestamp DESC",
    )
    .bind(id)
    .fetch_all(repo.pool())
    .await
    .map_err(db_error("Could not load engagement offers"))?;
    for offer in offers {
        let offer_id = offer
            .try_get::<i64, _>("id")
            .map_err(|error| ServerFnError::new(error.to_string()))?;
        let number = offer
            .try_get::<Option<i64>, _>("offer_number")
            .map_err(|error| ServerFnError::new(error.to_string()))?;
        let revision = offer
            .try_get::<i64, _>("revision")
            .map_err(|error| ServerFnError::new(error.to_string()))?;
        let subject = offer
            .try_get::<Option<String>, _>("subject")
            .map_err(|error| ServerFnError::new(error.to_string()))?;
        let label = number
            .map(|number| format!("Offer #{number}"))
            .or(subject)
            .unwrap_or_else(|| format!("Offer #{offer_id}"));
        let status = if offer
            .try_get::<Option<String>, _>("committed_timestamp")
            .map_err(|error| ServerFnError::new(error.to_string()))?
            .is_some()
        {
            "finalisiert"
        } else {
            "Entwurf"
        };
        links.push(EngagementLink {
            kind: EngagementLinkKind::Offer,
            id: offer_id,
            label: format!("{label} (Rev. {revision})"),
            status: status.to_string(),
        });
    }

    let invoices = sqlx::query(
        "SELECT i.id, i.invoice_number, i.subject, i.committed_timestamp, i.is_canceled FROM engagement_invoice link JOIN invoice i ON i.id = link.invoice_id WHERE link.engagement_id = $1 ORDER BY link.created_timestamp DESC",
    )
    .bind(id)
    .fetch_all(repo.pool())
    .await
    .map_err(db_error("Could not load engagement invoices"))?;
    for invoice in invoices {
        let invoice_id = invoice
            .try_get::<i64, _>("id")
            .map_err(|error| ServerFnError::new(error.to_string()))?;
        let number = invoice
            .try_get::<Option<i64>, _>("invoice_number")
            .map_err(|error| ServerFnError::new(error.to_string()))?;
        let subject = invoice
            .try_get::<Option<String>, _>("subject")
            .map_err(|error| ServerFnError::new(error.to_string()))?;
        let label = number
            .map(|number| format!("Invoice #{number}"))
            .or(subject)
            .unwrap_or_else(|| format!("Invoice #{invoice_id}"));
        let status = if invoice
            .try_get::<i64, _>("is_canceled")
            .map_err(|error| ServerFnError::new(error.to_string()))?
            != 0
        {
            "storniert"
        } else if invoice
            .try_get::<Option<String>, _>("committed_timestamp")
            .map_err(|error| ServerFnError::new(error.to_string()))?
            .is_some()
        {
            "finalisiert"
        } else {
            "Entwurf"
        };
        links.push(EngagementLink {
            kind: EngagementLinkKind::Invoice,
            id: invoice_id,
            label,
            status: status.to_string(),
        });
    }

    let mails = sqlx::query(
        "SELECT mail.id, mail.subject, mail.sender, mail.delivery_status FROM engagement_mail link JOIN mail_message mail ON mail.id = link.mail_message_id WHERE link.engagement_id = $1 AND mail.owner_username = $2 ORDER BY link.created_timestamp DESC",
    )
    .bind(id)
    .bind(actor)
    .fetch_all(repo.pool())
    .await
    .map_err(db_error("Could not load engagement emails"))?;
    for mail in mails {
        let mail_id = mail
            .try_get::<i64, _>("id")
            .map_err(|error| ServerFnError::new(error.to_string()))?;
        let subject = mail
            .try_get::<String, _>("subject")
            .map_err(|error| ServerFnError::new(error.to_string()))?;
        let sender = mail
            .try_get::<String, _>("sender")
            .map_err(|error| ServerFnError::new(error.to_string()))?;
        let label = if subject.trim().is_empty() {
            sender
        } else {
            subject
        };
        let status = mail
            .try_get::<String, _>("delivery_status")
            .map_err(|error| ServerFnError::new(error.to_string()))?;
        links.push(EngagementLink {
            kind: EngagementLinkKind::Email,
            id: mail_id,
            label,
            status,
        });
    }

    Ok(Engagement {
        id: Some(
            row.try_get::<i64, _>("id")
                .map_err(|error| ServerFnError::new(error.to_string()))?,
        ),
        title: row
            .try_get("title")
            .map_err(|error| ServerFnError::new(error.to_string()))?,
        description: row
            .try_get("description")
            .map_err(|error| ServerFnError::new(error.to_string()))?,
        customer_contact,
        created_timestamp: Some(timestamp(
            &row.try_get::<String, _>("created_timestamp")
                .map_err(|error| ServerFnError::new(error.to_string()))?,
        )?),
        links,
    })
}

#[cfg(feature = "ssr")]
pub async fn link_engagement_offer(
    repo: &ActiveRepository,
    actor: &str,
    engagement_id: i64,
    offer_id: i64,
) -> Result<(), ServerFnError> {
    link_record(
        repo,
        actor,
        engagement_id,
        offer_id,
        EngagementLinkKind::Offer,
    )
    .await
}

#[cfg(feature = "ssr")]
pub async fn link_engagement_invoice(
    repo: &ActiveRepository,
    actor: &str,
    engagement_id: i64,
    invoice_id: i64,
) -> Result<(), ServerFnError> {
    link_record(
        repo,
        actor,
        engagement_id,
        invoice_id,
        EngagementLinkKind::Invoice,
    )
    .await
}

#[cfg(feature = "ssr")]
pub async fn link_engagement_mail(
    repo: &ActiveRepository,
    actor: &str,
    engagement_id: i64,
    mail_id: i64,
) -> Result<(), ServerFnError> {
    link_record(
        repo,
        actor,
        engagement_id,
        mail_id,
        EngagementLinkKind::Email,
    )
    .await
}

#[cfg(feature = "ssr")]
async fn link_record(
    repo: &ActiveRepository,
    actor: &str,
    engagement_id: i64,
    record_id: i64,
    kind: EngagementLinkKind,
) -> Result<(), ServerFnError> {
    let kind_name = match kind {
        EngagementLinkKind::Offer => "offer",
        EngagementLinkKind::Invoice => "invoice",
        EngagementLinkKind::Email => "mail",
    };
    let exists = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM engagement WHERE id = $1 AND created_by = $2",
    )
    .bind(engagement_id)
    .bind(actor)
    .fetch_one(repo.pool())
    .await
    .map_err(db_error("Could not verify engagement"))?;
    if exists == 0 {
        return Err(ServerFnError::new("Engagement not found"));
    }
    let target_exists = match kind {
        EngagementLinkKind::Offer => {
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM offer WHERE id = $1")
                .bind(record_id)
                .fetch_one(repo.pool())
                .await
        }
        EngagementLinkKind::Invoice => {
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM invoice WHERE id = $1")
                .bind(record_id)
                .fetch_one(repo.pool())
                .await
        }
        EngagementLinkKind::Email => {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM mail_message WHERE id = $1 AND owner_username = $2",
            )
            .bind(record_id)
            .bind(actor)
            .fetch_one(repo.pool())
            .await
        }
    }
    .map_err(db_error("Could not verify engagement target"))?;
    if target_exists == 0 {
        return Err(ServerFnError::new("Linked record not found"));
    }
    let timestamp = Utc::now().timestamp().to_string();
    let mut tx = repo
        .pool()
        .begin()
        .await
        .map_err(db_error("Could not begin engagement link"))?;
    let inserted = match kind {
        EngagementLinkKind::Offer => sqlx::query("INSERT INTO engagement_offer (engagement_id, offer_id, created_timestamp) VALUES ($1, $2, $3) ON CONFLICT (engagement_id, offer_id) DO NOTHING").bind(engagement_id).bind(record_id).bind(&timestamp).execute(&mut *tx).await,
        EngagementLinkKind::Invoice => sqlx::query("INSERT INTO engagement_invoice (engagement_id, invoice_id, created_timestamp) VALUES ($1, $2, $3) ON CONFLICT (engagement_id, invoice_id) DO NOTHING").bind(engagement_id).bind(record_id).bind(&timestamp).execute(&mut *tx).await,
        EngagementLinkKind::Email => sqlx::query("INSERT INTO engagement_mail (engagement_id, mail_message_id, created_timestamp) VALUES ($1, $2, $3) ON CONFLICT (engagement_id, mail_message_id) DO NOTHING").bind(engagement_id).bind(record_id).bind(&timestamp).execute(&mut *tx).await,
    }.map_err(db_error("Could not save engagement link"))?;
    if inserted.rows_affected() > 0 {
        sqlx::query("INSERT INTO audit_log (entity_name, entity_id, action, timestamp, user_name, changes) VALUES ($1, $2, $3, $4, $5, $6)")
            .bind("engagement").bind(engagement_id).bind("link").bind(&timestamp).bind(actor)
            .bind(serde_json::json!({"kind": kind_name, "record_id": record_id}).to_string())
            .execute(&mut *tx).await.map_err(db_error("Could not audit engagement link"))?;
    }
    tx.commit()
        .await
        .map_err(db_error("Could not commit engagement link"))
}

#[server(name = ListEngagements, prefix = "/api", endpoint = "list_engagements")]
pub async fn list_engagements(
    offset: u32,
    limit: u32,
    prioritize_customer_contact_id: Option<i64>,
) -> Result<Page<EngagementListItem>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use sqlx::Row;
        let repo = use_context::<ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        let actor = current_user()?;
        let limit = limit.clamp(1, 100);
        let rows = sqlx::query("SELECT a.id, a.title, a.description, a.customer_contact_id, a.created_timestamp, c.name AS customer_name, (SELECT COUNT(*) FROM engagement_offer x WHERE x.engagement_id = a.id) AS offer_count, (SELECT COUNT(*) FROM engagement_invoice x WHERE x.engagement_id = a.id) AS invoice_count, (SELECT COUNT(*) FROM engagement_mail x WHERE x.engagement_id = a.id) AS email_count FROM engagement a LEFT JOIN contact c ON c.id = a.customer_contact_id WHERE a.created_by = $1 AND a.archived_timestamp IS NULL ORDER BY CASE WHEN $4 IS NOT NULL AND a.customer_contact_id = $4 THEN 0 ELSE 1 END, a.id DESC LIMIT $2 OFFSET $3")
            .bind(&actor).bind(i64::from(limit) + 1).bind(i64::from(offset)).bind(prioritize_customer_contact_id).fetch_all(repo.pool()).await
            .map_err(db_error("Could not load engagements"))?;
        let mut items = rows
            .iter()
            .map(|row| -> Result<EngagementListItem, ServerFnError> {
                Ok(EngagementListItem {
                    id: row
                        .try_get("id")
                        .map_err(|error| ServerFnError::new(error.to_string()))?,
                    title: row
                        .try_get("title")
                        .map_err(|error| ServerFnError::new(error.to_string()))?,
                    description: row
                        .try_get("description")
                        .map_err(|error| ServerFnError::new(error.to_string()))?,
                    customer_name: row
                        .try_get("customer_name")
                        .map_err(|error| ServerFnError::new(error.to_string()))?,
                    customer_contact_id: row
                        .try_get("customer_contact_id")
                        .map_err(|error| ServerFnError::new(error.to_string()))?,
                    created_timestamp: timestamp(
                        &row.try_get::<String, _>("created_timestamp")
                            .map_err(|error| ServerFnError::new(error.to_string()))?,
                    )?,
                    offer_count: row
                        .try_get("offer_count")
                        .map_err(|error| ServerFnError::new(error.to_string()))?,
                    invoice_count: row
                        .try_get("invoice_count")
                        .map_err(|error| ServerFnError::new(error.to_string()))?,
                    email_count: row
                        .try_get("email_count")
                        .map_err(|error| ServerFnError::new(error.to_string()))?,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let has_more = items.len() > limit as usize;
        items.truncate(limit as usize);
        Ok(Page { items, has_more })
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (offset, limit, prioritize_customer_contact_id);
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[server(name = GetEngagement, prefix = "/api", endpoint = "get_engagement")]
pub async fn get_engagement(id: i64) -> Result<Engagement, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        load_engagement(&repo, &current_user()?, id).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = id;
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[server(name = SaveEngagement, prefix = "/api", endpoint = "save_engagement")]
pub async fn save_engagement(input: EngagementInput) -> Result<Engagement, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use sqlx::Row;
        let repo = use_context::<ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        let actor = current_user()?;
        let title = input.title.trim().to_string();
        if title.is_empty() {
            return Err(ServerFnError::new("Engagement requires a title"));
        }
        let timestamp = Utc::now().timestamp().to_string();
        let mut tx = repo
            .pool()
            .begin()
            .await
            .map_err(db_error("Could not begin engagement"))?;
        let id = if let Some(id) = input.id {
            let updated = sqlx::query("UPDATE engagement SET title = $1, description = $2, customer_contact_id = $3 WHERE id = $4 AND created_by = $5")
                .bind(&title).bind(&input.description).bind(input.customer_contact_id).bind(id).bind(&actor).execute(&mut *tx).await
                .map_err(db_error("Could not save engagement"))?;
            if updated.rows_affected() == 0 {
                return Err(ServerFnError::new("Engagement not found"));
            }
            sqlx::query("INSERT INTO audit_log (entity_name, entity_id, action, timestamp, user_name, changes) VALUES ($1, $2, $3, $4, $5, $6)")
                .bind("engagement").bind(id).bind("update").bind(&timestamp).bind(&actor).bind(serde_json::json!({"title": title, "description": input.description, "customer_contact_id": input.customer_contact_id}).to_string())
                .execute(&mut *tx).await.map_err(db_error("Could not audit engagement"))?;
            id
        } else {
            let row = sqlx::query("INSERT INTO engagement (title, description, created_by, customer_contact_id, created_timestamp) VALUES ($1, $2, $3, $4, $5) RETURNING id")
                .bind(&title).bind(&input.description).bind(&actor).bind(input.customer_contact_id).bind(&timestamp).fetch_one(&mut *tx).await
                .map_err(db_error("Could not create engagement"))?;
            let id = row
                .try_get::<i64, _>("id")
                .map_err(|error| ServerFnError::new(error.to_string()))?;
            sqlx::query("INSERT INTO audit_log (entity_name, entity_id, action, timestamp, user_name, changes) VALUES ($1, $2, $3, $4, $5, $6)")
                .bind("engagement").bind(id).bind("create").bind(&timestamp).bind(&actor).bind(serde_json::json!({"title": title, "description": input.description, "customer_contact_id": input.customer_contact_id}).to_string())
                .execute(&mut *tx).await.map_err(db_error("Could not audit engagement"))?;
            id
        };
        tx.commit()
            .await
            .map_err(db_error("Could not commit engagement"))?;
        load_engagement(&repo, &actor, id).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = input;
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[server(name = LinkEngagement, prefix = "/api", endpoint = "link_engagement")]
pub async fn link_engagement(
    id: i64,
    kind: EngagementLinkKind,
    record_id: i64,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        let actor = current_user()?;
        link_record(&repo, &actor, id, record_id, kind).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (id, kind, record_id);
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}
