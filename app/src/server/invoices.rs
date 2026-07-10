use chrono::NaiveDate;
use leptos::*;
use shared::*;

#[cfg(feature = "ssr")]
use super::db::KlubuRepository;

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
pub async fn cancel_invoice(id: i64) -> Result<Invoice, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        repo.cancel_invoice(id).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = id;
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
