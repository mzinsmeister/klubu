use leptos::*;
use serde::{Deserialize, Serialize};
use shared::{
    EmailSummary, Engagement, EngagementInput, EngagementLinkKind, InvoiceListItem, OfferListItem,
};

use crate::components::EmptyState;
use crate::server::{
    create_invoice_from_offer, get_all_contacts, get_engagement, get_invoices, get_offers,
    link_engagement, list_emails, list_engagements, save_engagement,
};

const PAGE_SIZE: u32 = 100;

#[derive(Clone, Serialize, Deserialize)]
struct Candidates {
    offers: Vec<OfferListItem>,
    invoices: Vec<InvoiceListItem>,
    emails: Vec<EmailSummary>,
}

#[component]
pub fn EngagementsPage() -> impl IntoView {
    let (selected_id, set_selected_id) = create_signal(Option::<i64>::None);
    let (refresh, set_refresh) = create_signal(0_u64);
    let (new_open, set_new_open) = create_signal(false);
    let (title, set_title) = create_signal(String::new());
    let (description, set_description) = create_signal(String::new());
    let (customer_contact_id, set_customer_contact_id) = create_signal(String::new());
    let (offer_choice, set_offer_choice) = create_signal(String::new());
    let (invoice_choice, set_invoice_choice) = create_signal(String::new());
    let (email_choice, set_email_choice) = create_signal(String::new());
    let (error, set_error) = create_signal(Option::<String>::None);
    let (notice, set_notice) = create_signal(Option::<String>::None);
    let (customer_id_filter, set_customer_id_filter) = create_signal(Option::<i64>::None);

    create_effect(move |_| {
        if let Some(window) = web_sys::window() {
            if let Ok(search) = window.location().search() {
                if let Some(id) = web_sys::UrlSearchParams::new_with_str(&search)
                    .ok()
                    .and_then(|params| params.get("customer_id"))
                    .and_then(|value| value.parse::<i64>().ok())
                {
                    set_customer_id_filter.set(Some(id));
                }
            }
        }
    });

    let list = create_resource(
        move || (refresh.get(), customer_id_filter.get()),
        move |(_, filter_id)| async move { list_engagements(0, PAGE_SIZE, filter_id).await },
    );
    let selected = create_resource(
        move || selected_id.get(),
        |id| async move {
            match id {
                Some(id) => Some(get_engagement(id).await),
                None => None,
            }
        },
    );
    let contacts = create_resource(|| (), |_| async move { get_all_contacts().await });
    let candidates = create_resource(
        move || (selected_id.get(), refresh.get()),
        |(selected_id, _)| async move {
            let customer_id = match selected_id {
                Some(id) => get_engagement(id)
                    .await
                    .ok()
                    .and_then(|engagement| engagement.customer_contact.and_then(|c| c.id)),
                None => None,
            };
            let mut offers = get_offers(0, PAGE_SIZE, None, None, None).await?.items;
            let mut invoices = get_invoices(0, PAGE_SIZE, None, None, None).await?.items;
            let mut emails = list_emails("INBOX".to_string(), 0, PAGE_SIZE, None, None)
                .await?
                .items;
            emails.extend(
                list_emails("Sent".to_string(), 0, PAGE_SIZE, None, None)
                    .await?
                    .items,
            );
            let offer_rank = |offer: &OfferListItem| {
                (
                    offer
                        .customer_contact
                        .as_ref()
                        .and_then(|contact| contact.id)
                        != customer_id,
                    std::cmp::Reverse(offer.id),
                )
            };
            let invoice_rank = |invoice: &InvoiceListItem| {
                (
                    invoice
                        .customer_contact
                        .as_ref()
                        .and_then(|contact| contact.id)
                        != customer_id,
                    std::cmp::Reverse(invoice.id),
                )
            };
            let email_rank = |email: &EmailSummary| {
                (
                    email.customer_contact_id != customer_id,
                    std::cmp::Reverse(email.id),
                )
            };
            offers.sort_by_key(offer_rank);
            invoices.sort_by_key(invoice_rank);
            emails.sort_by_key(email_rank);
            Ok::<Candidates, leptos::ServerFnError>(Candidates {
                offers,
                invoices,
                emails,
            })
        },
    );

    create_effect(move |_| {
        if let Some(window) = web_sys::window() {
            if let Ok(search) = window.location().search() {
                if let Some(id) = web_sys::UrlSearchParams::new_with_str(&search)
                    .ok()
                    .and_then(|params| params.get("engagement_id"))
                    .and_then(|value| value.parse::<i64>().ok())
                {
                    set_selected_id.set(Some(id));
                }
            }
        }
    });

    create_effect(move |_| {
        if let Some(Ok(value)) = candidates.get() {
            if offer_choice.get_untracked().is_empty() {
                if let Some(offer) = value.offers.first() {
                    set_offer_choice.set(offer.id.to_string());
                }
            }
            if invoice_choice.get_untracked().is_empty() {
                if let Some(invoice) = value.invoices.first() {
                    set_invoice_choice.set(invoice.id.to_string());
                }
            }
            if email_choice.get_untracked().is_empty() {
                if let Some(email) = value.emails.first() {
                    set_email_choice.set(email.id.to_string());
                }
            }
        }
    });

    let save_action = create_action(move |input: &EngagementInput| {
        let input = input.clone();
        async move {
            set_error.set(None);
            match save_engagement(input).await {
                Ok(saved) => {
                    set_selected_id.set(saved.id);
                    set_new_open.set(false);
                    set_notice.set(Some("Auftrag gespeichert.".to_string()));
                    set_refresh.update(|value| *value = value.wrapping_add(1));
                }
                Err(save_error) => set_error.set(Some(save_error.to_string())),
            }
        }
    });

    let link_action = create_action(
        move |(engagement_id, kind, record_id): &(i64, EngagementLinkKind, i64)| {
            let (engagement_id, kind, record_id) = (*engagement_id, kind.clone(), *record_id);
            async move {
                set_error.set(None);
                match link_engagement(engagement_id, kind, record_id).await {
                    Ok(()) => {
                        set_notice.set(Some("Datensatz mit Auftrag verknüpft.".to_string()));
                        set_refresh.update(|value| *value = value.wrapping_add(1));
                    }
                    Err(link_error) => set_error.set(Some(link_error.to_string())),
                }
            }
        },
    );

    let create_invoice_action = create_action(move |(offer_id, engagement_id): &(i64, i64)| {
        let offer_id = *offer_id;
        let engagement_id = *engagement_id;
        async move {
            match create_invoice_from_offer(offer_id, Some(engagement_id)).await {
                Ok(_) => {
                    set_notice.set(Some(
                        "Rechnungsentwurf aus dem Angebot angelegt.".to_string(),
                    ));
                    set_refresh.update(|value| *value = value.wrapping_add(1));
                }
                Err(error) => set_error.set(Some(error.to_string())),
            }
        }
    });

    let save_new = move |_| {
        save_action.dispatch(EngagementInput {
            id: None,
            title: title.get_untracked(),
            description: (!description.get_untracked().trim().is_empty())
                .then(|| description.get_untracked()),
            customer_contact_id: customer_contact_id.get_untracked().parse::<i64>().ok(),
        });
    };

    let link_offer = move |_| {
        if let (Some(id), Ok(record_id)) = (
            selected_id.get_untracked(),
            offer_choice.get_untracked().parse::<i64>(),
        ) {
            link_action.dispatch((id, EngagementLinkKind::Offer, record_id));
        }
    };
    let link_invoice = move |_| {
        if let (Some(id), Ok(record_id)) = (
            selected_id.get_untracked(),
            invoice_choice.get_untracked().parse::<i64>(),
        ) {
            link_action.dispatch((id, EngagementLinkKind::Invoice, record_id));
        }
    };
    let link_email = move |_| {
        if let (Some(id), Ok(record_id)) = (
            selected_id.get_untracked(),
            email_choice.get_untracked().parse::<i64>(),
        ) {
            link_action.dispatch((id, EngagementLinkKind::Email, record_id));
        }
    };

    view! {
        <div class="container">
            <div class="level">
                <div class="level-left"><h1 class="title">"Aufträge"</h1></div>
                <div class="level-right">
                    <button class="button is-link" on:click=move |_| set_new_open.update(|value| *value = !*value)>
                        <span class="icon mr-1"><i class="mdi mdi-briefcase-plus-outline"></i></span>"Neuer Auftrag"
                    </button>
                </div>
            </div>

            {move || notice.get().map(|message| view! {
                <div class="message is-success"><div class="message-body">{message}</div></div>
            })}
            {move || error.get().map(|message| view! {
                <div class="message is-danger"><div class="message-body">{message}</div></div>
            })}
            {move || customer_id_filter.get().map(|cid| {
                let contact_name = contacts.get().and_then(Result::ok).and_then(|list| {
                    list.into_iter().find(|c| c.id == Some(cid)).map(|c| c.display_name())
                }).unwrap_or_else(|| format!("Kunde #{}", cid));
                view! {
                    <div class="notification is-info is-light py-2 px-3 mb-3 is-flex is-justify-content-space-between is-align-items-center">
                        <span>"Filter: Nur Aufträge von " <strong>{contact_name}</strong></span>
                        <button class="button is-small is-light" on:click=move |_| {
                            set_customer_id_filter.set(None);
                            if let Some(window) = web_sys::window() {
                                if let Ok(history) = window.history() {
                                    let _ = history.push_state_with_url(
                                        &wasm_bindgen::JsValue::null(),
                                        "",
                                        Some("/engagements")
                                    );
                                }
                            }
                        }>"Filter aufheben"</button>
                    </div>
                }.into_view()
            })}

            {move || new_open.get().then(|| view! {
                <div class="box mb-4">
                    <h2 class="subtitle">"Neuer Auftrag"</h2>
                    <div class="field">
                        <label class="label">"Titel"</label>
                        <input class="input" prop:value=title on:input=move |event| set_title.set(event_target_value(&event)) placeholder="z. B. Website-Relaunch Müller GmbH" />
                    </div>
                    <div class="field">
                        <label class="label">"Beschreibung"</label>
                        <textarea class="textarea" prop:value=description on:input=move |event| set_description.set(event_target_value(&event))></textarea>
                    </div>
                    <div class="field">
                        <label class="label">"Kunde"</label>
                        <div class="select is-fullwidth">
                            <select prop:value=customer_contact_id on:change=move |event| set_customer_contact_id.set(event_target_value(&event))>
                                <option value="">"-- Kein Kunde --"</option>
                                <Suspense fallback=move || view! { <option>"Lade Kontakte…"</option> }>
                                    {move || contacts.get().and_then(Result::ok).map(|items| items.into_iter().map(|contact| view! {
                                        <option value=contact.id.unwrap_or_default().to_string()>{contact.display_name()}</option>
                                    }).collect_view())}
                                </Suspense>
                            </select>
                        </div>
                    </div>
                    <div class="field is-grouped">
                        <button class="button is-link" prop:disabled=move || save_action.pending().get() on:click=save_new>"Auftrag anlegen"</button>
                        <button class="button" on:click=move |_| set_new_open.set(false)>"Abbrechen"</button>
                    </div>
                </div>
            })}

            <div class="columns is-split">
                <div class="column">
                    <div class="box">
                        <Suspense fallback=move || view! { <p class="text-muted">"Lade Aufträge…"</p> }>
                            {move || list.get().map(|result| match result {
                                Err(load_error) => view! { <div class="message is-danger"><div class="message-body">{load_error.to_string()}</div></div> }.into_view(),
                                Ok(page) if page.items.is_empty() => view! { <EmptyState icon="briefcase-outline" text="Noch kein Auftrag angelegt." /> }.into_view(),
                                Ok(page) => view! {
                                    <div>
                                        {page.items.into_iter().map(|item| {
                                            let id = item.id;
                                            view! {
                                                <div class="box list-item p-3 mb-2" class:is-active=move || selected_id.get() == Some(id) on:click=move |_| set_selected_id.set(Some(id))>
                                                    <div class="has-text-weight-bold">{item.title.clone()}</div>
                                                    <div class="text-muted is-size-7">{item.customer_name.clone().unwrap_or_else(|| "Kein Kontakt".to_string())}</div>
                                                    <div class="tags mt-2">
                                                        <span class="tag">{format!("{} Angebote", item.offer_count)}</span>
                                                        <span class="tag">{format!("{} Rechnungen", item.invoice_count)}</span>
                                                        <span class="tag">{format!("{} E-Mails", item.email_count)}</span>
                                                    </div>
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
                    <Suspense fallback=move || view! { <div class="box"><p class="text-muted">"Lade Auftrag…"</p></div> }>
                        {move || selected.get().and_then(|value| value).map(|result| match result {
                            Err(load_error) => view! { <div class="message is-danger"><div class="message-body">{load_error.to_string()}</div></div> }.into_view(),
                            Ok(engagement) => view! { <EngagementDetail
                                engagement=engagement
                                candidates=candidates
                                link_offer=Callback::new(link_offer)
                                link_invoice=Callback::new(link_invoice)
                                link_email=Callback::new(link_email)
                                on_offer_change=Callback::new(move |value| set_offer_choice.set(value))
                                on_invoice_change=Callback::new(move |value| set_invoice_choice.set(value))
                                on_email_change=Callback::new(move |value| set_email_choice.set(value))
                                on_create_invoice=Callback::new(move |pair| create_invoice_action.dispatch(pair))
                            /> }.into_view(),
                        })}
                    </Suspense>
                    {move || selected_id.get().is_none().then(|| view! {
                        <EmptyState icon="briefcase-outline" text="Wählen Sie einen Auftrag aus." />
                    })}
                </div>
            </div>
        </div>
    }
}

#[component]
fn EngagementDetail(
    engagement: Engagement,
    candidates: Resource<(Option<i64>, u64), Result<Candidates, leptos::ServerFnError>>,
    link_offer: Callback<()>,
    link_invoice: Callback<()>,
    link_email: Callback<()>,
    on_offer_change: Callback<String>,
    on_invoice_change: Callback<String>,
    on_email_change: Callback<String>,
    on_create_invoice: Callback<(i64, i64)>,
) -> impl IntoView {
    let links = engagement.links.clone();
    view! {
        <div class="box">
            <h2 class="subtitle mb-1">{engagement.title.clone()}</h2>
            {engagement.description.clone().map(|description| view! {
                <p class="text-muted mb-4">{description}</p>
            })}
            <h3 class="is-size-6 has-text-weight-bold mb-2">"Verknüpfte Vorgänge"</h3>
            {if links.is_empty() {
                view! { <p class="text-muted mb-4">"Noch keine Mails, Angebote oder Rechnungen verknüpft."</p> }.into_view()
            } else {
                view! { <div class="tags mb-4">{links.into_iter().map(|link| {
                    let offer_id = link.id;
                    let is_offer = matches!(link.kind, EngagementLinkKind::Offer);
                    view! {
                        <span class="tag is-info">{format!("{:?}: {} · {}", link.kind, link.label, link.status)}</span>
                        {is_offer.then(|| view! { <button class="button is-small is-light ml-1" on:click=move |_| on_create_invoice.call((offer_id, engagement.id.unwrap_or_default()))>{"Rechnung erstellen"}</button> })}
                    }
                }).collect_view()}</div> }.into_view()
            }}

            <hr/>
            <h3 class="is-size-6 has-text-weight-bold mb-3">"Vorgang verknüpfen"</h3>
            <Suspense fallback=move || view! { <p class="text-muted is-size-7">"Lade Auswahl…"</p> }>
                {move || candidates.get().map(|result| match result {
                    Err(error) => view! { <p class="text-muted">{error.to_string()}</p> }.into_view(),
                    Ok(candidates) => view! {
                        <div>
                            <div class="field-row">
                                <div class="field">
                                    <label class="label is-small">"Angebot"</label>
                                    <div class="select is-fullwidth"><select on:change=move |event| on_offer_change.call(event_target_value(&event))>{candidates.offers.into_iter().map(|offer| view! {
                                        <option value=offer.id.to_string()>{format!("{}{}", if offer.committed { "Finalisiert: " } else { "Entwurf: " }, offer.title.unwrap_or_else(|| "Angebot".to_string()))}</option>
                                    }).collect_view()}</select></div>
                                </div>
                                <button class="button is-light" on:click=move |_| link_offer.call(())>"Verknüpfen"</button>
                            </div>
                            <p class="help">"Wählen Sie einen Datensatz; bei mehreren Einträgen kann die Auswahl über die Browser-Auswahl getroffen werden."</p>
                            <div class="field-row mt-3">
                                <div class="field">
                                    <label class="label is-small">"Rechnung"</label>
                                    <div class="select is-fullwidth"><select on:change=move |event| on_invoice_change.call(event_target_value(&event))>{candidates.invoices.into_iter().map(|invoice| view! {
                                        <option value=invoice.id.to_string()>{format!("{}{}", if invoice.committed { "Finalisiert: " } else { "Entwurf: " }, invoice.subject.unwrap_or_else(|| "Rechnung".to_string()))}</option>
                                    }).collect_view()}</select></div>
                                </div>
                                <button class="button is-light" on:click=move |_| link_invoice.call(())>"Verknüpfen"</button>
                            </div>
                            <div class="field-row mt-3">
                                <div class="field">
                                    <label class="label is-small">"E-Mail"</label>
                                    <div class="select is-fullwidth"><select on:change=move |event| on_email_change.call(event_target_value(&event))>{candidates.emails.into_iter().map(|email| view! {
                                        <option value=email.id.to_string()>{format!("{}: {}", email.subject, email.sender)}</option>
                                    }).collect_view()}</select></div>
                                </div>
                                <button class="button is-light" on:click=move |_| link_email.call(())>"Verknüpfen"</button>
                            </div>
                        </div>
                    }.into_view(),
                })}
            </Suspense>
        </div>
    }
}
