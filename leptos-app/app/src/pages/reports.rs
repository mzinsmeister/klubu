use leptos::*;
use wasm_bindgen::JsCast;
use shared::{ReportInfo, ReportParamInfo};

use crate::components::EmptyState;
use crate::server::{export_report_pdf, list_reports, run_report};

/// One parameter's live input state, carrying enough to render the field and to
/// send `(name, value)` back to the server.
#[derive(Clone)]
struct ParamState {
    info: ReportParamInfo,
    value: RwSignal<String>,
}

#[component]
pub fn ReportsPage() -> impl IntoView {
    let reports = create_resource(|| (), |_| async move { list_reports().await });
    let (selected, set_selected) = create_signal(Option::<ReportInfo>::None);

    // Param inputs for the selected report. Rebuilt whenever the selection
    // changes, seeded from each param's default.
    let (params, set_params) = create_signal(Vec::<ParamState>::new());

    let select_report = move |report: ReportInfo| {
        let states = report
            .params
            .iter()
            .map(|p| ParamState {
                info: p.clone(),
                value: create_rw_signal(p.default.clone().unwrap_or_default()),
            })
            .collect::<Vec<_>>();
        set_params.set(states);
        set_selected.set(Some(report));
    };

    // The rendered report HTML, shown in a sandboxed iframe.
    let (report_html, set_report_html) = create_signal(Option::<String>::None);
    let (error, set_error) = create_signal(Option::<String>::None);

    let current_args = move || {
        params
            .get()
            .iter()
            .map(|p| (p.info.name.clone(), p.value.get()))
            .collect::<Vec<_>>()
    };

    let run = create_action(move |(name, args): &(String, Vec<(String, String)>)| {
        let name = name.clone();
        let args = args.clone();
        async move {
            set_error.set(None);
            match run_report(name, args).await {
                Ok(r) => set_report_html.set(Some(r.html)),
                Err(e) => set_error.set(Some(e.to_string())),
            }
        }
    });

    let export = create_action(move |(name, args): &(String, Vec<(String, String)>)| {
        let name = name.clone();
        let args = args.clone();
        async move {
            set_error.set(None);
            match export_report_pdf(name, args).await {
                Ok(dl) => {
                    // Hand the browser a data: URL and click it, so the PDF lands
                    // as a normal download without a round-trip through a route.
                    let href = format!("data:{};base64,{}", dl.media_type, dl.base64);
                    if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                        if let Ok(a) = doc.create_element("a") {
                            let _ = a.set_attribute("href", &href);
                            let _ = a.set_attribute("download", &dl.filename);
                            if let Some(a) = a.dyn_ref::<web_sys::HtmlElement>() {
                                a.click();
                            }
                        }
                    }
                }
                Err(e) => set_error.set(Some(e.to_string())),
            }
        }
    });

    view! {
        <div class="container">
            <h1 class="title">"Berichte"</h1>

            <div class="columns is-split">
                // Report list
                <div class="column is-4">
                    <div class="box">
                        <Suspense fallback=move || view! { <p class="text-muted">"Lade Berichte…"</p> }>
                            {move || reports.get().map(|res| match res {
                                Err(e) => view! { <p class="text-muted">{format!("Fehler: {e}")}</p> }.into_view(),
                                Ok(list) if list.is_empty() => view! {
                                    <p class="text-muted">"Keine Berichte gefunden."</p>
                                }.into_view(),
                                Ok(list) => list.into_iter().map(|report| {
                                    let r = report.clone();
                                    let name = report.name.clone();
                                    view! {
                                        <div
                                            class="box list-item p-3 mb-2"
                                            class:is-active=move || selected.get().map(|s| s.name) == Some(name.clone())
                                            on:click=move |_| select_report(r.clone())
                                        >
                                            <div class="has-text-weight-bold">{report.title}</div>
                                            {report.description.map(|d| view! {
                                                <div class="is-size-7 text-muted">{d}</div>
                                            })}
                                        </div>
                                    }
                                }).collect_view(),
                            })}
                        </Suspense>
                    </div>
                </div>

                // Parameters + rendered report
                <div class="column">
                    {move || match selected.get() {
                        None => view! {
                            <EmptyState icon="chart-box-outline" text="Wählen Sie einen Bericht aus." />
                        }.into_view(),
                        Some(report) => {
                            let report_name = report.name.clone();
                            let run_name = report_name.clone();
                            let export_name = report_name.clone();
                            view! {
                                <div class="box">
                                    <h2 class="subtitle">{report.title.clone()}</h2>

                                    <div class="field-row">
                                        {move || params.get().into_iter().map(|p| {
                                            let input_type = match p.info.kind.as_str() {
                                                "int" => "number",
                                                "date" => "date",
                                                _ => "text",
                                            };
                                            let value = p.value;
                                            view! {
                                                <div class="field">
                                                    <label class="label">{p.info.label.clone()}</label>
                                                    <div class="control">
                                                        <input class="input" type=input_type
                                                            prop:value=move || value.get()
                                                            on:input=move |ev| value.set(event_target_value(&ev)) />
                                                    </div>
                                                </div>
                                            }
                                        }).collect_view()}
                                    </div>

                                    <div class="field is-grouped mt-4">
                                        <div class="control">
                                            <button class="button is-link"
                                                prop:disabled=move || run.pending().get()
                                                on:click=move |_| run.dispatch((run_name.clone(), current_args()))>
                                                <span class="icon mr-1"><i class="mdi mdi-play"></i></span>
                                                "Anzeigen"
                                            </button>
                                        </div>
                                        <div class="control">
                                            <button class="button"
                                                prop:disabled=move || export.pending().get()
                                                on:click=move |_| export.dispatch((export_name.clone(), current_args()))>
                                                <span class="icon mr-1"><i class="mdi mdi-file-pdf-box"></i></span>
                                                "PDF herunterladen"
                                            </button>
                                        </div>
                                    </div>

                                    {move || error.get().map(|e| view! {
                                        <div class="message is-danger mt-3"><div class="message-body p-2 is-size-7">{e}</div></div>
                                    })}
                                </div>

                                {move || report_html.get().map(|html| view! {
                                    <div class="box mt-4 p-2">
                                        // Sandboxed: the report's own <!DOCTYPE html> document
                                        // renders isolated from the app's styles.
                                        <iframe
                                            sandbox=""
                                            srcdoc=html
                                            style="width: 100%; min-height: 70vh; border: none; background: #fff; border-radius: 8px;"
                                        ></iframe>
                                    </div>
                                })}
                            }.into_view()
                        }
                    }}
                </div>
            </div>
        </div>
    }
}
