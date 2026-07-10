pub mod ai;
pub mod contacts;
pub mod dashboard;
pub mod invoices;
pub mod offers;
pub mod receipts;
pub mod export;
pub mod documents;
pub mod reports;
pub mod auth;

#[cfg(feature = "ssr")]
pub mod db;

// Re-export all server functions for convenience
pub use ai::*;
pub use contacts::*;
pub use dashboard::*;
pub use invoices::*;
pub use offers::*;
pub use receipts::*;
pub use export::*;
pub use reports::*;
pub use auth::*;

// `documents` exposes nothing outside of `ssr`.
#[cfg(feature = "ssr")]
pub use documents::*;

#[cfg(feature = "ssr")]
pub fn register_server_fns() {
    let _ = leptos::server_fn::axum::register_explicit::<GetContacts>();
    let _ = leptos::server_fn::axum::register_explicit::<GetArchivedContacts>();
    let _ = leptos::server_fn::axum::register_explicit::<SaveContact>();
    let _ = leptos::server_fn::axum::register_explicit::<ArchiveContact>();
    let _ = leptos::server_fn::axum::register_explicit::<RestoreContact>();
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
    let _ = leptos::server_fn::axum::register_explicit::<CommitReceipt>();
    let _ = leptos::server_fn::axum::register_explicit::<ParseEInvoice>();
    let _ = leptos::server_fn::axum::register_explicit::<DeleteReceipt>();
    let _ = leptos::server_fn::axum::register_explicit::<GetCategories>();
    let _ = leptos::server_fn::axum::register_explicit::<AddReceiptPayment>();
    let _ = leptos::server_fn::axum::register_explicit::<DeleteReceiptPayment>();
    let _ = leptos::server_fn::axum::register_explicit::<ExportInvoicePdf>();
    let _ = leptos::server_fn::axum::register_explicit::<ExportOfferPdf>();
    let _ = leptos::server_fn::axum::register_explicit::<GetDashboardStats>();
    let _ = leptos::server_fn::axum::register_explicit::<GetAiStatus>();
    let _ = leptos::server_fn::axum::register_explicit::<PrefillReceipt>();
    let _ = leptos::server_fn::axum::register_explicit::<ListReports>();
    let _ = leptos::server_fn::axum::register_explicit::<RunReport>();
    let _ = leptos::server_fn::axum::register_explicit::<ExportReportPdf>();
    let _ = leptos::server_fn::axum::register_explicit::<ExportReportCsv>();
    let _ = leptos::server_fn::axum::register_explicit::<CheckSetupRequired>();
    let _ = leptos::server_fn::axum::register_explicit::<InitializeAdmin>();
    let _ = leptos::server_fn::axum::register_explicit::<Login>();
    let _ = leptos::server_fn::axum::register_explicit::<Logout>();
    let _ = leptos::server_fn::axum::register_explicit::<GetCurrentUser>();
    let _ = leptos::server_fn::axum::register_explicit::<ListManagedDocuments>();
    let _ = leptos::server_fn::axum::register_explicit::<ListManagedDocumentVersions>();
    let _ = leptos::server_fn::axum::register_explicit::<UploadManagedDocument>();
    let _ = leptos::server_fn::axum::register_explicit::<AddManagedDocumentVersion>();
    let _ = leptos::server_fn::axum::register_explicit::<TombstoneManagedDocument>();
    let _ = leptos::server_fn::axum::register_explicit::<DownloadManagedDocumentVersion>();
}
