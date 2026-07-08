use leptos::*;

use chrono::{NaiveDate, Utc};
use shared::*;
use crate::components::{EmptyState, MoneyInput, QuantityInput};
use crate::server::{
    get_contacts, get_invoices, get_invoice, save_invoice, cancel_invoice,
    commit_invoice, delete_invoice,
    export_invoice_pdf,
};

#[component]
fn InvoiceEditor(
    inv: Invoice,
    contacts: ReadSignal<Vec<Contact>>,
    on_change: Callback<()>,
    set_selected_invoice: WriteSignal<Option<Invoice>>,
) -> impl IntoView {
    let is_committed = inv.committed_timestamp.is_some();
    let is_canceled = inv.is_canceled;
    let invoice_id = inv.id;
    let invoice_number = inv.invoice_number;

    // The status badge next to the heading already says ENTWURF; repeating it
    // in the title read as a duplicate.
    let display_number = if is_committed {
        format!(" • Rechnung #{}", invoice_number.unwrap_or_default())
    } else {
        String::new()
    };

    let (invoice_date, set_invoice_date) = create_signal(inv.invoice_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default());
    let (subject, set_subject) = create_signal(inv.subject.clone().unwrap_or_default());
    let (header, set_header) = create_signal(inv.header_html.clone().unwrap_or_default());
    let (footer, set_footer) = create_signal(inv.footer_html.clone().unwrap_or_default());
    let (customer_contact, set_customer_contact) = create_signal(inv.customer_contact.clone());
    let (document, set_document) = create_signal(inv.document.clone());
    let (items_list, set_items_list) = create_signal(inv.items.clone());

    let recipient = inv.recipient.clone().unwrap_or(Recipient {
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
    let (recipient_first_name, set_recipient_first_name) = create_signal(recipient.first_name.clone().unwrap_or_default());
    let (recipient_title, set_recipient_title) = create_signal(recipient.title.clone().unwrap_or_default());
    let (recipient_form_of_address, set_recipient_form_of_address) = create_signal(recipient.form_of_address.clone().unwrap_or_default());
    let (recipient_street, set_recipient_street) = create_signal(recipient.street.clone().unwrap_or_default());
    let (recipient_house_number, set_recipient_house_number) = create_signal(recipient.house_number.clone().unwrap_or_default());
    let (recipient_zip_code, set_recipient_zip_code) = create_signal(recipient.zip_code.clone().unwrap_or_default());
    let (recipient_city, set_recipient_city) = create_signal(recipient.city.clone().unwrap_or_default());
    let (recipient_country, set_recipient_country) = create_signal(recipient.country.clone().unwrap_or_default());

    let (item_desc, set_item_desc) = create_signal(String::new());
    let item_qty = create_rw_signal(1.0f64);
    let item_price = create_rw_signal(0i64);

    let save_invoice_act = create_action(move |i: &Invoice| {
        let i = i.clone();
        async move {
            match save_invoice(i).await {
                Ok(saved) => {
                    on_change.call(());
                    set_selected_invoice.set(Some(saved));
                },
                Err(e) => logging::log!("Error saving invoice: {:?}", e),
            }
        }
    });

    let cancel_invoice_act = create_action(move |id: &i64| {
        let id = *id;
        async move {
            match cancel_invoice(id).await {
                Ok(_) => {
                    on_change.call(());
                    set_selected_invoice.set(None);
                },
                Err(e) => logging::log!("Error canceling invoice: {:?}", e),
            }
        }
    });

    let commit_invoice_act = create_action(move |id: &i64| {
        let id = *id;
        async move {
            match commit_invoice(id).await {
                Ok(_) => {
                    on_change.call(());
                    if let Ok(full_inv) = get_invoice(id).await {
                        set_selected_invoice.set(Some(full_inv));
                    }
                },
                Err(e) => logging::log!("Error finalizing invoice: {:?}", e),
            }
        }
    });

    let delete_invoice_act = create_action(move |id: &i64| {
        let id = *id;
        async move {
            match delete_invoice(id).await {
                Ok(_) => {
                    on_change.call(());
                    set_selected_invoice.set(None);
                },
                Err(e) => logging::log!("Error deleting invoice: {:?}", e),
            }
        }
    });

    let export_act = create_action(move |inv_id: &i64| {
        let inv_id = *inv_id;
        async move {
            match export_invoice_pdf(inv_id).await {
                Ok(doc) => set_document.set(Some(doc)),
                Err(e) => logging::log!("Error exporting invoice: {:?}", e),
            }
        }
    });

    view! {
        <div class="box">
            <h2 class="subtitle">
                "Rechnungsdetails" {display_number}
                {if is_committed {
                    if is_canceled {
                        view! { <span class="tag is-danger ml-2">"Storniert"</span> }.into_view()
                    } else {
                        view! { <span class="tag is-success ml-2">"Finalisiert"</span> }.into_view()
                    }
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
                <div class="columns is-multiline">
                    <div class="column is-3"><div class="field"><label class="label is-small">"Anrede"</label><div class="control"><input class="input is-small" type="text" prop:value=recipient_form_of_address on:input=move |ev| set_recipient_form_of_address.set(event_target_value(&ev)) prop:disabled=is_committed /></div></div></div>
                    <div class="column is-3"><div class="field"><label class="label is-small">"Titel"</label><div class="control"><input class="input is-small" type="text" prop:value=recipient_title on:input=move |ev| set_recipient_title.set(event_target_value(&ev)) prop:disabled=is_committed /></div></div></div>
                    <div class="column is-3"><div class="field"><label class="label is-small">"Vorname"</label><div class="control"><input class="input is-small" type="text" prop:value=recipient_first_name on:input=move |ev| set_recipient_first_name.set(event_target_value(&ev)) prop:disabled=is_committed /></div></div></div>
                    <div class="column is-3"><div class="field"><label class="label is-small">"Name (Pflichtfeld)"</label><div class="control"><input class="input is-small" type="text" prop:value=recipient_name on:input=move |ev| set_recipient_name.set(event_target_value(&ev)) prop:disabled=is_committed /></div></div></div>
                    <div class="column is-8"><div class="field"><label class="label is-small">"Straße"</label><div class="control"><input class="input is-small" type="text" prop:value=recipient_street on:input=move |ev| set_recipient_street.set(event_target_value(&ev)) prop:disabled=is_committed /></div></div></div>
                    <div class="column is-4"><div class="field"><label class="label is-small">"Hausnummer"</label><div class="control"><input class="input is-small" type="text" prop:value=recipient_house_number on:input=move |ev| set_recipient_house_number.set(event_target_value(&ev)) prop:disabled=is_committed /></div></div></div>
                    <div class="column is-3"><div class="field"><label class="label is-small">"PLZ"</label><div class="control"><input class="input is-small" type="text" prop:value=recipient_zip_code on:input=move |ev| set_recipient_zip_code.set(event_target_value(&ev)) prop:disabled=is_committed /></div></div></div>
                    <div class="column is-5"><div class="field"><label class="label is-small">"Ort"</label><div class="control"><input class="input is-small" type="text" prop:value=recipient_city on:input=move |ev| set_recipient_city.set(event_target_value(&ev)) prop:disabled=is_committed /></div></div></div>
                    <div class="column is-4"><div class="field"><label class="label is-small">"Land"</label><div class="control"><input class="input is-small" type="text" prop:value=recipient_country on:input=move |ev| set_recipient_country.set(event_target_value(&ev)) prop:disabled=is_committed /></div></div></div>
                </div>
            </div>

            <div class="field">
                <label class="label">"Datum"</label>
                <div class="control"><input class="input" type="date" prop:value=invoice_date on:input=move |ev| set_invoice_date.set(event_target_value(&ev)) prop:disabled=is_committed /></div>
            </div>

            <div class="field">
                <label class="label">"Betreff"</label>
                <div class="control"><input class="input" type="text" prop:value=subject on:input=move |ev| set_subject.set(event_target_value(&ev)) prop:disabled=is_committed /></div>
            </div>

            <div class="field">
                <label class="label">"Einleitungstext"</label>
                <div class="control"><textarea class="textarea" prop:value=header on:input=move |ev| set_header.set(event_target_value(&ev)) prop:disabled=is_committed></textarea></div>
            </div>

            {move || if !is_committed {
                view! {
                    <div class="box subbox p-4">
                        <h3 class="has-text-weight-bold mb-3">"Position hinzufügen"</h3>
                        <div class="columns is-vcentered">
                            <div class="column is-6"><div class="field"><label class="label is-small">"Beschreibung"</label><input class="input" type="text" placeholder="Beschreibung" prop:value=item_desc on:input=move |ev| set_item_desc.set(event_target_value(&ev)) /></div></div>
                            <div class="column is-2"><div class="field"><label class="label is-small">"Menge"</label><QuantityInput value=item_qty /></div></div>
                            <div class="column is-3"><div class="field"><label class="label is-small">"Einzelpreis (€)"</label><MoneyInput value=item_price /></div></div>
                            <div class="column is-1"><button class="button is-link is-fullwidth" title="Hinzufügen" on:click=move |_| {
                                if item_desc.get().trim().is_empty() { return; }
                                let new_item = Item { item: item_desc.get().trim().to_string(), quantity: item_qty.get(), unit: "Stk".to_string(), price: Money::new(item_price.get()) };
                                set_items_list.update(|items| items.push(new_item));
                                set_item_desc.set(String::new());
                                item_qty.set(1.0);
                                item_price.set(0);
                            }><span class="icon"><i class="mdi mdi-plus"></i></span></button></div>
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
            </div>

            <div class="field is-grouped mt-5">
                {if !is_committed {
                    view! {
                        <div class="control">
                            <button class="button is-success" on:click=move |_| {
                                let mut save_inv = inv.clone();
                                save_inv.invoice_date = NaiveDate::parse_from_str(&invoice_date.get(), "%Y-%m-%d").ok();
                                save_inv.subject = Some(subject.get());
                                save_inv.header_html = Some(header.get());
                                save_inv.footer_html = Some(footer.get());
                                save_inv.items = items_list.get();
                                save_inv.customer_contact = customer_contact.get();
                                save_inv.recipient = Some(Recipient {
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
                                save_invoice_act.dispatch(save_inv);
                            }>{"Speichern"}</button>
                        </div>
                    }.into_view()
                } else { "".into_view() }}

                {if let Some(id) = invoice_id {
                    if is_committed {
                        view! {
                            {if !is_canceled {
                                view! { <div class="control"><button class="button is-danger" on:click=move |_| cancel_invoice_act.dispatch(id)>{"Stornieren"}</button></div> }.into_view()
                            } else {
                                view! { <div class="control"><button class="button is-danger" disabled=true>{"Storniert"}</button></div> }.into_view()
                            }}
                            {move || if let Some(doc) = document.get() {
                                view! { <div class="control"><a class="button is-link" href=format!("/api/documents/{}", doc.id) target="_blank"><span class="icon mr-1"><i class="mdi mdi-download"></i></span>"PDF herunterladen"</a></div> }.into_view()
                            } else {
                                view! { <div class="control"><button class="button is-info" on:click=move |_| export_act.dispatch(id) prop:disabled=export_act.pending()>{"Exportieren (PDF generieren)"}</button></div> }.into_view()
                            }}
                        }.into_view()
                    } else {
                        view! {
                            <div class="control"><button class="button is-warning" prop:disabled=customer_contact.get().is_none() on:click=move |_| commit_invoice_act.dispatch(id)>{"Finalisieren"}</button></div>
                            <div class="control"><a class="button is-light" href=format!("/api/pdf/invoice/{}", id) target="_blank"><span class="icon mr-1"><i class="mdi mdi-file-pdf-box"></i></span>"Entwurf-Vorschau (PDF)"</a></div>
                            <div class="control"><button class="button is-danger" on:click=move |_| delete_invoice_act.dispatch(id)>{"Entwurf löschen"}</button></div>
                        }.into_view()
                    }
                } else { "".into_view() }}

                <div class="control"><button class="button is-light" on:click=move |_| set_selected_invoice.set(None)>{"Abbrechen"}</button></div>
            </div>

            {move || if !is_committed && customer_contact.get().is_none() {
                view! { <div class="message is-warning mt-2"><div class="message-body p-2 is-size-7"><span class="icon mr-1"><i class="mdi mdi-alert-circle"></i></span>"Ein Kontakt muss zugewiesen sein, bevor die Rechnung finalisiert werden kann."</div></div> }.into_view()
            } else { "".into_view() }}
        </div>
    }
}

#[component]
pub fn InvoicesPage() -> impl IntoView {
    let (invoices, set_invoices) = create_signal(Vec::<InvoiceListItem>::new());
    let (selected_invoice, set_selected_invoice) = create_signal(Option::<Invoice>::None);
    let (contacts, set_contacts) = create_signal(Vec::<Contact>::new());

    let load_invoices = create_action(move |_| async move {
        match get_invoices().await {
            Ok(list) => set_invoices.set(list),
            Err(e) => logging::log!("Error fetching invoices: {:?}", e),
        }
    });

    let load_contacts = create_action(move |_| async move {
        match get_contacts().await {
            Ok(list) => set_contacts.set(list),
            Err(e) => logging::log!("Error fetching contacts: {:?}", e),
        }
    });

    load_invoices.dispatch(());
    load_contacts.dispatch(());


    view! {
        <div class="container">
            <div class="level">
                <div class="level-left"><h1 class="title">"Rechnungen"</h1></div>
                <div class="level-right">
                    <button class="button is-link" on:click=move |_| {
                        set_selected_invoice.set(Some(Invoice {
                            id: None,
                            items: vec![],
                            created_timestamp: None,
                            committed_timestamp: None,
                            invoice_number: None,
                            payments: vec![],
                            invoice_date: Some(Utc::now().naive_utc().date()),
                            is_canceled: false,
                            is_cancelation: false,
                            corrected_invoice_id: None,
                            customer_contact: None,
                            document: None,
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
                            header_html: Some("Vielen Dank für Ihre Bestellung.".to_string()),
                            footer_html: Some("Bitte überweisen Sie den Betrag innerhalb von 14 Tagen.".to_string()),
                            title: Some("Rechnung".to_string()),
                            subject: Some("Rechnung".to_string()),
                        }));
                    }>{"Neue Rechnung"}</button>
                </div>
            </div>

            <div class="columns">
                <div class="column is-5">
                    <div class="box">
                        <div style="max-height: 70vh; overflow-y: auto;">
                            {move || invoices.get().into_iter().map(|inv| {
                                let inv_id = inv.id;
                                let contact_name = inv.customer_contact.as_ref().map(Contact::display_name).unwrap_or_else(|| "Gast".to_string());
                                let status_badge = if !inv.committed {
                                    view! { <span class="tag is-warning ml-2">"ENTWURF"</span> }.into_view()
                                } else if inv.is_canceled {
                                    view! { <span class="tag is-danger ml-2">"Storniert"</span> }.into_view()
                                } else {
                                    view! { <span class="tag is-success ml-2">"Finalisiert"</span> }.into_view()
                                };
                                let display_title = inv.subject.clone().unwrap_or_else(|| "Rechnung".to_string());
                                // The badge already says ENTWURF; show the date instead of repeating it.
                                let secondary = if inv.committed {
                                    format!("Rechnung #{}", inv.invoice_number.unwrap_or_default())
                                } else {
                                    inv.created_timestamp.format("%d.%m.%Y").to_string()
                                };
                                view! {
                                    <div
                                        class="box list-item p-3 mb-2"
                                        class:is-active=move || selected_invoice.get().and_then(|i| i.id) == Some(inv_id)
                                        on:click=move |_| {
                                            spawn_local(async move {
                                                if let Ok(full_inv) = get_invoice(inv_id).await {
                                                    set_selected_invoice.set(Some(full_inv));
                                                }
                                            });
                                        }
                                    >
                                        <div class="has-text-weight-bold">{display_title} {status_badge}</div>
                                        <div class="is-size-7 text-muted">{secondary} " • " {contact_name}</div>
                                    </div>
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    </div>
                </div>

                <div class="column">
                    {move || match selected_invoice.get() {
                        None => view! {
                            <EmptyState icon="file-document-outline" text="Wählen Sie eine Rechnung aus." />
                        }.into_view(),
                        Some(inv) => view! {
                            <InvoiceEditor inv=inv contacts=contacts on_change=Callback::new(move |_| load_invoices.dispatch(())) set_selected_invoice=set_selected_invoice />
                        }.into_view(),
                    }}
                </div>
            </div>
        </div>
    }
}