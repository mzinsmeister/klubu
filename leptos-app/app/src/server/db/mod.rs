use shared::*;
use chrono::NaiveDate;
use leptos::ServerFnError;

#[cfg(all(feature = "postgres", feature = "sqlite"))]
compile_error!(
    "features `postgres` and `sqlite` are mutually exclusive; \
     build the `sqlite` backend with --no-default-features --features sqlite"
);

#[cfg(not(any(feature = "postgres", feature = "sqlite")))]
compile_error!("exactly one of the `postgres` or `sqlite` features must be enabled");

#[cfg(feature = "postgres")]
pub type DbPool = sqlx::PgPool;
#[cfg(feature = "postgres")]
pub type DbRow = sqlx::postgres::PgRow;

#[cfg(feature = "sqlite")]
pub type DbPool = sqlx::SqlitePool;
#[cfg(feature = "sqlite")]
pub type DbRow = sqlx::sqlite::SqliteRow;

/// SQL placeholder for the Nth (1-based) bound parameter in the active dialect.
/// Report queries are dialect-specific, but the engine binds the same params to
/// both, so it needs to know how each names them.
#[cfg(feature = "postgres")]
pub const DIALECT: &str = "postgres";
#[cfg(feature = "sqlite")]
pub const DIALECT: &str = "sqlite";

pub mod repository;

pub use repository::SqlRepository;

pub type ActiveRepository = std::sync::Arc<SqlRepository>;

pub trait KlubuRepository: Send + Sync {
    // Contacts
    fn get_contacts(&self) -> impl std::future::Future<Output = Result<Vec<Contact>, ServerFnError>> + Send;
    fn save_contact(&self, contact: Contact) -> impl std::future::Future<Output = Result<Contact, ServerFnError>> + Send;
    fn delete_contact(&self, id: i64) -> impl std::future::Future<Output = Result<(), ServerFnError>> + Send;

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
    fn get_invoices(&self) -> impl std::future::Future<Output = Result<Vec<InvoiceListItem>, ServerFnError>> + Send;
    fn get_invoice(&self, id: i64) -> impl std::future::Future<Output = Result<Invoice, ServerFnError>> + Send;
    fn save_invoice(&self, invoice: Invoice) -> impl std::future::Future<Output = Result<Invoice, ServerFnError>> + Send;
    fn cancel_invoice(&self, id: i64) -> impl std::future::Future<Output = Result<(), ServerFnError>> + Send;
    fn add_invoice_payment(&self, invoice_id: i64, amount_cents: i64, date: NaiveDate) -> impl std::future::Future<Output = Result<(), ServerFnError>> + Send;
    fn delete_invoice_payment(&self, id: i64) -> impl std::future::Future<Output = Result<(), ServerFnError>> + Send;
    fn commit_invoice(&self, id: i64) -> impl std::future::Future<Output = Result<(), ServerFnError>> + Send;
    fn delete_invoice(&self, id: i64) -> impl std::future::Future<Output = Result<(), ServerFnError>> + Send;

    // Offers
    fn get_offers(&self) -> impl std::future::Future<Output = Result<Vec<OfferListItem>, ServerFnError>> + Send;
    fn get_offer(&self, id: i64) -> impl std::future::Future<Output = Result<Offer, ServerFnError>> + Send;
    fn save_offer(&self, offer: Offer) -> impl std::future::Future<Output = Result<Offer, ServerFnError>> + Send;
    fn commit_offer(&self, id: i64) -> impl std::future::Future<Output = Result<(), ServerFnError>> + Send;
    fn delete_offer(&self, id: i64) -> impl std::future::Future<Output = Result<(), ServerFnError>> + Send;
    fn get_offer_revisions(&self, offer_id: i64) -> impl std::future::Future<Output = Result<Vec<OfferRevision>, ServerFnError>> + Send;
    fn create_offer_revision(&self, offer_id: i64) -> impl std::future::Future<Output = Result<Offer, ServerFnError>> + Send;

    // Receipts
    fn get_receipts(&self) -> impl std::future::Future<Output = Result<Vec<ReceiptListItem>, ServerFnError>> + Send;
    fn get_receipt(&self, id: i64) -> impl std::future::Future<Output = Result<Receipt, ServerFnError>> + Send;
    fn save_receipt(&self, receipt: Receipt) -> impl std::future::Future<Output = Result<Receipt, ServerFnError>> + Send;
    fn delete_receipt(&self, id: i64) -> impl std::future::Future<Output = Result<(), ServerFnError>> + Send;
    fn get_categories(&self) -> impl std::future::Future<Output = Result<Vec<ReceiptItemCategory>, ServerFnError>> + Send;
    fn add_receipt_payment(&self, receipt_id: i64, amount_cents: i64, date: NaiveDate) -> impl std::future::Future<Output = Result<(), ServerFnError>> + Send;
    fn delete_receipt_payment(&self, id: i64) -> impl std::future::Future<Output = Result<(), ServerFnError>> + Send;

    // Seed
    fn seed_database(&self) -> impl std::future::Future<Output = Result<(), ServerFnError>> + Send;
}
