use leptos::*;


use super::invoices::get_invoice;
use super::offers::get_offer;
use crate::typst_gen::{generate_invoice_typst, generate_offer_typst};


#[server(name = ExportInvoicePdf, prefix = "/api", endpoint = "export_invoice_pdf")]
pub async fn export_invoice_pdf(invoice_id: i64) -> Result<shared::Document, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let res = async {
            let invoice = get_invoice(invoice_id).await?;
            if invoice.committed_timestamp.is_none() {
                return Err(ServerFnError::new("Can only export committed invoices"));
            }
            let typst_code = generate_invoice_typst(&invoice);
            
            let pool = use_context::<sqlx::PgPool>()
                .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
            let bytes = crate::pdf::compiler::compile_typst(typst_code)
                .map_err(|e| ServerFnError::new(format!("Typst compilation failed: {}", e)))?;
            
            let doc_id = invoice.document.as_ref().map(|d| d.id as i32);
            let prefix = format!("invoices/{}", invoice_id);
            let doc = super::documents::store_new_version(&pool, doc_id, "pdf", "application/pdf", &prefix, &bytes).await?;
            
            let doc_id_i32 = doc.id as i32;
            sqlx::query!("UPDATE invoice SET document_id = $1 WHERE id = $2", doc_id_i32, invoice_id as i32)
                .execute(&pool)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;
            
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
            
            let pool = use_context::<sqlx::PgPool>()
                .ok_or_else(|| ServerFnError::new("Database pool not found"))?;
            let bytes = crate::pdf::compiler::compile_typst(typst_code)
                .map_err(|e| ServerFnError::new(format!("Typst compilation failed: {}", e)))?;
            
            let doc_id = offer.document.as_ref().map(|d| d.id as i32);
            let prefix = format!("offers/{}-{}", offer_id, offer.revision.unwrap_or(1));
            let doc = super::documents::store_new_version(&pool, doc_id, "pdf", "application/pdf", &prefix, &bytes).await?;
            
            let doc_id_i32 = doc.id as i32;
            let rev_i32 = offer.revision.unwrap_or(1) as i32;
            sqlx::query!("UPDATE offer SET document_id = $1 WHERE id = $2 AND revision = $3", doc_id_i32, offer_id as i32, rev_i32)
                .execute(&pool)
                .await
                .map_err(|e| ServerFnError::new(e.to_string()))?;
            
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
