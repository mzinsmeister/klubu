use leptos::*;
use shared::*;

#[cfg(feature = "ssr")]
use super::db::KlubuRepository;
#[cfg(feature = "ssr")]
use chrono::{DateTime, Utc};

#[server(name = GetContacts, prefix = "/api", endpoint = "get_contacts")]
pub async fn get_contacts(
    offset: u32,
    limit: u32,
    query: Option<String>,
) -> Result<Page<Contact>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found in context"))?;
        repo.get_contacts(offset, limit, query, false).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (offset, limit, query);
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

/// The Archiv view: archived contacts, restorable. Same shape as
/// `get_contacts`, just the other side of the `archived_timestamp` filter.
#[server(name = GetArchivedContacts, prefix = "/api", endpoint = "get_archived_contacts")]
pub async fn get_archived_contacts(
    offset: u32,
    limit: u32,
    query: Option<String>,
) -> Result<Page<Contact>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found in context"))?;
        repo.get_contacts(offset, limit, query, true).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (offset, limit, query);
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[server(name = GetContactCrm, prefix = "/api", endpoint = "get_contact_crm")]
pub async fn get_contact_crm(id: i64) -> Result<ContactCrmSummary, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use sqlx::Row;
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        let contact = repo.get_contact(id).await?;
        let actor = use_context::<super::db::CurrentUser>()
            .map(|user| user.0)
            .ok_or_else(|| ServerFnError::new("No authenticated user"))?;

        let note_rows = sqlx::query(
            "SELECT id, body, author_username, created_timestamp FROM contact_note WHERE contact_id = $1 ORDER BY created_timestamp DESC, id DESC LIMIT 100",
        )
        .bind(id)
        .fetch_all(repo.pool())
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        let notes = note_rows
            .iter()
            .map(|row| {
                let raw = row
                    .try_get::<String, _>("created_timestamp")
                    .map_err(|e| ServerFnError::new(e.to_string()))?;
                let timestamp = raw
                    .parse::<i64>()
                    .ok()
                    .and_then(|value| DateTime::from_timestamp(value, 0))
                    .ok_or_else(|| ServerFnError::new("Invalid note timestamp"))?;
                Ok(ContactNote {
                    id: row
                        .try_get("id")
                        .map_err(|e| ServerFnError::new(e.to_string()))?,
                    body: row
                        .try_get("body")
                        .map_err(|e| ServerFnError::new(e.to_string()))?,
                    author_username: row
                        .try_get("author_username")
                        .map_err(|e| ServerFnError::new(e.to_string()))?,
                    created_timestamp: timestamp,
                })
            })
            .collect::<Result<Vec<_>, ServerFnError>>()?;

        let mail_rows = sqlx::query(
            "SELECT mail.id, mail.mailbox, mail.message_id, mail.sender, mail.recipients, mail.subject, mail.sent_timestamp, mail.archived_timestamp, mail.flags, mail.raw_size, mail.delivery_status, mail.customer_contact_id, c.name AS customer_name FROM mail_message mail LEFT JOIN contact c ON c.id = mail.customer_contact_id WHERE mail.owner_username = $1 AND mail.customer_contact_id = $2 AND mail.deleted_timestamp IS NULL ORDER BY mail.id DESC LIMIT 20",
        )
        .bind(&actor)
        .bind(id)
        .fetch_all(repo.pool())
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        let recent_emails = mail_rows
            .iter()
            .map(super::email::row_summary)
            .collect::<Result<Vec<_>, _>>()?;

        // The CRM view shows micro-lists with "Alle anzeigen" links and separate
        // exact counts, so one filtered page of each suffices — no full scans.
        let offers = super::offers::get_offers(0, 100, None, None, Some(id))
            .await?
            .items;
        let invoices = super::invoices::get_invoices(0, 100, None, None, Some(id))
            .await?
            .items;
        // `list_engagements` only *prioritizes* the contact's engagements, so
        // within the first page every match sorts to the front; keep the filter.
        let engagements = super::engagements::list_engagements(0, 100, Some(id))
            .await?
            .items
            .into_iter()
            .filter(|engagement| engagement.customer_contact_id == Some(id))
            .collect::<Vec<_>>();

        let offer_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM offer WHERE customer_contact_id = $1",
        )
        .bind(id)
        .fetch_one(repo.pool())
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        let invoice_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM invoice WHERE customer_contact_id = $1",
        )
        .bind(id)
        .fetch_one(repo.pool())
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        let engagement_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM engagement WHERE customer_contact_id = $1",
        )
        .bind(id)
        .fetch_one(repo.pool())
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

        Ok(ContactCrmSummary {
            contact,
            notes,
            recent_emails,
            offers,
            invoices,
            engagements,
            offer_count,
            invoice_count,
            engagement_count,
        })
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = id;
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[server(name = AddContactNote, prefix = "/api", endpoint = "add_contact_note")]
pub async fn add_contact_note(contact_id: i64, body: String) -> Result<ContactNote, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use sqlx::Row;
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        let actor = use_context::<super::db::CurrentUser>()
            .map(|user| user.0)
            .ok_or_else(|| ServerFnError::new("No authenticated user"))?;
        let body = body.trim().to_string();
        if body.is_empty() || body.len() > 20_000 {
            return Err(ServerFnError::new(
                "Note must contain 1 to 20,000 characters",
            ));
        }
        let timestamp = Utc::now().timestamp().to_string();
        let mut tx = repo
            .pool()
            .begin()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        let row = sqlx::query(
            "INSERT INTO contact_note (contact_id, author_username, body, created_timestamp) VALUES ($1, $2, $3, $4) RETURNING id",
        )
        .bind(contact_id)
        .bind(&actor)
        .bind(&body)
        .bind(&timestamp)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        let id = row
            .try_get::<i64, _>("id")
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        sqlx::query(
            "INSERT INTO audit_log (entity_name, entity_id, action, timestamp, user_name, changes) VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind("contact_note")
        .bind(id)
        .bind("create")
        .bind(&timestamp)
        .bind(&actor)
        .bind(serde_json::json!({"contact_id": contact_id, "body": body}).to_string())
        .execute(&mut *tx)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        tx.commit()
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(ContactNote {
            id,
            body,
            author_username: actor,
            created_timestamp: DateTime::from_timestamp(timestamp.parse().unwrap_or_default(), 0)
                .ok_or_else(|| ServerFnError::new("Invalid note timestamp"))?,
        })
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (contact_id, body);
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

/// Load every contact in bounded chunks for server-side matching/import paths.
/// Interactive lists stay paginated; this helper is reserved for operations
/// which must consider every possible supplier/customer match.
pub(crate) async fn get_all_contacts() -> Result<Vec<Contact>, ServerFnError> {
    const CHUNK_SIZE: u32 = 200;
    let mut contacts = Vec::new();

    loop {
        let page = get_contacts(contacts.len() as u32, CHUNK_SIZE, None).await?;
        let received = page.items.len();
        contacts.extend(page.items);
        if !page.has_more || received == 0 {
            return Ok(contacts);
        }
    }
}

#[server(name = SaveContact, prefix = "/api", endpoint = "save_contact")]
pub async fn save_contact(contact: Contact) -> Result<Contact, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        repo.save_contact(contact).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = contact;
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

/// Contacts are archived, never deleted: the id is the Kundennummer printed on
/// committed invoices and must stay resolvable.
#[server(name = ArchiveContact, prefix = "/api", endpoint = "archive_contact")]
pub async fn archive_contact(id: i64) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        repo.archive_contact(id).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = id;
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[server(name = RestoreContact, prefix = "/api", endpoint = "restore_contact")]
pub async fn restore_contact(id: i64) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        repo.restore_contact(id).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = id;
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}
