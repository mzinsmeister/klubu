use shared::*;
use chrono::NaiveDate;
use leptos::ServerFnError;

// Both database drivers are compiled in and the backend is picked at runtime
// from the DATABASE_URL scheme, via sqlx's `Any` driver. That keeps a single
// binary able to serve either database — and to copy from one to the other.
// The price is that queries are checked against the database at runtime rather
// than at compile time; the SQL itself is written to the portable subset both
// dialects share (`$N` placeholders work natively in SQLite too).
pub type DbPool = sqlx::AnyPool;
pub type DbRow = sqlx::any::AnyRow;
pub type DbTransaction<'a> = sqlx::Transaction<'a, sqlx::Any>;

pub use super::auth::CurrentUser;

/// Which dialect the pool is talking to: `"postgres"` or `"sqlite"`.
/// Report queries carry per-dialect SQL, so the engine has to ask.
pub fn dialect(pool: &DbPool) -> &'static str {
    if pool.connect_options().database_url.scheme().starts_with("sqlite") {
        "sqlite"
    } else {
        "postgres"
    }
}

pub mod repository;

pub use repository::SqlRepository;

pub type ActiveRepository = std::sync::Arc<SqlRepository>;

pub trait KlubuRepository: Send + Sync {
    // Contacts
    fn get_contacts(&self, offset: u32, limit: u32, query: Option<String>, archived: bool) -> impl std::future::Future<Output = Result<Page<Contact>, ServerFnError>> + Send;
    fn save_contact(&self, contact: Contact) -> impl std::future::Future<Output = Result<Contact, ServerFnError>> + Send;
    fn archive_contact(&self, id: i64) -> impl std::future::Future<Output = Result<(), ServerFnError>> + Send;
    fn restore_contact(&self, id: i64) -> impl std::future::Future<Output = Result<(), ServerFnError>> + Send;

    // Dashboard
    fn get_dashboard_stats(&self) -> impl std::future::Future<Output = Result<DashboardStats, ServerFnError>> + Send;

    // Documents
    fn store_new_version(
        &self,
        document_id: Option<i32>,
        extension: &str,
        media_type: &str,
        storage_key_prefix: &str,
        data: &[u8],
    ) -> impl std::future::Future<Output = Result<shared::Document, ServerFnError>> + Send;
    fn delete_document(&self, document_id: i32) -> impl std::future::Future<Output = Result<(), ServerFnError>> + Send;
    fn get_document_meta(&self, doc_id: i32) -> impl std::future::Future<Output = Result<Option<(String, String, String)>, ServerFnError>> + Send;
    fn get_latest_document_version(&self, doc_id: i32) -> impl std::future::Future<Output = Result<Option<(i32, i32)>, ServerFnError>> + Send;

    // Exports
    fn update_invoice_document(&self, invoice_id: i64, doc_id: i32) -> impl std::future::Future<Output = Result<(), ServerFnError>> + Send;
    fn update_offer_document(&self, offer_id: i64, doc_id: i32, revision: i32) -> impl std::future::Future<Output = Result<(), ServerFnError>> + Send;
    fn update_receipt_document(&self, receipt_id: i64, doc_id: i32) -> impl std::future::Future<Output = Result<(), ServerFnError>> + Send;

    // Invoices
    fn get_invoices(&self, offset: u32, limit: u32, from_date: Option<NaiveDate>, to_date: Option<NaiveDate>) -> impl std::future::Future<Output = Result<Page<InvoiceListItem>, ServerFnError>> + Send;
    fn get_invoice(&self, id: i64) -> impl std::future::Future<Output = Result<Invoice, ServerFnError>> + Send;
    fn save_invoice(&self, invoice: Invoice) -> impl std::future::Future<Output = Result<Invoice, ServerFnError>> + Send;
    fn cancel_invoice(&self, id: i64) -> impl std::future::Future<Output = Result<(), ServerFnError>> + Send;
    fn add_invoice_payment(&self, invoice_id: i64, amount_cents: i64, date: NaiveDate) -> impl std::future::Future<Output = Result<(), ServerFnError>> + Send;
    fn delete_invoice_payment(&self, id: i64) -> impl std::future::Future<Output = Result<(), ServerFnError>> + Send;
    fn commit_invoice(&self, id: i64) -> impl std::future::Future<Output = Result<(), ServerFnError>> + Send;
    fn delete_invoice(&self, id: i64) -> impl std::future::Future<Output = Result<(), ServerFnError>> + Send;

    // Offers
    fn get_offers(&self, offset: u32, limit: u32, from_date: Option<NaiveDate>, to_date: Option<NaiveDate>) -> impl std::future::Future<Output = Result<Page<OfferListItem>, ServerFnError>> + Send;
    fn get_offer(&self, id: i64) -> impl std::future::Future<Output = Result<Offer, ServerFnError>> + Send;
    fn save_offer(&self, offer: Offer) -> impl std::future::Future<Output = Result<Offer, ServerFnError>> + Send;
    fn commit_offer(&self, id: i64) -> impl std::future::Future<Output = Result<(), ServerFnError>> + Send;
    fn delete_offer(&self, id: i64) -> impl std::future::Future<Output = Result<(), ServerFnError>> + Send;
    fn get_offer_revisions(&self, offer_id: i64) -> impl std::future::Future<Output = Result<Vec<OfferRevision>, ServerFnError>> + Send;
    fn create_offer_revision(&self, offer_id: i64) -> impl std::future::Future<Output = Result<Offer, ServerFnError>> + Send;

    // Receipts
    fn get_receipts(&self, offset: u32, limit: u32, from_date: Option<NaiveDate>, to_date: Option<NaiveDate>) -> impl std::future::Future<Output = Result<Page<ReceiptListItem>, ServerFnError>> + Send;
    fn get_receipt(&self, id: i64) -> impl std::future::Future<Output = Result<Receipt, ServerFnError>> + Send;
    fn save_receipt(&self, receipt: Receipt) -> impl std::future::Future<Output = Result<Receipt, ServerFnError>> + Send;
    fn commit_receipt(&self, id: i64) -> impl std::future::Future<Output = Result<(), ServerFnError>> + Send;
    fn delete_receipt(&self, id: i64) -> impl std::future::Future<Output = Result<(), ServerFnError>> + Send;
    fn get_categories(&self) -> impl std::future::Future<Output = Result<Vec<ReceiptItemCategory>, ServerFnError>> + Send;
    fn add_receipt_payment(&self, receipt_id: i64, amount_cents: i64, date: NaiveDate) -> impl std::future::Future<Output = Result<(), ServerFnError>> + Send;
    fn delete_receipt_payment(&self, id: i64) -> impl std::future::Future<Output = Result<(), ServerFnError>> + Send;

    // Seed
    fn seed_database(&self) -> impl std::future::Future<Output = Result<(), ServerFnError>> + Send;
}
