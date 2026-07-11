use crate::server::get_notifications;
use leptos::*;
use shared::format_euro;

#[component]
pub fn NotificationsPage() -> impl IntoView {
    let notifications = create_resource(|| (), |_| get_notifications());
    view! {
        <div class="container">
            <div class="level"><div><h1 class="title mb-1">"Benachrichtigungen"</h1><p class="text-muted">"Fälligkeiten und notwendige nächste Schritte."</p></div></div>
            <Suspense fallback=move || view! { <p class="text-muted">"Lade Benachrichtigungen…"</p> }>
                {move || notifications.get().map(|result| match result {
                    Err(error) => view! { <div class="message is-danger"><div class="message-body">{error.to_string()}</div></div> }.into_view(),
                    Ok(items) if items.is_empty() => view! { <div class="box has-text-centered"><span class="icon is-size-3"><i class="mdi mdi-check-circle-outline"></i></span><p class="mt-2">"Alles erledigt – keine offenen Benachrichtigungen."</p></div> }.into_view(),
                    Ok(items) => view! { <div class="box">{items.into_iter().map(|item| view! {
                        <a class="notification-row" href=item.href>
                            <span class="notification-icon"><i class="mdi mdi-alert-circle-outline"></i></span>
                            <span class="notification-copy"><strong>{item.title}</strong><span class="text-muted is-size-7">{item.detail} " · fällig seit " {item.date.format("%d.%m.%Y").to_string()}</span></span>
                            {item.amount_cents.map(|amount| view! { <span class="has-text-weight-semibold">{format_euro(amount)}</span> })}
                            <i class="mdi mdi-chevron-right"></i>
                        </a>
                    }).collect_view()}</div> }.into_view(),
                })}
            </Suspense>
        </div>
    }
}
