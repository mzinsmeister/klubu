use chrono::NaiveDate;
#[cfg(feature = "ssr")]
use chrono::Utc;
use leptos::*;
use shared::*;

#[cfg(feature = "ssr")]
use super::db::KlubuRepository;

#[server(name = GetOffers, prefix = "/api", endpoint = "get_offers")]
pub async fn get_offers(
    offset: u32,
    limit: u32,
    from_date: Option<NaiveDate>,
    to_date: Option<NaiveDate>,
    customer_contact_id: Option<i64>,
) -> Result<Page<OfferListItem>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        repo.get_offers(offset, limit, from_date, to_date, customer_contact_id)
            .await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (offset, limit, from_date, to_date, customer_contact_id);
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[server(name = GetOffer, prefix = "/api", endpoint = "get_offer")]
pub async fn get_offer(id: i64) -> Result<Offer, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        repo.get_offer(id).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = id;
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[server(name = SaveOffer, prefix = "/api", endpoint = "save_offer")]
pub async fn save_offer(offer: Offer) -> Result<Offer, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        repo.save_offer(offer).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = offer;
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[server(name = CommitOffer, prefix = "/api", endpoint = "commit_offer")]
pub async fn commit_offer(id: i64) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        repo.commit_offer(id).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = id;
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[server(name = DeleteOffer, prefix = "/api", endpoint = "delete_offer")]
pub async fn delete_offer(id: i64) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        repo.delete_offer(id).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = id;
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[server(name = GetOfferRevisions, prefix = "/api", endpoint = "get_offer_revisions")]
pub async fn get_offer_revisions(
    offer_id: i64,
) -> Result<Vec<shared::OfferRevision>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        repo.get_offer_revisions(offer_id).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = offer_id;
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[server(name = CreateOfferRevision, prefix = "/api", endpoint = "create_offer_revision")]
pub async fn create_offer_revision(offer_id: i64) -> Result<Offer, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        repo.create_offer_revision(offer_id).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = offer_id;
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

/// Creates a new invoice draft from a finalized offer revision. The offer is
/// never consumed or changed; its items, recipient and texts are copied into a
/// normal editable invoice draft. Finalizing the invoice remains a separate,
/// explicit operation.
#[server(name = CreateInvoiceFromOffer, prefix = "/api", endpoint = "create_invoice_from_offer")]
pub async fn create_invoice_from_offer(
    offer_id: i64,
    engagement_id: Option<i64>,
) -> Result<Invoice, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        let actor = use_context::<super::db::CurrentUser>()
            .map(|user| user.0)
            .ok_or_else(|| ServerFnError::new("Kein angemeldeter Benutzer"))?;
        let offer = repo.get_offer(offer_id).await?;
        if offer.committed_timestamp.is_none() {
            return Err(ServerFnError::new(
                "Nur ein finalisiertes Angebot kann in eine Rechnung übernommen werden",
            ));
        }
        let subject = offer
            .offer_number
            .map(|number| format!("Rechnung zu Angebot #{number}"))
            .or_else(|| offer.subject.clone())
            .unwrap_or_else(|| "Rechnung aus Angebot".to_string());
        let invoice = repo
            .save_invoice(Invoice {
                id: None,
                items: offer.items.clone(),
                created_timestamp: None,
                committed_timestamp: None,
                invoice_number: None,
                payments: Vec::new(),
                invoice_date: Some(Utc::now().naive_utc().date()),
                is_canceled: false,
                is_cancelation: false,
                corrected_invoice_id: None,
                cancellation_invoice_id: None,
                customer_contact: offer.customer_contact.clone(),
                document: None,
                recipient: offer.recipient.clone(),
                header: offer.header.clone(),
                footer: offer.footer.clone(),
                title: Some("Rechnung".to_string()),
                subject: Some(subject),
            })
            .await?;
        let invoice_id = invoice
            .id
            .ok_or_else(|| ServerFnError::new("New invoice has no ID"))?;
        let engagement_ids = if let Some(engagement_id) = engagement_id {
            vec![engagement_id]
        } else {
            sqlx::query_scalar::<_, i64>(
                "SELECT engagement_id FROM engagement_offer WHERE offer_id = $1 ORDER BY created_timestamp, engagement_id",
            )
            .bind(offer_id)
            .fetch_all(repo.pool())
            .await
            .map_err(|error| ServerFnError::new(error.to_string()))?
        };
        for engagement_id in engagement_ids {
            super::engagements::link_engagement_offer(&repo, &actor, engagement_id, offer_id)
                .await?;
            super::engagements::link_engagement_invoice(&repo, &actor, engagement_id, invoice_id)
                .await?;
        }
        Ok(invoice)
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (offer_id, engagement_id);
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[server(name = SendOfferEmail, prefix = "/api", endpoint = "send_offer_email")]
pub async fn send_offer_email(
    offer_id: i64,
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
        let offer = repo.get_offer(offer_id).await?;
        let offer_number = offer.offer_number.ok_or_else(|| {
            ServerFnError::new("Nur ein finalisiertes Angebot kann versendet werden")
        })?;
        let pdf = crate::pdf::compiler::compile_typst_pdfa(crate::typst_gen::generate_offer_typst(
            &offer,
        ))
        .map_err(|error| {
            ServerFnError::new(format!(
                "Angebot konnte nicht als PDF erzeugt werden: {error}"
            ))
        })?;
        let subject = offer
            .subject
            .clone()
            .unwrap_or_else(|| format!("Angebot #{offer_number}"));
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
                    filename: format!("angebot-{offer_number}.pdf"),
                    media_type: "application/pdf".to_string(),
                    base64: base64::engine::general_purpose::STANDARD.encode(pdf),
                }],
                engagement_id,
            },
        )
        .await?;
        if let Some(engagement_id) = engagement_id {
            super::engagements::link_engagement_offer(&repo, &actor, engagement_id, offer_id)
                .await?;
        }
        Ok(sent)
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (offer_id, recipient, body, engagement_id);
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}
