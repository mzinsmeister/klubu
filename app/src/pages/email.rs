use leptos::*;
use shared::{
    ComposeEmail, EmailMessage, EmailSettings, EmailSummary, EngagementLinkKind,
    EngagementListItem, Page,
};
use wasm_bindgen::JsCast;

use crate::components::EmptyState;
use crate::server::{
    download_email, get_email, get_email_settings, list_emails, list_engagements, mark_email_read,
    send_email,
};

const PAGE_SIZE: u32 = 50;

fn offer_download(download: &shared::EmailDownload) {
    let href = format!("data:{};base64,{}", download.media_type, download.base64);
    if let Some(document) = web_sys::window().and_then(|window| window.document()) {
        if let Ok(anchor) = document.create_element("a") {
            let _ = anchor.set_attribute("href", &href);
            let _ = anchor.set_attribute("download", &download.filename);
            if let Some(anchor) = anchor.dyn_ref::<web_sys::HtmlElement>() {
                anchor.click();
            }
        }
    }
}

fn is_unread(message: &EmailSummary) -> bool {
    !message.flags.iter().any(|flag| flag == "\\Seen")
}

fn timestamp(value: &chrono::DateTime<chrono::Utc>) -> String {
    value.format("%d.%m.%Y %H:%M").to_string()
}

#[component]
pub fn EmailPage() -> impl IntoView {
    let (mailbox, set_mailbox) = create_signal("INBOX".to_string());
    let (refresh, set_refresh) = create_signal(0_u64);
    let (selected_id, set_selected_id) = create_signal(Option::<i64>::None);
    let (compose_open, set_compose_open) = create_signal(false);
    let (to, set_to) = create_signal(String::new());
    let (cc, set_cc) = create_signal(String::new());
    let (bcc, set_bcc) = create_signal(String::new());
    let (subject, set_subject) = create_signal(String::new());
    let (body, set_body) = create_signal(String::new());
    let (engagement_choice, set_engagement_choice) = create_signal(String::new());
    let (error, set_error) = create_signal(Option::<String>::None);
    let (notice, set_notice) = create_signal(Option::<String>::None);
    let (customer_filter, set_customer_filter) = create_signal(Option::<i64>::None);
    let (search, set_search) = create_signal(String::new());

    create_effect(move |_| {
        if let Some(window) = web_sys::window() {
            if let Ok(params) = web_sys::UrlSearchParams::new_with_str(
                &window.location().search().unwrap_or_default(),
            ) {
                set_customer_filter.set(
                    params
                        .get("customer_id")
                        .and_then(|value| value.parse().ok()),
                );
                if params.get("compose").as_deref() == Some("1") {
                    set_compose_open.set(true);
                    if let Some(address) = params.get("to") {
                        set_to.set(address);
                    }
                }
            }
        }
    });

    let messages = create_resource(
        move || {
            (
                mailbox.get(),
                refresh.get(),
                customer_filter.get(),
                search.get(),
            )
        },
        |(mailbox, _, customer_id, search)| async move {
            list_emails(mailbox, 0, PAGE_SIZE, customer_id, Some(search)).await
        },
    );
    let selected = create_resource(
        move || selected_id.get(),
        |id| async move {
            match id {
                Some(id) => Some(get_email(id).await),
                None => None,
            }
        },
    );
    let settings = create_resource(|| (), |_| async move { get_email_settings().await });
    let engagements = create_resource(
        || (),
        |_| async move { list_engagements(0, 100, None).await },
    );
    let engagement_suggestions = create_resource(
        move || selected_id.get(),
        |id| async move {
            match id {
                Some(id) => {
                    let message = get_email(id).await?;
                    list_engagements(0, 100, message.summary.customer_contact_id).await
                }
                None => Ok(Page {
                    items: Vec::<EngagementListItem>::new(),
                    has_more: false,
                }),
            }
        },
    );

    let mark_read_action = create_action(move |id: &i64| {
        let id = *id;
        async move {
            let _ = mark_email_read(id, true).await;
            set_refresh.update(|value| *value = value.wrapping_add(1));
        }
    });

    let send_action = create_action(move |compose: &ComposeEmail| {
        let compose = compose.clone();
        async move {
            set_error.set(None);
            set_notice.set(None);
            match send_email(compose).await {
                Ok(sent) => {
                    set_notice.set(Some(format!(
                        "E-Mail an {} wurde archiviert und angenommen.",
                        sent.recipients
                    )));
                    set_to.set(String::new());
                    set_cc.set(String::new());
                    set_bcc.set(String::new());
                    set_subject.set(String::new());
                    set_body.set(String::new());
                    set_engagement_choice.set(String::new());
                    set_compose_open.set(false);
                    set_mailbox.set("Sent".to_string());
                    set_refresh.update(|value| *value = value.wrapping_add(1));
                }
                Err(send_error) => set_error.set(Some(send_error.to_string())),
            }
        }
    });

    let download_action = create_action(move |id: &i64| {
        let id = *id;
        async move {
            match download_email(id).await {
                Ok(download) => offer_download(&download),
                Err(download_error) => set_error.set(Some(download_error.to_string())),
            }
        }
    });

    let link_action = create_action(move |(mail_id, engagement_id): &(i64, i64)| {
        let mail_id = *mail_id;
        let engagement_id = *engagement_id;
        async move {
            match crate::server::link_engagement(engagement_id, EngagementLinkKind::Email, mail_id)
                .await
            {
                Ok(()) => {
                    set_notice.set(Some("E-Mail wurde mit dem Auftrag verknüpft.".to_string()))
                }
                Err(error) => set_error.set(Some(error.to_string())),
            }
        }
    });

    let submit = move |_| {
        send_action.dispatch(ComposeEmail {
            to: to.get_untracked(),
            cc: cc.get_untracked(),
            bcc: bcc.get_untracked(),
            subject: subject.get_untracked(),
            body: body.get_untracked(),
            attachments: Vec::new(),
            engagement_id: engagement_choice.get_untracked().parse::<i64>().ok(),
        });
    };

    let open_message = move |id: i64| {
        set_selected_id.set(Some(id));
        mark_read_action.dispatch(id);
    };

    view! {
        <div class="container">
            <div class="level">
                <div class="level-left">
                    <div>
                        <h1 class="title mb-1">"E-Mail"</h1>
                        <p class="text-muted is-size-7">"Originale RFC-5322-Nachrichten werden unveränderbar als .eml archiviert."</p>
                    </div>
                </div>
                <div class="level-right">
                    <button class="button is-link" on:click=move |_| set_compose_open.update(|open| *open = !*open)>
                        <span class="icon mr-1"><i class="mdi mdi-email-plus-outline"></i></span>
                        "Neue E-Mail"
                    </button>
                </div>
            </div>

            {move || notice.get().map(|message| view! {
                <div class="message is-success"><div class="message-body">{message}</div></div>
            })}
            {move || error.get().map(|message| view! {
                <div class="message is-danger"><div class="message-body">{message}</div></div>
            })}

            <div class="box mb-4">
                <div class="level mb-2">
                    <div>
                        <span class="tag is-success mr-2">"GoBD-Archiv aktiv"</span>
                        <span class="text-muted is-size-7">"Inhaltshash, Erfassungszeitpunkt und Audit-Eintrag pro Nachricht"</span>
                    </div>
                    <Suspense fallback=move || view! { <span class="text-muted is-size-7">"Relay wird geladen…"</span> }>
                        {move || settings.get().and_then(Result::ok).map(|value: EmailSettings| view! {
                            <span class="text-muted is-size-7">{format!("SMTP {} · IMAP {} · {}", value.smtp_port, value.imap_port, value.address_domain)}</span>
                        })}
                    </Suspense>
                </div>
                <p class="help">"Für normale Mailprogramme: SMTP-Submission und IMAP auf den angezeigten Ports, Benutzername/Passwort des Klubu-Kontos. Die lokalen Ports sind standardmäßig nur an localhost gebunden."</p>
            </div>
            {move || compose_open.get().then(|| view! {
                <div class="box mb-4">
                    <h2 class="subtitle">"Neue E-Mail"</h2>
                    <div class="field-row">
                        <div class="field is-wide">
                            <label class="label">"An"</label>
                            <input class="input" placeholder="name@example.org" prop:value=to on:input=move |event| set_to.set(event_target_value(&event)) />
                        </div>
                        <div class="field">
                            <label class="label">"Cc"</label>
                            <input class="input" prop:value=cc on:input=move |event| set_cc.set(event_target_value(&event)) />
                        </div>
                        <div class="field">
                            <label class="label">"Bcc"</label>
                            <input class="input" prop:value=bcc on:input=move |event| set_bcc.set(event_target_value(&event)) />
                        </div>
                    </div>
                    <div class="field">
                        <label class="label">"Betreff"</label>
                        <input class="input" prop:value=subject on:input=move |event| set_subject.set(event_target_value(&event)) />
                    </div>
                    <div class="field">
                        <label class="label">"Nachricht"</label>
                        <textarea class="textarea email-compose-body" prop:value=body on:input=move |event| set_body.set(event_target_value(&event))></textarea>
                    </div>
                    <div class="field">
                        <label class="label">"Engagement (optional)"</label>
                        <div class="select is-fullwidth">
                            <select prop:value=engagement_choice on:change=move |event| set_engagement_choice.set(event_target_value(&event))>
                                <option value="">"-- Nicht verknüpfen --"</option>
                                <Suspense fallback=move || view! { <option>"Lade Aufträge…"</option> }>
                                    {move || engagements.get().and_then(Result::ok).map(|page| page.items.into_iter().map(|item| view! {
                                        <option value=item.id.to_string()>{item.title}</option>
                                    }).collect_view())}
                                </Suspense>
                            </select>
                        </div>
                    </div>
                    <div class="field is-grouped">
                        <button class="button is-link" prop:disabled=move || send_action.pending().get() on:click=submit>
                            <span class="icon mr-1"><i class="mdi mdi-send"></i></span>"Senden & archivieren"
                        </button>
                        <button class="button" on:click=move |_| set_compose_open.set(false)>"Abbrechen"</button>
                    </div>
                </div>
            })}

            <div class="columns is-split email-layout">
                <div class="column">
                    <div class="box email-mailbox">
                        <div class="email-toolbar mb-3">
                          <div class="buttons mb-0">
                            <button class="button" class:is-link=move || mailbox.get() == "INBOX" on:click=move |_| { set_mailbox.set("INBOX".to_string()); set_selected_id.set(None); }>
                                <span class="icon mr-1"><i class="mdi mdi-inbox"></i></span>"Posteingang"
                            </button>
                            <button class="button" class:is-link=move || mailbox.get() == "Sent" on:click=move |_| { set_mailbox.set("Sent".to_string()); set_selected_id.set(None); }>
                                <span class="icon mr-1"><i class="mdi mdi-send-check-outline"></i></span>"Gesendet"
                            </button>
                          </div>
                          <div class="control has-icons-left email-search">
                            <input class="input is-small" type="search" placeholder="Absender, Empfänger, Betreff …" prop:value=search on:input=move |event| set_search.set(event_target_value(&event)) />
                            <span class="icon is-left"><i class="mdi mdi-magnify"></i></span>
                          </div>
                        </div>
                        {move || customer_filter.get().map(|_| view! {
                            <div class="email-filter-banner mb-3">
                                <span><i class="mdi mdi-account-filter-outline mr-1"></i>"Nur E-Mails dieses Kontakts"</span>
                                <button class="button is-small is-light" on:click=move |_| set_customer_filter.set(None)>"Filter entfernen"</button>
                            </div>
                        })}
                        <Suspense fallback=move || view! { <p class="text-muted">"Lade Postfach…"</p> }>
                            {move || messages.get().map(|result| match result {
                                Err(load_error) => view! { <div class="message is-danger"><div class="message-body">{load_error.to_string()}</div></div> }.into_view(),
                                Ok(page) if page.items.is_empty() => view! { <div class="crm-empty text-muted p-4 has-text-centered"><i class="mdi mdi-email-search-outline is-size-4"></i><p class="mt-2">"Keine passenden E-Mails gefunden."</p></div> }.into_view(),
                                Ok(page) => view! {
                                    <div class="email-list">
                                        {page.items.into_iter().map(|message| {
                                            let id = message.id;
                                            let unread = is_unread(&message);
                                            let subject_text = if message.subject.is_empty() { "(ohne Betreff)".to_string() } else { message.subject.clone() };
                                            let peer = if mailbox.get_untracked() == "Sent" { message.recipients.clone() } else { message.sender.clone() };
                                            view! {
                                                <div class="box list-item email-list-item p-3 mb-2" class:is-active=move || selected_id.get() == Some(id) on:click=move |_| open_message(id)>
                                                    <div class="level mb-1">
                                                        <span class:has-text-weight-bold=unread>{peer}</span>
                                                        <span class="text-muted is-size-7">{timestamp(&message.timestamp)}</span>
                                                    </div>
                                                    <div class:has-text-weight-bold=unread>{subject_text}</div>
                                                    <div class="email-list-meta mt-1">
                                                        {message.customer_name.clone().map(|name| view! { <span class="tag">{name}</span> })}
                                                        <span class="text-muted is-size-7">{format!("{} Anhänge", message.attachment_count)}</span>
                                                    </div>
                                                    {(!message.delivery_status.eq_ignore_ascii_case("sent") && mailbox.get_untracked() == "Sent").then(|| view! { <span class="tag is-warning mt-2">{message.delivery_status.clone()}</span> })}
                                                </div>
                                            }
                                        }).collect_view()}
                                    </div>
                                }.into_view(),
                            })}
                        </Suspense>
                    </div>
                </div>
                <div class="column">
                    <Suspense fallback=move || view! { <div class="box"><p class="text-muted">"Lade Nachricht…"</p></div> }>
                        {move || selected.get().and_then(|value| value).map(|result| match result {
                            Err(load_error) => view! { <div class="message is-danger"><div class="message-body">{load_error.to_string()}</div></div> }.into_view(),
                            Ok(message) => {
                                let mail_id = message.summary.id;
                                view! {
                                    <EmailDetail message=message.clone() on_download=Callback::new(move |id| download_action.dispatch(id)) />
                                    {move || engagement_suggestions.get().map(|result| match result {
                                        Err(error) => view! { <div class="message is-danger mt-3"><div class="message-body">{error.to_string()}</div></div> }.into_view(),
                                        Ok(page) if page.items.is_empty() => view! { <div class="box mt-3"><p class="text-muted">"Keine passenden Aufträge gefunden."</p></div> }.into_view(),
                                        Ok(page) => view! { <EmailEngagementPicker
                                            email_id=mail_id
                                            customer_name=message.summary.customer_name.clone()
                                            suggestions=page.items
                                            on_link=Callback::new(move |pair| link_action.dispatch(pair))
                                        /> }.into_view(),
                                    })}
                                }.into_view()
                            }
                        })}
                    </Suspense>
                    {move || selected_id.get().is_none().then(|| view! {
                        <EmptyState icon="email-open-outline" text="Wählen Sie eine Nachricht aus." />
                    })}
                </div>
            </div>
        </div>
    }
}

#[component]
fn EmailDetail(message: EmailMessage, on_download: Callback<i64>) -> impl IntoView {
    let id = message.summary.id;
    view! {
        <div class="box email-detail">
            <div class="level">
                <div>
                    <h2 class="subtitle mb-1">{if message.summary.subject.is_empty() { "(ohne Betreff)".to_string() } else { message.summary.subject.clone() }}</h2>
                    <p class="text-muted is-size-7">{format!("Von: {} · An: {}", message.summary.sender, message.summary.recipients)}</p>
                    <p class="text-muted is-size-7">{format!("Gesendet: {} · Archiviert: {}", timestamp(&message.summary.timestamp), timestamp(&message.summary.archived_timestamp))}</p>
                    {message.summary.customer_name.clone().map(|name| view! { <span class="tag is-success">{format!("Kontakt: {name}")}</span> })}
                </div>
                <button class="button is-light" on:click=move |_| on_download.call(id)>
                    <span class="icon mr-1"><i class="mdi mdi-download"></i></span>"Original .eml"
                </button>
            </div>
            <hr/>
            <pre class="email-body">{message.body_text.clone()}</pre>
            {(!message.attachments.is_empty()).then(|| view! {
                <div class="mt-4">
                    <h3 class="is-size-6 has-text-weight-bold mb-2">"Anhänge"</h3>
                    {message.attachments.into_iter().map(|attachment| view! {
                        <div class="box p-3 mb-2">
                            <div class="level mb-1">
                                <span>{format!("{} · {} Bytes", attachment.filename, attachment.raw_size)}</span>
                                {attachment.document_id.map(|id| view! { <span class="tag is-success">{format!("Systemdokument #{id}")}</span> })}
                            </div>
                            {if attachment.document_links.is_empty() {
                                view! { <p class="text-muted is-size-7">"Im unveränderten .eml archiviert; kein passendes Systemdokument gefunden."</p> }.into_view()
                            } else {
                                view! { <div class="buttons">{attachment.document_links.into_iter().map(|link| {
                                    let (path, label) = match link.kind.as_str() {
                                        "invoice" => (format!("/invoices?invoice_id={}", link.entity_id), format!("Rechnung {} öffnen", link.reference.unwrap_or_else(|| link.entity_id.to_string()))),
                                        "offer" => (format!("/offers?offer_id={}", link.entity_id), format!("Angebot {} öffnen", link.reference.unwrap_or_else(|| link.entity_id.to_string()))),
                                        "receipt" => (format!("/receipts?receipt_id={}", link.entity_id), format!("Beleg {} öffnen", link.reference.unwrap_or_else(|| link.entity_id.to_string()))),
                                        _ => ("/documents".to_string(), "Dokument öffnen".to_string()),
                                    };
                                    view! { <a class="button is-small is-link" href=path>{label}</a> }
                                }).collect_view()}</div> }.into_view()
                            }}
                        </div>
                    }).collect_view()}
                </div>
            })}
            {message.has_html_body.then(|| view! {
                <p class="help mt-3">"HTML-Inhalt wird aus Sicherheitsgründen als Textansicht dargestellt. Das Original kann als .eml exportiert werden."</p>
            })}
            <div class="tags mt-4">
                <span class="tag is-info">{format!("{} Bytes", message.summary.raw_size)}</span>
                <span class="tag">{format!("SHA-256 im Archiv · {}", message.summary.message_id)}</span>
            </div>
        </div>
    }
}

#[component]
fn EmailEngagementPicker(
    email_id: i64,
    customer_name: Option<String>,
    suggestions: Vec<EngagementListItem>,
    on_link: Callback<(i64, i64)>,
) -> impl IntoView {
    let initial = suggestions
        .first()
        .map(|item| item.id.to_string())
        .unwrap_or_default();
    let (choice, set_choice) = create_signal(initial);
    view! {
        <div class="box mt-3">
            <h3 class="is-size-6 has-text-weight-bold">"Mit Auftrag verknüpfen"</h3>
            {customer_name.map(|name| view! { <p class="help">{format!("Vorschläge für Kontakt: {name}")}</p> })}
            <div class="field-row mt-2">
                <div class="select is-fullwidth">
                    <select prop:value=choice on:change=move |event| set_choice.set(event_target_value(&event))>
                        {suggestions.into_iter().map(|item| view! {
                            <option value=item.id.to_string()>{format!("{}{}", item.title, item.customer_name.map(|name| format!(" · {name}")).unwrap_or_default())}</option>
                        }).collect_view()}
                    </select>
                </div>
                <button class="button is-link" on:click=move |_| {
                    if let Ok(engagement_id) = choice.get_untracked().parse::<i64>() {
                        on_link.call((email_id, engagement_id));
                    }
                }>{"Verknüpfen"}</button>
            </div>
        </div>
    }
}
