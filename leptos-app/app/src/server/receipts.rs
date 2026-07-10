#[cfg(feature = "ssr")]
use super::documents::{delete_document, store_new_version};
use chrono::NaiveDate;
use leptos::server_fn::codec::Json;
use leptos::*;
use shared::*;

#[cfg(feature = "ssr")]
use super::db::KlubuRepository;

#[server(name = GetReceipts, prefix = "/api", endpoint = "get_receipts")]
pub async fn get_receipts(
    offset: u32,
    limit: u32,
    from_date: Option<NaiveDate>,
    to_date: Option<NaiveDate>,
) -> Result<Page<ReceiptListItem>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        repo.get_receipts(offset, limit, from_date, to_date).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (offset, limit, from_date, to_date);
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[server(name = GetReceipt, prefix = "/api", endpoint = "get_receipt")]
pub async fn get_receipt(id: i64) -> Result<Receipt, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        repo.get_receipt(id).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = id;
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[server(name = DeleteReceipt, prefix = "/api", endpoint = "delete_receipt")]
pub async fn delete_receipt(id: i64) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;

        let receipt = repo.get_receipt(id).await?;
        if let Some(doc) = receipt.document {
            let doc_id_i32 = doc.id as i32;
            delete_document(&repo, doc_id_i32).await?;
        }

        repo.delete_receipt(id).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = id;
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[server(name = SaveReceipt, prefix = "/api", endpoint = "save_receipt")]
pub async fn save_receipt(receipt: Receipt) -> Result<Receipt, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;

        let has_doc_data = receipt.document_data.is_some();
        let doc_data = receipt.document_data.clone();

        let mut saved = repo.save_receipt(receipt).await?;

        if has_doc_data {
            if let Some(doc) = doc_data {
                let bytes =
                    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &doc.data)
                        .map_err(|e| {
                        ServerFnError::new(format!("Datei konnte nicht dekodiert werden: {e}"))
                    })?;

                let doc_id = saved.document.as_ref().map(|d| d.id as i32);
                let prefix = format!("receipts/{}", saved.id.unwrap_or_default());
                let saved_doc = store_new_version(
                    &repo,
                    doc_id,
                    &doc.extension,
                    &doc.media_type,
                    &prefix,
                    &bytes,
                )
                .await?;

                let saved_doc_id = saved_doc.id as i32;
                repo.update_receipt_document(saved.id.unwrap_or_default(), saved_doc_id)
                    .await?;

                saved.document = Some(saved_doc);
            }
        }

        Ok(saved)
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = receipt;
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

/// Reads an uploaded document as an e-invoice and returns the fields it carries.
///
/// `Ok(None)` means "this is not an e-invoice" — a scan, a photo, a plain PDF.
/// The caller then falls back to the AI prefill, which guesses. This path does
/// not guess: an e-invoice states its number, date, supplier and line items, so
/// unlike the model it needs neither to be enabled nor to be right.
///
/// Advisory only: nothing is persisted here, the user confirms and saves.
// JSON input for the same reason as `prefill_receipt`: the document is base64.
#[server(name = ParseEInvoice, prefix = "/api", endpoint = "parse_einvoice", input = Json)]
pub async fn parse_einvoice(
    document: ReceiptDocumentData,
) -> Result<Option<ReceiptPrefill>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use base64::Engine;

        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&document.data)
            .map_err(|e| ServerFnError::new(format!("Datei konnte nicht dekodiert werden: {e}")))?;

        let media_type = document.media_type.clone();
        // lopdf and the XML parse are CPU-bound; keep them off the async runtime.
        let parsed = tokio::task::spawn_blocking(move || {
            crate::einvoice::parse_einvoice(&bytes, &media_type)
        })
        .await
        .map_err(|e| ServerFnError::new(format!("Auswertung abgebrochen: {e}")))?
        .map_err(ServerFnError::new)?;

        let Some(parsed) = parsed else {
            return Ok(None);
        };

        let mut prefill = parsed.prefill;
        let contacts = super::contacts::get_all_contacts().await?;
        prefill.supplier_contact = prefill
            .supplier_name
            .as_deref()
            .and_then(|n| super::ai::match_contact(n, &contacts));
        if let (Some(name), None) = (
            prefill.supplier_name.as_deref(),
            prefill.supplier_contact.as_ref(),
        ) {
            prefill.warnings.push(format!(
                "Kein Kontakt für Lieferant \"{name}\" gefunden. Bitte auswählen oder anlegen."
            ));
        }

        let origin = if parsed.from_pdf {
            "eingebettet in PDF"
        } else {
            "XML-Datei"
        };
        prefill.warnings.insert(
            0,
            format!("E-Rechnung erkannt: {} ({origin}). Die Werte stammen aus der Rechnung, nicht aus einer Schätzung.", parsed.syntax.label()),
        );

        Ok(Some(prefill))
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = document;
        Err(ServerFnError::new("Client side parsing not supported"))
    }
}

#[server(name = CommitReceipt, prefix = "/api", endpoint = "commit_receipt")]
pub async fn commit_receipt(id: i64) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        repo.commit_receipt(id).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = id;
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[server(name = GetCategories, prefix = "/api", endpoint = "get_categories")]
pub async fn get_categories() -> Result<Vec<ReceiptItemCategory>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        repo.get_categories().await
    }
    #[cfg(not(feature = "ssr"))]
    Err(ServerFnError::new("Client side DB access not supported"))
}

#[server(name = AddReceiptPayment, prefix = "/api", endpoint = "add_receipt_payment")]
pub async fn add_receipt_payment(
    receipt_id: i64,
    amount_cents: i64,
    date: NaiveDate,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        repo.add_receipt_payment(receipt_id, amount_cents, date)
            .await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (receipt_id, amount_cents, date);
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}

#[server(name = DeleteReceiptPayment, prefix = "/api", endpoint = "delete_receipt_payment")]
pub async fn delete_receipt_payment(id: i64) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        repo.delete_receipt_payment(id).await
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = id;
        Err(ServerFnError::new("Client side DB access not supported"))
    }
}
