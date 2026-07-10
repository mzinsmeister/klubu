use leptos::*;
use leptos_router::*;

#[cfg(feature = "ssr")]
pub mod pdf;
#[cfg(feature = "ssr")]
pub mod markdown;
#[cfg(feature = "ssr")]
pub mod einvoice;

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
#[cfg(feature = "ssr")]
pub use server::db;

use pages::*;
use crate::server::{check_setup_required, get_current_user, logout};

#[component]
pub fn App() -> impl IntoView {
    let (user, set_user) = create_signal(None::<String>);
    let (setup_required, set_setup_required) = create_signal(false);
    let (loading, set_loading) = create_signal(true);

    // Fetch auth status on mount
    create_effect(move |_| {
        spawn_local(async move {
            if let Ok(required) = check_setup_required().await {
                set_setup_required.set(required);
            }
            if let Ok(opt_user) = get_current_user().await {
                set_user.set(opt_user);
            }
            set_loading.set(false);
        });
    });

    let logout_action = create_action(move |_: &()| {
        async move {
            if logout().await.is_ok() {
                set_user.set(None);
            }
        }
    });

    view! {
        <Router>
            {move || {
                if loading.get() {
                    view! {
                        <div class="auth-container">
                            <p class="text-muted">"Lade Klubu..."</p>
                        </div>
                    }.into_view()
                } else if setup_required.get() {
                    view! {
                        <SetupPage on_initialized=move || {
                            set_setup_required.set(false);
                        } />
                    }.into_view()
                } else if user.get().is_none() {
                    view! {
                        <LoginPage on_login=move |u| {
                            set_user.set(Some(u));
                        } />
                    }.into_view()
                } else {
                    let current_user_name = user.get().unwrap_or_default();
                    view! {
                        <div class="app-shell">
                            // Sidebar Navigation
                            <aside class="app-sidebar" style="display: flex; flex-direction: column;">
                                <div class="app-brand">
                                    <span class="icon"><i class="mdi mdi-account-group"></i></span>
                                    "Klubu"
                                </div>
                                <nav class="menu" style="flex-grow: 1;">
                                    <p class="menu-label">"Verwaltung"</p>
                                    <ul class="menu-list">
                                        <li><A href="/" exact=true>"Übersicht"</A></li>
                                        <li><A href="/contacts">"Kontakte"</A></li>
                                        <li><A href="/invoices">"Rechnungen"</A></li>
                                        <li><A href="/offers">"Angebote"</A></li>
                                        <li><A href="/receipts">"Belege"</A></li>
                                        <li><A href="/documents">"Dokumente"</A></li>
                                        <li><A href="/reports">"Berichte"</A></li>
                                    </ul>
                                </nav>
                                <div class="app-sidebar-footer p-4 border-top" style="border-top: 1px solid var(--border); margin-top: auto;">
                                    <div class="is-flex is-align-items-center is-justify-content-space-between">
                                        <div class="is-size-7 text-muted" style="max-width: 140px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;" title=current_user_name.clone()>
                                            <span class="icon mr-1"><i class="mdi mdi-account"></i></span>
                                            {current_user_name}
                                        </div>
                                        <button class="button is-small is-danger is-outlined" title="Abmelden" on:click=move |_| logout_action.dispatch(())>
                                            <span class="icon"><i class="mdi mdi-logout"></i></span>
                                        </button>
                                    </div>
                                </div>
                            </aside>

                            // Main Content
                            <main class="app-main">
                                <Routes>
                                    <Route path="" view=DashboardPage />
                                    <Route path="contacts" view=ContactsPage />
                                    <Route path="invoices" view=InvoicesPage />
                                    <Route path="offers" view=OffersPage />
                                    <Route path="receipts" view=ReceiptsPage />
                                    <Route path="documents" view=DocumentsPage />
                                    <Route path="reports" view=ReportsPage />
                                </Routes>
                            </main>
                        </div>
                    }.into_view()
                }
            }}
        </Router>
    }
}

