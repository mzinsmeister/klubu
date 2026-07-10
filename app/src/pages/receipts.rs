use leptos::*;
use leptos_router::{use_navigate, use_params_map, NavigateOptions};

use chrono::{NaiveDate, Utc};
use shared::*;
use wasm_bindgen::JsCast;

use crate::components::{MoneyInput, PaymentsPanel};
use crate::server::{
    add_receipt_payment, commit_receipt, delete_receipt, delete_receipt_payment, get_ai_status,
    get_all_contacts, get_categories, get_receipt, get_receipts, parse_einvoice, prefill_receipt,
    save_receipt,
};

const RECEIPT_PAGE_SIZE: u32 = 50;

/// Reads the picked file as base64 and hands it to `on_load`.
fn read_file_as_base64(file: web_sys::File, on_load: impl Fn(ReceiptDocumentData) + 'static) {
    let name = file.name();
    let media_type = match file.type_().as_str() {
        // Some browsers report an empty type; fall back to the extension.
        "" if name.to_lowercase().ends_with(".pdf") => "application/pdf".to_string(),
        "" if name.to_lowercase().ends_with(".png") => "image/png".to_string(),
        "" if name.to_lowercase().ends_with(".jpg") || name.to_lowercase().ends_with(".jpeg") => {
            "image/jpeg".to_string()
        }
        "" if name.to_lowercase().ends_with(".webp") => "image/webp".to_string(),
        "" => "application/octet-stream".to_string(),
        t => t.to_string(),
    };
    let extension = name
        .rsplit_once('.')
        .map(|(_, ext)| ext.to_lowercase())
        .unwrap_or_else(|| "bin".to_string());

    let reader = match web_sys::FileReader::new() {
        Ok(r) => r,
        Err(_) => return,
    };
    let reader_clone = reader.clone();
    let onload = wasm_bindgen::closure::Closure::wrap(Box::new(move |_e: web_sys::Event| {
        if let Ok(res) = reader_clone.result() {
            if let Some(data_url) = res.as_string() {
                // "data:<mime>;base64,<payload>"
                if let Some(comma) = data_url.find(',') {
                    on_load(ReceiptDocumentData {
                        data: data_url[comma + 1..].to_string(),
                        extension: extension.clone(),
                        media_type: media_type.clone(),
                    });
                }
            }
        }
    }) as Box<dyn FnMut(web_sys::Event)>);
    reader.set_onload(Some(onload.as_ref().unchecked_ref()));
    let _ = reader.read_as_data_url(&file);
    onload.forget();
}

#[component]
pub fn ReceiptsPage() -> impl IntoView {
    let (receipts, set_receipts) = create_signal(Vec::<ReceiptListItem>::new());
    let (selected_receipt, set_selected_receipt) = create_signal(Option::<Receipt>::None);
    let (categories, set_categories) = create_signal(Vec::<ReceiptItemCategory>::new());
    let (contacts, set_contacts) = create_signal(Vec::<Contact>::new());
    let (ai_status, set_ai_status) = create_signal(AiStatus::default());
    let (from_date_filter, set_from_date_filter) = create_signal(String::new());
    let (to_date_filter, set_to_date_filter) = create_signal(String::new());
    let (has_more_receipts, set_has_more_receipts) = create_signal(false);
    let (list_generation, set_list_generation) = create_signal(0_u64);
    let (is_dirty, set_is_dirty) = create_signal(false);

    let params = use_params_map();
    let id_param = move || params.with(|p| p.get("id").cloned());

    create_effect(move |_| {
        let id_val = id_param();
        match id_val.as_deref() {
            None => {
                set_selected_receipt.set(None);
            }
            Some("new") => {
                set_selected_receipt.set(Some(Receipt {
                    id: None,
                    items: vec![],
                    created_timestamp: None,
                    committed_timestamp: None,
                    receipt_number: String::new(),
                    payments: vec![],
                    receipt_date: Some(Utc::now().naive_utc().date()),
                    due_date: None,
                    supplier_contact: None,
                    document: None,
                    document_data: None,
                }));
            }
            Some(id_str) => {
                if let Ok(id) = id_str.parse::<i64>() {
                    let already_selected =
                        selected_receipt.get_untracked().and_then(|r| r.id) == Some(id);
                    if !already_selected {
                        spawn_local(async move {
                            if let Ok(receipt) = get_receipt(id).await {
                                set_selected_receipt.set(Some(receipt));
                            }
                        });
                    }
                }
            }
        }
    });

    let load_receipts = create_action(
        move |(generation, offset, from, to): &(u64, u32, String, String)| {
            let generation = *generation;
            let offset = *offset;
            let from = from.clone();
            let to = to.clone();
            async move {
                let from_date = NaiveDate::parse_from_str(&from, "%Y-%m-%d").ok();
                let to_date = NaiveDate::parse_from_str(&to, "%Y-%m-%d").ok();
                match get_receipts(offset, RECEIPT_PAGE_SIZE, from_date, to_date).await {
                    Ok(page) => {
                        if list_generation.get_untracked() != generation
                            || from_date_filter.get_untracked() != from
                            || to_date_filter.get_untracked() != to
                        {
                            return;
                        }

                        if offset == 0 {
                            set_receipts.set(page.items);
                        } else if receipts.get_untracked().len() as u32 == offset {
                            set_receipts.update(|items| items.extend(page.items));
                        } else {
                            return;
                        }
                        set_has_more_receipts.set(page.has_more);
                    }
                    Err(e) => logging::log!("Error fetching receipts: {:?}", e),
                }
            }
        },
    );

    create_resource(
        || (),
        move |_| async move {
            if let Ok(list) = get_categories().await {
                set_categories.set(list);
            }
            if let Ok(list) = get_all_contacts().await {
                set_contacts.set(list);
            }
            // When the server has the feature switched off we never show the
            // prefill button, and no model needs to exist on the machine.
            if let Ok(status) = get_ai_status().await {
                set_ai_status.set(status);
            }
        },
    );

    create_effect(move |_| {
        if let Some(window) = web_sys::window() {
            if let Ok(search) = window.location().search() {
                if let Some(id) = web_sys::UrlSearchParams::new_with_str(&search)
                    .ok()
                    .and_then(|params| params.get("receipt_id"))
                    .and_then(|value| value.parse::<i64>().ok())
                {
                    let _ =
                        use_navigate()(&format!("/receipts/{}", id), NavigateOptions::default());
                }
            }
        }
    });

    load_receipts.dispatch((0, 0, String::new(), String::new()));

    let new_receipt = move |_| {
        let _ = use_navigate()("/receipts/new", NavigateOptions::default());
    };

    // Full-screen editor: while booking a receipt the other ones are just noise.
    let list_view = move || {
        view! {
            <div class="level">
                <div class="level-left">
                        <h1 class="title">"Belege"</h1>
                    </div>
                    <div class="level-right">
                        <button class="button is-link" on:click=new_receipt>
                            <span class="icon mr-1"><i class="mdi mdi-plus"></i></span>
                            "Neuer Beleg"
                        </button>
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
                                                set_receipts.set(Vec::new());
                                                set_has_more_receipts.set(false);
                                                load_receipts.dispatch((
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
                                                set_receipts.set(Vec::new());
                                                set_has_more_receipts.set(false);
                                                load_receipts.dispatch((
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
                            <Show
                                when=move || !receipts.get().is_empty() || load_receipts.pending().get()
                                fallback=|| view! {
                                    <p class="text-muted has-text-centered p-4">"Noch keine Belege erfasst."</p>
                                }
                            >
                                <div class="scroll-list">
                                    <For
                                        each=move || receipts.get()
                                        key=|rec| (rec.id, rec.total_cents, rec.receipt_number.clone())
                                        let:rec
                                    >
                                        {
                                            let id = rec.id;
                                            let supplier = rec.supplier_contact.as_ref()
                                                .map(Contact::display_name)
                                                .unwrap_or_else(|| "Kein Lieferant".to_string());
                                            let number = rec.receipt_number.clone()
                                                .filter(|n| !n.trim().is_empty())
                                                .unwrap_or_else(|| format!("#{}", rec.id));
                                            let date = rec.receipt_date
                                                .map(|d| d.format("%d.%m.%Y").to_string())
                                                .unwrap_or_else(|| "ohne Datum".to_string());
                                            let total = format_euro(rec.total_cents);
                                            let has_doc = rec.has_document;
                                            let pay_status = rec.payment_status();
                                            view! {
                                                <div
                                                    class="box list-item p-3 mb-2"
                                                    on:click=move |_| {
                                                        let _ = use_navigate()(&format!("/receipts/{}", id), NavigateOptions::default());
                                                    }
                                                >
                                                    <div class="is-flex is-justify-content-space-between">
                                                        <span class="has-text-weight-bold">
                                                            {number}
                                                            {has_doc.then(|| view! {
                                                                <span class="icon is-small ml-1 text-muted" title="Dokument hinterlegt">
                                                                    <i class="mdi mdi-paperclip"></i>
                                                                </span>
                                                            })}
                                                            {rec.committed.then(|| view! {
                                                                <span class="tag is-warning is-small ml-1 py-0 px-1" style="height: 1.25rem; font-size: 0.65rem;" title="Festgeschrieben">
                                                                    <span class="icon is-small"><i class="mdi mdi-lock"></i></span>
                                                                </span>
                                                            })}
                                                        </span>
                                                        <span class="has-text-weight-semibold is-numeric">{total}</span>
                                                    </div>
                                                    <div class="is-size-7 text-muted">
                                                        {supplier} " • " {date}
                                                        <span class=format!("tag {} is-small ml-2", pay_status.tag_class())>{pay_status.label()}</span>
                                                    </div>
                                                </div>
                                            }
                                        }
                                    </For>
                                </div>
                            </Show>
                            <Show when=move || has_more_receipts.get()>
                                <div class="has-text-centered mt-3">
                                    <button
                                        class="button is-light"
                                        prop:disabled=load_receipts.pending()
                                        on:click=move |_| {
                                        let offset = receipts.get_untracked().len() as u32;
                                        load_receipts.dispatch((
                                            list_generation.get_untracked(),
                                            offset,
                                            from_date_filter.get_untracked(),
                                                to_date_filter.get_untracked(),
                                            ));
                                        }
                                    >
                                        {move || if load_receipts.pending().get() { "Lädt…" } else { "Mehr laden" }}
                                    </button>
                                </div>
                            </Show>
            </div>
        }
    };

    view! {
        <div class="container">
            {move || match selected_receipt.get() {
                None => list_view().into_view(),
                Some(rec) => view! {
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
                                    let _ = use_navigate()("/receipts", NavigateOptions::default());
                                }
                            }>
                                <span class="icon mr-1"><i class="mdi mdi-arrow-left"></i></span>
                                "Zurück zur Übersicht"
                            </button>
                        </div>
                    </div>
                    <ReceiptEditor
                        // Remount the editor when a different receipt is picked so
                        // its internal signals restart from the new data.
                        rec=rec
                        contacts=contacts
                        categories=categories
                        ai_status=ai_status
                        on_change=Callback::new(move |_| {
                            let generation = list_generation.get_untracked().wrapping_add(1);
                            set_list_generation.set(generation);
                            set_receipts.set(Vec::new());
                            set_has_more_receipts.set(false);
                            load_receipts.dispatch((
                                generation,
                                0,
                                from_date_filter.get_untracked(),
                                to_date_filter.get_untracked(),
                            ));
                        })
                        set_selected_receipt=set_selected_receipt
                        set_dirty=set_is_dirty
                    />
                }.into_view(),
            }}
        </div>
    }
}

#[component]
fn ReceiptEditor(
    rec: Receipt,
    contacts: ReadSignal<Vec<Contact>>,
    categories: ReadSignal<Vec<ReceiptItemCategory>>,
    ai_status: ReadSignal<AiStatus>,
    on_change: Callback<()>,
    set_selected_receipt: WriteSignal<Option<Receipt>>,
    set_dirty: WriteSignal<bool>,
) -> impl IntoView {
    let receipt_id = rec.id;
    let is_committed = rec.committed_timestamp.is_some();

    let (receipt_num, set_receipt_num) = create_signal(rec.receipt_number.clone());
    let (receipt_date, set_receipt_date) = create_signal(
        rec.receipt_date
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_default(),
    );
    let (supplier_contact, set_supplier_contact) = create_signal(rec.supplier_contact.clone());
    let (document_data, set_document_data) = create_signal(rec.document_data.clone());
    let (doc_metadata, set_doc_metadata) = create_signal(rec.document.clone());
    let (items_list, set_items_list) = create_signal(rec.items.clone());
    let (error, set_error) = create_signal(Option::<String>::None);
    let (warnings, set_warnings) = create_signal(Vec::<String>::new());

    let (payments_list, set_payments_list) = create_signal(rec.payments.clone());

    let refresh_payments = move || {
        if let Some(id) = receipt_id {
            spawn_local(async move {
                if let Ok(full) = get_receipt(id).await {
                    set_payments_list.set(full.payments);
                }
            });
        }
    };

    let add_payment_act = create_action(move |(amount, date): &(i64, NaiveDate)| {
        let (amount, date) = (*amount, *date);
        async move {
            let Some(id) = receipt_id else { return };
            match add_receipt_payment(id, amount, date).await {
                Ok(()) => {
                    refresh_payments();
                    on_change.call(());
                }
                Err(e) => set_error.set(Some(format!("Zahlung konnte nicht erfasst werden: {e}"))),
            }
        }
    });

    let delete_payment_act = create_action(move |payment_id: &i64| {
        let payment_id = *payment_id;
        async move {
            match delete_receipt_payment(payment_id).await {
                Ok(()) => {
                    refresh_payments();
                    on_change.call(());
                }
                Err(e) => set_error.set(Some(format!("Zahlung konnte nicht gelöscht werden: {e}"))),
            }
        }
    });
    let (file_name, set_file_name) = create_signal(Option::<String>::None);

    // New-item form.
    let (item_desc, set_item_desc) = create_signal(String::new());
    let item_price = create_rw_signal(0i64);
    let (item_cat_id, set_item_cat_id) = create_signal(Option::<i64>::None);

    let total_cents = move || {
        items_list
            .get()
            .iter()
            .map(|i| i.price.amount_cents)
            .sum::<i64>()
    };

    let has_unsaved_changes = {
        let rec = rec.clone();
        move || {
            let orig_date = rec
                .receipt_date
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_default();
            receipt_num.get() != rec.receipt_number
                || receipt_date.get() != orig_date
                || supplier_contact.get().as_ref().and_then(|c| c.id)
                    != rec.supplier_contact.as_ref().and_then(|c| c.id)
                || items_list.get() != rec.items
                || document_data.get() != rec.document_data
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

    let save_act = create_action(move |r: &Receipt| {
        let r = r.clone();
        async move {
            match save_receipt(r).await {
                Ok(saved) => {
                    on_change.call(());
                    set_error.set(None);
                    // Reflect what the server actually stored (ids, document version).
                    let target_path = format!("/receipts/{}", saved.id.unwrap_or_default());
                    let _ = use_navigate()(
                        &target_path,
                        NavigateOptions {
                            replace: true,
                            ..NavigateOptions::default()
                        },
                    );
                    set_selected_receipt.set(Some(saved));
                }
                Err(e) => set_error.set(Some(format!("Speichern fehlgeschlagen: {e}"))),
            }
        }
    });

    let delete_act = create_action(move |id: &i64| {
        let id = *id;
        async move {
            match delete_receipt(id).await {
                Ok(()) => {
                    on_change.call(());
                    let _ = use_navigate()("/receipts", NavigateOptions::default());
                }
                Err(e) => set_error.set(Some(format!("Löschen fehlgeschlagen: {e}"))),
            }
        }
    });

    let commit_act = create_action(move |id: &i64| {
        let id = *id;
        async move {
            match commit_receipt(id).await {
                Ok(()) => {
                    on_change.call(());
                    set_error.set(None);
                    if let Ok(saved) = get_receipt(id).await {
                        set_selected_receipt.set(Some(saved));
                    }
                }
                Err(e) => set_error.set(Some(format!("Festschreiben fehlgeschlagen: {e}"))),
            }
        }
    });

    // Fills the form from a prefill, whatever produced it.
    let apply_prefill = move |p: ReceiptPrefill| {
        if let Some(n) = p.receipt_number {
            set_receipt_num.set(n);
        }
        if let Some(d) = p.receipt_date {
            set_receipt_date.set(d.format("%Y-%m-%d").to_string());
        }
        if let Some(c) = p.supplier_contact {
            set_supplier_contact.set(Some(c));
        }
        if !p.items.is_empty() {
            set_items_list.set(p.items);
        }
        set_warnings.set(p.warnings);
    };

    // Runs the uploaded file through the fast local parser (or the explicitly
    // selected LLM fallback) and fills the form.
    let prefill_act = create_action(move |doc: &ReceiptDocumentData| {
        let doc = doc.clone();
        async move {
            set_error.set(None);
            set_warnings.set(vec![]);
            match prefill_receipt(doc).await {
                Ok(p) => apply_prefill(p),
                Err(e) => set_error.set(Some(e.to_string())),
            }
        }
    });

    // An uploaded document is checked for embedded invoice data before anything
    // else. An e-invoice *states* its fields, so there is nothing to guess and
    // no reason to make the user press a button — or to have the AI enabled.
    let einvoice_act = create_action(move |doc: &ReceiptDocumentData| {
        let doc = doc.clone();
        async move {
            match parse_einvoice(doc).await {
                // Not an e-invoice: stay quiet, the AI button is still there.
                Ok(None) => {}
                Ok(Some(p)) => {
                    set_error.set(None);
                    apply_prefill(p);
                }
                Err(e) => set_error.set(Some(e.to_string())),
            }
        }
    });

    let add_item = move |_| {
        let desc = item_desc.get();
        if desc.trim().is_empty() {
            set_error.set(Some("Bitte eine Beschreibung eingeben.".to_string()));
            return;
        }
        set_error.set(None);
        let category = categories
            .get()
            .into_iter()
            .find(|c| Some(c.id) == item_cat_id.get());
        set_items_list.update(|items| {
            items.push(ReceiptItem {
                item: desc.trim().to_string(),
                price: Money::new(item_price.get()),
                category,
            })
        });
        set_item_desc.set(String::new());
        item_price.set(0);
    };

    let save = move |_| {
        let mut r = rec.clone();
        r.receipt_number = receipt_num.get();
        r.receipt_date = NaiveDate::parse_from_str(&receipt_date.get(), "%Y-%m-%d").ok();
        r.items = items_list.get();
        r.supplier_contact = supplier_contact.get();
        r.document_data = document_data.get();
        r.document = doc_metadata.get();
        save_act.dispatch(r);
    };

    view! {
        <div class="box">
            <h2 class="subtitle">
                {move || if receipt_id.is_some() { "Belegdetails" } else { "Neuer Beleg" }}
            </h2>

            {move || error.get().map(|e| view! {
                <div class="message is-danger">
                    <div class="message-body p-3 is-size-7">{e}</div>
                </div>
            })}

            {move || (!warnings.get().is_empty()).then(|| view! {
                <div class="message is-warning">
                    <div class="message-body p-3 is-size-7">
                        <ul>
                            {warnings.get().into_iter()
                                .map(|w| view! { <li>"• " {w}</li> })
                                .collect::<Vec<_>>()}
                        </ul>
                    </div>
                </div>
            })}

            <div class="field-row">
                <div class="field">
                    <label class="label">"Belegnummer"</label>
                    <div class="control">
                        <input class="input" type="text" placeholder="z. B. RG-2026-0042"
                            prop:value=receipt_num
                            prop:disabled=is_committed
                            on:input=move |ev| set_receipt_num.set(event_target_value(&ev)) />
                    </div>
                </div>
                <div class="field">
                    <label class="label">"Belegdatum"</label>
                    <div class="control">
                        <input class="input" type="date"
                            prop:value=receipt_date
                            prop:disabled=is_committed
                            on:input=move |ev| set_receipt_date.set(event_target_value(&ev)) />
                    </div>
                </div>
            </div>

            <div class="field">
                <label class="label">"Lieferant (Kontakt)"</label>
                <div class="control">
                    <div class="select is-fullwidth">
                        <select prop:disabled=is_committed on:change=move |ev| {
                            let val = event_target_value(&ev);
                            match val.parse::<i64>() {
                                Ok(id) => set_supplier_contact.set(
                                    contacts.get().iter().find(|c| c.id == Some(id)).cloned()
                                ),
                                Err(_) => set_supplier_contact.set(None),
                            }
                        }>
                            <option value="">"-- Kein Lieferant ausgewählt --"</option>
                            {move || contacts.get().into_iter().map(|c| {
                                let selected = supplier_contact.get().and_then(|s| s.id) == c.id;
                                view! {
                                    <option value=c.id.unwrap_or_default() selected=selected>
                                        {c.display_name()}
                                    </option>
                                }
                            }).collect::<Vec<_>>()}
                        </select>
                    </div>
                </div>
            </div>

            <DocumentField
                doc_metadata=doc_metadata
                set_doc_metadata=set_doc_metadata
                document_data=document_data
                set_document_data=set_document_data

                file_name=file_name
                set_file_name=set_file_name
                ai_status=ai_status
                prefill_pending=prefill_act.pending()
                on_prefill=Callback::new(move |doc| prefill_act.dispatch(doc))
                on_upload=Callback::new(move |doc| einvoice_act.dispatch(doc))
                is_committed=is_committed
            />


            // Add item
            {if !is_committed {
                view! {
                    <div class="box subbox p-4 mt-4">
                        <h3 class="has-text-weight-bold mb-3">"Position hinzufügen"</h3>
                        <div class="field-row">
                            <div class="field is-wide">
                                <label class="label is-small">"Beschreibung"</label>
                                <input class="input" type="text" placeholder="Beschreibung"
                                    prop:value=item_desc
                                    on:input=move |ev| set_item_desc.set(event_target_value(&ev)) />
                            </div>
                            <div class="field is-narrow">
                                <label class="label is-small">"Betrag (€)"</label>
                                <MoneyInput value=item_price />
                            </div>
                            <div class="field">
                                <label class="label is-small">"Kategorie"</label>
                                <div class="select is-fullwidth">
                                    <select on:change=move |ev| {
                                        set_item_cat_id.set(event_target_value(&ev).parse::<i64>().ok())
                                    }>
                                        <option value="">"– wählen –"</option>
                                        {move || categories.get().into_iter().map(|cat| view! {
                                            <option value=cat.id.to_string()>{cat.name}</option>
                                        }).collect::<Vec<_>>()}
                                    </select>
                                </div>
                            </div>
                            <button class="button is-link" title="Hinzufügen" on:click=add_item>
                                <span class="icon"><i class="mdi mdi-plus"></i></span>
                            </button>
                        </div>
                    </div>
                }.into_view()
            } else { "".into_view() }}

            <div class="table-wrap mt-4">
                <table class="table is-fullwidth is-striped">
                    <thead>
                        <tr>
                            <th>"Beschreibung"</th>
                            <th>"Kategorie"</th>
                            <th class="has-text-right">"Betrag"</th>
                            <th></th>
                        </tr>
                    </thead>
                    <tbody>
                        {move || {
                            let items = items_list.get();
                            if items.is_empty() {
                                return view! {
                                    <tr><td colspan="4" class="has-text-centered text-muted">
                                        "Noch keine Positionen."
                                    </td></tr>
                                }.into_view();
                            }
                            items.into_iter().enumerate().map(|(idx, item)| {
                                let cat = item.category.as_ref()
                                    .map(|c| c.name.clone())
                                    .unwrap_or_else(|| "–".to_string());
                                view! {
                                    <tr>
                                        <td>{item.item.clone()}</td>
                                        <td>{cat}</td>
                                        <td class="is-numeric">{format_euro(item.price.amount_cents)}</td>
                                        <td class="has-text-right">
                                            {if !is_committed {
                                                view! {
                                                    <button
                                                        class="button is-small is-danger is-outlined"
                                                        title="Position entfernen"
                                                        on:click=move |_| set_items_list.update(|items| { items.remove(idx); })
                                                    >
                                                        <span class="icon is-small"><i class="mdi mdi-delete"></i></span>
                                                    </button>
                                                }.into_view()
                                            } else { "".into_view() }}
                                        </td>
                                    </tr>
                                }
                            }).collect::<Vec<_>>().into_view()
                        }}
                    </tbody>
                    <tfoot>
                        <tr>
                            <td colspan="2">"Summe"</td>
                            <td class="is-numeric">{move || format_euro(total_cents())}</td>
                            <td></td>
                        </tr>
                    </tfoot>
                </table>
            </div>

            // Unlike an invoice, a receipt records money *you* paid out, and that
            // can happen before it is festgeschrieben — so the panel appears as
            // soon as the receipt exists. Deletion stays possible until commit.
            {if receipt_id.is_some() {
                view! {
                    <PaymentsPanel
                        payments=payments_list
                        total_cents=Signal::derive(move || items_list.get().iter().map(|i| i.price.amount_cents).sum::<i64>())
                        on_add=Callback::new(move |(amount, date)| add_payment_act.dispatch((amount, date)))
                        on_delete=Callback::new(move |id| delete_payment_act.dispatch(id))
                    />
                }.into_view()
            } else { "".into_view() }}

            <div class="field is-grouped mt-5">
                {if is_committed {
                    view! {
                        <div class="control">
                            <span class="tag is-warning is-large">
                                <span class="icon mr-1"><i class="mdi mdi-lock"></i></span>
                                "Festgeschrieben"
                            </span>
                        </div>
                    }.into_view()
                } else {
                    view! {
                        <div class="control">
                            <button class="button is-success" prop:disabled=save_act.pending() on:click=save>
                                {move || if save_act.pending().get() { "Speichern…" } else { "Speichern" }}
                            </button>
                        </div>
                        {if let Some(id) = receipt_id {
                            view! {
                                <div class="control">
                                    <button class="button is-warning" prop:disabled=commit_act.pending() on:click=move |_| commit_act.dispatch(id)>
                                        {move || if commit_act.pending().get() { "Finalisieren…" } else { "Finalisieren" }}
                                    </button>
                                </div>
                                <div class="control">
                                    <button
                                        class="button is-danger"
                                        prop:disabled=delete_act.pending()
                                        on:click=move |_| delete_act.dispatch(id)
                                    >
                                        "Entwurf löschen"
                                    </button>
                                </div>
                            }.into_view()
                        } else { "".into_view() }}
                    }.into_view()
                }}
                <div class="control">
                    <button class="button is-light" on:click=move |_| set_selected_receipt.set(None)>
                        "Abbrechen"
                    </button>
                </div>
            </div>
        </div>
    }
}

/// Attachment field plus the optional local prefill trigger.
#[component]
fn DocumentField(
    doc_metadata: ReadSignal<Option<Document>>,
    set_doc_metadata: WriteSignal<Option<Document>>,
    document_data: ReadSignal<Option<ReceiptDocumentData>>,
    set_document_data: WriteSignal<Option<ReceiptDocumentData>>,
    file_name: ReadSignal<Option<String>>,
    set_file_name: WriteSignal<Option<String>>,
    ai_status: ReadSignal<AiStatus>,
    prefill_pending: ReadSignal<bool>,
    on_prefill: Callback<ReceiptDocumentData>,
    /// Fired as soon as a file is picked, before the user does anything else.
    on_upload: Callback<ReceiptDocumentData>,
    is_committed: bool,
) -> impl IntoView {
    view! {
        <div class="field">
            <label class="label">"Beleg-Dokument"</label>

            {move || if let Some(doc) = doc_metadata.get() {
                view! {
                    <div class="field is-grouped">
                        <div class="control">
                            <a class="button is-link" href=format!("/api/documents/{}", doc.id) target="_blank">
                                <span class="icon mr-1"><i class="mdi mdi-download"></i></span>
                                "Dokument öffnen"
                            </a>
                        </div>
                        {if !is_committed {
                            view! {
                                <div class="control">
                                    <button class="button is-danger is-outlined" on:click=move |_| {
                                        set_doc_metadata.set(None);
                                        set_document_data.set(None);
                                        set_file_name.set(None);
                                    }>
                                        "Entfernen"
                                    </button>
                                </div>
                            }.into_view()
                        } else { "".into_view() }}
                    </div>
                }.into_view()
            } else if let Some(upload) = document_data.get() {
                let is_pdf = upload.media_type == "application/pdf";
                let is_image = upload.media_type.starts_with("image/");
                view! {
                    <div class="field is-grouped is-align-items-center">
                        <div class="control">
                            <span class="tag is-info">
                                {file_name.get().unwrap_or_else(|| "Datei ausgewählt".to_string())}
                            </span>
                        </div>
                        // Uploading an image is always allowed. Only the
                        // prefill action depends on OCR being installed.
                        <Show when=move || {
                            ai_status.get().enabled
                                && !is_committed
                                && (is_pdf || (is_image && ai_status.get().ocr_available))
                        }>
                            <div class="control">
                                <button
                                    class="button is-link"
                                    prop:disabled=move || prefill_pending.get()
                                    title=move || if is_pdf {
                                        format!("Lokale Auswertung: {}", ai_status.get().model)
                                    } else {
                                        "Bild wird per OCR und anschließend mit dem lokalen Modell ausgewertet".to_string()
                                    }
                                    // Read the upload back out of the signal rather than
                                    // capturing it: `Show`'s children must stay `Fn`.
                                    on:click=move |_| {
                                        if let Some(doc) = document_data.get() {
                                            on_prefill.call(doc);
                                        }
                                    }
                                >
                                    <span class="icon mr-1"><i class="mdi mdi-auto-fix"></i></span>
                                    {move || if prefill_pending.get() {
                                        "Beleg wird gelesen…"
                                    } else {
                                        "Mit KI ausfüllen"
                                    }}
                                </button>
                            </div>
                        </Show>
                        {if !is_committed {
                            view! {
                                <div class="control">
                                    <button class="button is-danger is-outlined" on:click=move |_| {
                                        set_document_data.set(None);
                                        set_file_name.set(None);
                                    }>
                                        "Entfernen"
                                    </button>
                                </div>
                            }.into_view()
                        } else { "".into_view() }}
                    </div>
                }.into_view()
            } else {
                if !is_committed {
                    view! {
                        <div class="control">
                            <div class="file is-fullwidth">
                                <label class="file-label">
                                    <input
                                        class="file-input"
                                        type="file"
                                    accept="application/pdf,application/xml,text/xml,.xml,image/png,image/jpeg,image/webp"
                                        on:change=move |ev| {
                                            let Some(target) = ev.target() else { return };
                                            let input = target.unchecked_into::<web_sys::HtmlInputElement>();
                                            let Some(files) = input.files() else { return };
                                            let Some(file) = files.get(0) else { return };
                                            set_file_name.set(Some(file.name()));
                                            read_file_as_base64(file, move |doc| {
                                                set_document_data.set(Some(doc.clone()));
                                                // Structured invoice data, if any, beats any later guess.
                                                on_upload.call(doc);
                                            });
                                        }
                                    />
                                    <span class="file-cta">
                                        <span class="file-icon"><i class="mdi mdi-upload"></i></span>
                                        <span class="file-label">"PDF, XML (E-Rechnung) oder Bild hochladen…"</span>
                                    </span>
                                </label>
                            </div>
                            <p class="help">
                                <span class="icon is-small mr-1"><i class="mdi mdi-file-check-outline"></i></span>
                                "E-Rechnungen (ZUGFeRD/Factur-X, XRechnung) werden beim Hochladen automatisch ausgelesen."
                            </p>
                            <Show when=move || ai_status.get().enabled>
                                <p class="help">
                                    <span class="icon is-small mr-1"><i class="mdi mdi-auto-fix"></i></span>
                                    "Andere PDFs können lokal per KI ausgelesen werden."
                                </p>
                            </Show>
                        </div>
                    }.into_view()
                } else {
                    view! {
                        <p class="text-muted is-size-7">"Kein Dokument hinterlegt."</p>
                    }.into_view()
                }
            }}
        </div>
    }
}
