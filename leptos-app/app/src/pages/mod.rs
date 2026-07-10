pub mod dashboard;
pub mod contacts;
pub mod invoices;
pub mod offers;
pub mod receipts;
pub mod reports;
pub mod documents;
pub mod auth;

pub use dashboard::DashboardPage;
pub use contacts::ContactsPage;
pub use invoices::InvoicesPage;
pub use offers::OffersPage;
pub use receipts::ReceiptsPage;
pub use reports::ReportsPage;
pub use documents::DocumentsPage;
pub use auth::{LoginPage, SetupPage};

