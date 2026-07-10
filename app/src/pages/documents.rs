use chrono::{DateTime, NaiveDate, Utc};
use leptos::*;
use leptos_router::{use_navigate, use_params_map, NavigateOptions};
use wasm_bindgen::JsCast;

use crate::server::documents::{
    add_managed_document_version, download_managed_document_version, get_managed_document,
    list_managed_document_versions, list_managed_documents, tombstone_managed_document,
    upload_managed_document, DocumentLinkKind, ManagedDocument, ManagedDocumentDownload,
    ManagedDocumentLink, ManagedDocumentUpload, ManagedDocumentVersion,
};

const DOCUMENT_PAGE_SIZE: u32 = 50;

#[derive(Clone)]
struct DocumentListRequest {
    generation: u64,
    reset: bool,
    offset: u32,
    uploaded_from: Option<NaiveDate>,
    uploaded_to: Option<NaiveDate>,
}

fn inferred_media_type(file: &web_sys::File) -> String {
    let browser_type = file.type_();
    if !browser_type.trim().is_empty() {
        return browser_type.to_ascii_lowercase();
    }
    let extension = file
        .name()
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase());
    match extension.as_deref() {
        Some("pdf") => "application/pdf",
        Some("xml") => "application/xml",
        Some("json") => "application/json",
        Some("csv") => "text/csv",
        Some("txt") => "text/plain",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("docx") => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        Some("xlsx") => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        _ => "application/octet-stream",
    }
    .to_string()
}

fn read_upload(file: web_sys::File, on_load: impl Fn(ManagedDocumentUpload) + 'static) {
    let file_name = file.name();
    let media_type = inferred_media_type(&file);
    let Ok(reader) = web_sys::FileReader::new() else {
        return;
    };
    let reader_for_callback = reader.clone();
    let onload = wasm_bindgen::closure::Closure::wrap(Box::new(move |_event: web_sys::Event| {
        let Ok(result) = reader_for_callback.result() else {
            return;
        };
        let Some(data_url) = result.as_string() else {
            return;
        };
        let Some(comma) = data_url.find(',') else {
            return;
        };
        on_load(ManagedDocumentUpload {
            file_name: file_name.clone(),
            media_type: media_type.clone(),
            base64: data_url[comma + 1..].to_string(),
        });
    }) as Box<dyn FnMut(web_sys::Event)>);
    reader.set_onload(Some(onload.as_ref().unchecked_ref()));
    let _ = reader.read_as_data_url(&file);
    onload.forget();
}

fn pick_file(event: web_sys::Event, on_load: impl Fn(ManagedDocumentUpload) + 'static) {
    let Some(target) = event.target() else {
        return;
    };
    let input = target.unchecked_into::<web_sys::HtmlInputElement>();
    let Some(files) = input.files() else {
        return;
    };
    let Some(file) = files.get(0) else {
        return;
    };
    read_upload(file, on_load);
}

fn offer_download(download: &ManagedDocumentDownload) {
    let href = format!("data:{};base64,{}", download.media_type, download.base64);
    if let Some(document) = web_sys::window().and_then(|window| window.document()) {
        if let Ok(anchor) = document.create_element("a") {
            let _ = anchor.set_attribute("href", &href);
            let _ = anchor.set_attribute("download", &download.filename);
            if let Some(anchor) = anchor.dyn_ref::<web_sys::HtmlElement>() {
                anchor.click();
            }
        }
    }
}

fn formatted_timestamp(timestamp: Option<DateTime<Utc>>) -> String {
    timestamp
        .map(|value| value.format("%d.%m.%Y, %H:%M UTC").to_string())
        .unwrap_or_else(|| "Zeitpunkt nicht überliefert".to_string())
}

fn link_label(link: &ManagedDocumentLink) -> String {
    match link.kind {
        DocumentLinkKind::Invoice => link
            .reference
            .as_deref()
            .map(|number| format!("Rechnung #{number}"))
            .unwrap_or_else(|| format!("Rechnungsentwurf (ID {})", link.entity_id)),
        DocumentLinkKind::Offer => {
            let base = link
                .reference
                .as_deref()
                .map(|number| format!("Angebot #{number}"))
                .unwrap_or_else(|| format!("Angebot (ID {})", link.entity_id));
            link.revision
                .map(|revision| format!("{base}, Revision {revision}"))
                .unwrap_or(base)
        }
        DocumentLinkKind::Receipt => link
            .reference
            .as_deref()
            .filter(|reference| !reference.trim().is_empty())
            .map(|reference| format!("Beleg {reference}"))
            .unwrap_or_else(|| format!("Beleg (ID {})", link.entity_id)),
    }
}

fn link_href(kind: &DocumentLinkKind) -> &'static str {
    match kind {
        DocumentLinkKind::Invoice => "/invoices",
        DocumentLinkKind::Offer => "/offers",
        DocumentLinkKind::Receipt => "/receipts",
    }
}

fn parse_optional_date(value: &str) -> Option<NaiveDate> {
    if value.trim().is_empty() {
        None
    } else {
        NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()
    }
}

#[component]
pub fn DocumentsPage() -> impl IntoView {
    let (documents, set_documents) = create_signal(Vec::<ManagedDocument>::new());
    let (has_more, set_has_more) = create_signal(false);
    let (generation, set_generation) = create_signal(1_u64);
    let (selected_document, set_selected_document) = create_signal(Option::<ManagedDocument>::None);
    let (versions, set_versions) = create_signal(Vec::<ManagedDocumentVersion>::new());
    let (error, set_error) = create_signal(Option::<String>::None);
    let (notice, set_notice) = create_signal(Option::<String>::None);

    let (from_input, set_from_input) = create_signal(String::new());
    let (to_input, set_to_input) = create_signal(String::new());
    let (active_from, set_active_from) = create_signal(Option::<NaiveDate>::None);
    let (active_to, set_active_to) = create_signal(Option::<NaiveDate>::None);

    let (new_document_upload, set_new_document_upload) =
        create_signal(Option::<ManagedDocumentUpload>::None);
    let (new_version_upload, set_new_version_upload) =
        create_signal(Option::<ManagedDocumentUpload>::None);

    let load_versions = create_action(move |document_id: &i64| {
        let document_id = *document_id;
        async move {
            match list_managed_document_versions(document_id).await {
                Ok(items) => {
                    if selected_document
                        .get_untracked()
                        .map(|document| document.id)
                        == Some(document_id)
                    {
                        set_versions.set(items);
                    }
                }
                Err(load_error) => set_error.set(Some(format!(
                    "Versionshistorie konnte nicht geladen werden: {load_error}"
                ))),
            }
        }
    });

    let load_documents = create_action(move |request: &DocumentListRequest| {
        let request = request.clone();
        async move {
            match list_managed_documents(
                request.offset,
                DOCUMENT_PAGE_SIZE,
                request.uploaded_from,
                request.uploaded_to,
            )
            .await
            {
                Ok(page) if generation.get_untracked() == request.generation => {
                    let selected_id = selected_document
                        .get_untracked()
                        .map(|document| document.id);
                    if request.reset {
                        let refreshed_selection = selected_id.and_then(|id| {
                            page.items
                                .iter()
                                .find(|document| document.id == id)
                                .cloned()
                        });
                        set_documents.set(page.items);
                        set_selected_document.set(refreshed_selection);
                    } else {
                        set_documents.update(|items| items.extend(page.items));
                    }
                    set_has_more.set(page.has_more);
                    set_error.set(None);
                }
                Ok(_) => {}
                Err(load_error) if generation.get_untracked() == request.generation => {
                    set_error.set(Some(format!(
                        "Dokumente konnten nicht geladen werden: {load_error}"
                    )));
                }
                Err(_) => {}
            }
        }
    });

    let refresh_list = move || {
        let next_generation = generation.get_untracked().wrapping_add(1);
        set_generation.set(next_generation);
        load_documents.dispatch(DocumentListRequest {
            generation: next_generation,
            reset: true,
            offset: 0,
            uploaded_from: active_from.get_untracked(),
            uploaded_to: active_to.get_untracked(),
        });
    };

    let upload_document_action = create_action(move |upload: &ManagedDocumentUpload| {
        let upload = upload.clone();
        async move {
            set_error.set(None);
            set_notice.set(None);
            match upload_managed_document(upload).await {
                Ok(result) => {
                    set_new_document_upload.set(None);
                    set_notice.set(Some(format!(
                        "Dokument #{} wurde als Version {} archiviert.",
                        result.document_id, result.version
                    )));
                    refresh_list();
                }
                Err(upload_error) => set_error.set(Some(format!(
                    "Dokument konnte nicht hochgeladen werden: {upload_error}"
                ))),
            }
        }
    });

    let upload_version_action = create_action(
        move |(document_id, upload): &(i64, ManagedDocumentUpload)| {
            let document_id = *document_id;
            let upload = upload.clone();
            async move {
                set_error.set(None);
                set_notice.set(None);
                match add_managed_document_version(document_id, upload).await {
                    Ok(result) => {
                        set_new_version_upload.set(None);
                        set_notice.set(Some(format!(
                            "Version {} wurde dem Dokument #{} hinzugefügt.",
                            result.version, result.document_id
                        )));
                        load_versions.dispatch(document_id);
                        refresh_list();
                    }
                    Err(upload_error) => set_error.set(Some(format!(
                        "Version konnte nicht hochgeladen werden: {upload_error}"
                    ))),
                }
            }
        },
    );

    let tombstone_action = create_action(move |document_id: &i64| {
        let document_id = *document_id;
        async move {
            set_error.set(None);
            set_notice.set(None);
            match tombstone_managed_document(document_id).await {
                Ok(result) => {
                    set_notice.set(Some(format!(
                        "Dokument #{} wurde mit Version {} als gelöscht markiert. Alle früheren Versionen bleiben erhalten.",
                        result.document_id, result.version
                    )));
                    load_versions.dispatch(document_id);
                    refresh_list();
                }
                Err(delete_error) => set_error.set(Some(format!(
                    "Dokument konnte nicht gelöscht werden: {delete_error}"
                ))),
            }
        }
    });

    let download_action = create_action(move |(document_id, version): &(i64, i32)| {
        let (document_id, version) = (*document_id, *version);
        async move {
            set_error.set(None);
            match download_managed_document_version(document_id, version).await {
                Ok(download) => offer_download(&download),
                Err(download_error) => set_error.set(Some(format!(
                    "Dokumentversion konnte nicht geladen werden: {download_error}"
                ))),
            }
        }
    });

    load_documents.dispatch(DocumentListRequest {
        generation: generation.get_untracked(),
        reset: true,
        offset: 0,
        uploaded_from: None,
        uploaded_to: None,
    });

    let apply_filters = move |_| {
        let from = parse_optional_date(&from_input.get_untracked());
        let to = parse_optional_date(&to_input.get_untracked());
        if matches!((from, to), (Some(from), Some(to)) if from > to) {
            set_error.set(Some(
                "Das Upload-Datum 'von' darf nicht nach 'bis' liegen.".to_string(),
            ));
            return;
        }
        set_active_from.set(from);
        set_active_to.set(to);
        set_selected_document.set(None);
        set_versions.set(Vec::new());
        set_documents.set(Vec::new());
        let next_generation = generation.get_untracked().wrapping_add(1);
        set_generation.set(next_generation);
        load_documents.dispatch(DocumentListRequest {
            generation: next_generation,
            reset: true,
            offset: 0,
            uploaded_from: from,
            uploaded_to: to,
        });
    };
    let params = use_params_map();
    let id_param = move || params.with(|p| p.get("id").cloned());

    create_effect(move |_| {
        let id_val = id_param();
        match id_val.as_deref() {
            None => {
                set_selected_document.set(None);
                set_versions.set(Vec::new());
            }
            Some(id_str) => {
                if let Ok(id) = id_str.parse::<i64>() {
                    let already_selected =
                        selected_document.get_untracked().map(|doc| doc.id) == Some(id);
                    if !already_selected {
                        spawn_local(async move {
                            match get_managed_document(id).await {
                                Ok(doc) => {
                                    set_selected_document.set(Some(doc));
                                    set_versions.set(Vec::new());
                                    load_versions.dispatch(id);
                                }
                                Err(e) => {
                                    set_error.set(Some(format!(
                                        "Dokument konnte nicht geladen werden: {e}"
                                    )));
                                }
                            }
                        });
                    }
                }
            }
        }
    });

    view! {
        <div class="container dms-page">
            <div class="dms-header">
                <div>
                    <p class="dms-eyebrow">"DOKUMENTENMANAGEMENT"</p>
                    <h1 class="title">"Dokumente"</h1>
                    <p class="dms-description">
                        "Versionssicheres Archiv für Geschäftsbelege und beliebige weitere Dateien"
                    </p>
                </div>
                <div class="dms-header-status"><span class="dms-status-dot"></span>"Archiv aktiv"</div>
            </div>

            <Show when=move || error.get().is_some()>
                <div class="notification is-danger is-light">
                    <button class="delete" on:click=move |_| set_error.set(None)></button>
                    {move || error.get().unwrap_or_default()}
                </div>
            </Show>
            <Show when=move || notice.get().is_some()>
                <div class="notification is-success is-light">
                    <button class="delete" on:click=move |_| set_notice.set(None)></button>
                    {move || notice.get().unwrap_or_default()}
                </div>
            </Show>

            <div class="dms-workspace-grid">
                // Left Column: Upload, Search, and Document List
                <div class="dms-sidebar">
                    // Upload Card
                    <div class="dms-upload-card mb-4">
                        <p class="dms-box-title">"Neues Dokument"</p>
                        <div class="dms-upload-dropzone">
                            <label class="dms-upload-label">
                                <input
                                    class="file-input"
                                    type="file"
                                    on:change=move |event| {
                                        pick_file(event, move |upload| {
                                            set_new_document_upload.set(Some(upload));
                                        });
                                    }
                                />
                                <span class="dms-upload-icon"><i class="mdi mdi-cloud-upload-outline"></i></span>
                                <span class="dms-upload-text">"Datei wählen oder ablegen"</span>
                                <span class="dms-upload-subtext">"Maximal 50 MB"</span>
                            </label>
                        </div>
                        {move || new_document_upload.get().map(|upload| view! {
                            <div class="dms-selected-file mt-3">
                                <div class="file-info-row">
                                    <span class="icon text-link"><i class="mdi mdi-file-document-outline"></i></span>
                                    <span class="file-name-text">{upload.file_name.clone()}</span>
                                </div>
                                <div class="buttons is-centered mt-3">
                                    <button
                                        class="button is-link is-small"
                                        prop:disabled=move || upload_document_action.pending().get()
                                        on:click=move |_| {
                                            if let Some(upload) = new_document_upload.get_untracked() {
                                                upload_document_action.dispatch(upload);
                                            }
                                        }
                                    >
                                        <span class="icon mr-1"><i class="mdi mdi-archive-arrow-up"></i></span>
                                        {move || if upload_document_action.pending().get() { "Archiviert…" } else { "Archivieren" }}
                                    </button>
                                    <button class="button is-light is-small" on:click=move |_| set_new_document_upload.set(None)>
                                        "Abbrechen"
                                    </button>
                                </div>
                            </div>
                        })}
                    </div>

                    // Search & Filters Card
                    <div class="dms-filter-card mb-4">
                        <p class="dms-box-title">"Zeitraum filtern"</p>
                        <div class="field">
                            <label class="label is-small text-muted">"Upload von"</label>
                            <div class="control has-icons-left">
                                <input
                                    class="input is-small"
                                    type="date"
                                    prop:value=from_input
                                    on:input=move |event| set_from_input.set(event_target_value(&event))
                                />
                                <span class="icon is-small is-left"><i class="mdi mdi-calendar"></i></span>
                            </div>
                        </div>
                        <div class="field mt-2">
                            <label class="label is-small text-muted">"Upload bis"</label>
                            <div class="control has-icons-left">
                                <input
                                    class="input is-small"
                                    type="date"
                                    prop:value=to_input
                                    on:input=move |event| set_to_input.set(event_target_value(&event))
                                />
                                <span class="icon is-small is-left"><i class="mdi mdi-calendar"></i></span>
                            </div>
                        </div>
                        <button class="button is-light is-small is-fullwidth mt-3" prop:disabled=load_documents.pending() on:click=apply_filters>
                            <span class="icon mr-1"><i class="mdi mdi-filter"></i></span>
                            "Filter anwenden"
                        </button>
                    </div>

                    // Document List Card
                    <div class="dms-list-card">
                        <div class="is-flex is-justify-content-space-between is-align-items-center mb-3">
                            <h2 class="subtitle is-6 mb-0 has-text-weight-bold">"Archiv"</h2>
                            <span class="tag is-link is-light">{move || format!("{} geladen", documents.get().len())}</span>
                        </div>
                        <Show
                            when=move || !documents.get().is_empty()
                            fallback=move || view! {
                                <div class="has-text-centered text-muted p-5">
                                    <span class="icon is-large"><i class="mdi mdi-archive-outline mdi-36px"></i></span>
                                    <p>{move || if load_documents.pending().get() { "Dokumente werden geladen…" } else { "Keine Dokumente im gewählten Zeitraum." }}</p>
                                </div>
                            }
                        >
                            <div class="dms-list-scroll">
                                <For
                                    each=move || documents.get()
                                    key=|document| document.id
                                    let:document
                                >
                                    {
                                        let document_id = document.id;
                                        let upload_time = formatted_timestamp(document.latest_uploaded_timestamp);
                                        let status_class = if document.is_deleted { "is-deleted" } else { "" };
                                        view! {
                                            <div
                                                class=format!("dms-document-item {} mb-2", status_class)
                                                class:is-active=move || selected_document.get().map(|selected| selected.id) == Some(document_id)
                                                on:click=move |_| {
                                                    let target = format!("/documents/{}", document_id);
                                                    let _ = use_navigate()(&target, NavigateOptions::default());
                                                }
                                            >
                                                <div class="is-flex is-justify-content-space-between is-align-items-flex-start">
                                                    <div style="min-width: 0; flex: 1;">
                                                        <div class="document-title" title=document.display_name.clone()>
                                                            <span class="icon mr-1"><i class="mdi mdi-file-document-outline"></i></span>
                                                            {document.display_name.clone()}
                                                        </div>
                                                        <div class="is-size-7 text-muted mt-1">{upload_time}</div>
                                                    </div>
                                                    {document.is_deleted.then(|| view! {
                                                        <span class="tag is-danger is-light is-small ml-2">"Gelöscht"</span>
                                                    })}
                                                </div>
                                                <div class="tags mt-2 mb-0">
                                                    <span class="tag is-light is-small">{format!("{} Version(en)", document.version_count)}</span>
                                                    {if document.links.is_empty() {
                                                        view! { <span class="tag is-info is-light is-small">"Eigenständig"</span> }.into_view()
                                                    } else {
                                                        view! { <span class="tag is-link is-light is-small">"Verknüpft"</span> }.into_view()
                                                    }}
                                                    {document.is_write_protected().then(|| view! {
                                                        <span class="tag is-warning is-light is-small" title="Schreibgeschützt">
                                                            <span class="icon is-small"><i class="mdi mdi-lock"></i></span>
                                                        </span>
                                                    })}
                                                </div>
                                            </div>
                                        }
                                    }
                                </For>
                                <Show when=move || has_more.get()>
                                    <div class="has-text-centered mt-4">
                                        <button
                                            class="button is-light is-small"
                                            prop:disabled=load_documents.pending()
                                            on:click=move |_| {
                                                load_documents.dispatch(DocumentListRequest {
                                                    generation: generation.get_untracked(),
                                                    reset: false,
                                                    offset: documents.get_untracked().len() as u32,
                                                    uploaded_from: active_from.get_untracked(),
                                                    uploaded_to: active_to.get_untracked(),
                                                });
                                            }
                                        >
                                            {move || if load_documents.pending().get() { "Lädt…" } else { "Mehr laden" }}
                                        </button>
                                    </div>
                                </Show>
                            </div>
                        </Show>
                    </div>
                </div>

                // Right Column: Details & Version Timeline
                <div class="dms-main">
                    {move || match selected_document.get() {
                        None => view! {
                            <div class="dms-empty-state-card">
                                <span class="dms-empty-icon"><i class="mdi mdi-file-tree-outline"></i></span>
                                <h3>"Wählen Sie ein Dokument aus"</h3>
                                <p>"Klicken Sie auf ein Dokument in der Liste, um dessen Details, verknüpfte Geschäftsvorfälle und die gesamte Versionshistorie anzuzeigen."</p>
                            </div>
                        }.into_view(),
                        Some(document) => {
                            let document_id = document.id;
                            let protected = document.is_write_protected();
                            let deleted = document.is_deleted;
                            let links = document.links.clone();
                            view! {
                                <div class="dms-details-panel">
                                    <div class="dms-details-header">
                                        <div class="details-title-area">
                                            <span class="details-file-icon"><i class="mdi mdi-file-document-outline"></i></span>
                                            <div>
                                                <h2 class="title is-5 mb-1" style="overflow-wrap: anywhere;">{document.display_name.clone()}</h2>
                                                <p class="is-size-7 text-muted">{format!("Dokument #{} • {}", document.id, document.media_type)}</p>
                                            </div>
                                        </div>
                                        <button class="delete is-medium" title="Details schließen" on:click=move |_| {
                                            let _ = use_navigate()("/documents", NavigateOptions::default());
                                        }></button>
                                    </div>

                                    {(!links.is_empty()).then(|| view! {
                                        <div class="dms-links-box mb-4">
                                            <p class="dms-box-title">"Verknüpfte Geschäftsvorfälle"</p>
                                            <div class="tags">
                                                {links.iter().map(|link| {
                                                    let href = link_href(&link.kind);
                                                    let label = link_label(link);
                                                    view! {
                                                        <a class="tag is-link is-light" href=href>
                                                            <span class="icon is-small mr-1"><i class="mdi mdi-link-variant"></i></span>
                                                            {label}
                                                        </a>
                                                    }
                                                }).collect::<Vec<_>>()}
                                            </div>
                                        </div>
                                    })}

                                    {if protected {
                                        view! {
                                            <div class="notification is-warning is-light py-2 px-3 mb-4 is-size-7">
                                                <span class="icon mr-1"><i class="mdi mdi-lock"></i></span>
                                                "Dieses Dokument gehört zu einem festgeschriebenen Geschäftsvorfall. Historie und Downloads bleiben verfügbar; neue Versionen und Löschmarken sind gesperrt."
                                            </div>
                                        }.into_view()
                                    } else {
                                        view! {
                                            <div class="dms-action-box mb-4">
                                                <p class="dms-box-title">{if deleted { "Dokument wiederherstellen (neue Version)" } else { "Neue Version hochladen" }}</p>
                                                <div class="field is-grouped">
                                                    <div class="control is-expanded">
                                                        <div class="file is-small has-name is-fullwidth">
                                                            <label class="file-label">
                                                                <input
                                                                    class="file-input"
                                                                    type="file"
                                                                    on:change=move |event| {
                                                                        pick_file(event, move |upload| set_new_version_upload.set(Some(upload)));
                                                                    }
                                                                />
                                                                <span class="file-cta">
                                                                    <span class="file-icon"><i class="mdi mdi-upload"></i></span>
                                                                    <span class="file-label">"Datei wählen…"</span>
                                                                </span>
                                                                <span class="file-name">
                                                                    {move || new_version_upload.get()
                                                                        .map(|upload| upload.file_name)
                                                                        .unwrap_or_else(|| "Keine Datei gewählt".to_string())}
                                                                </span>
                                                            </label>
                                                        </div>
                                                    </div>
                                                    <div class="control">
                                                        <button
                                                            class="button is-link is-small"
                                                            prop:disabled=move || new_version_upload.get().is_none() || upload_version_action.pending().get()
                                                            on:click=move |_| {
                                                                if let Some(upload) = new_version_upload.get_untracked() {
                                                                    upload_version_action.dispatch((document_id, upload));
                                                                }
                                                            }
                                                        >
                                                            {move || if upload_version_action.pending().get() { "Speichert…" } else { "Speichern" }}
                                                        </button>
                                                    </div>
                                                </div>
                                                <p class="help">
                                                    {format!("Die Dateiendung .{} und der Dateityp müssen übereinstimmen.", document.extension)}
                                                </p>
                                            </div>
                                        }.into_view()
                                    }}

                                    <div class="dms-history-section">
                                        <h3 class="subtitle is-6 mb-3 has-text-weight-bold">"Versionshistorie"</h3>
                                        <Show
                                            when=move || !versions.get().is_empty()
                                            fallback=move || view! {
                                                <p class="text-muted is-size-7">{move || if load_versions.pending().get() { "Historie wird geladen…" } else { "Keine Versionen vorhanden." }}</p>
                                            }
                                        >
                                            <div class="dms-timeline">
                                                <For
                                                    each=move || versions.get()
                                                    key=|version| (version.document_id, version.version)
                                                    let:version
                                                >
                                                    {
                                                        let version_number = version.version;
                                                        let checksum = version.checksum_sha256.clone();
                                                        let short_checksum = checksum.as_deref()
                                                            .map(|value| format!("{}…", &value[..value.len().min(12)]))
                                                            .unwrap_or_else(|| "—".to_string());
                                                        view! {
                                                            <div class="dms-timeline-item">
                                                                <div class="dms-timeline-marker">
                                                                    <span class="marker-dot"></span>
                                                                </div>
                                                                <div class="dms-timeline-content">
                                                                    <div class="is-flex is-justify-content-space-between is-align-items-center">
                                                                        <div>
                                                                            <span class="has-text-weight-semibold">{format!("Version {}", version_number)}</span>
                                                                            {version.is_tombstone.then(|| view! {
                                                                                <span class="tag is-danger is-light is-small ml-2">"Löschmarke"</span>
                                                                            })}
                                                                        </div>
                                                                        <div class="buttons">
                                                                            {if version.is_tombstone {
                                                                                view! { <span class="text-muted is-size-7">"gelöscht"</span> }.into_view()
                                                                            } else {
                                                                                view! {
                                                                                    <button
                                                                                        class="button is-small is-light"
                                                                                        prop:disabled=download_action.pending()
                                                                                        on:click=move |_| download_action.dispatch((document_id, version_number))
                                                                                    >
                                                                                        <span class="icon"><i class="mdi mdi-download"></i></span>
                                                                                        <span>"Download"</span>
                                                                                    </button>
                                                                                }.into_view()
                                                                            }}
                                                                        </div>
                                                                    </div>
                                                                    <div class="is-size-7 text-muted mt-1">
                                                                        <span>{formatted_timestamp(version.created_timestamp)}</span>
                                                                        <span class="mx-1">"•"</span>
                                                                        <code title=checksum.unwrap_or_default()>{short_checksum}</code>
                                                                    </div>
                                                                </div>
                                                            </div>
                                                        }
                                                    }
                                                </For>
                                            </div>
                                        </Show>
                                    </div>

                                    {(!protected && !deleted).then(|| view! {
                                        <div class="mt-5 border-top pt-4">
                                            <button
                                                class="button is-danger is-outlined is-small is-fullwidth"
                                                prop:disabled=tombstone_action.pending()
                                                on:click=move |_| {
                                                    let confirmed = web_sys::window()
                                                        .and_then(|window| window.confirm_with_message(
                                                            "Dokument als gelöscht markieren? Die Dateien und die vollständige Versionshistorie bleiben unveränderbar erhalten."
                                                        ).ok())
                                                        .unwrap_or(false);
                                                    if confirmed {
                                                        tombstone_action.dispatch(document_id);
                                                    }
                                                }
                                            >
                                                <span class="icon mr-1"><i class="mdi mdi-delete-outline"></i></span>
                                                "Dokument löschen (Löschmarke anlegen)"
                                            </button>
                                        </div>
                                    })}
                                </div>
                            }.into_view()
                        }
                    }}
                </div>
            </div>
        </div>
    }
}
