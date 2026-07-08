use leptos::*;

use chrono::{NaiveDate, Utc};
use shared::*;
use wasm_bindgen::JsCast;

use crate::components::{EmptyState, MoneyInput};
use crate::server::{
    delete_receipt, get_ai_status, get_categories, get_contacts, get_receipt, get_receipts,
    prefill_receipt, save_receipt,
};

/// Reads the picked file as base64 and hands it to `on_load`.
fn read_file_as_base64(file: web_sys::File, on_load: impl Fn(ReceiptDocumentData) + 'static) {
    let name = file.name();
    let media_type = match file.type_().as_str() {
        // Some browsers report an empty type; fall back to the extension.
        "" if name.to_lowercase().ends_with(".pdf") => "application/pdf".to_string(),
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

    let load_receipts = create_action(move |_: &()| async move {
        match get_receipts().await {
            Ok(list) => set_receipts.set(list),
            Err(e) => logging::log!("Error fetching receipts: {:?}", e),
        }
    });

    create_resource(
        || (),
        move |_| async move {
            if let Ok(list) = get_categories().await {
                set_categories.set(list);
            }
            if let Ok(list) = get_contacts().await {
                set_contacts.set(list);
            }
            // When the server has the feature switched off we never show the
            // prefill button, and no model needs to exist on the machine.
            if let Ok(status) = get_ai_status().await {
                set_ai_status.set(status);
            }
        },
    );

    load_receipts.dispatch(());


    let new_receipt = move |_| {
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
    };

    let selected_id = move || selected_receipt.get().and_then(|r| r.id);

    view! {
        <div class="container">
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

            <div class="columns is-split">
                <div class="column is-5">
                    <div class="box">
                        <Show
                            when=move || !receipts.get().is_empty()
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
                                        view! {
                                            <div
                                                class="box list-item p-3 mb-2"
                                                class:is-active=move || selected_id() == Some(id)
                                                on:click=move |_| {
                                                    spawn_local(async move {
                                                        match get_receipt(id).await {
                                                            Ok(full) => set_selected_receipt.set(Some(full)),
                                                            Err(e) => logging::log!("Error fetching receipt: {:?}", e),
                                                        }
                                                    });
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
                                                    </span>
                                                    <span class="has-text-weight-semibold is-numeric">{total}</span>
                                                </div>
                                                <div class="is-size-7 text-muted">{supplier} " • " {date}</div>
                                            </div>
                                        }
                                    }
                                </For>
                            </div>
                        </Show>
                    </div>
                </div>

                <div class="column">
                    {move || match selected_receipt.get() {
                        None => view! {
                            <EmptyState icon="receipt-text-outline" text="Wählen Sie einen Beleg aus." />
                        }.into_view(),
                        Some(rec) => view! {
                            <ReceiptEditor
                                // Remount the editor when a different receipt is picked so
                                // its internal signals restart from the new data.
                                rec=rec
                                contacts=contacts
                                categories=categories
                                ai_status=ai_status
                                on_change=Callback::new(move |_| load_receipts.dispatch(()))
                                set_selected_receipt=set_selected_receipt
                            />
                        }.into_view(),
                    }}
                </div>
            </div>
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
) -> impl IntoView {
    let receipt_id = rec.id;

    let (receipt_num, set_receipt_num) = create_signal(rec.receipt_number.clone());
    let (receipt_date, set_receipt_date) = create_signal(
        rec.receipt_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default(),
    );
    let (supplier_contact, set_supplier_contact) = create_signal(rec.supplier_contact.clone());
    let (document_data, set_document_data) = create_signal(rec.document_data.clone());
    let (doc_metadata, set_doc_metadata) = create_signal(rec.document.clone());
    let (items_list, set_items_list) = create_signal(rec.items.clone());
    let (error, set_error) = create_signal(Option::<String>::None);
    let (warnings, set_warnings) = create_signal(Vec::<String>::new());
    let (file_name, set_file_name) = create_signal(Option::<String>::None);

    // New-item form.
    let (item_desc, set_item_desc) = create_signal(String::new());
    let item_price = create_rw_signal(0i64);
    let (item_cat_id, set_item_cat_id) = create_signal(Option::<i64>::None);

    let total_cents = move || items_list.get().iter().map(|i| i.price.amount_cents).sum::<i64>();

    let save_act = create_action(move |r: &Receipt| {
        let r = r.clone();
        async move {
            match save_receipt(r).await {
                Ok(saved) => {
                    on_change.call(());
                    set_error.set(None);
                    // Reflect what the server actually stored (ids, document version).
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
                    set_selected_receipt.set(None);
                }
                Err(e) => set_error.set(Some(format!("Löschen fehlgeschlagen: {e}"))),
            }
        }
    });

    // Runs the uploaded file through the local model and fills the form.
    let prefill_act = create_action(move |doc: &ReceiptDocumentData| {
        let doc = doc.clone();
        async move {
            set_error.set(None);
            set_warnings.set(vec![]);
            match prefill_receipt(doc).await {
                Ok(p) => {
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
        let category = categories.get().into_iter().find(|c| Some(c.id) == item_cat_id.get());
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
                            on:input=move |ev| set_receipt_num.set(event_target_value(&ev)) />
                    </div>
                </div>
                <div class="field">
                    <label class="label">"Belegdatum"</label>
                    <div class="control">
                        <input class="input" type="date"
                            prop:value=receipt_date
                            on:input=move |ev| set_receipt_date.set(event_target_value(&ev)) />
                    </div>
                </div>
            </div>

            <div class="field">
                <label class="label">"Lieferant (Kontakt)"</label>
                <div class="control">
                    <div class="select is-fullwidth">
                        <select on:change=move |ev| {
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
            />

            // Add item
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
                                            <button
                                                class="button is-small is-danger is-outlined"
                                                title="Position entfernen"
                                                on:click=move |_| set_items_list.update(|items| { items.remove(idx); })
                                            >
                                                <span class="icon is-small"><i class="mdi mdi-delete"></i></span>
                                            </button>
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

            <div class="field is-grouped mt-5">
                <div class="control">
                    <button class="button is-success" prop:disabled=save_act.pending() on:click=save>
                        {move || if save_act.pending().get() { "Speichern…" } else { "Speichern" }}
                    </button>
                </div>
                {receipt_id.map(|id| view! {
                    <div class="control">
                        <button
                            class="button is-danger"
                            prop:disabled=delete_act.pending()
                            on:click=move |_| delete_act.dispatch(id)
                        >
                            "Löschen"
                        </button>
                    </div>
                })}
                <div class="control">
                    <button class="button is-light" on:click=move |_| set_selected_receipt.set(None)>
                        "Abbrechen"
                    </button>
                </div>
            </div>
        </div>
    }
}

/// Attachment field plus the optional local-AI prefill trigger.
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
                        <div class="control">
                            <button class="button is-danger is-outlined" on:click=move |_| {
                                set_doc_metadata.set(None);
                                set_document_data.set(None);
                                set_file_name.set(None);
                            }>
                                "Entfernen"
                            </button>
                        </div>
                    </div>
                }.into_view()
            } else if let Some(upload) = document_data.get() {
                let is_pdf = upload.media_type == "application/pdf";
                view! {
                    <div class="field is-grouped is-align-items-center">
                        <div class="control">
                            <span class="tag is-info">
                                {file_name.get().unwrap_or_else(|| "Datei ausgewählt".to_string())}
                            </span>
                        </div>
                        // Hidden entirely when the server has AI switched off.
                        <Show when=move || ai_status.get().enabled>
                            <div class="control">
                                <button
                                    class="button is-link"
                                    prop:disabled=move || prefill_pending.get() || !is_pdf
                                    title=move || if is_pdf {
                                        format!("Lokales Modell: {}", ai_status.get().model)
                                    } else {
                                        "Nur PDFs mit Textebene können ausgelesen werden".to_string()
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
                        <div class="control">
                            <button class="button is-danger is-outlined" on:click=move |_| {
                                set_document_data.set(None);
                                set_file_name.set(None);
                            }>
                                "Entfernen"
                            </button>
                        </div>
                    </div>
                }.into_view()
            } else {
                view! {
                    <div class="control">
                        <div class="file is-fullwidth">
                            <label class="file-label">
                                <input
                                    class="file-input"
                                    type="file"
                                    accept="application/pdf,image/png,image/jpeg"
                                    on:change=move |ev| {
                                        let Some(target) = ev.target() else { return };
                                        let input = target.unchecked_into::<web_sys::HtmlInputElement>();
                                        let Some(files) = input.files() else { return };
                                        let Some(file) = files.get(0) else { return };
                                        set_file_name.set(Some(file.name()));
                                        read_file_as_base64(file, move |doc| set_document_data.set(Some(doc)));
                                    }
                                />
                                <span class="file-cta">
                                    <span class="file-icon"><i class="mdi mdi-upload"></i></span>
                                    <span class="file-label">"PDF oder Bild hochladen…"</span>
                                </span>
                            </label>
                        </div>
                        <Show when=move || ai_status.get().enabled>
                            <p class="help">
                                <span class="icon is-small mr-1"><i class="mdi mdi-auto-fix"></i></span>
                                "Nach dem Hochladen kann ein PDF lokal per KI ausgelesen werden."
                            </p>
                        </Show>
                    </div>
                }.into_view()
            }}
        </div>
    }
}
