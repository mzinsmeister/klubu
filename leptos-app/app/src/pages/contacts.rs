use crate::components::EmptyState;
use crate::server::{
    add_contact_note, archive_contact, get_archived_contacts, get_contact_crm, get_contacts,
    restore_contact, save_contact,
};
use leptos::*;
use leptos_router::{use_navigate, use_params_map, NavigateOptions};
use shared::*;

const CONTACT_PAGE_SIZE: u32 = 50;

#[component]
pub fn ContactsPage() -> impl IntoView {
    let (contacts, set_contacts) = create_signal(Vec::<Contact>::new());
    let (selected_contact, set_selected_contact) = create_signal(Option::<Contact>::None);
    let (search_query, set_search_query) = create_signal(String::new());

    let params = use_params_map();
    let id_param = move || params.with(|p| p.get("id").cloned());

    create_effect(move |_| {
        let id_val = id_param();
        match id_val.as_deref() {
            None => {
                set_selected_contact.set(None);
            }
            Some("new") => {
                set_selected_contact.set(Some(Contact {
                    id: None,
                    form_of_address: Some("Herr".to_string()),
                    title: None,
                    name: "".to_string(),
                    first_name: Some("".to_string()),
                    street: Some("".to_string()),
                    zip_code: Some("".to_string()),
                    city: Some("".to_string()),
                    house_number: Some("".to_string()),
                    country: Some("Deutschland".to_string()),
                    phones: Vec::new(),
                    is_person: true,
                    archived_timestamp: None,
                    emails: Vec::new(),
                }));
            }
            Some(id_str) => {
                if let Ok(id) = id_str.parse::<i64>() {
                    let already_selected =
                        selected_contact.get_untracked().and_then(|c| c.id) == Some(id);
                    if !already_selected {
                        spawn_local(async move {
                            if let Ok(summary) = get_contact_crm(id).await {
                                set_selected_contact.set(Some(summary.contact));
                            }
                        });
                    }
                }
            }
        }
    });
    let (has_more_contacts, set_has_more_contacts) = create_signal(false);
    let (list_generation, set_list_generation) = create_signal(0_u64);
    // Archived contacts live in a separate view, not interleaved with active
    // ones: they are out of the pickers, and the list should say so clearly.
    let (show_archive, set_show_archive) = create_signal(false);
    let (crm_refresh, set_crm_refresh) = create_signal(0_u64);
    let crm = create_resource(
        move || {
            (
                selected_contact.get().and_then(|contact| contact.id),
                crm_refresh.get(),
            )
        },
        |(id, _)| async move {
            match id {
                Some(id) => Some(get_contact_crm(id).await),
                None => None,
            }
        },
    );

    // Each request carries the filter it was made for. This prevents a slow
    // response to an earlier keystroke from replacing a newer search result.
    let load_contacts = create_action(
        move |(generation, offset, query, archived): &(u64, u32, String, bool)| {
            let generation = *generation;
            let offset = *offset;
            let query = query.clone();
            let archived = *archived;
            async move {
                let server_query = (!query.trim().is_empty()).then(|| query.trim().to_string());
                let result = if archived {
                    get_archived_contacts(offset, CONTACT_PAGE_SIZE, server_query).await
                } else {
                    get_contacts(offset, CONTACT_PAGE_SIZE, server_query).await
                };
                match result {
                    Ok(page) => {
                        if list_generation.get_untracked() != generation
                            || search_query.get_untracked() != query
                            || show_archive.get_untracked() != archived
                        {
                            return;
                        }

                        if offset == 0 {
                            set_contacts.set(page.items);
                        } else if contacts.get_untracked().len() as u32 == offset {
                            set_contacts.update(|items| items.extend(page.items));
                        } else {
                            return;
                        }
                        set_has_more_contacts.set(page.has_more);
                    }
                    Err(e) => logging::log!("Error fetching contacts: {:?}", e),
                }
            }
        },
    );

    // Save contact action
    let save_contact_act = create_action(move |c: &Contact| {
        let c = c.clone();
        async move {
            match save_contact(c).await {
                Ok(saved) => {
                    let generation = list_generation.get_untracked().wrapping_add(1);
                    set_list_generation.set(generation);
                    set_contacts.set(Vec::new());
                    set_has_more_contacts.set(false);
                    load_contacts.dispatch((
                        generation,
                        0,
                        search_query.get_untracked(),
                        show_archive.get_untracked(),
                    ));
                    let target_path = format!("/contacts/{}", saved.id.unwrap_or_default());
                    let _ = use_navigate()(
                        &target_path,
                        NavigateOptions {
                            replace: true,
                            ..NavigateOptions::default()
                        },
                    );
                    set_selected_contact.set(Some(saved));
                }
                Err(e) => logging::log!("Error saving contact: {:?}", e),
            }
        }
    });

    // Archive moves a contact out of pickers and lists; restore brings it
    // back. Both reload the current view afterwards.
    let archive_contact_act = create_action(move |(id, restore): &(i64, bool)| {
        let id = *id;
        let restore = *restore;
        async move {
            let result = if restore {
                restore_contact(id).await
            } else {
                archive_contact(id).await
            };
            match result {
                Ok(_) => {
                    let generation = list_generation.get_untracked().wrapping_add(1);
                    set_list_generation.set(generation);
                    set_contacts.set(Vec::new());
                    set_has_more_contacts.set(false);
                    load_contacts.dispatch((
                        generation,
                        0,
                        search_query.get_untracked(),
                        show_archive.get_untracked(),
                    ));
                    let _ = use_navigate()("/contacts", NavigateOptions::default());
                }
                Err(e) => logging::log!("Error archiving/restoring contact: {:?}", e),
            }
        }
    });

    // Initial load
    load_contacts.dispatch((0, 0, String::new(), false));

    view! {
        <div class="container">
            {move || match selected_contact.get() {
                None => {
                    view! {
                        <div class="level">
                            <div class="level-left">
                                <h1 class="title">
                                    {move || if show_archive.get() { "Kontakte — Archiv" } else { "Kunden & Kontakte" }}
                                </h1>
                            </div>
                            <div class="level-right">
                                <button class="button is-light mr-2" on:click=move |_| {
                                    let archived = !show_archive.get_untracked();
                                    set_show_archive.set(archived);
                                    let generation = list_generation.get_untracked().wrapping_add(1);
                                    set_list_generation.set(generation);
                                    set_contacts.set(Vec::new());
                                    set_has_more_contacts.set(false);
                                    let _ = use_navigate()("/contacts", NavigateOptions::default());
                                    load_contacts.dispatch((generation, 0, search_query.get_untracked(), archived));
                                }>
                                    <span class="icon"><i class="mdi mdi-archive-outline"></i></span>
                                    <span>{move || if show_archive.get() { "Aktive Kontakte" } else { "Archiv" }}</span>
                                </button>
                                <button class="button is-link" on:click=move |_| {
                                    let _ = use_navigate()("/contacts/new", NavigateOptions::default());
                                }>
                                    "Neuer Kontakt"
                                </button>
                            </div>
                        </div>

                        <div class="box">
                            <div class="field mb-4">
                                <p class="control has-icons-left">
                                    <input class="input" type="text" placeholder="Suchen..."
                                        prop:value=search_query
                                        on:input=move |ev| {
                                            let query = event_target_value(&ev);
                                            let generation = list_generation.get_untracked().wrapping_add(1);
                                            set_list_generation.set(generation);
                                            set_search_query.set(query.clone());
                                            set_contacts.set(Vec::new());
                                            set_has_more_contacts.set(false);
                                            load_contacts.dispatch((generation, 0, query, show_archive.get_untracked()));
                                        } />
                                    <span class="icon is-small is-left">
                                        <i class="mdi mdi-magnify"></i>
                                    </span>
                                </p>
                            </div>
                            <div class="contact-list">
                                {move || contacts.get().into_iter().map(|contact| {
                                    let name = contact.display_name();
                                    let address = contact.display_address();
                                    let email = contact.emails.first().cloned();
                                    let contact_id = contact.id;
                                    view! {
                                        <div
                                            class="box list-item p-3 mb-2"
                                            on:click=move |_| {
                                                if let Some(id) = contact_id {
                                                    let target = format!("/contacts/{}", id);
                                                    let _ = use_navigate()(&target, NavigateOptions::default());
                                                }
                                            }
                                        >
                                            <div class="has-text-weight-bold">{name}</div>
                                            <div class="is-size-7 text-muted">{address}</div>
                                            {email.map(|value| view! { <div class="is-size-7 text-muted">{value}</div> })}
                                        </div>
                                    }
                                }).collect::<Vec<_>>()}
                                <Show when=move || has_more_contacts.get()>
                                    <div class="has-text-centered mt-3">
                                        <button
                                            class="button is-light"
                                            prop:disabled=load_contacts.pending()
                                            on:click=move |_| {
                                                let offset = contacts.get_untracked().len() as u32;
                                                load_contacts.dispatch((
                                                    list_generation.get_untracked(),
                                                    offset,
                                                    search_query.get_untracked(),
                                                    show_archive.get_untracked(),
                                                ));
                                            }
                                        >
                                            {move || if load_contacts.pending().get() { "Lädt…" } else { "Mehr laden" }}
                                        </button>
                                    </div>
                                </Show>
                                {move || if contacts.get().is_empty() && !load_contacts.pending().get() {
                                    view! { <EmptyState icon="account-outline" text="Keine Kontakte gefunden." /> }.into_view()
                                } else { "".into_view() }}
                            </div>
                        </div>
                    }.into_view()
                }
                Some(mut contact) => {
                    let (c_name, set_c_name) = create_signal(contact.name.clone());
                    let (c_first, set_c_first) = create_signal(contact.first_name.clone().unwrap_or_default());
                    let (c_title, set_c_title) = create_signal(contact.title.clone().unwrap_or_default());
                    let (c_form_of_address, set_c_form_of_address) = create_signal(contact.form_of_address.clone().unwrap_or_default());
                    let (c_street, set_c_street) = create_signal(contact.street.clone().unwrap_or_default());
                    let (c_zip, set_c_zip) = create_signal(contact.zip_code.clone().unwrap_or_default());
                    let (c_city, set_c_city) = create_signal(contact.city.clone().unwrap_or_default());
                    let (c_house, set_c_house) = create_signal(contact.house_number.clone().unwrap_or_default());
                    let (c_country, set_c_country) = create_signal(contact.country.clone().unwrap_or_else(|| "Deutschland".to_string()));
                    let (c_phones, set_c_phones) = create_signal(contact.phones.clone());
                    let (new_phone, set_new_phone) = create_signal(String::new());
                    let (c_emails, set_c_emails) = create_signal(contact.emails.clone());
                    let (new_email, set_new_email) = create_signal(String::new());
                    let (c_is_person, set_c_is_person) = create_signal(contact.is_person);
                    let has_unsaved_changes = {
                        let contact = contact.clone();
                        move || {
                            c_name.get() != contact.name
                                || Some(c_first.get()) != contact.first_name
                                || Some(c_title.get()) != contact.title
                                || Some(c_form_of_address.get()) != contact.form_of_address
                                || Some(c_street.get()) != contact.street
                                || Some(c_zip.get()) != contact.zip_code
                                || Some(c_city.get()) != contact.city
                                || Some(c_house.get()) != contact.house_number
                                || Some(c_country.get()) != contact.country
                                || c_phones.get() != contact.phones
                                || c_emails.get() != contact.emails
                                || c_is_person.get() != contact.is_person
                        }
                    };

                    #[cfg(target_arch = "wasm32")]
                    {
                        use wasm_bindgen::prelude::*;
                        use wasm_bindgen::JsCast;
                        let has_changes = has_unsaved_changes.clone();
                        let listener = Closure::<dyn FnMut(web_sys::BeforeUnloadEvent) -> String>::new(move |e: web_sys::BeforeUnloadEvent| {
                            if has_changes() {
                                let msg = "Sie haben ungespeicherte Änderungen.";
                                e.set_return_value(msg);
                                msg.to_string()
                            } else {
                                "".to_string()
                            }
                        });
                        if let Some(w) = web_sys::window() {
                            let _ = w.add_event_listener_with_callback("beforeunload", listener.as_ref().unchecked_ref());
                            let cb_ref = listener.as_ref().clone();
                            leptos::on_cleanup(move || {
                                if let Some(w) = web_sys::window() {
                                    let _ = w.remove_event_listener_with_callback("beforeunload", cb_ref.unchecked_ref());
                                }
                            });
                        }
                        listener.forget();
                    }

                    let is_edit = contact.id.is_some();
                    let contact_id = contact.id;
                    let contact_archived = contact.archived_timestamp.is_some();
                    let (note_body, set_note_body) = create_signal(String::new());
                    let add_note_action = create_action(move |body: &String| {
                        let body = body.clone();
                        async move {
                            if let Some(contact_id) = contact_id {
                                match add_contact_note(contact_id, body).await {
                                    Ok(_) => {
                                        set_note_body.set(String::new());
                                        set_crm_refresh.update(|value| *value = value.wrapping_add(1));
                                    }
                                    Err(error) => logging::log!("Error adding contact note: {:?}", error),
                                }
                            }
                        }
                    });

                    let form_fields = {
                        let has_unsaved_changes = has_unsaved_changes.clone();
                        move || {
                        view! {
                            <>
                                <div class="field-row">
                                <div class="field is-narrow">
                                    <label class="label">"Anrede"</label>
                                    <div class="control">
                                        <input class="input" type="text" placeholder="Herr / Frau etc" prop:value=c_form_of_address on:input=move |ev| set_c_form_of_address.set(event_target_value(&ev)) />
                                    </div>
                                </div>
                                <div class="field is-narrow">
                                    <label class="label">"Titel"</label>
                                    <div class="control">
                                        <input class="input" type="text" placeholder="Dr. / Prof. etc" prop:value=c_title on:input=move |ev| set_c_title.set(event_target_value(&ev)) />
                                    </div>
                                </div>
                            </div>
                            <div class="field-row">
                                <div class="field">
                                    <label class="label">"Vorname"</label>
                                    <div class="control">
                                        <input class="input" type="text" prop:value=c_first on:input=move |ev| set_c_first.set(event_target_value(&ev)) />
                                    </div>
                                </div>
                                <div class="field">
                                    <label class="label">"Nachname / Firmenname*"</label>
                                    <div class="control">
                                        <input class="input" type="text" prop:value=c_name on:input=move |ev| set_c_name.set(event_target_value(&ev)) />
                                    </div>
                                </div>
                            </div>
                            <div class="field">
                                <label class="checkbox">
                                    <input type="checkbox" prop:checked=c_is_person on:change=move |ev| set_c_is_person.set(event_target_checked(&ev)) />
                                    " Privatperson"
                                </label>
                            </div>
                            <div class="field-row">
                                <div class="field is-wide">
                                    <label class="label">"Straße"</label>
                                    <div class="control">
                                        <input class="input" type="text" prop:value=c_street on:input=move |ev| set_c_street.set(event_target_value(&ev)) />
                                    </div>
                                </div>
                                <div class="field is-narrow">
                                    <label class="label">"Hausnummer"</label>
                                    <div class="control">
                                        <input class="input" type="text" prop:value=c_house on:input=move |ev| set_c_house.set(event_target_value(&ev)) />
                                    </div>
                                </div>
                            </div>
                            <div class="field-row">
                                <div class="field is-narrow">
                                    <label class="label">"PLZ"</label>
                                    <div class="control">
                                        <input class="input" type="text" prop:value=c_zip on:input=move |ev| set_c_zip.set(event_target_value(&ev)) />
                                    </div>
                                </div>
                                <div class="field">
                                    <label class="label">"Stadt"</label>
                                    <div class="control">
                                        <input class="input" type="text" prop:value=c_city on:input=move |ev| set_c_city.set(event_target_value(&ev)) />
                                    </div>
                                </div>
                                <div class="field">
                                    <label class="label">"Land"</label>
                                    <div class="control">
                                        <input class="input" type="text" prop:value=c_country on:input=move |ev| set_c_country.set(event_target_value(&ev)) />
                                    </div>
                                </div>
                            </div>
                            <div class="field">
                                <label class="label">"E-Mail-Adressen"</label>
                                {move || c_emails.get().into_iter().enumerate().map(|(index, email)| view! {
                                    <div class="field is-grouped mb-2">
                                        <div class="control is-expanded"><input class="input" type="email" prop:value=email readonly=true /></div>
                                        <div class="control"><button class="button is-danger is-outlined" type="button" title="Adresse entfernen" on:click=move |_| set_c_emails.update(|values| { values.remove(index); })><span class="icon"><i class="mdi mdi-delete"></i></span></button></div>
                                    </div>
                                }).collect_view()}
                                <div class="field is-grouped">
                                    <div class="control is-expanded"><input class="input" type="email" placeholder="kunde@example.org" prop:value=new_email on:input=move |event| set_new_email.set(event_target_value(&event)) /></div>
                                    <div class="control"><button class="button is-light" type="button" on:click=move |_| {
                                        let email = new_email.get().trim().to_string();
                                        if email.contains('@') && !email.contains([' ', '\r', '\n']) {
                                            set_c_emails.update(|values| if !values.contains(&email) { values.push(email); });
                                            set_new_email.set(String::new());
                                        }
                                    }><span class="icon mr-1"><i class="mdi mdi-plus"></i></span>"Hinzufügen"</button></div>
                                </div>
                                <p class="help">"Die erste Adresse wird als bevorzugte Adresse verwendet. Mehrere Adressen sind möglich."</p>
                            </div>
                            <div class="field">
                                <label class="label">"Telefonnummern"</label>
                                {move || c_phones.get().into_iter().enumerate().map(|(index, phone)| view! {
                                    <div class="field is-grouped mb-2">
                                        <div class="control is-expanded"><input class="input" type="text" prop:value=phone readonly=true /></div>
                                        <div class="control"><button class="button is-danger is-outlined" type="button" title="Nummer entfernen" on:click=move |_| set_c_phones.update(|values| { values.remove(index); })><span class="icon"><i class="mdi mdi-delete"></i></span></button></div>
                                    </div>
                                }).collect_view()}
                                <div class="field is-grouped">
                                    <div class="control is-expanded"><input class="input" type="text" placeholder="+49 123 456789" prop:value=new_phone on:input=move |event| set_new_phone.set(event_target_value(&event)) /></div>
                                    <div class="control"><button class="button is-light" type="button" on:click=move |_| {
                                        let phone = new_phone.get().trim().to_string();
                                        if !phone.is_empty() {
                                            set_c_phones.update(|values| if !values.contains(&phone) { values.push(phone); });
                                            set_new_phone.set(String::new());
                                        }
                                    }><span class="icon mr-1"><i class="mdi mdi-plus"></i></span>"Hinzufügen"</button></div>
                                </div>
                                <p class="help">"Mehrere Telefonnummern sind möglich."</p>
                            </div>

                            <div class="field is-grouped mt-5">
                                <div class="control">
                                    <button class="button is-success" on:click=move |_| {
                                        contact.name = c_name.get();
                                        contact.first_name = Some(c_first.get());
                                        contact.title = Some(c_title.get());
                                        contact.form_of_address = Some(c_form_of_address.get());
                                        contact.street = Some(c_street.get());
                                        contact.zip_code = Some(c_zip.get());
                                        contact.city = Some(c_city.get());
                                        contact.house_number = Some(c_house.get());
                                        contact.country = Some(c_country.get());
                                        contact.phones = c_phones.get();
                                        contact.emails = c_emails.get();
                                        contact.is_person = c_is_person.get();
                                        save_contact_act.dispatch(contact.clone());
                                    }>
                                        "Speichern"
                                    </button>
                                </div>
                                {if is_edit {
                                    let archived = contact_archived;
                                    view! {
                                        <div class="control">
                                            <button
                                                class="button"
                                                class:is-warning=!archived
                                                class:is-success=archived
                                                on:click=move |_| {
                                                    if let Some(id) = contact_id {
                                                        archive_contact_act.dispatch((id, archived));
                                                    }
                                                }
                                            >
                                                <span class="icon"><i class=if archived { "mdi mdi-archive-arrow-up-outline" } else { "mdi mdi-archive-arrow-down-outline" }></i></span>
                                                <span>{if archived { "Wiederherstellen" } else { "Archivieren" }}</span>
                                            </button>
                                        </div>
                                    }.into_view()
                                } else {
                                    "".into_view()
                                }}
                                <div class="control">
                                    <button class="button is-light" on:click={
                                        let has_changes = has_unsaved_changes.clone();
                                        move |_| {
                                            let confirm_ok = if has_changes() {
                                                web_sys::window()
                                                    .and_then(|w| w.confirm_with_message("Sie haben ungespeicherte Änderungen. Möchten Sie die Seite wirklich verlassen?").ok())
                                                    .unwrap_or(false)
                                            } else {
                                                true
                                            };
                                            if confirm_ok {
                                                let _ = use_navigate()("/contacts", NavigateOptions::default());
                                            }
                                        }
                                    }>
                                        "Abbrechen"
                                    </button>
                                </div>
                            </div>
                            </>
                        }
                    }};

                    view! {
                        <div class="level mb-4">
                            <div class="level-left">
                                <button class="button is-light" on:click={
                                    let has_changes = has_unsaved_changes.clone();
                                    move |_| {
                                        let confirm_ok = if has_changes() {
                                            web_sys::window()
                                                .and_then(|w| w.confirm_with_message("Sie haben ungespeicherte Änderungen. Möchten Sie die Seite wirklich verlassen?").ok())
                                                .unwrap_or(false)
                                        } else {
                                            true
                                        };
                                        if confirm_ok {
                                            let _ = use_navigate()("/contacts", NavigateOptions::default());
                                        }
                                    }
                                }>
                                    <span class="icon mr-1"><i class="mdi mdi-arrow-left"></i></span>"Zurück zur Kundenliste"
                                </button>
                            </div>
                        </div>

                        {if is_edit {
                            view! {
                                <div class="columns">
                                    // Left Column: CRM stuff
                                    <div class="column is-7">
                                        <div class="box">
                                            <h3 class="title is-5 mb-4">"CRM-Übersicht"</h3>
                                            <Suspense fallback=move || view! { <p class="text-muted">"Lade Aktivitäten…"</p> }>
                                                {move || crm.get().flatten().and_then(Result::ok).map(|summary| {
                                                    let customer_id = summary.contact.id.unwrap_or_default();
                                                    view! {
                                                        <div class="level mb-4">
                                                            <div class="level-left">
                                                                <div class="tags">
                                                                    <a class="tag is-link is-light" href=format!("/offers?customer_id={}", customer_id)>
                                                                        <span class="icon mr-1"><i class="mdi mdi-file-document-outline"></i></span>
                                                                        {format!("{} Angebote", summary.offer_count)}
                                                                    </a>
                                                                    <a class="tag is-link is-light" href=format!("/invoices?customer_id={}", customer_id)>
                                                                        <span class="icon mr-1"><i class="mdi mdi-receipt"></i></span>
                                                                        {format!("{} Rechnungen", summary.invoice_count)}
                                                                    </a>
                                                                    <a class="tag is-link is-light" href=format!("/engagements?customer_id={}", customer_id)>
                                                                        <span class="icon mr-1"><i class="mdi mdi-briefcase-outline"></i></span>
                                                                        {format!("{} Aufträge", summary.engagement_count)}
                                                                    </a>
                                                                    <span class="tag is-light">
                                                                        <span class="icon mr-1"><i class="mdi mdi-email-outline"></i></span>
                                                                        {format!("{} E-Mails", summary.recent_emails.len())}
                                                                    </span>
                                                                </div>
                                                            </div>
                                                        </div>

                                                        <div class="field mb-5">
                                                            <label class="label is-small">"Neue CRM-Notiz hinzufügen"</label>
                                                            <div class="control">
                                                                <textarea class="textarea is-small" prop:value=note_body on:input=move |event| set_note_body.set(event_target_value(&event)) placeholder="z. B. Telefonat bezüglich neuem Projekt geführt..."></textarea>
                                                            </div>
                                                            <button class="button is-link is-small mt-2" prop:disabled=move || add_note_action.pending().get() on:click=move |_| add_note_action.dispatch(note_body.get_untracked())>
                                                                <span class="icon mr-1"><i class="mdi mdi-note-plus-outline"></i></span>
                                                                {"Notiz speichern"}
                                                            </button>
                                                        </div>

                                                        <div class="columns">
                                                            // Left sub-column: Notes & Emails
                                                            <div class="column is-6">
                                                                <h4 class="title is-6 mb-3"><span class="icon mr-1"><i class="mdi mdi-note-text-outline"></i></span>"Verlauf & Notizen"</h4>
                                                                {if summary.notes.is_empty() {
                                                                    view! { <p class="text-muted is-size-7 p-3 has-background-white-bis rounded">"Noch keine Notizen vorhanden."</p> }.into_view()
                                                                } else {
                                                                    view! {
                                                                        <div style="max-height: 300px; overflow-y: auto; padding-right: 0.25rem;">
                                                                            {summary.notes.into_iter().map(|note| view! {
                                                                                <div class="box p-3 mb-2 is-size-7 has-background-white-bis">
                                                                                    <div style="white-space: pre-wrap;">{note.body}</div>
                                                                                    <div class="text-muted mt-1 is-size-7" style="display: flex; justify-content: space-between;">
                                                                                        <span><i class="mdi mdi-account mr-1"></i>{note.author_username}</span>
                                                                                        <span>{note.created_timestamp.format("%d.%m.%Y %H:%M").to_string()}</span>
                                                                                    </div>
                                                                                </div>
                                                                            }).collect_view()}
                                                                        </div>
                                                                    }.into_view()
                                                                }}

                                                                <h4 class="title is-6 mb-3 mt-4"><span class="icon mr-1"><i class="mdi mdi-email-outline"></i></span>"Letzte E-Mails"</h4>
                                                                {if summary.recent_emails.is_empty() {
                                                                    view! { <p class="text-muted is-size-7 p-3 has-background-white-bis rounded">"Keine archivierten E-Mails."</p> }.into_view()
                                                                } else {
                                                                    view! {
                                                                        <div style="max-height: 250px; overflow-y: auto; padding-right: 0.25rem;">
                                                                            {summary.recent_emails.into_iter().map(|mail| view! {
                                                                                <div class="box p-3 mb-2 is-size-7 has-background-white-bis">
                                                                                    <div class="has-text-weight-semibold">{mail.subject}</div>
                                                                                    <div class="text-muted mt-1" style="display: flex; justify-content: space-between;">
                                                                                        <span>"Von: " {mail.sender}</span>
                                                                                    </div>
                                                                                </div>
                                                                            }).collect_view()}
                                                                        </div>
                                                                    }.into_view()
                                                                }}
                                                            </div>

                                                            // Right sub-column: Offers, Invoices, Engagements lists
                                                            <div class="column is-6">
                                                                <div class="is-flex is-justify-content-space-between is-align-items-center mb-3">
                                                                    <h4 class="title is-6 mb-0"><span class="icon mr-1"><i class="mdi mdi-file-document-outline"></i></span>"Angebote"</h4>
                                                                    <a class="is-size-7" href=format!("/offers?customer_id={}", customer_id)>"Alle anzeigen »"</a>
                                                                </div>
                                                                {if summary.offers.is_empty() {
                                                                    view! { <p class="text-muted is-size-7 p-3 has-background-white-bis rounded mb-4">"Keine Angebote für diesen Kunden."</p> }.into_view()
                                                                } else {
                                                                    view! {
                                                                        <div class="mb-4" style="max-height: 200px; overflow-y: auto; padding-right: 0.25rem;">
                                                                            {summary.offers.into_iter().map(|offer| {
                                                                                let date_str = offer.offer_date.map(|d| d.format("%d.%m.%Y").to_string()).unwrap_or_else(|| "—".to_string());
                                                                                let number_str = offer.offer_number.map(|num| format!("#{}", num)).unwrap_or_else(|| "Entwurf".to_string());
                                                                                let title_str = offer.title.clone().unwrap_or_default();
                                                                                let status_cls = if offer.committed { "tag is-success is-light" } else { "tag is-warning is-light" };
                                                                                let status_lbl = if offer.committed { "Finalisiert" } else { "Entwurf" };
                                                                                view! {
                                                                                    <div class="is-flex is-justify-content-space-between is-align-items-center p-2 mb-1 has-background-white-bis rounded is-size-7">
                                                                                        <div style="min-width: 0; flex: 1; padding-right: 0.5rem;">
                                                                                            <div class="has-text-weight-semibold" style="white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">
                                                                                                {number_str} " · " {title_str}
                                                                                            </div>
                                                                                            <div class="text-muted">{date_str}</div>
                                                                                        </div>
                                                                                        <div class="is-flex is-align-items-center">
                                                                                            <span class=format!("{} mr-2", status_cls)>{status_lbl}</span>
                                                                                            <a class="button is-small is-light px-2" href=format!("/offers?offer_id={}", offer.id)>
                                                                                                <span class="icon is-small"><i class="mdi mdi-arrow-right"></i></span>
                                                                                            </a>
                                                                                        </div>
                                                                                    </div>
                                                                                }
                                                                            }).collect_view()}
                                                                        </div>
                                                                    }.into_view()
                                                                }}

                                                                <div class="is-flex is-justify-content-space-between is-align-items-center mb-3">
                                                                    <h4 class="title is-6 mb-0"><span class="icon mr-1"><i class="mdi mdi-receipt"></i></span>"Rechnungen"</h4>
                                                                    <a class="is-size-7" href=format!("/invoices?customer_id={}", customer_id)>"Alle anzeigen »"</a>
                                                                </div>
                                                                {if summary.invoices.is_empty() {
                                                                    view! { <p class="text-muted is-size-7 p-3 has-background-white-bis rounded mb-4">"Keine Rechnungen für diesen Kunden."</p> }.into_view()
                                                                } else {
                                                                    view! {
                                                                        <div class="mb-4" style="max-height: 200px; overflow-y: auto; padding-right: 0.25rem;">
                                                                            {summary.invoices.into_iter().map(|invoice| {
                                                                                let date_str = invoice.invoice_date.map(|d| d.format("%d.%m.%Y").to_string()).unwrap_or_else(|| "—".to_string());
                                                                                let number_str = invoice.invoice_number.map(|num| format!("#{}", num)).unwrap_or_else(|| "Entwurf".to_string());
                                                                                let title_str = invoice.subject.clone().unwrap_or_default();
                                                                                let amount_str = format!("{:.2} €", invoice.total_cents as f64 / 100.0);
                                                                                let paid = invoice.paid_cents >= invoice.total_cents;
                                                                                let status_cls = if invoice.is_canceled { "tag is-danger is-light" } else if !invoice.committed { "tag is-warning is-light" } else if paid { "tag is-success is-light" } else { "tag is-info is-light" };
                                                                                let status_lbl = if invoice.is_canceled { "Storniert" } else if !invoice.committed { "Entwurf" } else if paid { "Bezahlt" } else { "Offen" };
                                                                                view! {
                                                                                    <div class="is-flex is-justify-content-space-between is-align-items-center p-2 mb-1 has-background-white-bis rounded is-size-7">
                                                                                        <div style="min-width: 0; flex: 1; padding-right: 0.5rem;">
                                                                                            <div class="has-text-weight-semibold" style="white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">
                                                                                                {number_str} " · " {title_str}
                                                                                            </div>
                                                                                            <div class="text-muted">{date_str} " · " <strong>{amount_str}</strong></div>
                                                                                        </div>
                                                                                        <div class="is-flex is-align-items-center">
                                                                                            <span class=format!("{} mr-2", status_cls)>{status_lbl}</span>
                                                                                            <a class="button is-small is-light px-2" href=format!("/invoices?invoice_id={}", invoice.id)>
                                                                                                <span class="icon is-small"><i class="mdi mdi-arrow-right"></i></span>
                                                                                            </a>
                                                                                        </div>
                                                                                    </div>
                                                                                }
                                                                            }).collect_view()}
                                                                        </div>
                                                                    }.into_view()
                                                                }}

                                                                <div class="is-flex is-justify-content-space-between is-align-items-center mb-3">
                                                                    <h4 class="title is-6 mb-0"><span class="icon mr-1"><i class="mdi mdi-briefcase-outline"></i></span>"Aufträge"</h4>
                                                                    <a class="is-size-7" href=format!("/engagements?customer_id={}", customer_id)>"Alle anzeigen »"</a>
                                                                </div>
                                                                {if summary.engagements.is_empty() {
                                                                    view! { <p class="text-muted is-size-7 p-3 has-background-white-bis rounded">"Keine Aufträge für diesen Kunden."</p> }.into_view()
                                                                } else {
                                                                    view! {
                                                                        <div style="max-height: 200px; overflow-y: auto; padding-right: 0.25rem;">
                                                                            {summary.engagements.into_iter().map(|engagement| {
                                                                                let date_str = engagement.created_timestamp.format("%d.%m.%Y").to_string();
                                                                                let title_str = engagement.title.clone();
                                                                                let desc_str = engagement.description.clone().unwrap_or_default();
                                                                                view! {
                                                                                    <div class="is-flex is-justify-content-space-between is-align-items-center p-2 mb-1 has-background-white-bis rounded is-size-7">
                                                                                        <div style="min-width: 0; flex: 1; padding-right: 0.5rem;">
                                                                                            <div class="has-text-weight-semibold" style="white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">
                                                                                                {title_str}
                                                                                            </div>
                                                                                            <div class="text-muted" style="white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">{desc_str}</div>
                                                                                            <div class="text-muted is-size-8">{date_str}</div>
                                                                                        </div>
                                                                                        <div class="is-flex is-align-items-center">
                                                                                            <a class="button is-small is-light px-2" href=format!("/engagements?engagement_id={}", engagement.id)>
                                                                                                <span class="icon is-small"><i class="mdi mdi-arrow-right"></i></span>
                                                                                            </a>
                                                                                        </div>
                                                                                    </div>
                                                                                }
                                                                            }).collect_view()}
                                                                        </div>
                                                                    }.into_view()
                                                                }}
                                                            </div>
                                                        </div>
                                                    }
                                                })}
                                                </Suspense>
                                            </div>
                                        </div>

                                    // Right Column: Stammdaten
                                    <div class="column is-5">
                                        <div class="box">
                                            <h3 class="title is-5 mb-4">"Kontakt-Stammdaten"</h3>
                                            {form_fields()}
                                        </div>
                                    </div>
                                </div>
                            }.into_view()
                        } else {
                            view! {
                                <div class="box">
                                    <h2 class="subtitle">"Neuer Kontakt"</h2>
                                    {form_fields()}
                                </div>
                            }.into_view()
                        }}
                    }.into_view()
                }
            }}
        </div>
    }
}
