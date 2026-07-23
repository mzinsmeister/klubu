//! Chat assistant page.
//!
//! Renders the transcript of a chat run: assistant Markdown (sanitized to
//! HTML, with clickable links into the app such as `/invoices/42`), inline
//! SVG charts from ```chart fenced blocks, tool activity, and confirmation
//! cards for irreversible actions. The page polls the server for run events;
//! the durable history the model sees is only the user/assistant text.

use crate::server::{get_chat_status, poll_chat_run, resolve_chat_confirmation, start_chat_run};
use leptos::*;
use pulldown_cmark::{html as md_html, Event, Options, Parser, Tag, TagEnd};
use shared::{ChatEvent, ChatHistoryMessage};

const POLL_INTERVAL_MS: u64 = 700;

// ------------------------------------------------------------------ markdown

/// Only these link targets survive: in-app routes, web links, and mail.
/// Everything else (`javascript:`, `data:`, …) is neutralized.
fn safe_href(dest: &str) -> bool {
    dest.starts_with('/')
        || dest.starts_with("https://")
        || dest.starts_with("http://")
        || dest.starts_with("mailto:")
}

/// What Markdown image syntax turns into. The model chooses deliberately:
/// a plain link stays a link, image syntax on an app URL means "the user
/// should see this file right here".
fn image_replacement(dest: &str, alt: &str) -> Event<'static> {
    let label = if alt.trim().is_empty() { "Dokument" } else { alt };
    let label = escape_xml(label);
    if dest.starts_with('/') {
        // Same-origin app resource (e.g. /api/documents/9, /api/pdf/invoice/42):
        // embed a collapsible preview. The iframe is same-origin on purpose —
        // the session cookie must accompany the request — which is why the
        // server refuses to render scriptable upload types inline.
        let url = escape_xml(dest);
        Event::Html(
            format!(
                "<details class=\"chat-preview\" open>\
                 <summary>{label} <a href=\"{url}\" target=\"_blank\" rel=\"noopener\">öffnen</a></summary>\
                 <iframe src=\"{url}\" loading=\"lazy\"></iframe>\
                 </details>"
            )
            .into(),
        )
    } else if safe_href(dest) {
        // External images never load inline; the chat fetches nothing from
        // third parties on its own.
        Event::Html(format!("<a href=\"{url}\">{label}</a>", url = escape_xml(dest)).into())
    } else {
        Event::Text(label.into())
    }
}

/// Markdown → HTML for `inner_html`. Raw HTML from the model is rendered as
/// visible text, never executed; unsafe link targets are stripped; image
/// syntax becomes an inline preview (app URLs) or a plain link (external).
/// What remains is exclusively markup this function emitted.
fn render_markdown(md: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);

    let mut events: Vec<Event> = Vec::new();
    // Between Start(Image) and End(Image) the parser emits the alt text as
    // ordinary events; collect it as the preview label.
    let mut image: Option<(String, String)> = None;
    for event in Parser::new_ext(md, options) {
        if let Some((_, alt)) = &mut image {
            match event {
                Event::Text(text) | Event::Code(text) => alt.push_str(&text),
                Event::End(TagEnd::Image) => {
                    let (dest, alt) = image.take().expect("image state just matched");
                    events.push(image_replacement(&dest, &alt));
                }
                _ => {} // markup nested in alt text carries no meaning here
            }
            continue;
        }
        match event {
            Event::Html(raw) | Event::InlineHtml(raw) => events.push(Event::Text(raw)),
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                title,
                id,
            }) => events.push(Event::Start(Tag::Link {
                link_type,
                dest_url: if safe_href(&dest_url) {
                    dest_url
                } else {
                    "".into()
                },
                title,
                id,
            })),
            Event::Start(Tag::Image { dest_url, .. }) => {
                image = Some((dest_url.to_string(), String::new()));
            }
            event => events.push(event),
        }
    }

    let mut out = String::new();
    md_html::push_html(&mut out, events.into_iter());
    out
}

// -------------------------------------------------------------------- charts

/// Categorical palette for the dark app surface. Validated (dataviz six
/// checks) against the chat surface #201a36: lightness band, chroma floor,
/// adjacent CVD ΔE ≥ 8.4, normal-vision ΔE ≥ 19.3, contrast ≥ 3:1.
const SERIES_COLORS: [&str; 8] = [
    "#3987e5", "#008300", "#d55181", "#c98500", "#199e70", "#d95926", "#9085e9", "#e66767",
];
const CHART_SURFACE: &str = "#201a36";
const CHART_W: f64 = 640.0;
const CHART_H: f64 = 300.0;
const MARGIN_L: f64 = 64.0;
const MARGIN_R: f64 = 16.0;
const MARGIN_T: f64 = 16.0;
const MARGIN_B: f64 = 44.0;

#[derive(Clone, PartialEq, serde::Deserialize)]
struct ChartSeries {
    #[serde(default)]
    name: String,
    #[serde(default)]
    values: Vec<f64>,
}

#[derive(Clone, PartialEq, serde::Deserialize)]
struct ChartSpec {
    #[serde(rename = "type")]
    chart_type: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default)]
    series: Vec<ChartSeries>,
}

fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// 1.234,56 — German number formatting for axis and legend values.
fn format_number(value: f64) -> String {
    let rounded = (value * 100.0).round() / 100.0;
    let sign = if rounded < 0.0 { "-" } else { "" };
    let abs = rounded.abs();
    let whole = abs.trunc() as i64;
    let cents = ((abs - abs.trunc()) * 100.0).round() as i64;

    let digits = whole.to_string();
    let mut grouped = String::new();
    for (index, ch) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            grouped.push('.');
        }
        grouped.push(ch);
    }
    if cents == 0 {
        format!("{sign}{grouped}")
    } else {
        format!("{sign}{grouped},{cents:02}")
    }
}

/// A rounded step (1/2/5 × 10ⁿ) so the y axis gets human ticks.
fn nice_step(range: f64) -> f64 {
    let raw = (range / 4.0).max(f64::MIN_POSITIVE);
    let magnitude = 10f64.powf(raw.log10().floor());
    let normalized = raw / magnitude;
    let factor = if normalized <= 1.0 {
        1.0
    } else if normalized <= 2.0 {
        2.0
    } else if normalized <= 5.0 {
        5.0
    } else {
        10.0
    };
    factor * magnitude
}

fn truncate_label(label: &str, max: usize) -> String {
    if label.chars().count() <= max {
        label.to_string()
    } else {
        let cut: String = label.chars().take(max.saturating_sub(1)).collect();
        format!("{cut}…")
    }
}

struct BuiltChart {
    svg: String,
    /// (color, label) pairs; shown when identity needs more than the title.
    legend: Vec<(String, String)>,
}

fn y_axis(svg: &mut String, min: f64, max: f64, plot_w: f64) -> impl Fn(f64) -> f64 {
    let step = nice_step(max - min);
    let mut tick = (min / step).floor() * step;
    let scale = move |value: f64| {
        MARGIN_T + (max - value) / (max - min) * (CHART_H - MARGIN_T - MARGIN_B)
    };
    while tick <= max + step * 0.001 {
        let y = scale(tick);
        if y >= MARGIN_T - 1.0 && y <= CHART_H - MARGIN_B + 1.0 {
            svg.push_str(&format!(
                "<line x1='{MARGIN_L}' y1='{y:.1}' x2='{x2:.1}' y2='{y:.1}' stroke='rgba(255,255,255,0.08)' stroke-width='1'/>\
                 <text x='{tx:.1}' y='{ty:.1}' fill='#9aa4bb' font-size='11' text-anchor='end'>{label}</text>",
                x2 = MARGIN_L + plot_w,
                tx = MARGIN_L - 8.0,
                ty = y + 4.0,
                label = escape_xml(&format_number(tick)),
            ));
        }
        tick += step;
    }
    scale
}

fn x_labels(svg: &mut String, labels: &[String], position: impl Fn(usize) -> f64) {
    let stride = (labels.len() / 12).max(1);
    for (index, label) in labels.iter().enumerate() {
        if index % stride != 0 {
            continue;
        }
        svg.push_str(&format!(
            "<text x='{x:.1}' y='{y:.1}' fill='#9aa4bb' font-size='11' text-anchor='middle'>{text}</text>",
            x = position(index),
            y = CHART_H - MARGIN_B + 18.0,
            text = escape_xml(&truncate_label(label, 12)),
        ));
    }
}

fn build_chart(spec: &ChartSpec) -> Result<BuiltChart, String> {
    let series: Vec<&ChartSeries> = spec
        .series
        .iter()
        .filter(|series| !series.values.is_empty())
        .take(SERIES_COLORS.len())
        .collect();
    if series.is_empty() {
        return Err("Diagramm ohne Datenreihen".to_string());
    }
    let points = series.iter().map(|s| s.values.len()).max().unwrap_or(0);
    let labels: Vec<String> = (0..points)
        .map(|index| {
            spec.labels
                .get(index)
                .cloned()
                .unwrap_or_else(|| (index + 1).to_string())
        })
        .collect();
    let series_name = |index: usize, series: &ChartSeries| {
        if series.name.trim().is_empty() {
            format!("Reihe {}", index + 1)
        } else {
            series.name.clone()
        }
    };

    match spec.chart_type.as_str() {
        "pie" => {
            let values: Vec<(String, f64)> = labels
                .iter()
                .cloned()
                .zip(series[0].values.iter().copied())
                .filter(|(_, value)| *value > 0.0)
                .take(SERIES_COLORS.len())
                .collect();
            let total: f64 = values.iter().map(|(_, value)| value).sum();
            if total <= 0.0 {
                return Err("Kreisdiagramm ohne positive Werte".to_string());
            }
            let (cx, cy) = (CHART_W / 2.0, CHART_H / 2.0);
            let outer = (CHART_H / 2.0) - 24.0;
            let inner = outer * 0.55;
            let mut svg = format!(
                "<svg viewBox='0 0 {CHART_W} {CHART_H}' role='img' xmlns='http://www.w3.org/2000/svg'>"
            );
            let mut angle = -std::f64::consts::FRAC_PI_2;
            let mut legend = Vec::new();
            for (index, (label, value)) in values.iter().enumerate() {
                let share = value / total;
                let sweep = share * std::f64::consts::TAU;
                let end = angle + sweep;
                let large = i32::from(sweep > std::f64::consts::PI);
                let (x0, y0) = (cx + outer * angle.cos(), cy + outer * angle.sin());
                let (x1, y1) = (cx + outer * end.cos(), cy + outer * end.sin());
                let (x2, y2) = (cx + inner * end.cos(), cy + inner * end.sin());
                let (x3, y3) = (cx + inner * angle.cos(), cy + inner * angle.sin());
                let color = SERIES_COLORS[index];
                svg.push_str(&format!(
                    "<path d='M {x0:.1} {y0:.1} A {outer:.1} {outer:.1} 0 {large} 1 {x1:.1} {y1:.1} \
                     L {x2:.1} {y2:.1} A {inner:.1} {inner:.1} 0 {large} 0 {x3:.1} {y3:.1} Z' \
                     fill='{color}' stroke='{CHART_SURFACE}' stroke-width='2'>\
                     <title>{title}: {value} ({percent}%)</title></path>",
                    title = escape_xml(label),
                    value = escape_xml(&format_number(*value)),
                    percent = (share * 100.0).round(),
                ));
                if share >= 0.08 {
                    let mid = angle + sweep / 2.0;
                    let radius = (outer + inner) / 2.0;
                    svg.push_str(&format!(
                        "<text x='{x:.1}' y='{y:.1}' fill='#ffffff' font-size='11' text-anchor='middle'>{p}%</text>",
                        x = cx + radius * mid.cos(),
                        y = cy + radius * mid.sin() + 4.0,
                        p = (share * 100.0).round(),
                    ));
                }
                legend.push((
                    color.to_string(),
                    format!("{label} · {}", format_number(*value)),
                ));
                angle = end;
            }
            svg.push_str("</svg>");
            Ok(BuiltChart { svg, legend })
        }
        "line" => {
            let all: Vec<f64> = series.iter().flat_map(|s| s.values.iter().copied()).collect();
            let mut min = all.iter().copied().fold(f64::INFINITY, f64::min).min(0.0);
            let mut max = all.iter().copied().fold(f64::NEG_INFINITY, f64::max).max(0.0);
            if max - min < f64::EPSILON {
                max += 1.0;
                min -= 1.0;
            }
            let plot_w = CHART_W - MARGIN_L - MARGIN_R;
            let mut svg = format!(
                "<svg viewBox='0 0 {CHART_W} {CHART_H}' role='img' xmlns='http://www.w3.org/2000/svg'>"
            );
            let scale_y = y_axis(&mut svg, min, max, plot_w);
            let position = |index: usize| {
                MARGIN_L
                    + if points <= 1 {
                        plot_w / 2.0
                    } else {
                        plot_w * index as f64 / (points - 1) as f64
                    }
            };
            x_labels(&mut svg, &labels, position);
            for (series_index, series) in series.iter().enumerate() {
                let color = SERIES_COLORS[series_index];
                let path: Vec<String> = series
                    .values
                    .iter()
                    .enumerate()
                    .map(|(index, value)| format!("{:.1},{:.1}", position(index), scale_y(*value)))
                    .collect();
                svg.push_str(&format!(
                    "<polyline points='{points}' fill='none' stroke='{color}' stroke-width='2'/>",
                    points = path.join(" "),
                ));
                for (index, value) in series.values.iter().enumerate() {
                    svg.push_str(&format!(
                        "<circle cx='{x:.1}' cy='{y:.1}' r='4' fill='{color}' stroke='{CHART_SURFACE}' stroke-width='2'>\
                         <title>{label} · {name}: {value}</title></circle>",
                        x = position(index),
                        y = scale_y(*value),
                        label = escape_xml(&labels[index]),
                        name = escape_xml(&series_name(series_index, series)),
                        value = escape_xml(&format_number(*value)),
                    ));
                }
            }
            svg.push_str("</svg>");
            let legend = if series.len() >= 2 {
                series
                    .iter()
                    .enumerate()
                    .map(|(index, s)| {
                        (SERIES_COLORS[index].to_string(), series_name(index, s))
                    })
                    .collect()
            } else {
                Vec::new()
            };
            Ok(BuiltChart { svg, legend })
        }
        // Everything else renders as bars; "bar" is the documented name.
        _ => {
            let all: Vec<f64> = series.iter().flat_map(|s| s.values.iter().copied()).collect();
            let mut min = all.iter().copied().fold(f64::INFINITY, f64::min).min(0.0);
            let mut max = all.iter().copied().fold(f64::NEG_INFINITY, f64::max).max(0.0);
            if max - min < f64::EPSILON {
                max += 1.0;
                min -= 1.0;
            }
            let plot_w = CHART_W - MARGIN_L - MARGIN_R;
            let mut svg = format!(
                "<svg viewBox='0 0 {CHART_W} {CHART_H}' role='img' xmlns='http://www.w3.org/2000/svg'>"
            );
            let scale_y = y_axis(&mut svg, min, max, plot_w);
            let series_count = series.len() as f64;
            let group_w = plot_w / points.max(1) as f64;
            // 2px gap between bars of a group; groups get breathing room too.
            let bar_w = ((group_w * 0.72 / series_count) - 2.0).clamp(2.0, 44.0);
            let position =
                |index: usize| MARGIN_L + group_w * index as f64 + group_w / 2.0;
            x_labels(&mut svg, &labels, position);
            let zero_y = scale_y(0.0);
            for (series_index, series) in series.iter().enumerate() {
                let color = SERIES_COLORS[series_index];
                for (index, value) in series.values.iter().enumerate() {
                    let x = position(index)
                        + (series_index as f64 - series_count / 2.0) * (bar_w + 2.0)
                        + 1.0;
                    let y = scale_y(*value);
                    let (top, height) = if *value >= 0.0 {
                        (y, (zero_y - y).max(1.0))
                    } else {
                        (zero_y, (y - zero_y).max(1.0))
                    };
                    svg.push_str(&format!(
                        "<rect x='{x:.1}' y='{top:.1}' width='{bar_w:.1}' height='{height:.1}' rx='2' fill='{color}'>\
                         <title>{label} · {name}: {value}</title></rect>",
                        label = escape_xml(&labels[index]),
                        name = escape_xml(&series_name(series_index, series)),
                        value = escape_xml(&format_number(*value)),
                    ));
                }
            }
            svg.push_str(&format!(
                "<line x1='{MARGIN_L}' y1='{zero_y:.1}' x2='{x2:.1}' y2='{zero_y:.1}' stroke='rgba(255,255,255,0.35)' stroke-width='1'/>",
                x2 = MARGIN_L + plot_w,
            ));
            svg.push_str("</svg>");
            let legend = if series.len() >= 2 {
                series
                    .iter()
                    .enumerate()
                    .map(|(index, s)| {
                        (SERIES_COLORS[index].to_string(), series_name(index, s))
                    })
                    .collect()
            } else {
                Vec::new()
            };
            Ok(BuiltChart { svg, legend })
        }
    }
}

// ---------------------------------------------------------------- transcript

#[derive(Clone, PartialEq)]
enum Segment {
    Markdown(String),
    Chart(ChartSpec),
    ChartError(String),
}

/// Splits assistant text into Markdown and ```chart fenced blocks.
fn split_segments(text: &str) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut markdown = String::new();
    let mut chart: Option<String> = None;
    for line in text.lines() {
        match &mut chart {
            None if line.trim() == "```chart" => {
                if !markdown.trim().is_empty() {
                    segments.push(Segment::Markdown(std::mem::take(&mut markdown)));
                }
                markdown.clear();
                chart = Some(String::new());
            }
            None => {
                markdown.push_str(line);
                markdown.push('\n');
            }
            Some(body) if line.trim() == "```" => {
                match serde_json::from_str::<ChartSpec>(body) {
                    Ok(spec) => segments.push(Segment::Chart(spec)),
                    Err(error) => segments.push(Segment::ChartError(format!(
                        "Diagramm konnte nicht gelesen werden: {error}"
                    ))),
                }
                chart = None;
            }
            Some(body) => {
                body.push_str(line);
                body.push('\n');
            }
        }
    }
    // An unterminated chart fence degrades to visible code, not silence.
    if let Some(body) = chart {
        markdown.push_str("```\n");
        markdown.push_str(&body);
        markdown.push_str("```\n");
    }
    if !markdown.trim().is_empty() {
        segments.push(Segment::Markdown(markdown));
    }
    segments
}

#[derive(Clone, PartialEq)]
enum ToolStatus {
    Running,
    Succeeded,
    Failed,
}

#[derive(Clone, PartialEq)]
enum Entry {
    User(String),
    Assistant(String),
    Tool {
        call_id: String,
        title: String,
        arguments: String,
        status: ToolStatus,
        summary: String,
    },
    Confirmation {
        call_id: String,
        title: String,
        arguments: String,
        resolved: Option<bool>,
    },
    Error(String),
}

fn apply_event(entries: &mut Vec<Entry>, event: ChatEvent) {
    match event {
        ChatEvent::AssistantMessage { text } => entries.push(Entry::Assistant(text)),
        ChatEvent::ToolCall {
            call_id,
            name: _,
            title,
            arguments,
        } => entries.push(Entry::Tool {
            call_id,
            title,
            arguments,
            status: ToolStatus::Running,
            summary: String::new(),
        }),
        ChatEvent::ToolResult {
            call_id,
            ok,
            summary,
        } => {
            if let Some(Entry::Tool {
                status,
                summary: entry_summary,
                ..
            }) = entries.iter_mut().rev().find(|entry| {
                matches!(entry, Entry::Tool { call_id: id, .. } if *id == call_id)
            }) {
                *status = if ok {
                    ToolStatus::Succeeded
                } else {
                    ToolStatus::Failed
                };
                *entry_summary = summary;
            }
        }
        ChatEvent::ConfirmationRequest {
            call_id,
            name: _,
            title,
            arguments,
        } => entries.push(Entry::Confirmation {
            call_id,
            title,
            arguments,
            resolved: None,
        }),
        ChatEvent::ConfirmationResolved { call_id, approved } => {
            if let Some(Entry::Confirmation { resolved, .. }) =
                entries.iter_mut().rev().find(|entry| {
                    matches!(entry, Entry::Confirmation { call_id: id, .. } if *id == call_id)
                })
            {
                *resolved = Some(approved);
            }
        }
        ChatEvent::Error { message } => entries.push(Entry::Error(message)),
    }
}

fn poll_loop(run_id: String, cursor: u32, entries: RwSignal<Vec<Entry>>, running: RwSignal<bool>) {
    spawn_local(async move {
        match poll_chat_run(run_id.clone(), cursor).await {
            Ok(update) => {
                if !update.events.is_empty()
                    && entries
                        .try_update(|list| {
                            for event in update.events {
                                apply_event(list, event);
                            }
                        })
                        .is_none()
                {
                    return; // page left; stop polling
                }
                if update.done {
                    let _ = running.try_set(false);
                } else {
                    let next_cursor = update.next_cursor;
                    set_timeout(
                        move || poll_loop(run_id, next_cursor, entries, running),
                        std::time::Duration::from_millis(POLL_INTERVAL_MS),
                    );
                }
            }
            Err(error) => {
                let _ = entries.try_update(|list| list.push(Entry::Error(error.to_string())));
                let _ = running.try_set(false);
            }
        }
    });
}

// ---------------------------------------------------------------------- view

fn chart_view(spec: &ChartSpec) -> View {
    let title = spec.title.clone();
    match build_chart(spec) {
        Ok(chart) => view! {
            <div class="chat-chart">
                {(!title.is_empty()).then(|| view! { <div class="chat-chart-title">{title}</div> })}
                <div class="chat-chart-svg" inner_html=chart.svg></div>
                {(!chart.legend.is_empty()).then(|| view! {
                    <div class="chat-legend">
                        {chart.legend.into_iter().map(|(color, label)| view! {
                            <span class="chat-legend-item">
                                <span class="chat-legend-dot" style=format!("background:{color}")></span>
                                {label}
                            </span>
                        }).collect_view()}
                    </div>
                })}
            </div>
        }
        .into_view(),
        Err(message) => view! { <div class="chat-chart-error">{message}</div> }.into_view(),
    }
}

fn entry_view(
    entry: Entry,
    run_id: RwSignal<Option<String>>,
    entries: RwSignal<Vec<Entry>>,
) -> View {
    match entry {
        Entry::User(text) => view! {
            <div class="chat-row is-user"><div class="chat-bubble chat-bubble-user">{text}</div></div>
        }
        .into_view(),
        Entry::Assistant(text) => view! {
            <div class="chat-row">
                <div class="chat-bubble chat-bubble-assistant">
                    {split_segments(&text).into_iter().map(|segment| match segment {
                        Segment::Markdown(md) => view! {
                            <div class="chat-markdown" inner_html=render_markdown(&md)></div>
                        }.into_view(),
                        Segment::Chart(spec) => chart_view(&spec),
                        Segment::ChartError(message) => view! {
                            <div class="chat-chart-error">{message}</div>
                        }.into_view(),
                    }).collect_view()}
                </div>
            </div>
        }
        .into_view(),
        Entry::Tool {
            title,
            arguments,
            status,
            summary,
            ..
        } => {
            let (icon, class) = match status {
                ToolStatus::Running => ("mdi mdi-loading mdi-spin", "chat-tool"),
                ToolStatus::Succeeded => ("mdi mdi-check-circle-outline", "chat-tool is-ok"),
                ToolStatus::Failed => ("mdi mdi-alert-circle-outline", "chat-tool is-failed"),
            };
            view! {
                <div class="chat-row">
                    <details class=class>
                        <summary>
                            <span class="icon"><i class=icon></i></span>
                            <span>{title}</span>
                        </summary>
                        <pre class="chat-tool-detail">{arguments}</pre>
                        {(!summary.is_empty()).then(|| view! {
                            <pre class="chat-tool-detail">{summary}</pre>
                        })}
                    </details>
                </div>
            }
            .into_view()
        }
        Entry::Confirmation {
            call_id,
            title,
            arguments,
            resolved,
        } => {
            let decide = move |approved: bool| {
                let Some(id) = run_id.get_untracked() else {
                    return;
                };
                let call = call_id.clone();
                // Disable the buttons right away; the authoritative
                // ConfirmationResolved event arrives via polling.
                entries.update(|list| {
                    if let Some(Entry::Confirmation { resolved, .. }) =
                        list.iter_mut().rev().find(|entry| {
                            matches!(entry, Entry::Confirmation { call_id: id, .. } if *id == call)
                        })
                    {
                        *resolved = Some(approved);
                    }
                });
                spawn_local(async move {
                    let _ = resolve_chat_confirmation(id, call, approved).await;
                });
            };
            view! {
                <div class="chat-row">
                    <div class="chat-confirm">
                        <p class="chat-confirm-head">
                            <span class="icon"><i class="mdi mdi-shield-alert-outline"></i></span>
                            <strong>"Bestätigung erforderlich: "{title}</strong>
                        </p>
                        <pre class="chat-tool-detail">{arguments}</pre>
                        {match resolved {
                            None => view! {
                                <div class="buttons mt-2">
                                    <button class="button is-danger is-small" on:click={let decide = decide.clone(); move |_| decide(true)}>
                                        <span class="icon"><i class="mdi mdi-check"></i></span>
                                        <span>"Ausführen"</span>
                                    </button>
                                    <button class="button is-small" on:click=move |_| decide(false)>
                                        <span class="icon"><i class="mdi mdi-close"></i></span>
                                        <span>"Ablehnen"</span>
                                    </button>
                                </div>
                            }.into_view(),
                            Some(true) => view! { <p class="text-muted is-size-7">"Vom Benutzer bestätigt."</p> }.into_view(),
                            Some(false) => view! { <p class="text-muted is-size-7">"Vom Benutzer abgelehnt."</p> }.into_view(),
                        }}
                    </div>
                </div>
            }
            .into_view()
        }
        Entry::Error(message) => view! {
            <div class="chat-row">
                <div class="message is-danger"><div class="message-body">{message}</div></div>
            </div>
        }
        .into_view(),
    }
}

#[component]
pub fn ChatPage() -> impl IntoView {
    let status = create_resource(|| (), |_| get_chat_status());
    let entries = create_rw_signal(Vec::<Entry>::new());
    let running = create_rw_signal(false);
    let run_id = create_rw_signal(Option::<String>::None);
    let input = create_rw_signal(String::new());
    let transcript_ref = create_node_ref::<html::Div>();

    // Keep the newest message in view. The frame callback runs after the DOM
    // update this effect was triggered by.
    create_effect(move |_| {
        entries.track();
        running.track();
        request_animation_frame(move || {
            if let Some(element) = transcript_ref.get_untracked() {
                element.set_scroll_top(element.scroll_height());
            }
        });
    });

    let send = move || {
        let text = input.get_untracked().trim().to_string();
        if text.is_empty() || running.get_untracked() {
            return;
        }
        input.set(String::new());
        entries.update(|list| list.push(Entry::User(text)));
        let history: Vec<ChatHistoryMessage> = entries
            .get_untracked()
            .iter()
            .filter_map(|entry| match entry {
                Entry::User(content) => Some(ChatHistoryMessage {
                    role: "user".to_string(),
                    content: content.clone(),
                }),
                Entry::Assistant(content) => Some(ChatHistoryMessage {
                    role: "assistant".to_string(),
                    content: content.clone(),
                }),
                _ => None,
            })
            .collect();
        running.set(true);
        spawn_local(async move {
            match start_chat_run(history).await {
                Ok(id) => {
                    let _ = run_id.try_set(Some(id.clone()));
                    poll_loop(id, 0, entries, running);
                }
                Err(error) => {
                    let _ = entries.try_update(|list| list.push(Entry::Error(error.to_string())));
                    let _ = running.try_set(false);
                }
            }
        });
    };

    view! {
        <div class="container chat-page">
            <div class="level mb-2">
                <div>
                    <h1 class="title mb-1">"Assistent"</h1>
                    <p class="text-muted">
                        "Fragen zu deinen Geschäftsdaten – mit Zugriff auf Rechnungen, Angebote, Belege, Kontakte und Berichte."
                        {move || status.get().and_then(Result::ok).map(|status| view! {
                            <span class="is-size-7">{format!(" (Modell: {})", status.model)}</span>
                        })}
                    </p>
                </div>
            </div>

            <div class="chat-transcript" node_ref=transcript_ref>
                {move || entries.get().is_empty().then(|| view! {
                    <div class="box has-text-centered chat-empty">
                        <span class="icon is-size-3"><i class="mdi mdi-robot-outline"></i></span>
                        <p class="mt-2">"Stelle eine Frage, z. B.:"</p>
                        <p class="text-muted is-size-7">"„Welche Rechnungen sind überfällig?“ · „Zeige den Umsatz der letzten 6 Monate als Diagramm.“ · „Lege einen Angebotsentwurf für Kontakt Schmidt an.“"</p>
                    </div>
                })}
                {move || entries.get().into_iter().map(|entry| entry_view(entry, run_id, entries)).collect_view()}
                {move || running.get().then(|| view! {
                    <div class="chat-row">
                        <span class="chat-working"><span class="icon"><i class="mdi mdi-loading mdi-spin"></i></span>" Assistent arbeitet…"</span>
                    </div>
                })}
            </div>

            <div class="chat-input">
                <textarea
                    class="textarea"
                    rows="2"
                    placeholder="Nachricht an den Assistenten… (Enter senden, Umschalt+Enter neue Zeile)"
                    prop:value=move || input.get()
                    on:input=move |ev| input.set(event_target_value(&ev))
                    on:keydown=move |ev| {
                        if ev.key() == "Enter" && !ev.shift_key() {
                            ev.prevent_default();
                            send();
                        }
                    }
                ></textarea>
                <button
                    class="button is-link"
                    disabled=move || running.get()
                    on:click=move |_| send()
                >
                    <span class="icon"><i class="mdi mdi-send"></i></span>
                </button>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_neutralizes_raw_html_and_bad_links() {
        let html = render_markdown("Hallo <script>alert(1)</script> [ok](/invoices/4) [bad](javascript:alert(1))");
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
        assert!(html.contains("href=\"/invoices/4\""));
        assert!(!html.contains("javascript:"));
    }

    #[test]
    fn image_syntax_on_app_urls_becomes_an_inline_preview() {
        let html = render_markdown("![Vertrag](/api/documents/9)");
        assert!(html.contains("<details class=\"chat-preview\" open>"));
        assert!(html.contains("<iframe src=\"/api/documents/9\""));
        assert!(html.contains("Vertrag"));

        // A plain link to the same URL stays a link — the model's choice.
        let html = render_markdown("[Vertrag](/api/documents/9)");
        assert!(!html.contains("<iframe"));
        assert!(html.contains("href=\"/api/documents/9\""));
    }

    #[test]
    fn external_and_unsafe_images_never_embed() {
        let html = render_markdown("![extern](https://example.com/a.png)");
        assert!(!html.contains("<iframe"));
        assert!(!html.contains("<img"));
        assert!(html.contains("href=\"https://example.com/a.png\""));

        let html = render_markdown("![böse](javascript:alert(1))");
        assert!(!html.contains("javascript:"));
        assert!(html.contains("böse"));
    }

    #[test]
    fn chart_blocks_are_split_out_of_markdown() {
        let text = "Vorher\n```chart\n{\"type\":\"bar\",\"labels\":[\"A\"],\"series\":[{\"name\":\"S\",\"values\":[1]}]}\n```\nNachher";
        let segments = split_segments(text);
        assert_eq!(segments.len(), 3);
        assert!(matches!(&segments[0], Segment::Markdown(md) if md.contains("Vorher")));
        assert!(matches!(&segments[1], Segment::Chart(spec) if spec.chart_type == "bar"));
        assert!(matches!(&segments[2], Segment::Markdown(md) if md.contains("Nachher")));
    }

    #[test]
    fn german_number_formatting() {
        assert_eq!(format_number(1234.5), "1.234,50");
        assert_eq!(format_number(1234.0), "1.234");
        assert_eq!(format_number(-7.25), "-7,25");
        assert_eq!(format_number(0.0), "0");
    }

    #[test]
    fn bar_chart_svg_contains_marks_and_tooltips() {
        let spec = ChartSpec {
            chart_type: "bar".to_string(),
            title: "Umsatz".to_string(),
            labels: vec!["Jan".to_string(), "Feb".to_string()],
            series: vec![ChartSeries {
                name: "Umsatz".to_string(),
                values: vec![100.0, 250.0],
            }],
        };
        let chart = build_chart(&spec).unwrap();
        assert!(chart.svg.contains("<rect"));
        assert!(chart.svg.contains("<title>Feb · Umsatz: 250</title>"));
        // A single series carries its identity in the title, not a legend.
        assert!(chart.legend.is_empty());
    }
}
