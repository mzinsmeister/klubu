use leptos::*;
use shared::{format_euro, DashboardStats};

use crate::server::get_dashboard_stats;

#[component]
fn StatCard(
    #[prop(into)] label: String,
    #[prop(into)] value: String,
    #[prop(optional, into)] sub: String,
    /// `""`, `"is-positive"` or `"is-negative"`.
    #[prop(optional, into)] tone: String,
) -> impl IntoView {
    let value_class = if tone.is_empty() {
        "stat-value".to_string()
    } else {
        format!("stat-value {tone}")
    };
    view! {
        <div class="column">
            <div class=format!("box stat-card {tone}")>
                <div class="stat-label">{label}</div>
                <div class=value_class>{value}</div>
                {(!sub.is_empty()).then(|| view! { <div class="stat-sub">{sub}</div> })}
            </div>
        </div>
    }
}

#[component]
pub fn DashboardPage() -> impl IntoView {
    let stats = create_resource(|| (), |_| async move { get_dashboard_stats().await });

    view! {
        <div class="container">
            <h1 class="title">"Übersicht"</h1>

            <Suspense fallback=move || view! { <p class="text-muted">"Lade Kennzahlen…"</p> }>
                {move || stats.get().map(|res| match res {
                    Err(e) => view! {
                        <div class="message is-danger">
                            <div class="message-body">
                                "Kennzahlen konnten nicht geladen werden: " {e.to_string()}
                            </div>
                        </div>
                    }.into_view(),
                    Ok(s) => view! { <StatsView stats=s /> }.into_view(),
                })}
            </Suspense>
        </div>
    }
}

#[component]
fn StatsView(stats: DashboardStats) -> impl IntoView {
    let result = stats.result_cents();
    let result_tone = if result < 0 { "is-negative" } else { "is-positive" };

    let open_sub = if stats.open_invoice_count > 0 {
        format!("{} offen", format_euro(stats.open_invoice_cents))
    } else {
        "Alles bezahlt".to_string()
    };

    let draft_sub = match stats.draft_invoice_count {
        0 => "Keine Entwürfe".to_string(),
        1 => "1 Entwurf".to_string(),
        n => format!("{n} Entwürfe"),
    };

    view! {
        <p class="text-muted mb-4">
            "Geschäftsjahr " {stats.year}
        </p>

        <div class="columns">
            <StatCard
                label="Einnahmen"
                value=format_euro(stats.revenue_cents)
                sub="Finalisierte Rechnungen"
            />
            <StatCard
                label="Ausgaben"
                value=format_euro(stats.expenses_cents)
                sub="Belege der Kategorie Ausgaben"
            />
            <StatCard
                label="Ergebnis"
                value=format_euro(result)
                sub="Einnahmenüberschussrechnung"
                tone=result_tone
            />
        </div>

        <div class="columns">
            <StatCard
                label="Offene Rechnungen"
                value=stats.open_invoice_count.to_string()
                sub=open_sub
            />
            <StatCard
                label="Rechnungsentwürfe"
                value=stats.draft_invoice_count.to_string()
                sub=draft_sub
            />
            <StatCard
                label="Belege"
                value=stats.receipt_count.to_string()
                sub=format!("{} Kontakte", stats.contact_count)
            />
        </div>
    }
}
