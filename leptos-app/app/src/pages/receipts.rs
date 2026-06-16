use leptos::*;

use chrono::Utc;
use shared::*;
use wasm_bindgen::JsCast;
use crate::server::{
    get_contacts, get_receipts, get_receipt, save_receipt, get_categories,

};

#[component]
pub fn ReceiptsPage() -> impl IntoView {
    let (receipts, set_receipts) = create_signal(Vec::<ReceiptListItem>::new());
    let (selected_receipt, set_selected_receipt) = create_signal(Option::<Receipt>::None);
    let (categories, set_categories) = create_signal(Vec::<ReceiptItemCategory>::new());
    
    // Load receipts action
    let load_receipts = create_action(move |_| async move {
        match get_receipts().await {
            Ok(list) => set_receipts.set(list),
            Err(e) => logging::log!("Error fetching receipts: {:?}", e),
        }
    });

    // Load categories action
    let load_cats = create_action(move |_| async move {
        match get_categories().await {
            Ok(list) => set_categories.set(list),
            Err(e) => logging::log!("Error fetching categories: {:?}", e),
        }
    });

    // Save receipt action
    let save_receipt_act = create_action(move |r: &Receipt| {
        let r = r.clone();
        async move {
            match save_receipt(r).await {
                Ok(_) => {
                    load_receipts.dispatch(());
                    set_selected_receipt.set(None);
                },
                Err(e) => logging::log!("Error saving receipt: {:?}", e),
            }
        }
    });

    // Load contacts action
    let (contacts, set_contacts) = create_signal(Vec::<Contact>::new());
    let load_contacts = create_action(move |_| async move {
        match get_contacts().await {
            Ok(list) => set_contacts.set(list),
            Err(e) => logging::log!("Error fetching contacts: {:?}", e),
        }
    });

    // Initial load
    load_receipts.dispatch(());
    load_cats.dispatch(());
    load_contacts.dispatch(());

    view! {
        <div class="container">
            <div class="level">
                <div class="level-left">
                    <h1 class="title">"Belege"</h1>
                </div>
                <div class="level-right">
                    <button class="button is-link" on:click=move |_| {
                        set_selected_receipt.set(Some(Receipt {
                            id: None,
                            items: vec![],
                            created_timestamp: None,
                            committed_timestamp: None,
                            receipt_number: "".to_string(),
                            payments: vec![],
                            receipt_date: Some(Utc::now().naive_utc().date()),
                            due_date: None,
                            supplier_contact: None,
                            document: None,
                            document_data: None,
                        }));
                    }>
                        "Neuer Beleg"
                    </button>
                </div>
            </div>

            <div class="columns">
                // List Receipts
                <div class="column is-5">
                    <div class="box">
                        <div style="max-height: 70vh; overflow-y: auto;">
                            {move || receipts.get().into_iter().map(|rec| {
                                let supplier_name = rec.supplier_contact.map(|c| format!("{}, {}", c.name, c.first_name.unwrap_or_default())).unwrap_or_else(|| "Supplier".to_string());
                                let display_num = rec.receipt_number.unwrap_or_default();
                                view! {
                                    <div class="box p-3 mb-2 is-clickable" on:click=move |_| {
                                        let id = rec.id;
                                        spawn_local(async move {
                                            if let Ok(full_rec) = get_receipt(id).await {
                                                set_selected_receipt.set(Some(full_rec));
                                            }
                                        });
                                    }>
                                        <div class="has-text-weight-bold">"Beleg #" {display_num}</div>
                                        <div class="is-size-7 gray">{supplier_name}</div>
                                    </div>
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    </div>
                </div>

                // View & Edit Receipt
                <div class="column">
                    {move || match selected_receipt.get() {
                        None => view! {
                            <div class="box has-text-centered p-6">
                                <p class="is-size-5 has-text-grey">"Wählen Sie einen Beleg aus."</p>
                            </div>
                        }.into_view(),
                        Some(mut rec) => {
                            let (receipt_num, set_receipt_num) = create_signal(rec.receipt_number.clone());
                            let _receipt_id = rec.id;
                            let (supplier_contact, set_supplier_contact) = create_signal(rec.supplier_contact.clone());
                            let (document_data, set_document_data) = create_signal(rec.document_data.clone());
                            let (doc_metadata, set_doc_metadata) = create_signal(rec.document.clone());

                            let (item_desc, set_item_desc) = create_signal(String::new());
                            let (item_price, set_item_price) = create_signal(0.0);
                            let (item_cat_id, set_item_cat_id) = create_signal(Option::<i64>::None);
                            let (items_list, set_items_list) = create_signal(rec.items.clone());

                            view! {
                                <div class="box">
                                    <h2 class="subtitle">"Belegsdetails"</h2>
                                    <div class="field">
                                        <label class="label">"Belegsnummer"</label>
                                        <div class="control">
                                            <input class="input" type="text" prop:value=receipt_num on:input=move |ev| set_receipt_num.set(event_target_value(&ev)) />
                                        </div>
                                    </div>
                                    <div class="field">
                                        <label class="label">"Lieferant (Kontakt)"</label>
                                        <div class="control">
                                            <div class="select is-fullwidth">
                                                <select
                                                    on:change=move |ev| {
                                                        let val = event_target_value(&ev);
                                                        if let Ok(id) = val.parse::<i64>() {
                                                            if let Some(c) = contacts.get().iter().find(|con| con.id == Some(id)) {
                                                                set_supplier_contact.set(Some(c.clone()));
                                                            }
                                                        } else {
                                                            set_supplier_contact.set(None);
                                                        }
                                                    }
                                                >
                                                    <option value="">"-- Kein Lieferant ausgewählt --"</option>
                                                    {move || contacts.get().iter().map(|c| {
                                                        let sel = supplier_contact.get().as_ref().and_then(|cc| cc.id) == c.id;
                                                        let display_name = format!("{}, {}", c.name, c.first_name.clone().unwrap_or_default());
                                                        view! {
                                                            <option value=c.id.unwrap_or_default() selected=sel>{display_name}</option>
                                                        }
                                                    }).collect::<Vec<_>>()}
                                                </select>
                                            </div>
                                        </div>
                                    </div>

                                    <div class="field">
                                        <label class="label">"Dokument (PDF)"</label>
                                        {move || if let Some(doc) = doc_metadata.get() {
                                            view! {
                                                <div class="field is-grouped">
                                                    <div class="control">
                                                        <a class="button is-link" href=format!("/api/documents/{}", doc.id) target="_blank">
                                                            <span class="icon mr-1"><i class="mdi mdi-download"></i></span>
                                                            "Bestehendes PDF herunterladen"
                                                        </a>
                                                    </div>
                                                    <div class="control">
                                                        <button class="button is-danger is-outlined" on:click=move |_| {
                                                            set_doc_metadata.set(None);
                                                            set_document_data.set(None);
                                                        }>
                                                            "Entfernen"
                                                        </button>
                                                    </div>
                                                </div>
                                            }.into_view()
                                        } else if let Some(_upload) = document_data.get() {
                                            view! {
                                                <div class="field is-grouped">
                                                    <div class="control" style="align-self: center;">
                                                        <span class="tag is-info">"PDF ausgewählt"</span>
                                                    </div>
                                                    <div class="control">
                                                        <button class="button is-danger is-outlined" on:click=move |_| {
                                                            set_document_data.set(None);
                                                        }>
                                                            "Entfernen"
                                                        </button>
                                                    </div>
                                                </div>
                                            }.into_view()
                                        } else {
                                            view! {
                                                <div class="control">
                                                    <div class="file has-name is-fullwidth">
                                                        <label class="file-label">
                                                            <input
                                                                class="file-input"
                                                                type="file"
                                                                accept="application/pdf"
                                                                on:change=move |ev| {
                                                                    let target = ev.target().unwrap();
                                                                    let input = target.unchecked_into::<web_sys::HtmlInputElement>();
                                                                    if let Some(files) = input.files() {
                                                                        if let Some(file) = files.get(0) {
                                                                            let set_doc = set_document_data;
                                                                            let reader = web_sys::FileReader::new().unwrap();
                                                                            let r_c = reader.clone();
                                                                            let onload = wasm_bindgen::closure::Closure::wrap(Box::new(move |_e: web_sys::Event| {
                                                                                if let Ok(res) = r_c.result() {
                                                                                    if let Some(s) = res.as_string() {
                                                                                        if let Some(comma_idx) = s.find(',') {
                                                                                            let b64 = s[comma_idx + 1..].to_string();
                                                                                            set_doc.set(Some(ReceiptDocumentData {
                                                                                                data: b64,
                                                                                                extension: "pdf".to_string(),
                                                                                                media_type: "application/pdf".to_string(),
                                                                                            }));
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }) as Box<dyn FnMut(web_sys::Event)>);
                                                                            reader.set_onload(Some(onload.as_ref().unchecked_ref()));
                                                                            let _ = reader.read_as_data_url(&file);
                                                                            onload.forget();
                                                                        }
                                                                    }
                                                                }
                                                            />
                                                            <span class="file-cta">
                                                                <span class="file-icon">
                                                                    <i class="mdi mdi-upload"></i>
                                                                </span>
                                                                <span class="file-label">
                                                                    "PDF hochladen..."
                                                                </span>
                                                            </span>
                                                        </label>
                                                    </div>
                                                </div>
                                            }.into_view()
                                        }}
                                    </div>

                                    // Add Item Section
                                    <div class="box has-background-white-ter p-3">
                                        <h3 class="has-text-weight-bold mb-2">"Position hinzufügen"</h3>
                                        <div class="columns">
                                            <div class="column is-5">
                                                <div class="field">
                                                    <label class="label is-small">"Beschreibung"</label>
                                                    <input class="input" type="text" placeholder="Beschreibung" prop:value=item_desc on:input=move |ev| set_item_desc.set(event_target_value(&ev)) />
                                                </div>
                                            </div>
                                            <div class="column is-2">
                                                <div class="field">
                                                    <label class="label is-small">"Preis (€)"</label>
                                                    <input class="input" type="number" placeholder="Preis (€)" prop:value=item_price on:input=move |ev| set_item_price.set(event_target_value(&ev).parse::<f64>().unwrap_or(0.0)) />
                                                </div>
                                            </div>
                                            <div class="column is-3">
                                                <div class="field">
                                                    <label class="label is-small">"Kategorie"</label>
                                                    <div class="select is-fullwidth">
                                                        <select on:change=move |ev| {
                                                            let val = event_target_value(&ev);
                                                            set_item_cat_id.set(val.parse::<i64>().ok());
                                                        }>
                                                            <option value="">"Kategorie wählen"</option>
                                                            {move || categories.get().into_iter().map(|cat| {
                                                                view! {
                                                                    <option value=cat.id.to_string()>{cat.name}</option>
                                                                }
                                                            }).collect::<Vec<_>>()}
                                                        </select>
                                                    </div>
                                                </div>
                                            </div>
                                            <div class="column is-2">
                                                <div class="field">
                                                    <label class="label is-small">"Aktion"</label>
                                                    <button class="button is-link is-fullwidth" on:click=move |_| {
                                                        let cents = (item_price.get() * 100.0) as i64;
                                                        let matched_cat = categories.get().into_iter().find(|c| Some(c.id) == item_cat_id.get());
                                                        let new_item = ReceiptItem {
                                                            item: item_desc.get(),
                                                            price: Money::new(cents),
                                                            category: matched_cat,
                                                        };
                                                        let mut current = items_list.get();
                                                        current.push(new_item);
                                                        set_items_list.set(current);
                                                        set_item_desc.set("".to_string());
                                                    }>
                                                        "Hinzufügen"
                                                    </button>
                                                </div>
                                            </div>
                                        </div>
                                    </div>

                                    // Items List
                                    <table class="table is-fullwidth is-striped">
                                        <thead>
                                            <tr>
                                                <th>"Beschreibung"</th>
                                                <th>"Kategorie"</th>
                                                <th>"Betrag"</th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            {move || items_list.get().iter().map(|item| {
                                                let cat_name = item.category.as_ref().map(|c| c.name.clone()).unwrap_or_else(|| "-".to_string());
                                                view! {
                                                    <tr>
                                                        <td>{item.item.clone()}</td>
                                                        <td>{cat_name}</td>
                                                        <td>{format!("{:.2} €", item.price.amount_cents as f64 / 100.0)}</td>
                                                    </tr>
                                                }
                                            }).collect::<Vec<_>>()}
                                        </tbody>
                                    </table>

                                    <div class="field is-grouped mt-5">
                                        <div class="control">
                                            <button class="button is-success" on:click=move |_| {
                                                rec.receipt_number = receipt_num.get();
                                                rec.items = items_list.get();
                                                rec.supplier_contact = supplier_contact.get();
                                                rec.document_data = document_data.get();
                                                rec.document = doc_metadata.get();
                                                save_receipt_act.dispatch(rec.clone());
                                            }>
                                                "Speichern"
                                            </button>
                                        </div>
                                        <div class="control">
                                            <button class="button is-light" on:click=move |_| set_selected_receipt.set(None)>
                                                "Abbrechen"
                                            </button>
                                        </div>
                                    </div>
                                </div>
                            }.into_view()
                        }
                    }}
                </div>
            </div>
        </div>
    }
}
