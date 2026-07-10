use leptos::*;

// Only the server half of these `#[server]` fns touches the DB and the PDF
// pipeline; on the client they compile down to HTTP calls.
#[cfg(feature = "ssr")]
use super::db::KlubuRepository;
#[cfg(feature = "ssr")]
use super::invoices::get_invoice;
#[cfg(feature = "ssr")]
use super::offers::get_offer;
#[cfg(feature = "ssr")]
use crate::typst_gen::generate_offer_typst;

#[server(name = ExportInvoicePdf, prefix = "/api", endpoint = "export_invoice_pdf")]
pub async fn export_invoice_pdf(invoice_id: i64) -> Result<shared::Document, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let res = async {
            let invoice = get_invoice(invoice_id).await?;
            if invoice.committed_timestamp.is_none() {
                return Err(ServerFnError::new("Can only export committed invoices"));
            }

            let repo = use_context::<super::db::ActiveRepository>()
                .ok_or_else(|| ServerFnError::new("Repository not found"))?;
            // A committed invoice is archived as a ZUGFeRD PDF: PDF/A-3b with the
            // EN 16931 XML embedded. It stays a perfectly ordinary PDF to read.
            let bytes = crate::einvoice::render_invoice_pdf(&invoice)
                .map_err(|e| ServerFnError::new(format!("Typst compilation failed: {}", e)))?;

            let doc_id = invoice.document.as_ref().map(|d| d.id as i32);
            let prefix = format!("invoices/{}", invoice_id);
            let doc = super::documents::store_new_version(&repo, doc_id, "pdf", "application/pdf", &prefix, &bytes).await?;
            
            let doc_id_i32 = doc.id as i32;
            repo.update_invoice_document(invoice_id, doc_id_i32).await?;
            
            Ok(doc)
        }.await;

        if let Err(ref e) = res {
            logging::log!("Error in export_invoice_pdf({}): {:?}", invoice_id, e);
        }
        res
    }
    
    #[cfg(not(feature = "ssr"))]
    {
        _ = invoice_id;
        Err(ServerFnError::new("Client side PDF generation not supported"))
    }
}

#[server(name = ExportOfferPdf, prefix = "/api", endpoint = "export_offer_pdf")]
pub async fn export_offer_pdf(offer_id: i64) -> Result<shared::Document, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let res = async {
            let offer = get_offer(offer_id).await?;
            if offer.committed_timestamp.is_none() {
                return Err(ServerFnError::new("Can only export committed offers"));
            }
            let typst_code = generate_offer_typst(&offer);
            
            let repo = use_context::<super::db::ActiveRepository>()
                .ok_or_else(|| ServerFnError::new("Repository not found"))?;
            let bytes = crate::pdf::compiler::compile_typst_pdfa(typst_code)
                .map_err(|e| ServerFnError::new(format!("Typst compilation failed: {}", e)))?;
            
            let doc_id = offer.document.as_ref().map(|d| d.id as i32);
            let prefix = format!("offers/{}-{}", offer_id, offer.revision.unwrap_or(1));
            let doc = super::documents::store_new_version(&repo, doc_id, "pdf", "application/pdf", &prefix, &bytes).await?;
            
            let doc_id_i32 = doc.id as i32;
            let rev_i32 = offer.revision.unwrap_or(1) as i32;
            repo.update_offer_document(offer_id, doc_id_i32, rev_i32).await?;
            
            Ok(doc)
        }.await;

        if let Err(ref e) = res {
            logging::log!("Error in export_offer_pdf({}): {:?}", offer_id, e);
        }
        res
    }
    
    #[cfg(not(feature = "ssr"))]
    {
        _ = offer_id;
        Err(ServerFnError::new("Client side PDF generation not supported"))
    }
}
