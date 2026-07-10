use crate::components::EmptyState;
use crate::server::{archive_contact, get_archived_contacts, get_contacts, restore_contact, save_contact};
use leptos::*;
use shared::*;

const CONTACT_PAGE_SIZE: u32 = 50;

#[component]
pub fn ContactsPage() -> impl IntoView {
    let (contacts, set_contacts) = create_signal(Vec::<Contact>::new());
    let (selected_contact, set_selected_contact) = create_signal(Option::<Contact>::None);
    let (search_query, set_search_query) = create_signal(String::new());
    let (has_more_contacts, set_has_more_contacts) = create_signal(false);
    let (list_generation, set_list_generation) = create_signal(0_u64);
    // Archived contacts live in a separate view, not interleaved with active
    // ones: they are out of the pickers, and the list should say so clearly.
    let (show_archive, set_show_archive) = create_signal(false);

    // Each request carries the filter it was made for. This prevents a slow
    // response to an earlier keystroke from replacing a newer search result.
    let load_contacts = create_action(move |(generation, offset, query, archived): &(u64, u32, String, bool)| {
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
    });

    // Save contact action
    let save_contact_act = create_action(move |c: &Contact| {
        let c = c.clone();
        async move {
            match save_contact(c).await {
                Ok(_) => {
                    let generation = list_generation.get_untracked().wrapping_add(1);
                    set_list_generation.set(generation);
                    set_contacts.set(Vec::new());
                    set_has_more_contacts.set(false);
                    load_contacts.dispatch((generation, 0, search_query.get_untracked(), show_archive.get_untracked()));
                    set_selected_contact.set(None);
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
                    load_contacts.dispatch((generation, 0, search_query.get_untracked(), show_archive.get_untracked()));
                    set_selected_contact.set(None);
                }
                Err(e) => logging::log!("Error archiving/restoring contact: {:?}", e),
            }
        }
    });

    // Initial load
    load_contacts.dispatch((0, 0, String::new(), false));

    view! {
        <div class="container">
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
                        set_selected_contact.set(None);
                        load_contacts.dispatch((generation, 0, search_query.get_untracked(), archived));
                    }>
                        <span class="icon"><i class="mdi mdi-archive-outline"></i></span>
                        <span>{move || if show_archive.get() { "Aktive Kontakte" } else { "Archiv" }}</span>
                    </button>
                    <button class="button is-link" on:click=move |_| {
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
                            phone: Some("".to_string()),
                            is_person: true,
                            archived_timestamp: None,
                        }));
                    }>
                        "Neuer Kontakt"
                    </button>
                </div>
            </div>

            <div class="columns is-split">
                // Search & List
                <div class="column is-5">
                    <div class="box">
                        <div class="field">
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
                        <hr/>
                        <div style="max-height: 60vh; overflow-y: auto;">
                            {move || contacts.get().into_iter().map(|contact| {
                                let name = contact.display_name();
                                let address = contact.display_address();
                                let contact_id = contact.id;
                                let click_contact = contact.clone();
                                view! {
                                    <div
                                        class="box list-item p-3 mb-2"
                                        class:is-active=move || selected_contact.get().and_then(|c| c.id) == contact_id && contact_id.is_some()
                                        on:click=move |_| set_selected_contact.set(Some(click_contact.clone()))
                                    >
                                        <div class="has-text-weight-bold">{name}</div>
                                        <div class="is-size-7 text-muted">{address}</div>
                                    </div>
                                }
                            }).collect::<Vec<_>>()}
                            <Show when=move || has_more_contacts.get()>
                                <div class="has-text-centered mt-3">
                                    <button
                                        class="button is-light is-small"
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
                        </div>
                    </div>
                </div>

                // Detail View / Form
                <div class="column">
                    {move || match selected_contact.get() {
                        None => view! {
                            <EmptyState icon="account-outline" text="Wählen Sie einen Kontakt aus oder legen Sie einen neuen an." />
                        }.into_view(),
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
                            let (c_phone, set_c_phone) = create_signal(contact.phone.clone().unwrap_or_default());
                            let (c_is_person, set_c_is_person) = create_signal(contact.is_person);
                            let is_edit = contact.id.is_some();
                            let contact_id = contact.id;
                            let contact_archived = contact.archived_timestamp.is_some();

                            view! {
                                <div class="box">
                                    <h2 class="subtitle">{if is_edit { "Kontakt bearbeiten" } else { "Neuer Kontakt" }}</h2>
                                    // Anrede and Titel are short and optional, so they get their own
                                    // compact row rather than squeezing the two name fields.
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
                                    <div class="field-row">
                                        <div class="field">
                                            <label class="label">"Telefonnummer"</label>
                                            <div class="control">
                                                <input class="input" type="text" prop:value=c_phone on:input=move |ev| set_c_phone.set(event_target_value(&ev)) />
                                            </div>
                                        </div>
                                    </div>
                                    <div class="field">
                                        <label class="checkbox">
                                            <input type="checkbox" prop:checked=c_is_person on:change=move |ev| set_c_is_person.set(event_target_checked(&ev)) />
                                            " Privatperson"
                                        </label>
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
                                                contact.phone = Some(c_phone.get());
                                                contact.is_person = c_is_person.get();
                                                save_contact_act.dispatch(contact.clone());
                                            }>
                                                "Speichern"
                                            </button>
                                        </div>
                                        {if is_edit {
                                            // An archived contact offers restore; an active one
                                            // offers archive. There is no hard delete: the id is
                                            // the Kundennummer on committed invoices.
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
                                            <button class="button is-light" on:click=move |_| set_selected_contact.set(None)>
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
