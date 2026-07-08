use leptos::*;
use chrono::NaiveDate;
use shared::*;

#[cfg(feature = "ssr")]
use super::db::KlubuRepository;

#[server(name = GetInvoices, prefix = "/api", endpoint = "get_invoices")]
pub async fn get_invoices() -> Result<Vec<InvoiceListItem>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        repo.get_invoices().await
    }
    #[cfg(not(feature = "ssr"))]
    Err(ServerFnError::new("Client side DB access not supported"))
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
pub async fn cancel_invoice(id: i64) -> Result<(), ServerFnError> {
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
pub async fn add_invoice_payment(invoice_id: i64, amount_cents: i64, date: NaiveDate) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let repo = use_context::<super::db::ActiveRepository>()
            .ok_or_else(|| ServerFnError::new("Repository not found"))?;
        repo.add_invoice_payment(invoice_id, amount_cents, date).await
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
