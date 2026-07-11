use chrono::NaiveDate;
use leptos::*;
use shared::*;

#[cfg(feature = "ssr")]
use super::db::KlubuRepository;

#[server(name = GetInvoiceTextDefaults, prefix = "/api", endpoint = "get_invoice_text_defaults")]
pub async fn get_invoice_text_defaults(
    kind: String,
) -> Result<DocumentTextDefaults, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        Ok(crate::typst_gen::load_document_text_defaults(&kind))
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = kind;
        Err(ServerFnError::new(
            "Client side config access not supported",
        ))
    }
}

#[server(name = GetNotifications, prefix = "/api", endpoint = "get_notifications")]
pub async fn get_notifications() -> Result<Vec<Notification>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use sqlx::Row;
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        let today = chrono::Utc::now()
            .date_naive()
            .format("%Y-%m-%d")
            .to_string();
        let rows = sqlx::query(
            r#"SELECT i.id, i.invoice_number, i.due_date, i.subject,
                COALESCE((SELECT SUM(ii.total) FROM invoice_item ii WHERE ii.invoice_id = i.id), 0)
                - COALESCE((SELECT -SUM(cii.total) FROM invoice credit JOIN invoice_item cii ON cii.invoice_id = credit.id WHERE credit.corrected_invoice_id = i.id AND credit.is_cancelation = 1 AND credit.committed_timestamp IS NOT NULL), 0)
                - COALESCE((SELECT SUM(p.amount) FROM invoice_payment p WHERE p.invoice_id = i.id), 0) AS outstanding
               FROM invoice i
               WHERE i.committed_timestamp IS NOT NULL AND i.is_cancelation = 0
                 AND i.due_date IS NOT NULL AND i.due_date < $1
               ORDER BY i.due_date, i.id"#,
        )
        .bind(&today)
        .fetch_all(repo.pool())
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        rows.into_iter()
            .filter_map(|row| {
                let outstanding = row.try_get::<i64, _>("outstanding").ok()?;
                if outstanding <= 0 {
                    return None;
                }
                let id = row.try_get::<i64, _>("id").ok()?;
                let number = row
                    .try_get::<Option<i64>, _>("invoice_number")
                    .ok()
                    .flatten()
                    .unwrap_or(id);
                let date = NaiveDate::parse_from_str(
                    &row.try_get::<String, _>("due_date").ok()?,
                    "%Y-%m-%d",
                )
                .ok()?;
                Some(Ok(Notification {
                    kind: "invoice_due".to_string(),
                    title: format!("Rechnung #{number} ist fällig"),
                    detail: row
                        .try_get::<Option<String>, _>("subject")
                        .ok()
                        .flatten()
                        .unwrap_or_else(|| "Zahlung überfällig".to_string()),
                    href: format!("/invoices/{id}"),
                    date,
                    amount_cents: Some(outstanding),
                }))
            })
            .collect()
    }
    #[cfg(not(feature = "ssr"))]
    {
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[server(name = CreateInvoiceReminder, prefix = "/api", endpoint = "create_invoice_reminder")]
pub async fn create_invoice_reminder(
    invoice_id: i64,
    fee_cents: i64,
    note: String,
) -> Result<InvoiceReminder, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use sqlx::Row;
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        let invoice = repo.get_invoice(invoice_id).await?;
        let today = chrono::Utc::now().date_naive();
        if invoice.committed_timestamp.is_none() || invoice.is_cancelation || invoice.is_canceled {
            return Err(ServerFnError::new(
                "Nur aktive finalisierte Rechnungen können gemahnt werden",
            ));
        }
        if invoice.due_date.map(|date| date >= today).unwrap_or(true) {
            return Err(ServerFnError::new("Die Rechnung ist noch nicht fällig"));
        }
        let level: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(level), 0) + 1 FROM invoice_reminder WHERE invoice_id = $1",
        )
        .bind(invoice_id)
        .fetch_one(repo.pool())
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
        let created = chrono::Utc::now();
        let row = sqlx::query("INSERT INTO invoice_reminder (invoice_id, level, reminder_date, fee_cents, note, created_timestamp) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id")
            .bind(invoice_id).bind(level).bind(today.format("%Y-%m-%d").to_string()).bind(fee_cents.max(0)).bind(note.trim()).bind(created.timestamp().to_string())
            .fetch_one(repo.pool()).await.map_err(|e| ServerFnError::new(e.to_string()))?;
        Ok(InvoiceReminder {
            id: row
                .try_get("id")
                .map_err(|e| ServerFnError::new(e.to_string()))?,
            invoice_id,
            level,
            reminder_date: today,
            fee_cents: fee_cents.max(0),
            note: note.trim().to_string(),
            created_timestamp: created,
            sent_timestamp: None,
        })
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (invoice_id, fee_cents, note);
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[server(name = GetInvoices, prefix = "/api", endpoint = "get_invoices")]
pub async fn get_invoices(
    offset: u32,
    limit: u32,
    from_date: Option<NaiveDate>,
    to_date: Option<NaiveDate>,
    customer_contact_id: Option<i64>,
) -> Result<Page<InvoiceListItem>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        repo.get_invoices(offset, limit, from_date, to_date, customer_contact_id)
            .await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (offset, limit, from_date, to_date, customer_contact_id);
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[server(name = GetInvoice, prefix = "/api", endpoint = "get_invoice")]
pub async fn get_invoice(id: i64) -> Result<Invoice, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        repo.get_invoice(id).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = id;
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[server(name = SaveInvoice, prefix = "/api", endpoint = "save_invoice")]
pub async fn save_invoice(invoice: Invoice) -> Result<Invoice, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        repo.save_invoice(invoice).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = invoice;
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[server(name = CancelInvoice, prefix = "/api", endpoint = "cancel_invoice")]
pub async fn cancel_invoice(
    id: i64,
    amount_cents: Option<i64>,
    reason: Option<String>,
) -> Result<Invoice, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        let amount_cents = match amount_cents {
            Some(amount) => amount,
            None => {
                let invoice = repo.get_invoice(id).await?;
                invoice
                    .items
                    .iter()
                    .map(shared::Item::total_cents)
                    .sum::<i64>()
                    - invoice.credited_cents
            }
        };
        repo.cancel_invoice(id, amount_cents, reason.unwrap_or_default())
            .await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (id, amount_cents, reason);
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[server(name = AddInvoicePayment, prefix = "/api", endpoint = "add_invoice_payment")]
pub async fn add_invoice_payment(
    invoice_id: i64,
    amount_cents: i64,
    date: NaiveDate,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        repo.add_invoice_payment(invoice_id, amount_cents, date)
            .await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (invoice_id, amount_cents, date);
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[server(name = DeleteInvoicePayment, prefix = "/api", endpoint = "delete_invoice_payment")]
pub async fn delete_invoice_payment(id: i64) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        repo.delete_invoice_payment(id).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = id;
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[server(name = CommitInvoice, prefix = "/api", endpoint = "commit_invoice")]
pub async fn commit_invoice(id: i64) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        repo.commit_invoice(id).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = id;
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[server(name = DeleteInvoice, prefix = "/api", endpoint = "delete_invoice")]
pub async fn delete_invoice(id: i64) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        repo.delete_invoice(id).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = id;
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[server(name = SendInvoiceEmail, prefix = "/api", endpoint = "send_invoice_email")]
pub async fn send_invoice_email(
    invoice_id: i64,
    recipient: String,
    body: String,
    engagement_id: Option<i64>,
) -> Result<shared::EmailSummary, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use base64::Engine;
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        let actor = use_context::<super::db::CurrentUser>()
            .map(|user| user.0)
            .ok_or_else(|| ServerFnError::new("Kein angemeldeter Benutzer"))?;
        let invoice = repo.get_invoice(invoice_id).await?;
        let invoice_number = invoice.invoice_number.ok_or_else(|| {
            ServerFnError::new("Nur eine finalisierte Rechnung kann versendet werden")
        })?;
        // The storno itself (is_cancelation) is sendable; the voided original is
        // not — the customer must receive the Stornorechnung instead.
        if invoice.is_canceled {
            return Err(ServerFnError::new(
                "Eine stornierte Rechnung kann nicht versendet werden",
            ));
        }
        let pdf = crate::einvoice::render_invoice_pdf(&invoice).map_err(|error| {
            ServerFnError::new(format!(
                "Rechnung konnte nicht als PDF erzeugt werden: {error}"
            ))
        })?;
        let subject = invoice
            .subject
            .clone()
            .unwrap_or_else(|| format!("Rechnung #{invoice_number}"));
        let sent = super::email::send_composed_as_user(
            &repo,
            &actor,
            shared::ComposeEmail {
                to: recipient,
                cc: String::new(),
                bcc: String::new(),
                subject,
                body,
                attachments: vec![shared::EmailAttachment {
                    filename: format!("rechnung-{invoice_number}.pdf"),
                    media_type: "application/pdf".to_string(),
                    base64: base64::engine::general_purpose::STANDARD.encode(pdf),
                }],
                engagement_id,
            },
        )
        .await?;
        if let Some(engagement_id) = engagement_id {
            super::engagements::link_engagement_invoice(&repo, &actor, engagement_id, invoice_id)
                .await?;
        }
        Ok(sent)
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (invoice_id, recipient, body, engagement_id);
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}
