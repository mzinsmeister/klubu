use leptos::*;
use leptos_router::{use_navigate, use_params_map, NavigateOptions};

use crate::components::{EmptyState, MoneyInput, PaymentsPanel, QuantityInput, TextFieldHint};
use crate::server::{
    add_invoice_payment, cancel_invoice, commit_invoice, create_invoice_reminder, delete_invoice,
    delete_invoice_payment, export_invoice_pdf, get_all_contacts, get_invoice,
    get_invoice_text_defaults, get_invoices, list_engagements, save_invoice, send_invoice_email,
};
use chrono::{NaiveDate, Utc};
use shared::*;

const INVOICE_PAGE_SIZE: u32 = 50;

#[component]
fn InvoiceEditor(
    inv: Invoice,
    contacts: ReadSignal<Vec<Contact>>,
    on_change: Callback<()>,
    set_selected_invoice: WriteSignal<Option<Invoice>>,
    set_dirty: WriteSignal<bool>,
) -> impl IntoView {
    let is_committed = inv.committed_timestamp.is_some();
    let is_canceled = inv.is_canceled;
    let is_cancelation = inv.is_cancelation;
    let is_credit_note = inv.is_credit_note;
    let invoice_id = inv.id;
    let invoice_number = inv.invoice_number;
    let original_invoice_id = inv.corrected_invoice_id;
    let original_invoice_number = inv.corrected_invoice_number;
    let cancellation_invoice_id = inv.cancellation_invoice_id;
    let credited_cents = inv.credited_cents;
    let discount_taken_cents = inv.discount_taken_cents;
    let invoice_total_cents = inv.items.iter().map(Item::total_cents).sum::<i64>();
    let invoice_is_due = inv
        .due_date
        .map(|date| date < Utc::now().date_naive())
        .unwrap_or(false);
    let (cancel_reason, set_cancel_reason) = create_signal(String::new());
    let (cancel_open, set_cancel_open) = create_signal(false);
    let (cancel_error, set_cancel_error) = create_signal(Option::<String>::None);

    // The status badge next to the heading already says ENTWURF; repeating it
    // in the title read as a duplicate.
    let display_number = if is_committed {
        format!(" • Rechnung #{}", invoice_number.unwrap_or_default())
    } else {
        String::new()
    };

    let (invoice_date, set_invoice_date) = create_signal(
        inv.invoice_date
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_default(),
    );
    let (due_date, set_due_date) = create_signal(
        inv.due_date
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_default(),
    );
    let (discount_date, set_discount_date) = create_signal(
        inv.discount_date
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_default(),
    );
    let (discount_percent, set_discount_percent) =
        create_signal(format!("{:.2}", inv.discount_basis_points as f64 / 100.0));
    let (subject, set_subject) = create_signal(inv.subject.clone().unwrap_or_default());
    let (header, set_header) = create_signal(inv.header.clone().unwrap_or_default());
    let (footer, set_footer) = create_signal(inv.footer.clone().unwrap_or_default());
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

    let (payments_list, set_payments_list) = create_signal(inv.payments.clone());
    let (reminders, set_reminders) = create_signal(inv.reminders.clone());
    let reminder_fee = create_rw_signal(0_i64);
    let (reminder_note, set_reminder_note) = create_signal(String::new());
    let (mail_open, set_mail_open) = create_signal(false);
    let default_mail_recipient = inv
        .customer_contact
        .as_ref()
        .and_then(|contact| contact.emails.first())
        .cloned()
        .unwrap_or_default();
    let (mail_recipient, set_mail_recipient) = create_signal(default_mail_recipient);
    let (mail_body, set_mail_body) = create_signal(String::new());
    let (mail_engagement, set_mail_engagement) = create_signal(String::new());
    let engagement_customer_id = inv.customer_contact.as_ref().and_then(|contact| contact.id);
    let engagements = create_resource(
        move || (),
        move |_| async move { list_engagements(0, 100, engagement_customer_id).await },
    );

    let has_unsaved_changes = {
        let inv = inv.clone();
        let recipient = recipient.clone();
        move || {
            let orig_date = inv
                .invoice_date
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_default();
            invoice_date.get() != orig_date
                || due_date.get()
                    != inv
                        .due_date
                        .map(|d| d.format("%Y-%m-%d").to_string())
                        .unwrap_or_default()
                || discount_date.get()
                    != inv
                        .discount_date
                        .map(|d| d.format("%Y-%m-%d").to_string())
                        .unwrap_or_default()
                || discount_percent
                    .get()
                    .replace(',', ".")
                    .parse::<f64>()
                    .unwrap_or(0.0)
                    .mul_add(100.0, 0.0)
                    .round() as i64
                    != inv.discount_basis_points
                || subject.get() != inv.subject.clone().unwrap_or_default()
                || header.get() != inv.header.clone().unwrap_or_default()
                || footer.get() != inv.footer.clone().unwrap_or_default()
                || customer_contact.get().as_ref().and_then(|c| c.id)
                    != inv.customer_contact.as_ref().and_then(|c| c.id)
                || items_list.get() != inv.items
                || recipient_name.get() != recipient.name
                || recipient_first_name.get() != recipient.first_name.clone().unwrap_or_default()
                || recipient_title.get() != recipient.title.clone().unwrap_or_default()
                || recipient_form_of_address.get()
                    != recipient.form_of_address.clone().unwrap_or_default()
                || recipient_street.get() != recipient.street.clone().unwrap_or_default()
                || recipient_house_number.get()
                    != recipient.house_number.clone().unwrap_or_default()
                || recipient_zip_code.get() != recipient.zip_code.clone().unwrap_or_default()
                || recipient_city.get() != recipient.city.clone().unwrap_or_default()
                || recipient_country.get() != recipient.country.clone().unwrap_or_default()
        }
    };

    create_effect({
        let has_changes = has_unsaved_changes.clone();
        move |_| {
            set_dirty.set(has_changes());
        }
    });

    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::prelude::*;
        use wasm_bindgen::JsCast;
        let has_changes = has_unsaved_changes.clone();
        let listener = Closure::<dyn FnMut(web_sys::BeforeUnloadEvent) -> String>::new(
            move |e: web_sys::BeforeUnloadEvent| {
                if has_changes() {
                    let msg = "Sie haben ungespeicherte Änderungen.";
                    e.set_return_value(msg);
                    msg.to_string()
                } else {
                    "".to_string()
                }
            },
        );
        if let Some(w) = web_sys::window() {
            let _ = w.add_event_listener_with_callback(
                "beforeunload",
                listener.as_ref().unchecked_ref(),
            );
            let cb_ref = listener.as_ref().clone();
            leptos::on_cleanup(move || {
                if let Some(w) = web_sys::window() {
                    let _ = w.remove_event_listener_with_callback(
                        "beforeunload",
                        cb_ref.unchecked_ref(),
                    );
                }
            });
        }
        listener.forget();
    }

    // After booking or removing a tranche, re-read the invoice: the server owns
    // the payment ids and rejects bookings the client cannot foresee.
    let refresh_payments = move || {
        if let Some(id) = invoice_id {
            spawn_local(async move {
                if let Ok(full) = get_invoice(id).await {
                    set_payments_list.set(full.payments);
                }
            });
        }
    };

    let add_payment_act = create_action(move |(amount, date): &(i64, NaiveDate)| {
        let (amount, date) = (*amount, *date);
        async move {
            let Some(id) = invoice_id else { return };
            match add_invoice_payment(id, amount, date).await {
                Ok(()) => {
                    refresh_payments();
                    on_change.call(());
                }
                Err(e) => logging::log!("Error adding payment: {:?}", e),
            }
        }
    });

    let delete_payment_act = create_action(move |payment_id: &i64| {
        let payment_id = *payment_id;
        async move {
            match delete_invoice_payment(payment_id).await {
                Ok(()) => {
                    refresh_payments();
                    on_change.call(());
                }
                Err(e) => logging::log!("Error deleting payment: {:?}", e),
            }
        }
    });

    let create_reminder_act = create_action(move |(invoice_id, fee, note): &(i64, i64, String)| {
        let (invoice_id, fee, note) = (*invoice_id, *fee, note.clone());
        async move {
            if let Ok(reminder) = create_invoice_reminder(invoice_id, fee, note).await {
                set_reminders.update(|items| items.push(reminder));
                reminder_fee.set(0);
                set_reminder_note.set(String::new());
            }
        }
    });

    let navigate = use_navigate();
    let navigate_for_save = navigate.clone();
    let navigate_for_cancel = navigate.clone();
    let navigate_for_related = navigate.clone();
    let navigate_for_delete = navigate.clone();

    let save_invoice_act = create_action(move |i: &Invoice| {
        let i = i.clone();
        let navigate = navigate_for_save.clone();
        async move {
            match save_invoice(i).await {
                Ok(saved) => {
                    on_change.call(());
                    let target_path = format!("/invoices/{}", saved.id.unwrap_or_default());
                    let _ = navigate(
                        &target_path,
                        NavigateOptions {
                            replace: true,
                            ..NavigateOptions::default()
                        },
                    );
                    set_selected_invoice.set(Some(saved));
                }
                Err(e) => logging::log!("Error saving invoice: {:?}", e),
            }
        }
    });

    let cancel_invoice_act = create_action(move |(id, reason): &(i64, String)| {
        let (id, reason) = (*id, reason.clone());
        let navigate = navigate_for_cancel.clone();
        async move {
            set_cancel_error.set(None);
            match cancel_invoice(id, Some(invoice_total_cents), Some(reason)).await {
                Ok(updated) => {
                    leptos::set_timeout(
                        move || {
                            on_change.call(());
                            let target_path =
                                format!("/invoices/{}", updated.id.unwrap_or_default());
                            let _ = navigate(&target_path, NavigateOptions::default());
                            set_selected_invoice.try_set(Some(updated));
                        },
                        std::time::Duration::ZERO,
                    );
                }
                Err(e) => set_cancel_error.set(Some(e.to_string())),
            }
        }
    });

    let open_related_invoice_act = create_action(move |id: &i64| {
        let id = *id;
        let navigate = navigate_for_related.clone();
        async move {
            let target_path = format!("/invoices/{}", id);
            let _ = navigate(&target_path, NavigateOptions::default());
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
                }
                Err(e) => logging::log!("Error finalizing invoice: {:?}", e),
            }
        }
    });

    let delete_invoice_act = create_action(move |id: &i64| {
        let id = *id;
        let navigate = navigate_for_delete.clone();
        async move {
            match delete_invoice(id).await {
                Ok(_) => {
                    on_change.call(());
                    let _ = navigate("/invoices", NavigateOptions::default());
                }
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

    let send_mail_act = create_action(move |inv_id: &i64| {
        let inv_id = *inv_id;
        let recipient = mail_recipient.get_untracked();
        let body = mail_body.get_untracked();
        let engagement_id = mail_engagement.get_untracked().parse::<i64>().ok();
        async move {
            match send_invoice_email(inv_id, recipient, body, engagement_id).await {
                Ok(_) => {
                    set_mail_open.set(false);
                    logging::log!("Invoice {inv_id} sent by email");
                }
                Err(error) => logging::log!("Error sending invoice email: {:?}", error),
            }
        }
    });

    view! {
        <div class="box">
            <h2 class="subtitle">
                {if is_credit_note { "Gutschriftdetails" } else if is_cancelation { "Stornodetails" } else { "Rechnungsdetails" }} {display_number}
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
            {original_invoice_id.map(|id| view! {
                <div class="message is-warning mb-3"><div class="message-body p-2">"Dieses Storno gehört zwingend zu:" <button class="button is-small is-light ml-2" on:click=move |_| open_related_invoice_act.dispatch(id)>{original_invoice_number.map(|number| format!("Rechnung Nr. {number}")).unwrap_or_else(|| format!("Originalrechnung (ID {id})"))}</button></div></div>
            })}
            {cancellation_invoice_id.map(|id| view! {
                <div class="message is-info mb-3"><div class="message-body p-2">"Storno zu dieser Rechnung:" <button class="button is-small is-light ml-2" on:click=move |_| open_related_invoice_act.dispatch(id)>{format!("Dokument #{id}")}</button></div></div>
            })}

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
                                        set_mail_recipient.set(c.emails.first().cloned().unwrap_or_default());
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
                    <div class="field"><label class="label is-small">"Name (Pflichtfeld)"</label><div class="control"><input class="input is-small" type="text" prop:value=recipient_name on:input=move |ev| set_recipient_name.set(event_target_value(&ev)) prop:disabled=is_committed /></div></div>
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
                <div class="control"><input class="input" type="date" prop:value=invoice_date on:input=move |ev| set_invoice_date.set(event_target_value(&ev)) prop:disabled=is_committed /></div>
            </div>
            // Nothing is owed on a Gutschrift or a Storno — the money moves the
            // other way — so neither a due date nor a Skonto applies.
            {(!is_credit_note && !is_cancelation).then(|| view! {
                <div class="field-row">
                    <div class="field"><label class="label">"Zahlbar bis"</label><input class="input" type="date" prop:value=due_date on:input=move |ev| set_due_date.set(event_target_value(&ev)) prop:disabled=is_committed /></div>
                    <div class="field"><label class="label">"Skonto bis"</label><input class="input" type="date" prop:value=discount_date on:input=move |ev| set_discount_date.set(event_target_value(&ev)) prop:disabled=is_committed /></div>
                    <div class="field is-narrow"><label class="label">"Skonto (%)"</label><input class="input is-amount" inputmode="decimal" prop:value=discount_percent on:input=move |ev| set_discount_percent.set(event_target_value(&ev)) prop:disabled=is_committed /></div>
                </div>
            })}

            <div class="field">
                <label class="label">"Betreff"</label>
                <div class="control"><input class="input" type="text" prop:value=subject on:input=move |ev| set_subject.set(event_target_value(&ev)) prop:disabled=is_committed /></div>
            </div>

            <div class="field">
                <label class="label">"Einleitungstext"</label>
                <div class="control"><textarea class="textarea" prop:value=header on:input=move |ev| set_header.set(event_target_value(&ev)) prop:disabled=is_committed></textarea></div>
                <TextFieldHint />
            </div>

            {move || if !is_committed && !is_cancelation {
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
                                        {(!is_committed && !is_cancelation).then(|| view! {
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
                                let mut save_inv = inv.clone();
                                save_inv.invoice_date = NaiveDate::parse_from_str(&invoice_date.get(), "%Y-%m-%d").ok();
                                if is_credit_note || is_cancelation {
                                    save_inv.due_date = None;
                                    save_inv.discount_date = None;
                                    save_inv.discount_basis_points = 0;
                                } else {
                                    save_inv.due_date = NaiveDate::parse_from_str(&due_date.get(), "%Y-%m-%d").ok();
                                    save_inv.discount_date = NaiveDate::parse_from_str(&discount_date.get(), "%Y-%m-%d").ok();
                                    save_inv.discount_basis_points = (discount_percent.get().replace(',', ".").parse::<f64>().unwrap_or(0.0) * 100.0).round() as i64;
                                }
                                save_inv.subject = Some(subject.get());
                                save_inv.header = Some(header.get());
                                save_inv.footer = Some(footer.get());
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
                            {if !is_canceled && !is_cancelation && !is_credit_note {
                                view! { <div class="control"><button class="button is-danger" prop:disabled=!payments_list.get_untracked().is_empty() || cancellation_invoice_id.is_some() on:click=move |_| set_cancel_open.set(true)>{"Stornieren"}</button></div> }.into_view()
                            } else {
                                "".into_view()
                            }}
                            {move || if let Some(doc) = document.get() {
                                view! { <div class="control"><a class="button is-link" href=format!("/api/documents/{}", doc.id) target="_blank"><span class="icon mr-1"><i class="mdi mdi-download"></i></span>"PDF herunterladen"</a></div> }.into_view()
                            } else {
                                view! { <div class="control"><button class="button is-info" on:click=move |_| export_act.dispatch(id) prop:disabled=export_act.pending()>{"Exportieren (PDF generieren)"}</button></div> }.into_view()
                            }}
                            <div class="control"><button class="button is-link" on:click=move |_| set_mail_open.update(|value| *value = !*value)>{"Per E-Mail senden"}</button></div>
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

            {move || (cancel_open.get() && invoice_id.is_some()).then(|| view! {
                <div class="box subbox mt-4 cancellation-panel">
                    <h3 class="is-size-6 has-text-weight-bold mb-2">"Storno erstellen"</h3>
                    <p class="text-muted is-size-7 mb-3">{format!("Vollstorno über {}. Alle Positionen werden unveränderbar aus der Originalrechnung übernommen.", format_euro(invoice_total_cents))}</p>
                    {move || cancel_error.get().map(|message| view! { <div class="message is-danger"><div class="message-body p-2">{message}</div></div> })}
                    <div class="field-row">
                        <div class="field is-wide"><label class="label">"Grund"</label><input class="input" prop:value=cancel_reason on:input=move |event| set_cancel_reason.set(event_target_value(&event)) placeholder="z. B. Leistungsumfang reduziert" /></div>
                    </div>
                    <div class="buttons">
                        <button class="button is-danger" prop:disabled=cancel_invoice_act.pending() on:click=move |_| cancel_invoice_act.dispatch((invoice_id.unwrap_or_default(), cancel_reason.get_untracked()))>"Vollständigen Stornoentwurf erstellen"</button>
                        <button class="button is-light" on:click=move |_| set_cancel_open.set(false)>"Abbrechen"</button>
                    </div>
                </div>
            })}

            // Payments belong to an *issued* invoice. A draft has not been sent,
            // so there is nothing for a customer to have paid yet.
            {if is_committed && invoice_id.is_some() && !is_cancelation && cancellation_invoice_id.is_none() {
                view! {
                    <PaymentsPanel
                        payments=payments_list
                        total_cents=Signal::derive(move || (items_list.get().iter().map(Item::total_cents).sum::<i64>() - credited_cents - discount_taken_cents).max(0))
                        on_add=Callback::new(move |(amount, date)| add_payment_act.dispatch((amount, date)))
                        on_delete=Callback::new(move |id| delete_payment_act.dispatch(id))
                    />
                }.into_view()
            } else { "".into_view() }}

            {if is_committed && !is_cancelation {
                view! {
                    <div class="box subbox mt-4">
                        <h3 class="is-size-6 has-text-weight-bold mb-2">"Mahnungen"</h3>
                        {move || if reminders.get().is_empty() { view! { <p class="text-muted is-size-7 mb-3">"Noch keine Mahnung erstellt."</p> }.into_view() } else { view! { <div class="mb-3">{reminders.get().into_iter().map(|reminder| view! { <div class="crm-record p-2 mb-1 is-size-7"><strong>{format!("{}. Mahnung · {}", reminder.level, reminder.reminder_date.format("%d.%m.%Y"))}</strong><span class="text-muted ml-2">{format!("Gebühr {}", format_euro(reminder.fee_cents))}</span><div>{reminder.note}</div></div> }).collect_view()}</div> }.into_view() }}
                        {(invoice_is_due && !is_canceled).then(|| view! {
                            <div class="field-row">
                                <div class="field is-narrow"><label class="label">"Mahngebühr"</label><MoneyInput value=reminder_fee /></div>
                                <div class="field is-wide"><label class="label">"Hinweis"</label><input class="input" prop:value=reminder_note on:input=move |event| set_reminder_note.set(event_target_value(&event)) placeholder="Zahlungserinnerung / Frist" /></div>
                                <button class="button is-warning" prop:disabled=create_reminder_act.pending() on:click=move |_| create_reminder_act.dispatch((invoice_id.unwrap_or_default(), reminder_fee.get_untracked(), reminder_note.get_untracked()))>"Mahnung erstellen"</button>
                            </div>
                        })}
                    </div>
                }.into_view()
            } else { "".into_view() }}

            {move || if is_committed && mail_open.get() {
                view! {
                    <div class="box subbox mt-4">
                        <h3 class="is-size-6 has-text-weight-bold mb-3">"Rechnung per E-Mail senden"</h3>
                        <div class="field"><label class="label">"Empfänger"</label><input class="input" placeholder="kunde@example.org" prop:value=mail_recipient on:input=move |event| set_mail_recipient.set(event_target_value(&event)) /></div>
                        <div class="field"><label class="label">"Nachricht"</label><textarea class="textarea" prop:value=mail_body on:input=move |event| set_mail_body.set(event_target_value(&event)) placeholder="Anbei erhalten Sie unsere Rechnung als PDF."></textarea></div>
                        <div class="field"><label class="label">"Auftrag (optional)"</label><div class="select is-fullwidth"><select prop:value=mail_engagement on:change=move |event| set_mail_engagement.set(event_target_value(&event))><option value="">"-- Nicht verknüpfen --"</option><Suspense fallback=move || view! { <option>"Lade Aufträge…"</option> }>{move || engagements.get().and_then(Result::ok).map(|page| page.items.into_iter().map(|item| view! { <option value=item.id.to_string()>{item.title}</option> }).collect_view())}</Suspense></select></div></div>
                        <div class="field is-grouped"><button class="button is-link" prop:disabled=send_mail_act.pending() on:click=move |_| send_mail_act.dispatch(invoice_id.unwrap_or_default())>"Senden (PDF anhängen)"</button><button class="button" on:click=move |_| set_mail_open.set(false)>"Abbrechen"</button></div>
                    </div>
                }.into_view()
            } else { "".into_view() }}
        </div>
    }
}

#[component]
pub fn InvoicesPage() -> impl IntoView {
    let (invoices, set_invoices) = create_signal(Vec::<InvoiceListItem>::new());
    let (selected_invoice, set_selected_invoice) = create_signal(Option::<Invoice>::None);
    let (contacts, set_contacts) = create_signal(Vec::<Contact>::new());
    let (from_date_filter, set_from_date_filter) = create_signal(String::new());
    let (to_date_filter, set_to_date_filter) = create_signal(String::new());
    let (has_more_invoices, set_has_more_invoices) = create_signal(false);
    let (list_generation, set_list_generation) = create_signal(0_u64);
    let (customer_id_filter, set_customer_id_filter) = create_signal(Option::<i64>::None);
    let (is_dirty, set_is_dirty) = create_signal(false);

    let params = use_params_map();
    let id_param = move || params.with(|p| p.get("id").cloned());

    create_effect(move |_| {
        let id_val = id_param();
        match id_val.as_deref() {
            None => {
                set_selected_invoice.set(None);
            }
            Some("new") => {
                let is_credit_note = web_sys::window()
                    .and_then(|window| window.location().search().ok())
                    .and_then(|search| web_sys::UrlSearchParams::new_with_str(&search).ok())
                    .and_then(|params| params.get("type"))
                    .as_deref()
                    == Some("credit_note");
                set_selected_invoice.set(Some(Invoice {
                    id: None,
                    items: vec![],
                    created_timestamp: None,
                    committed_timestamp: None,
                    invoice_number: None,
                    payments: vec![],
                    invoice_date: Some(Utc::now().naive_utc().date()),
                    due_date: (!is_credit_note)
                        .then(|| Utc::now().naive_utc().date() + chrono::Duration::days(14)),
                    discount_date: None,
                    discount_basis_points: 0,
                    discount_taken_cents: 0,
                    reminders: vec![],
                    is_canceled: false,
                    is_cancelation: false,
                    is_credit_note,
                    corrected_invoice_id: None,
                    corrected_invoice_number: None,
                    cancellation_invoice_id: None,
                    credited_cents: 0,
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
                    header: Some(if is_credit_note {
                        "Wir schreiben Ihnen die nachfolgend aufgeführten Beträge gut.".to_string()
                    } else {
                        "Vielen Dank für Ihre Bestellung.".to_string()
                    }),
                    footer: Some(if is_credit_note {
                        "Der Gutschriftbetrag wird Ihnen zeitnah erstattet.".to_string()
                    } else {
                        "Bitte überweisen Sie den Betrag innerhalb von 14 Tagen.".to_string()
                    }),
                    title: Some(
                        if is_credit_note {
                            "Gutschrift"
                        } else {
                            "Rechnung"
                        }
                        .to_string(),
                    ),
                    subject: Some(
                        if is_credit_note {
                            "Gutschrift"
                        } else {
                            "Rechnung"
                        }
                        .to_string(),
                    ),
                }));
                spawn_local(async move {
                    let kind = if is_credit_note {
                        "credit_note"
                    } else {
                        "invoice"
                    };
                    if let Ok(defaults) = get_invoice_text_defaults(kind.to_string()).await {
                        if let Some(mut draft) = selected_invoice.get_untracked() {
                            if draft.id.is_none() && draft.is_credit_note == is_credit_note {
                                draft.title = Some(defaults.title);
                                draft.subject = Some(defaults.subject);
                                draft.header = Some(defaults.header);
                                draft.footer = Some(defaults.footer);
                                set_selected_invoice.try_set(Some(draft));
                            }
                        }
                    }
                });
            }
            Some(id_str) => {
                if let Ok(id) = id_str.parse::<i64>() {
                    let already_selected =
                        selected_invoice.get_untracked().and_then(|inv| inv.id) == Some(id);
                    if !already_selected {
                        spawn_local(async move {
                            if let Ok(invoice) = get_invoice(id).await {
                                set_selected_invoice.try_set(Some(invoice));
                            }
                        });
                    }
                }
            }
        }
    });

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

    let load_invoices = create_action(
        move |(generation, offset, from, to): &(u64, u32, String, String)| {
            let generation = *generation;
            let offset = *offset;
            let from = from.clone();
            let to = to.clone();
            async move {
                let from_date = NaiveDate::parse_from_str(&from, "%Y-%m-%d").ok();
                let to_date = NaiveDate::parse_from_str(&to, "%Y-%m-%d").ok();
                match get_invoices(offset, INVOICE_PAGE_SIZE, from_date, to_date, None).await {
                    Ok(page) => {
                        if list_generation.get_untracked() != generation
                            || from_date_filter.get_untracked() != from
                            || to_date_filter.get_untracked() != to
                        {
                            return;
                        }

                        if offset == 0 {
                            set_invoices.set(page.items);
                        } else if invoices.get_untracked().len() as u32 == offset {
                            set_invoices.update(|items| items.extend(page.items));
                        } else {
                            return;
                        }
                        set_has_more_invoices.set(page.has_more);
                    }
                    Err(e) => logging::log!("Error fetching invoices: {:?}", e),
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

    // Mail attachments can deep-link directly into the invoice editor.
    create_effect(move |_| {
        if let Some(window) = web_sys::window() {
            if let Ok(search) = window.location().search() {
                if let Some(id) = web_sys::UrlSearchParams::new_with_str(&search)
                    .ok()
                    .and_then(|params| params.get("invoice_id"))
                    .and_then(|value| value.parse::<i64>().ok())
                {
                    let _ =
                        use_navigate()(&format!("/invoices/{}", id), NavigateOptions::default());
                }
            }
        }
    });

    load_invoices.dispatch((0, 0, String::new(), String::new()));
    load_contacts.dispatch(());

    // The editor takes the whole page: while writing an invoice, the list of the
    // other ones is noise. Selecting swaps the pane, "Zurück" swaps it back.
    let list_view = move || {
        view! {
            <div class="level">
                <div class="level-left"><h1 class="title">"Rechnungen"</h1></div>
                    <div class="level-right">
                        <button class="button is-link" on:click=move |_| {
                            let _ = use_navigate()("/invoices/new", NavigateOptions::default());
                        }>{"Neue Rechnung"}</button>
                        <button class="button is-light" on:click=move |_| {
                            let _ = use_navigate()("/invoices/new?type=credit_note", NavigateOptions::default());
                        }>{"Neue Gutschrift"}</button>
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
                                    set_invoices.set(Vec::new());
                                    set_has_more_invoices.set(false);
                                    load_invoices.dispatch((
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
                                    set_invoices.set(Vec::new());
                                    set_has_more_invoices.set(false);
                                    load_invoices.dispatch((
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
                {move || customer_id_filter.get().map(|cid| {
                    let contact_name = contacts.get().iter().find(|c| c.id == Some(cid)).map(|c| c.display_name()).unwrap_or_else(|| format!("Kunde #{}", cid));
                    view! {
                        <div class="notification is-info is-light py-2 px-3 mb-3 is-flex is-justify-content-space-between is-align-items-center">
                            <span>"Filter: Nur Rechnungen von " <strong>{contact_name}</strong></span>
                            <button class="button is-small is-light" on:click=move |_| {
                                set_customer_id_filter.set(None);
                                if let Some(window) = web_sys::window() {
                                    if let Ok(history) = window.history() {
                                        let _ = history.push_state_with_url(
                                            &wasm_bindgen::JsValue::null(),
                                            "",
                                            Some("/invoices")
                                        );
                                    }
                                }
                            }>"Filter aufheben"</button>
                        </div>
                    }.into_view()
                })}
                <div>
                    {move || {
                        let filtered: Vec<_> = invoices.get().into_iter().filter(|inv| {
                            match customer_id_filter.get() {
                                None => true,
                                Some(cid) => inv.customer_contact.as_ref().and_then(|c| c.id) == Some(cid),
                            }
                        }).collect();
                        if filtered.is_empty() {
                            view! {
                                <EmptyState icon="file-document-outline" text="Keine passende Rechnung gefunden." />
                            }.into_view()
                        } else {
                            filtered.into_iter().map(|inv| {
                                let inv_id = inv.id;
                                let contact_name = inv.customer_contact.as_ref().map(Contact::display_name).unwrap_or_else(|| "Gast".to_string());
                                let status_badge = if !inv.committed {
                                    view! { <span class="tag is-warning ml-2">"ENTWURF"</span> }.into_view()
                                } else if inv.is_cancelation {
                                    view! { <span class="tag is-info ml-2">"Storno"</span> }.into_view()
                                } else if inv.is_credit_note {
                                    view! { <span class="tag is-info ml-2">"Gutschrift"</span> }.into_view()
                                } else if inv.is_canceled {
                                    view! { <span class="tag is-danger ml-2">"Storniert"</span> }.into_view()
                                } else if inv.due_date.map(|date| date < Utc::now().date_naive()).unwrap_or(false) && inv.outstanding_cents() > 0 {
                                    view! { <span class="tag is-danger ml-2">"Fällig"</span> }.into_view()
                                } else {
                                    let status = inv.payment_status();
                                    view! { <span class=format!("tag {} ml-2", status.tag_class())>{status.label()}</span> }.into_view()
                                };
                                let amount_line = if inv.committed && inv.is_cancelation {
                                    view! { <div class="is-size-7 text-muted">{format!("{} · mit Originalrechnung verrechnet", format_euro(-inv.total_cents))}</div> }.into_view()
                                } else if inv.committed && !inv.is_canceled {
                                    let status = inv.payment_status();
                                    let text = match status {
                                        PaymentStatus::Partial => format!(
                                            "{} von {} • offen {}",
                                            format_euro(inv.paid_cents),
                                            format_euro(inv.total_cents),
                                            format_euro(inv.outstanding_cents()),
                                        ),
                                        PaymentStatus::Overpaid => format!(
                                            "{} von {} • überzahlt {}",
                                            format_euro(inv.paid_cents),
                                            format_euro(inv.total_cents),
                                            format_euro(-inv.outstanding_cents()),
                                        ),
                                        _ => format_euro(inv.total_cents),
                                    };
                                    view! { <div class="is-size-7 text-muted">{text}</div> }.into_view()
                                } else { "".into_view() };
                                let display_title = inv.subject.clone().unwrap_or_else(|| "Rechnung".to_string());
                                let secondary = if inv.committed {
                                    format!("Rechnung #{}", inv.invoice_number.unwrap_or_default())
                                } else {
                                    inv.created_timestamp.format("%d.%m.%Y").to_string()
                                };
                                view! {
                                    <div
                                        class="box list-item p-3 mb-2"
                                        on:click=move |_| {
                                            let target = format!("/invoices/{}", inv_id);
                                            let _ = use_navigate()(&target, NavigateOptions::default());
                                        }
                                    >
                                        <div class="has-text-weight-bold">{display_title} {status_badge}</div>
                                        <div class="is-size-7 text-muted">{secondary} " • " {contact_name}</div>
                                        {amount_line}
                                    </div>
                                }
                            }).collect::<Vec<_>>().into_view()
                        }
                    }}
                </div>
                <Show when=move || has_more_invoices.get() && customer_id_filter.get().is_none()>
                    <div class="has-text-centered mt-3">
                        <button
                            class="button is-light"
                            prop:disabled=load_invoices.pending()
                            on:click=move |_| {
                                let offset = invoices.get_untracked().len() as u32;
                                load_invoices.dispatch((
                                    list_generation.get_untracked(),
                                    offset,
                                    from_date_filter.get_untracked(),
                                    to_date_filter.get_untracked(),
                                ));
                            }
                        >
                            {move || if load_invoices.pending().get() { "Lädt…" } else { "Mehr laden" }}
                        </button>
                    </div>
                </Show>
            </div>
        }
    };

    view! {
        <div class="container">
            {move || match selected_invoice.get() {
                None => list_view().into_view(),
                Some(inv) => view! {
                    <div class="level">
                        <div class="level-left">
                            <button class="button is-light" on:click=move |_| {
                                let confirm_ok = if is_dirty.get() {
                                    web_sys::window()
                                        .and_then(|w| w.confirm_with_message("Sie haben ungespeicherte Änderungen. Möchten Sie die Seite wirklich verlassen?").ok())
                                        .unwrap_or(false)
                                } else {
                                    true
                                };
                                if confirm_ok {
                                    let _ = use_navigate()("/invoices", NavigateOptions::default());
                                }
                            }>
                                <span class="icon mr-1"><i class="mdi mdi-arrow-left"></i></span>
                                "Zurück zur Übersicht"
                            </button>
                        </div>
                    </div>
                    <InvoiceEditor
                        inv=inv
                        contacts=contacts
                        on_change=Callback::new(move |_| {
                            let generation = list_generation.get_untracked().wrapping_add(1);
                            set_list_generation.set(generation);
                            set_invoices.set(Vec::new());
                            set_has_more_invoices.set(false);
                            load_invoices.dispatch((
                                generation,
                                0,
                                from_date_filter.get_untracked(),
                                to_date_filter.get_untracked(),
                            ));
                        })
                        set_selected_invoice=set_selected_invoice
                        set_dirty=set_is_dirty
                    />
                }.into_view(),
            }}
        </div>
    }
}
