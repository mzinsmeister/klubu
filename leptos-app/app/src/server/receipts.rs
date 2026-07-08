#[cfg(feature = "ssr")]
use super::documents::{store_new_version, delete_document};
use leptos::*;
use chrono::NaiveDate;
use shared::*;

#[cfg(feature = "ssr")]
use super::db::KlubuRepository;

#[server(name = GetReceipts, prefix = "/api", endpoint = "get_receipts")]
pub async fn get_receipts() -> Result<Vec<ReceiptListItem>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        repo.get_receipts().await
    }
    #[cfg(not(feature = "ssr"))]
    Err(ServerFnError::new("Client side DB access not supported"))
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
                let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &doc.data)
                    .map_err(|e| ServerFnError::new(format!("Datei konnte nicht dekodiert werden: {e}")))?;
                    
                let doc_id = saved.document.as_ref().map(|d| d.id as i32);
                let prefix = format!("receipts/{}", saved.id.unwrap_or_default());
                let saved_doc = store_new_version(&repo, doc_id, &doc.extension, &doc.media_type, &prefix, &bytes).await?;
                
                let saved_doc_id = saved_doc.id as i32;
                repo.update_receipt_document(saved.id.unwrap_or_default(), saved_doc_id).await?;
                    
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
pub async fn add_receipt_payment(receipt_id: i64, amount_cents: i64, date: NaiveDate) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        repo.add_receipt_payment(receipt_id, amount_cents, date).await
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
