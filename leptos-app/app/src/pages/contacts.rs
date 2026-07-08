use leptos::*;
use shared::*;
use crate::components::EmptyState;
use crate::server::{get_contacts, save_contact, delete_contact};

#[component]
pub fn ContactsPage() -> impl IntoView {
    let (contacts, set_contacts) = create_signal(Vec::<Contact>::new());
    let (selected_contact, set_selected_contact) = create_signal(Option::<Contact>::None);
    let (search_query, set_search_query) = create_signal(String::new());
    
    // Load contacts action
    let load_contacts = create_action(move |_| async move {
        match get_contacts().await {
            Ok(list) => set_contacts.set(list),
            Err(e) => logging::log!("Error fetching contacts: {:?}", e),
        }
    });

    // Save contact action
    let save_contact_act = create_action(move |c: &Contact| {
        let c = c.clone();
        async move {
            match save_contact(c).await {
                Ok(_) => {
                    load_contacts.dispatch(());
                    set_selected_contact.set(None);
                },
                Err(e) => logging::log!("Error saving contact: {:?}", e),
            }
        }
    });

    // Delete contact action
    let delete_contact_act = create_action(move |id: &i64| {
        let id = *id;
        async move {
            match delete_contact(id).await {
                Ok(_) => {
                    load_contacts.dispatch(());
                    set_selected_contact.set(None);
                },
                Err(e) => logging::log!("Error deleting contact: {:?}", e),
            }
        }
    });

    // Initial load
    load_contacts.dispatch(());

    let filtered_contacts = move || {
        let query = search_query.get().to_lowercase();
        contacts.get().into_iter().filter(|c| {
            c.name.to_lowercase().contains(&query) || 
            c.first_name.as_ref().map_or(false, |f| f.to_lowercase().contains(&query))
        }).collect::<Vec<_>>()
    };

    view! {
        <div class="container">
            <div class="level">
                <div class="level-left">
                    <h1 class="title">"Kunden & Kontakte"</h1>
                </div>
                <div class="level-right">
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
                        }));
                    }>
                        "Neuer Kontakt"
                    </button>
                </div>
            </div>

            <div class="columns">
                // Search & List
                <div class="column is-5">
                    <div class="box">
                        <div class="field">
                            <p class="control has-icons-left">
                                <input class="input" type="text" placeholder="Suchen..."
                                    on:input=move |ev| set_search_query.set(event_target_value(&ev)) />
                                <span class="icon is-small is-left">
                                    <i class="mdi mdi-magnify"></i>
                                </span>
                            </p>
                        </div>
                        <hr/>
                        <div style="max-height: 60vh; overflow-y: auto;">
                            {move || filtered_contacts().into_iter().map(|contact| {
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

                            view! {
                                <div class="box">
                                    <h2 class="subtitle">{if is_edit { "Kontakt bearbeiten" } else { "Neuer Kontakt" }}</h2>
                                    <div class="columns">
                                        <div class="column is-3">
                                            <div class="field">
                                                <label class="label">"Anrede"</label>
                                                <div class="control">
                                                    <input class="input" type="text" placeholder="Herr / Frau etc" prop:value=c_form_of_address on:input=move |ev| set_c_form_of_address.set(event_target_value(&ev)) />
                                                </div>
                                            </div>
                                        </div>
                                        <div class="column is-3">
                                            <div class="field">
                                                <label class="label">"Titel"</label>
                                                <div class="control">
                                                    <input class="input" type="text" placeholder="Dr. / Prof. etc" prop:value=c_title on:input=move |ev| set_c_title.set(event_target_value(&ev)) />
                                                </div>
                                            </div>
                                        </div>
                                        <div class="column is-3">
                                            <div class="field">
                                                <label class="label">"Vorname"</label>
                                                <div class="control">
                                                    <input class="input" type="text" prop:value=c_first on:input=move |ev| set_c_first.set(event_target_value(&ev)) />
                                                </div>
                                            </div>
                                        </div>
                                        <div class="column is-3">
                                            <div class="field">
                                                <label class="label">"Nachname / Firmenname*"</label>
                                                <div class="control">
                                                    <input class="input" type="text" prop:value=c_name on:input=move |ev| set_c_name.set(event_target_value(&ev)) />
                                                </div>
                                            </div>
                                        </div>
                                    </div>
                                    <div class="columns">
                                        <div class="column is-6">
                                            <div class="field">
                                                <label class="label">"Straße"</label>
                                                <div class="control">
                                                    <input class="input" type="text" prop:value=c_street on:input=move |ev| set_c_street.set(event_target_value(&ev)) />
                                                </div>
                                            </div>
                                        </div>
                                        <div class="column is-3">
                                            <div class="field">
                                                <label class="label">"Hausnummer"</label>
                                                <div class="control">
                                                    <input class="input" type="text" prop:value=c_house on:input=move |ev| set_c_house.set(event_target_value(&ev)) />
                                                </div>
                                            </div>
                                        </div>
                                        <div class="column is-3">
                                            <div class="field">
                                                <label class="label">"Land"</label>
                                                <div class="control">
                                                    <input class="input" type="text" prop:value=c_country on:input=move |ev| set_c_country.set(event_target_value(&ev)) />
                                                </div>
                                            </div>
                                        </div>
                                    </div>
                                    <div class="columns">
                                        <div class="column is-3">
                                            <div class="field">
                                                <label class="label">"PLZ"</label>
                                                <div class="control">
                                                    <input class="input" type="text" prop:value=c_zip on:input=move |ev| set_c_zip.set(event_target_value(&ev)) />
                                                </div>
                                            </div>
                                        </div>
                                        <div class="column is-5">
                                            <div class="field">
                                                <label class="label">"Stadt"</label>
                                                <div class="control">
                                                    <input class="input" type="text" prop:value=c_city on:input=move |ev| set_c_city.set(event_target_value(&ev)) />
                                                </div>
                                            </div>
                                        </div>
                                        <div class="column is-4">
                                            <div class="field">
                                                <label class="label">"Telefonnummer"</label>
                                                <div class="control">
                                                    <input class="input" type="text" prop:value=c_phone on:input=move |ev| set_c_phone.set(event_target_value(&ev)) />
                                                </div>
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
                                            view! {
                                                <div class="control">
                                                    <button class="button is-danger" on:click=move |_| {
                                                        if let Some(id) = contact_id {
                                                            delete_contact_act.dispatch(id);
                                                        }
                                                    }>
                                                        "Löschen"
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
