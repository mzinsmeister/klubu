use leptos::*;

use crate::components::{EmptyState, MoneyInput, QuantityInput, TextFieldHint};
use crate::server::{
    commit_offer, create_offer_revision, delete_offer, export_offer_pdf, get_all_contacts,
    get_offer, get_offers, save_offer,
};
use chrono::{NaiveDate, Utc};
use shared::*;

const OFFER_PAGE_SIZE: u32 = 50;

#[component]
fn OfferEditor(
    off: Offer,
    contacts: ReadSignal<Vec<Contact>>,
    on_change: Callback<()>,
    set_selected_offer: WriteSignal<Option<Offer>>,
) -> impl IntoView {
    let is_committed = off.committed_timestamp.is_some();
    let offer_id = off.id;
    let offer_number = off.offer_number;

    let display_number = if is_committed {
        format!(" • Angebot #{}", offer_number.unwrap_or_default())
    } else {
        String::new()
    };

    let (offer_date, set_offer_date) = create_signal(
        off.offer_date
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_default(),
    );
    let (subject, set_subject) = create_signal(off.subject.clone().unwrap_or_default());
    let (header, set_header) = create_signal(off.header.clone().unwrap_or_default());
    let (footer, set_footer) = create_signal(off.footer.clone().unwrap_or_default());
    let (customer_contact, set_customer_contact) = create_signal(off.customer_contact.clone());
    let (document, set_document) = create_signal(off.document.clone());
    let (items_list, set_items_list) = create_signal(off.items.clone());

    let recipient = off.recipient.clone().unwrap_or(Recipient {
        form_of_address: None,
        title: None,
        name: String::new(),
        first_name: None,
        street: None,
        zip_code: None,
        city: None,
        house_number: None,
        country: None,
    });

    let (recipient_name, set_recipient_name) = create_signal(recipient.name.clone());
    let (recipient_first_name, set_recipient_first_name) =
        create_signal(recipient.first_name.clone().unwrap_or_default());
    let (recipient_title, set_recipient_title) =
        create_signal(recipient.title.clone().unwrap_or_default());
    let (recipient_form_of_address, set_recipient_form_of_address) =
        create_signal(recipient.form_of_address.clone().unwrap_or_default());
    let (recipient_street, set_recipient_street) =
        create_signal(recipient.street.clone().unwrap_or_default());
    let (recipient_house_number, set_recipient_house_number) =
        create_signal(recipient.house_number.clone().unwrap_or_default());
    let (recipient_zip_code, set_recipient_zip_code) =
        create_signal(recipient.zip_code.clone().unwrap_or_default());
    let (recipient_city, set_recipient_city) =
        create_signal(recipient.city.clone().unwrap_or_default());
    let (recipient_country, set_recipient_country) =
        create_signal(recipient.country.clone().unwrap_or_default());

    let (item_desc, set_item_desc) = create_signal(String::new());
    let item_qty = create_rw_signal(1.0f64);
    let item_price = create_rw_signal(0i64);

    let save_offer_act = create_action(move |o: &Offer| {
        let o = o.clone();
        async move {
            match save_offer(o).await {
                Ok(saved) => {
                    on_change.call(());
                    set_selected_offer.set(Some(saved));
                }
                Err(e) => logging::log!("Error saving offer: {:?}", e),
            }
        }
    });

    let commit_offer_act = create_action(move |id: &i64| {
        let id = *id;
        async move {
            match commit_offer(id).await {
                Ok(_) => {
                    on_change.call(());
                    if let Ok(full_off) = get_offer(id).await {
                        set_selected_offer.set(Some(full_off));
                    }
                }
                Err(e) => logging::log!("Error finalizing offer: {:?}", e),
            }
        }
    });

    let delete_offer_act = create_action(move |id: &i64| {
        let id = *id;
        async move {
            match delete_offer(id).await {
                Ok(_) => {
                    on_change.call(());
                    set_selected_offer.set(None);
                }
                Err(e) => logging::log!("Error deleting offer: {:?}", e),
            }
        }
    });

    let create_revision_act = create_action(move |id: &i64| {
        let id = *id;
        async move {
            match create_offer_revision(id).await {
                Ok(new_offer) => set_selected_offer.set(Some(new_offer)),
                Err(e) => logging::log!("Error creating revision: {:?}", e),
            }
        }
    });

    let export_act = create_action(move |off_id: &i64| {
        let off_id = *off_id;
        async move {
            match export_offer_pdf(off_id).await {
                Ok(doc) => set_document.set(Some(doc)),
                Err(e) => logging::log!("Error exporting offer: {:?}", e),
            }
        }
    });

    view! {
        <div class="box">
            <h2 class="subtitle">
                "Angebotsdetails" {display_number}
                {if is_committed {
                    view! { <span class="tag is-success ml-2">"Finalisiert"</span> }.into_view()
                } else {
                    view! { <span class="tag is-warning ml-2">"ENTWURF"</span> }.into_view()
                }}
            </h2>

            <div class="field">
                <label class="label">"Kunde (Kontakt)"</label>
                <div class="control">
                    <div class="select is-fullwidth">
                        <select
                            prop:disabled=is_committed
                            on:change=move |ev| {
                                let val = event_target_value(&ev);
                                if let Ok(id) = val.parse::<i64>() {
                                    if let Some(c) = contacts.get().iter().find(|con| con.id == Some(id)) {
                                        set_recipient_name.set(c.name.clone());
                                        set_recipient_first_name.set(c.first_name.clone().unwrap_or_default());
                                        set_recipient_title.set(c.title.clone().unwrap_or_default());
                                        set_recipient_form_of_address.set(c.form_of_address.clone().unwrap_or_default());
                                        set_recipient_street.set(c.street.clone().unwrap_or_default());
                                        set_recipient_house_number.set(c.house_number.clone().unwrap_or_default());
                                        set_recipient_zip_code.set(c.zip_code.clone().unwrap_or_default());
                                        set_recipient_city.set(c.city.clone().unwrap_or_default());
                                        set_recipient_country.set(c.country.clone().unwrap_or_default());
                                        set_customer_contact.set(Some(c.clone()));
                                    }
                                } else {
                                    set_customer_contact.set(None);
                                }
                            }
                        >
                            <option value="">"-- Kein Kunde ausgewählt --"</option>
                            {move || contacts.get().iter().map(|c| {
                                let sel = customer_contact.get().as_ref().and_then(|cc| cc.id) == c.id;
                                let display_name = c.display_name();
                                view! { <option value=c.id.unwrap_or_default() selected=sel>{display_name}</option> }
                            }).collect::<Vec<_>>()}
                        </select>
                    </div>
                </div>
            </div>

            <div class="box subbox p-4 mt-3 mb-3">
                <h3 class="has-text-weight-bold mb-3">"Empfängeradresse"</h3>
                <div class="field-row">
                    <div class="field is-narrow"><label class="label is-small">"Anrede"</label><div class="control"><input class="input is-small" type="text" prop:value=recipient_form_of_address on:input=move |ev| set_recipient_form_of_address.set(event_target_value(&ev)) prop:disabled=is_committed /></div></div>
                    <div class="field is-narrow"><label class="label is-small">"Titel"</label><div class="control"><input class="input is-small" type="text" prop:value=recipient_title on:input=move |ev| set_recipient_title.set(event_target_value(&ev)) prop:disabled=is_committed /></div></div>
                </div>
                <div class="field-row">
                    <div class="field"><label class="label is-small">"Vorname"</label><div class="control"><input class="input is-small" type="text" prop:value=recipient_first_name on:input=move |ev| set_recipient_first_name.set(event_target_value(&ev)) prop:disabled=is_committed /></div></div>
                    <div class="field"><label class="label is-small">"Name"</label><div class="control"><input class="input is-small" type="text" prop:value=recipient_name on:input=move |ev| set_recipient_name.set(event_target_value(&ev)) prop:disabled=is_committed /></div></div>
                </div>
                <div class="field-row">
                    <div class="field is-wide"><label class="label is-small">"Straße"</label><div class="control"><input class="input is-small" type="text" prop:value=recipient_street on:input=move |ev| set_recipient_street.set(event_target_value(&ev)) prop:disabled=is_committed /></div></div>
                    <div class="field is-narrow"><label class="label is-small">"Hausnummer"</label><div class="control"><input class="input is-small" type="text" prop:value=recipient_house_number on:input=move |ev| set_recipient_house_number.set(event_target_value(&ev)) prop:disabled=is_committed /></div></div>
                </div>
                <div class="field-row">
                    <div class="field is-narrow"><label class="label is-small">"PLZ"</label><div class="control"><input class="input is-small" type="text" prop:value=recipient_zip_code on:input=move |ev| set_recipient_zip_code.set(event_target_value(&ev)) prop:disabled=is_committed /></div></div>
                    <div class="field"><label class="label is-small">"Ort"</label><div class="control"><input class="input is-small" type="text" prop:value=recipient_city on:input=move |ev| set_recipient_city.set(event_target_value(&ev)) prop:disabled=is_committed /></div></div>
                    <div class="field"><label class="label is-small">"Land"</label><div class="control"><input class="input is-small" type="text" prop:value=recipient_country on:input=move |ev| set_recipient_country.set(event_target_value(&ev)) prop:disabled=is_committed /></div></div>
                </div>
            </div>

            <div class="field">
                <label class="label">"Datum"</label>
                <div class="control"><input class="input" type="date" prop:value=offer_date on:input=move |ev| set_offer_date.set(event_target_value(&ev)) prop:disabled=is_committed /></div>
            </div>

            <div class="field">
                <label class="label">"Betreff"</label>
                <div class="control"><input class="input" type="text" prop:value=subject on:input=move |ev| set_subject.set(event_target_value(&ev)) prop:disabled=is_committed /></div>
            </div>

            <div class="field">
                <label class="label">"Einleitungstext"</label>
                <div class="control"><textarea class="textarea" prop:value=header on:input=move |ev| set_header.set(event_target_value(&ev)) prop:disabled=is_committed></textarea></div>
                <TextFieldHint />
            </div>

            {move || if !is_committed {
                view! {
                    <div class="box subbox p-4">
                        <h3 class="has-text-weight-bold mb-3">"Position hinzufügen"</h3>
                        <div class="field-row">
                            <div class="field is-wide"><label class="label is-small">"Beschreibung"</label><input class="input" type="text" placeholder="Beschreibung" prop:value=item_desc on:input=move |ev| set_item_desc.set(event_target_value(&ev)) /></div>
                            <div class="field is-narrow"><label class="label is-small">"Menge"</label><QuantityInput value=item_qty /></div>
                            <div class="field is-narrow"><label class="label is-small">"Einzelpreis (€)"</label><MoneyInput value=item_price /></div>
                            <button class="button is-link" title="Hinzufügen" on:click=move |_| {
                                if item_desc.get().trim().is_empty() { return; }
                                let new_item = Item { item: item_desc.get().trim().to_string(), quantity: item_qty.get(), unit: "Stk".to_string(), price: Money::new(item_price.get()) };
                                set_items_list.update(|items| items.push(new_item));
                                set_item_desc.set(String::new());
                                item_qty.set(1.0);
                                item_price.set(0);
                            }><span class="icon"><i class="mdi mdi-plus"></i></span></button>
                        </div>
                    </div>
                }.into_view()
            } else { "".into_view() }}

            <div class="table-wrap mt-4">
            <table class="table is-fullwidth is-striped">
                <thead><tr><th>"Beschreibung"</th><th class="has-text-right">"Menge"</th><th class="has-text-right">"Einzelpreis"</th><th class="has-text-right">"Summe"</th><th></th></tr></thead>
                <tbody>
                    {move || {
                        let items = items_list.get();
                        if items.is_empty() {
                            return view! { <tr><td colspan="5" class="has-text-centered text-muted">"Noch keine Positionen."</td></tr> }.into_view();
                        }
                        items.into_iter().enumerate().map(|(idx, item)| {
                            let line_total = item.total_cents();
                            view! {
                                <tr>
                                    <td>{item.item.clone()}</td>
                                    <td class="is-numeric">{format_quantity(item.quantity)} " " {item.unit.clone()}</td>
                                    <td class="is-numeric">{format_euro(item.price.amount_cents)}</td>
                                    <td class="is-numeric">{format_euro(line_total)}</td>
                                    <td class="has-text-right">
                                        {(!is_committed).then(|| view! {
                                            <button class="button is-small is-danger is-outlined" title="Position entfernen"
                                                on:click=move |_| set_items_list.update(|items| { items.remove(idx); })>
                                                <span class="icon is-small"><i class="mdi mdi-delete"></i></span>
                                            </button>
                                        })}
                                    </td>
                                </tr>
                            }
                        }).collect::<Vec<_>>().into_view()
                    }}
                </tbody>
                <tfoot>
                    <tr>
                        <td colspan="3">"Gesamtbetrag"</td>
                        <td class="is-numeric">{move || format_euro(items_list.get().iter().map(Item::total_cents).sum::<i64>())}</td>
                        <td></td>
                    </tr>
                </tfoot>
            </table>
            </div>

            <div class="field">
                <label class="label">"Schlusstext"</label>
                <div class="control"><textarea class="textarea" prop:value=footer on:input=move |ev| set_footer.set(event_target_value(&ev)) prop:disabled=is_committed></textarea></div>
                <TextFieldHint />
            </div>

            <div class="field is-grouped mt-5">
                {if !is_committed {
                    view! {
                        <div class="control">
                            <button class="button is-success" on:click=move |_| {
                                let mut save_off = off.clone();
                                save_off.offer_date = NaiveDate::parse_from_str(&offer_date.get(), "%Y-%m-%d").ok();
                                save_off.subject = Some(subject.get());
                                save_off.header = Some(header.get());
                                save_off.footer = Some(footer.get());
                                save_off.items = items_list.get();
                                save_off.customer_contact = customer_contact.get();
                                save_off.recipient = Some(Recipient {
                                    form_of_address: Some(recipient_form_of_address.get()),
                                    title: Some(recipient_title.get()),
                                    name: recipient_name.get(),
                                    first_name: Some(recipient_first_name.get()),
                                    street: Some(recipient_street.get()),
                                    zip_code: Some(recipient_zip_code.get()),
                                    city: Some(recipient_city.get()),
                                    house_number: Some(recipient_house_number.get()),
                                    country: Some(recipient_country.get()),
                                });
                                save_offer_act.dispatch(save_off);
                            }>{"Speichern"}</button>
                        </div>
                    }.into_view()
                } else { "".into_view() }}

                {if let Some(id) = offer_id {
                    if is_committed {
                        view! {
                            {move || if let Some(doc) = document.get() {
                                view! { <div class="control"><a class="button is-link" href=format!("/api/documents/{}", doc.id) target="_blank"><span class="icon mr-1"><i class="mdi mdi-download"></i></span>"PDF herunterladen"</a></div> }.into_view()
                            } else {
                                view! { <div class="control"><button class="button is-info" on:click=move |_| export_act.dispatch(id) prop:disabled=export_act.pending()>{"Exportieren (PDF generieren)"}</button></div> }.into_view()
                            }}
                            <div class="control"><button class="button is-warning" prop:disabled=create_revision_act.pending() on:click=move |_| create_revision_act.dispatch(id)>{"Revision erstellen"}</button></div>
                        }.into_view()
                    } else {
                        view! {
                            <div class="control"><button class="button is-warning" prop:disabled=customer_contact.get().is_none() on:click=move |_| commit_offer_act.dispatch(id)>{"Finalisieren"}</button></div>
                            <div class="control"><a class="button is-light" href=format!("/api/pdf/offer/{}", id) target="_blank"><span class="icon mr-1"><i class="mdi mdi-file-pdf-box"></i></span>"Entwurf-Vorschau (PDF)"</a></div>
                            <div class="control"><button class="button is-danger" on:click=move |_| delete_offer_act.dispatch(id)>{"Entwurf löschen"}</button></div>
                        }.into_view()
                    }
                } else { "".into_view() }}

                <div class="control"><button class="button is-light" on:click=move |_| set_selected_offer.set(None)>{"Abbrechen"}</button></div>
            </div>

            {move || if !is_committed && customer_contact.get().is_none() {
                view! { <div class="message is-warning mt-2"><div class="message-body p-2 is-size-7"><span class="icon mr-1"><i class="mdi mdi-alert-circle"></i></span>"Ein Kontakt muss zugewiesen sein, bevor das Angebot finalisiert werden kann."</div></div> }.into_view()
            } else { "".into_view() }}
        </div>
    }
}

#[component]
pub fn OffersPage() -> impl IntoView {
    let (offers, set_offers) = create_signal(Vec::<OfferListItem>::new());
    let (selected_offer, set_selected_offer) = create_signal(Option::<Offer>::None);
    let (contacts, set_contacts) = create_signal(Vec::<Contact>::new());
    let (from_date_filter, set_from_date_filter) = create_signal(String::new());
    let (to_date_filter, set_to_date_filter) = create_signal(String::new());
    let (has_more_offers, set_has_more_offers) = create_signal(false);
    let (list_generation, set_list_generation) = create_signal(0_u64);

    let load_offers = create_action(
        move |(generation, offset, from, to): &(u64, u32, String, String)| {
            let generation = *generation;
            let offset = *offset;
            let from = from.clone();
            let to = to.clone();
            async move {
                let from_date = NaiveDate::parse_from_str(&from, "%Y-%m-%d").ok();
                let to_date = NaiveDate::parse_from_str(&to, "%Y-%m-%d").ok();
                match get_offers(offset, OFFER_PAGE_SIZE, from_date, to_date).await {
                    Ok(page) => {
                        if list_generation.get_untracked() != generation
                            || from_date_filter.get_untracked() != from
                            || to_date_filter.get_untracked() != to
                        {
                            return;
                        }

                        if offset == 0 {
                            set_offers.set(page.items);
                        } else if offers.get_untracked().len() as u32 == offset {
                            set_offers.update(|items| items.extend(page.items));
                        } else {
                            return;
                        }
                        set_has_more_offers.set(page.has_more);
                    }
                    Err(e) => logging::log!("Error fetching offers: {:?}", e),
                }
            }
        },
    );

    let load_contacts = create_action(move |_| async move {
        match get_all_contacts().await {
            Ok(list) => set_contacts.set(list),
            Err(e) => logging::log!("Error fetching contacts: {:?}", e),
        }
    });

    load_offers.dispatch((0, 0, String::new(), String::new()));
    load_contacts.dispatch(());

    create_effect(move |_| {
        logging::log!(
            "DEBUG: selected_offer is now {:?}",
            selected_offer.get().map(|o| o.id)
        );
    });

    // Full-screen editor: the list is only useful for picking, not while editing.
    let list_view = move || {
        view! {
            <div class="level">
                <div class="level-left"><h1 class="title">"Angebote"</h1></div>
                    <div class="level-right">
                        <button class="button is-link" on:click=move |_| {
                            set_selected_offer.set(Some(Offer {
                                id: None,
                                revision: None,
                                offer_number: None,
                                title: Some("Angebot".to_string()),
                                customer_contact: None,
                                offer_date: Some(Utc::now().naive_utc().date()),
                                valid_until_date: None,
                                recipient: Some(Recipient {
                                    form_of_address: Some("Herr".to_string()),
                                    title: None,
                                    name: "Name".to_string(),
                                    first_name: Some("Vorname".to_string()),
                                    street: Some("Musterstraße".to_string()),
                                    zip_code: Some("12345".to_string()),
                                    city: Some("Stadt".to_string()),
                                    house_number: Some("1".to_string()),
                                    country: Some("Deutschland".to_string()),
                                }),
                                items: vec![],
                                created_timestamp: None,
                                committed_timestamp: None,
                                subject: Some("Angebot".to_string()),
                                header: Some("Gerne bieten wir Ihnen Folgendes an:".to_string()),
                                footer: Some("Das Angebot ist unverbindlich.".to_string()),
                                document: None,
                            }));
                        }>{"Neues Angebot"}</button>
                    </div>
            </div>

            <div class="box">
                <div class="field-row mb-4">
                    <div class="field">
                        <label class="label is-small">"Von"</label>
                        <div class="control">
                            <input
                                class="input"
                                type="date"
                                prop:value=from_date_filter
                                on:input=move |ev| {
                                    let from = event_target_value(&ev);
                                    let generation = list_generation.get_untracked().wrapping_add(1);
                                    set_list_generation.set(generation);
                                    set_from_date_filter.set(from.clone());
                                    set_offers.set(Vec::new());
                                    set_has_more_offers.set(false);
                                    load_offers.dispatch((
                                        generation,
                                        0,
                                        from,
                                        to_date_filter.get_untracked(),
                                    ));
                                }
                            />
                        </div>
                    </div>
                    <div class="field">
                        <label class="label is-small">"Bis (einschließlich)"</label>
                        <div class="control">
                            <input
                                class="input"
                                type="date"
                                prop:value=to_date_filter
                                on:input=move |ev| {
                                    let to = event_target_value(&ev);
                                    let generation = list_generation.get_untracked().wrapping_add(1);
                                    set_list_generation.set(generation);
                                    set_to_date_filter.set(to.clone());
                                    set_offers.set(Vec::new());
                                    set_has_more_offers.set(false);
                                    load_offers.dispatch((
                                        generation,
                                        0,
                                        from_date_filter.get_untracked(),
                                        to,
                                    ));
                                }
                            />
                        </div>
                    </div>
                </div>
                <div>
                                {move || offers.get().into_iter().map(|off| {
                                    let contact_name = off.customer_contact.as_ref().map(Contact::display_name).unwrap_or_else(|| "Gast".to_string());
                                    let status_badge = if !off.committed {
                                        view! { <span class="tag is-warning ml-2">"ENTWURF"</span> }.into_view()
                                    } else {
                                        view! { <span class="tag is-success ml-2">"Finalisiert"</span> }.into_view()
                                    };
                                    let display_title = if off.committed {
                                        if let Some(num) = off.offer_number {
                                            format!("Angebot #{} - {}", num, off.title.clone().unwrap_or_else(|| "Angebot".to_string()))
                                        } else {
                                            off.title.clone().unwrap_or_else(|| "Angebot".to_string())
                                        }
                                    } else {
                                        format!("ENTWURF - {}", off.title.clone().unwrap_or_else(|| "Angebot".to_string()))
                                    };
                                    view! {
                                        <div class="box list-item p-3 mb-2" on:click=move |_| {
                                            let id = off.id;
                                            spawn_local(async move {
                                                if let Ok(full_off) = get_offer(id).await {
                                                    set_selected_offer.set(Some(full_off));
                                                }
                                            });
                                        }>
                                            <div class="has-text-weight-bold">{display_title} {status_badge} " (Rev: " {off.revision} ")"</div>
                                            <div class="is-size-7 text-muted">{contact_name}</div>
                                        </div>
                                    }
                                }).collect::<Vec<_>>()}
                </div>
                <Show when=move || has_more_offers.get()>
                    <div class="has-text-centered mt-3">
                        <button
                            class="button is-light"
                            prop:disabled=load_offers.pending()
                            on:click=move |_| {
                                let offset = offers.get_untracked().len() as u32;
                                load_offers.dispatch((
                                    list_generation.get_untracked(),
                                    offset,
                                    from_date_filter.get_untracked(),
                                    to_date_filter.get_untracked(),
                                ));
                            }
                        >
                            {move || if load_offers.pending().get() { "Lädt…" } else { "Mehr laden" }}
                        </button>
                    </div>
                </Show>
                {move || if offers.get().is_empty() && !load_offers.pending().get() {
                    view! { <EmptyState icon="file-sign" text="Noch kein Angebot angelegt." /> }.into_view()
                } else { "".into_view() }}
            </div>
        }
    };

    view! {
        <div class="container">
            {move || match selected_offer.get() {
                None => list_view().into_view(),
                Some(off) => view! {
                    <div class="level">
                        <div class="level-left">
                            <button class="button is-light" on:click=move |_| set_selected_offer.set(None)>
                                <span class="icon mr-1"><i class="mdi mdi-arrow-left"></i></span>
                                "Zurück zur Übersicht"
                            </button>
                        </div>
                    </div>
                    <OfferEditor
                        off=off
                        contacts=contacts
                        on_change=Callback::new(move |_| {
                            let generation = list_generation.get_untracked().wrapping_add(1);
                            set_list_generation.set(generation);
                            set_offers.set(Vec::new());
                            set_has_more_offers.set(false);
                            load_offers.dispatch((
                                generation,
                                0,
                                from_date_filter.get_untracked(),
                                to_date_filter.get_untracked(),
                            ));
                        })
                        set_selected_offer=set_selected_offer
                    />
                }.into_view(),
            }}
        </div>
    }
}
