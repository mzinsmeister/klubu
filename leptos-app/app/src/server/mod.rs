pub mod ai;
pub mod contacts;
pub mod dashboard;
pub mod invoices;
pub mod offers;
pub mod receipts;
pub mod export;
pub mod documents;

// Re-export all server functions for convenience
pub use ai::*;
pub use contacts::*;
pub use dashboard::*;
pub use invoices::*;
pub use offers::*;
pub use receipts::*;
pub use export::*;
pub use documents::*;

#[cfg(feature = "ssr")]
pub fn register_server_fns() {
    let _ = leptos::server_fn::axum::register_explicit::<GetContacts>();
    let _ = leptos::server_fn::axum::register_explicit::<SaveContact>();
    let _ = leptos::server_fn::axum::register_explicit::<DeleteContact>();
    let _ = leptos::server_fn::axum::register_explicit::<GetInvoices>();
    let _ = leptos::server_fn::axum::register_explicit::<GetInvoice>();
    let _ = leptos::server_fn::axum::register_explicit::<SaveInvoice>();
    let _ = leptos::server_fn::axum::register_explicit::<CancelInvoice>();
    let _ = leptos::server_fn::axum::register_explicit::<AddInvoicePayment>();
    let _ = leptos::server_fn::axum::register_explicit::<DeleteInvoicePayment>();
    let _ = leptos::server_fn::axum::register_explicit::<CommitInvoice>();
    let _ = leptos::server_fn::axum::register_explicit::<DeleteInvoice>();
    let _ = leptos::server_fn::axum::register_explicit::<GetOffers>();
    let _ = leptos::server_fn::axum::register_explicit::<GetOffer>();
    let _ = leptos::server_fn::axum::register_explicit::<SaveOffer>();
    let _ = leptos::server_fn::axum::register_explicit::<CommitOffer>();
    let _ = leptos::server_fn::axum::register_explicit::<DeleteOffer>();
    let _ = leptos::server_fn::axum::register_explicit::<GetOfferRevisions>();
    let _ = leptos::server_fn::axum::register_explicit::<CreateOfferRevision>();
    let _ = leptos::server_fn::axum::register_explicit::<GetReceipts>();
    let _ = leptos::server_fn::axum::register_explicit::<GetReceipt>();
    let _ = leptos::server_fn::axum::register_explicit::<SaveReceipt>();
    let _ = leptos::server_fn::axum::register_explicit::<DeleteReceipt>();
    let _ = leptos::server_fn::axum::register_explicit::<GetCategories>();
    let _ = leptos::server_fn::axum::register_explicit::<AddReceiptPayment>();
    let _ = leptos::server_fn::axum::register_explicit::<DeleteReceiptPayment>();
    let _ = leptos::server_fn::axum::register_explicit::<ExportInvoicePdf>();
    let _ = leptos::server_fn::axum::register_explicit::<ExportOfferPdf>();
    let _ = leptos::server_fn::axum::register_explicit::<GetDashboardStats>();
    let _ = leptos::server_fn::axum::register_explicit::<GetAiStatus>();
    let _ = leptos::server_fn::axum::register_explicit::<PrefillReceipt>();
}
