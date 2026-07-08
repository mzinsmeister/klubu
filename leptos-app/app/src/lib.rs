use leptos::*;
use leptos_router::*;

#[cfg(feature = "ssr")]
pub mod pdf;

pub mod typst_gen;
pub mod server;
pub mod pages;
pub mod components;

// Re-export for backend usage
pub use typst_gen::{html_to_typst, generate_invoice_typst, generate_offer_typst};
#[cfg(feature = "ssr")]
pub use typst_gen::{init_templates, load_config, AppConfig, BankConfig};
#[cfg(feature = "ssr")]
pub use server::{register_server_fns, store_new_version, delete_document};

use pages::*;

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <div class="app-shell">
                // Sidebar Navigation
                <aside class="app-sidebar">
                    <div class="app-brand">
                        <span class="icon"><i class="mdi mdi-account-group"></i></span>
                        "Klubu"
                    </div>
                    <nav class="menu">
                        <p class="menu-label">"Verwaltung"</p>
                        <ul class="menu-list">
                            <li><A href="/" exact=true>"Übersicht"</A></li>
                            <li><A href="/contacts">"Kontakte"</A></li>
                            <li><A href="/invoices">"Rechnungen"</A></li>
                            <li><A href="/offers">"Angebote"</A></li>
                            <li><A href="/receipts">"Belege"</A></li>
                        </ul>
                    </nav>
                </aside>

                // Main Content
                <main class="app-main">
                    <Routes>
                        <Route path="" view=DashboardPage />
                        <Route path="contacts" view=ContactsPage />
                        <Route path="invoices" view=InvoicesPage />
                        <Route path="offers" view=OffersPage />
                        <Route path="receipts" view=ReceiptsPage />
                    </Routes>
                </main>
            </div>
        </Router>
    }
}
