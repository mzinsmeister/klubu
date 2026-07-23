//! Chat assistant backed by any OpenAI-compatible chat-completions endpoint
//! (a remote hosted API or a local Ollama under `/v1`).
//!
//! The assistant is opt-in: unless `klubu.chat.url` and `klubu.chat.model` are
//! configured, [`get_chat_status`] reports it disabled and the client never
//! shows the page. Tool access shares the MCP tool table, but not the MCP HTTP
//! transport: the backend injects a [`ChatToolBackend`] into every server-fn
//! context, so the chat keeps working when `KLUBU_MCP_TOKEN` is unset and the
//! external `/mcp` endpoint does not exist.
//!
//! A chat run is asynchronous: `start_chat_run` spawns the agent loop and the
//! client polls `poll_chat_run` for [`ChatEvent`]s. Tools whose MCP definition
//! carries `_meta["klubu/requiresConfirmation"]` pause the loop until the user
//! approves or rejects the call via `resolve_chat_confirmation` — the model
//! cannot finalize, delete, or send anything on its own.

use leptos::server_fn::codec::Json;
use leptos::*;
use shared::{ChatHistoryMessage, ChatRunUpdate, ChatStatus};

#[cfg(feature = "ssr")]
use serde_json::{json, Value};
#[cfg(feature = "ssr")]
use shared::ChatEvent;
#[cfg(feature = "ssr")]
use std::collections::{HashMap, HashSet};
#[cfg(feature = "ssr")]
use std::sync::{Arc, Mutex, OnceLock};
#[cfg(feature = "ssr")]
use std::time::{Duration, Instant};

/// The model may loop over tools, but not forever: a runaway plan is cut off
/// with a visible error instead of silently burning tokens.
#[cfg(feature = "ssr")]
const MAX_TOOL_ROUNDS: usize = 12;

/// Tool output handed back to the model. Large enough for full business
/// records, small enough that a base64 PDF cannot blow up the context window.
#[cfg(feature = "ssr")]
const TOOL_RESULT_LLM_LIMIT: usize = 20_000;

/// Tool output shown in the chat transcript.
#[cfg(feature = "ssr")]
const TOOL_RESULT_UI_LIMIT: usize = 700;

/// Prior turns sent back to the model. The client keeps the full transcript;
/// the model only needs recent context.
#[cfg(feature = "ssr")]
const MAX_HISTORY_MESSAGES: usize = 40;

/// An unanswered confirmation counts as a rejection after this long, so an
/// abandoned browser tab cannot leave a run blocked forever.
#[cfg(feature = "ssr")]
const CONFIRMATION_TIMEOUT: Duration = Duration::from_secs(15 * 60);

#[cfg(feature = "ssr")]
#[derive(Debug, Clone)]
pub struct ChatConfig {
    /// Normalized OpenAI-compatible base URL, e.g. `http://localhost:11434/v1`.
    pub url: String,
    pub api_key: String,
    pub model: String,
    pub timeout_secs: u64,
}

#[cfg(feature = "ssr")]
impl ChatConfig {
    pub fn enabled(&self) -> bool {
        !self.url.is_empty() && !self.model.is_empty()
    }
}

#[cfg(feature = "ssr")]
pub fn load_chat_config() -> ChatConfig {
    let props = crate::typst_gen::load_props();
    let get =
        |key: &str, env: &str, default: &str| crate::typst_gen::get_prop(&props, key, env, default);

    ChatConfig {
        url: normalize_chat_url(&get("klubu.chat.url", "KLUBU_CHAT_URL", "")),
        api_key: get("klubu.chat.apiKey", "KLUBU_CHAT_API_KEY", "")
            .trim()
            .to_string(),
        model: get("klubu.chat.model", "KLUBU_CHAT_MODEL", "")
            .trim()
            .to_string(),
        timeout_secs: get(
            "klubu.chat.timeoutSeconds",
            "KLUBU_CHAT_TIMEOUT_SECONDS",
            "120",
        )
        .trim()
        .parse()
        .unwrap_or(120)
        .clamp(10, 600),
    }
}

/// A bare origin like `http://localhost:11434` means the API root: both
/// Ollama and OpenAI serve the compatible API under `/v1`. An explicit path
/// (Azure-style deployments, reverse proxies) is respected as given.
#[cfg(feature = "ssr")]
fn normalize_chat_url(raw: &str) -> String {
    let url = raw.trim().trim_end_matches('/').to_string();
    match url.split_once("://") {
        Some((_, rest)) if !rest.is_empty() && !rest.contains('/') => format!("{url}/v1"),
        _ => url,
    }
}

/// Individually configurable assistant tools. These gate both the chat and the
/// external MCP endpoint: a disabled tool is not listed and not callable.
#[cfg(feature = "ssr")]
#[derive(Debug, Clone, Copy)]
pub struct AssistantToolGates {
    pub sql: bool,
    pub python: bool,
}

#[cfg(feature = "ssr")]
pub fn load_assistant_tool_gates() -> AssistantToolGates {
    let props = crate::typst_gen::load_props();
    let truthy = |key: &str, env: &str, default: &str| {
        matches!(
            crate::typst_gen::get_prop(&props, key, env, default)
                .trim()
                .to_ascii_lowercase()
                .as_str(),
            "true" | "1" | "yes" | "on"
        )
    };
    AssistantToolGates {
        sql: truthy(
            "klubu.tools.sqlQueriesEnabled",
            "KLUBU_TOOLS_SQL_QUERIES_ENABLED",
            "true",
        ),
        python: truthy(
            "klubu.tools.pythonEnabled",
            "KLUBU_TOOLS_PYTHON_ENABLED",
            "true",
        ),
    }
}

#[cfg(feature = "ssr")]
pub type ToolFuture =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value, String>> + Send + 'static>>;

/// What the chat needs from the tool layer. Implemented in the backend crate
/// on top of the MCP tool table, so both surfaces expose the same tools with
/// the same business rules and audit attribution.
#[cfg(feature = "ssr")]
pub trait ChatToolBackend: Send + Sync {
    /// MCP tool definitions: name, title, description, inputSchema,
    /// annotations, and `_meta` (which carries the confirmation flag).
    fn definitions(&self) -> Vec<Value>;
    /// The stable operating guide shared with the MCP endpoint.
    fn instructions(&self) -> &'static str;
    /// Executes one tool as `actor`, with full business validation and audit.
    fn call(&self, actor: String, name: String, arguments: Value) -> ToolFuture;
}

/// Provided by the backend as request context for every server function.
#[cfg(feature = "ssr")]
#[derive(Clone)]
pub struct ChatTools(pub Arc<dyn ChatToolBackend>);

#[cfg(feature = "ssr")]
struct ChatRun {
    /// Session user the run belongs to; polls by anyone else are rejected.
    user: String,
    events: Vec<ChatEvent>,
    done: bool,
    /// Pending confirmation: call id and the channel the decision resolves.
    pending: Option<(String, tokio::sync::oneshot::Sender<bool>)>,
    last_touched: Instant,
}

#[cfg(feature = "ssr")]
fn runs() -> &'static Mutex<HashMap<String, ChatRun>> {
    static RUNS: OnceLock<Mutex<HashMap<String, ChatRun>>> = OnceLock::new();
    RUNS.get_or_init(Default::default)
}

/// Appends an event; returns false when the run no longer exists (purged),
/// which tells the agent loop to stop doing work nobody can see.
#[cfg(feature = "ssr")]
fn push_event(run_id: &str, event: ChatEvent) -> bool {
    let mut map = runs().lock().unwrap();
    match map.get_mut(run_id) {
        Some(run) => {
            run.events.push(event);
            run.last_touched = Instant::now();
            true
        }
        None => false,
    }
}

#[cfg(feature = "ssr")]
fn mark_done(run_id: &str) {
    let mut map = runs().lock().unwrap();
    if let Some(run) = map.get_mut(run_id) {
        run.done = true;
        run.last_touched = Instant::now();
    }
}

/// Finished runs linger briefly so the client can fetch the tail; anything
/// idle for hours is gone regardless. Confirmation waits time out well before
/// the idle horizon, so an active run is never purged mid-flight.
#[cfg(feature = "ssr")]
fn purge_stale(map: &mut HashMap<String, ChatRun>) {
    map.retain(|_, run| {
        let idle = run.last_touched.elapsed();
        if run.done {
            idle < Duration::from_secs(10 * 60)
        } else {
            idle < Duration::from_secs(2 * 60 * 60)
        }
    });
}

#[cfg(feature = "ssr")]
fn current_user() -> Result<String, ServerFnError> {
    use_context::<super::auth::CurrentUser>()
        .map(|user| user.0)
        .ok_or_else(|| ServerFnError::new("Nicht angemeldet"))
}

/// Truncates on a character boundary and says so, instead of feeding the model
/// (or the user) a silently incomplete blob.
#[cfg(feature = "ssr")]
fn truncated(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let mut cut: String = text.chars().take(max_chars).collect();
    cut.push_str(" …[gekürzt]");
    cut
}

#[server(name = GetChatStatus, prefix = "/api", endpoint = "get_chat_status")]
pub async fn get_chat_status() -> Result<ChatStatus, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let cfg = load_chat_config();
        let gates = load_assistant_tool_gates();
        Ok(ChatStatus {
            enabled: cfg.enabled(),
            model: cfg.model,
            sql_tool_enabled: gates.sql,
            python_tool_enabled: gates.python,
        })
    }
    #[cfg(not(feature = "ssr"))]
    Ok(ChatStatus::default())
}

// JSON input: the history is structured data; url-encoded form fields would
// mangle it for no benefit.
#[server(name = StartChatRun, prefix = "/api", endpoint = "start_chat_run", input = Json)]
pub async fn start_chat_run(history: Vec<ChatHistoryMessage>) -> Result<String, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let cfg = load_chat_config();
        if !cfg.enabled() {
            return Err(ServerFnError::new(
                "Der Chat-Assistent ist nicht konfiguriert (klubu.chat.url / klubu.chat.model).",
            ));
        }
        let actor = current_user()?;
        let tools = use_context::<ChatTools>()
            .ok_or_else(|| ServerFnError::new("Chat-Werkzeuge sind nicht initialisiert"))?;

        let run_id = super::auth::generate_random_token();
        {
            let mut map = runs().lock().unwrap();
            purge_stale(&mut map);
            map.insert(
                run_id.clone(),
                ChatRun {
                    user: actor.clone(),
                    events: Vec::new(),
                    done: false,
                    pending: None,
                    last_touched: Instant::now(),
                },
            );
        }

        tokio::spawn(run_agent_loop(run_id.clone(), cfg, tools, actor, history));
        Ok(run_id)
    }
    #[cfg(not(feature = "ssr"))]
    {
        _ = history;
        Err(ServerFnError::new("Nur serverseitig verfügbar"))
    }
}

#[server(name = PollChatRun, prefix = "/api", endpoint = "poll_chat_run")]
pub async fn poll_chat_run(run_id: String, cursor: u32) -> Result<ChatRunUpdate, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let actor = current_user()?;
        let map = runs().lock().unwrap();
        let run = map
            .get(&run_id)
            .filter(|run| run.user == actor)
            .ok_or_else(|| ServerFnError::new("Unbekannter Chat-Lauf"))?;
        let events = run.events.iter().skip(cursor as usize).cloned().collect();
        Ok(ChatRunUpdate {
            events,
            next_cursor: run.events.len() as u32,
            done: run.done,
        })
    }
    #[cfg(not(feature = "ssr"))]
    {
        _ = (run_id, cursor);
        Err(ServerFnError::new("Nur serverseitig verfügbar"))
    }
}

#[server(
    name = ResolveChatConfirmation,
    prefix = "/api",
    endpoint = "resolve_chat_confirmation"
)]
pub async fn resolve_chat_confirmation(
    run_id: String,
    call_id: String,
    approved: bool,
) -> Result<(), ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        let actor = current_user()?;
        let sender = {
            let mut map = runs().lock().unwrap();
            let run = map
                .get_mut(&run_id)
                .filter(|run| run.user == actor)
                .ok_or_else(|| ServerFnError::new("Unbekannter Chat-Lauf"))?;
            match run.pending.take() {
                Some((pending_id, sender)) if pending_id == call_id => sender,
                other => {
                    run.pending = other;
                    return Err(ServerFnError::new(
                        "Zu dieser Aktion ist keine Bestätigung offen",
                    ));
                }
            }
        };
        // The agent loop may have timed out concurrently; that is not an error
        // the user can act on, the resolved state arrives via polling anyway.
        let _ = sender.send(approved);
        Ok(())
    }
    #[cfg(not(feature = "ssr"))]
    {
        _ = (run_id, call_id, approved);
        Err(ServerFnError::new("Nur serverseitig verfügbar"))
    }
}

#[cfg(feature = "ssr")]
fn system_prompt(instructions: &str) -> String {
    format!(
        "You are the assistant built into Klubu, a German bookkeeping, CRM, \
         document, engagement, and mail application. You operate on the live, \
         audited business data of the signed-in user via tools.\n\n\
         {instructions}\n\n\
         ## Chat integration rules\n\
         - Answer in the user's language (usually German). Keep answers concise.\n\
         - Today is {today}.\n\
         - Link records the user should open as relative Markdown links: \
           [Rechnung 42](/invoices/42), [Angebot 7](/offers/7), [Beleg 3](/receipts/3), \
           [Kontakt 5](/contacts/5), [Dokument 9](/documents/9). \
           Never invent ids; only link ids that appeared in tool results.\n\
         - Files themselves: /api/documents/9 serves the current version of managed \
           document 9 (view/download), /api/pdf/invoice/42 and /api/pdf/offer/7 serve \
           record PDFs. A normal Markdown link renders as a download/open link. \
           Markdown IMAGE syntax on such an app URL embeds an inline preview instead: \
           ![Vertrag](/api/documents/9) or ![Rechnung 42](/api/pdf/invoice/42). \
           You decide which fits: default to a plain link; embed the preview when the \
           user wants to look at the document directly in the chat. External image \
           URLs always render as plain links, never inline.\n\
         - Monetary tool values are integer cents; render them as euro amounts (1.234,56 €).\n\
         - For a diagram, emit a fenced code block with language `chart` containing exactly one JSON object: \
           {{\"type\": \"bar\"|\"line\"|\"pie\", \"title\": \"...\", \"labels\": [\"...\"], \
           \"series\": [{{\"name\": \"...\", \"values\": [1.5, 2.0]}}]}}. \
           Values are plain numbers (euros, not cents). Use charts when the user asks for a \
           visualization or when a trend/distribution is clearer as a picture.\n\
         - Do not call tools that return base64 files (downloads/exports); link to the record instead.\n\
         - Irreversible actions (finalize, delete, cancel, send) require an explicit user \
           confirmation in the chat UI. A rejected confirmation is a decision, not an error: \
           do not retry it. Never claim an action happened unless its tool result confirms it.",
        today = chrono::Local::now().format("%Y-%m-%d (%A)"),
    )
}

#[cfg(feature = "ssr")]
async fn chat_completion(
    client: &reqwest::Client,
    cfg: &ChatConfig,
    messages: &[Value],
    tools: &[Value],
) -> Result<Value, String> {
    let mut body = json!({
        "model": cfg.model,
        "messages": messages,
        "stream": false,
    });
    if !tools.is_empty() {
        body["tools"] = json!(tools);
        body["tool_choice"] = json!("auto");
    }

    let mut request = client
        .post(format!("{}/chat/completions", cfg.url))
        .json(&body);
    if !cfg.api_key.is_empty() {
        request = request.bearer_auth(&cfg.api_key);
    }

    let response = request.send().await.map_err(|error| {
        format!(
            "LLM-Endpunkt {} nicht erreichbar: {error}. Läuft der Dienst (z. B. `ollama serve`)?",
            cfg.url
        )
    })?;

    let status = response.status();
    if !status.is_success() {
        let detail = response.text().await.unwrap_or_default();
        return Err(format!(
            "Fehler vom LLM-Endpunkt ({status}): {}",
            truncated(detail.trim(), 500)
        ));
    }

    let envelope: Value = response
        .json()
        .await
        .map_err(|error| format!("Ungültige Antwort vom LLM-Endpunkt: {error}"))?;
    envelope
        .pointer("/choices/0/message")
        .cloned()
        .ok_or_else(|| "Antwort des LLM-Endpunkts enthielt keine Nachricht".to_string())
}

#[cfg(feature = "ssr")]
async fn run_agent_loop(
    run_id: String,
    cfg: ChatConfig,
    tools: ChatTools,
    actor: String,
    history: Vec<ChatHistoryMessage>,
) {
    let definitions = tools.0.definitions();
    let confirm_required: HashSet<String> = definitions
        .iter()
        .filter(|definition| {
            definition
                .pointer("/_meta/klubu~1requiresConfirmation")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .filter_map(|definition| definition["name"].as_str().map(str::to_string))
        .collect();
    let titles: HashMap<String, String> = definitions
        .iter()
        .filter_map(|definition| {
            Some((
                definition["name"].as_str()?.to_string(),
                definition["title"].as_str().unwrap_or_default().to_string(),
            ))
        })
        .collect();
    let llm_tools: Vec<Value> = definitions
        .iter()
        .map(|definition| {
            json!({
                "type": "function",
                "function": {
                    "name": definition["name"],
                    "description": definition["description"],
                    "parameters": definition["inputSchema"],
                }
            })
        })
        .collect();

    let mut messages = vec![json!({
        "role": "system",
        "content": system_prompt(tools.0.instructions()),
    })];
    let skip = history.len().saturating_sub(MAX_HISTORY_MESSAGES);
    for message in history.into_iter().skip(skip) {
        if matches!(message.role.as_str(), "user" | "assistant") {
            messages.push(json!({"role": message.role, "content": message.content}));
        }
    }

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(cfg.timeout_secs))
        .build()
    {
        Ok(client) => client,
        Err(error) => {
            push_event(
                &run_id,
                ChatEvent::Error {
                    message: format!("HTTP-Client konnte nicht erstellt werden: {error}"),
                },
            );
            mark_done(&run_id);
            return;
        }
    };

    for round in 0.. {
        let message = match chat_completion(&client, &cfg, &messages, &llm_tools).await {
            Ok(message) => message,
            Err(error) => {
                push_event(&run_id, ChatEvent::Error { message: error });
                break;
            }
        };

        let content = message
            .get("content")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        if !content.is_empty()
            && !push_event(&run_id, ChatEvent::AssistantMessage { text: content })
        {
            // The run was purged (client long gone); stop calling the model.
            return;
        }

        // Re-append the assistant turn verbatim (including tool_calls) so the
        // follow-up request is a valid OpenAI conversation.
        let tool_calls = message
            .get("tool_calls")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        messages.push(message);

        if tool_calls.is_empty() {
            break;
        }
        if round >= MAX_TOOL_ROUNDS {
            push_event(
                &run_id,
                ChatEvent::Error {
                    message: format!(
                        "Abbruch nach {MAX_TOOL_ROUNDS} Werkzeug-Runden. Bitte die Aufgabe in kleineren Schritten stellen."
                    ),
                },
            );
            break;
        }

        for (index, call) in tool_calls.iter().enumerate() {
            let call_id = call
                .get("id")
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| format!("call-{round}-{index}"));
            let name = call
                .pointer("/function/name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let arguments: Value = call
                .pointer("/function/arguments")
                .and_then(Value::as_str)
                .and_then(|raw| serde_json::from_str(raw).ok())
                .unwrap_or_else(|| json!({}));
            let title = titles
                .get(&name)
                .filter(|title| !title.is_empty())
                .cloned()
                .unwrap_or_else(|| name.clone());
            let pretty_arguments =
                serde_json::to_string_pretty(&arguments).unwrap_or_else(|_| "{}".to_string());

            let approved = if confirm_required.contains(&name) {
                let (sender, receiver) = tokio::sync::oneshot::channel();
                {
                    let mut map = runs().lock().unwrap();
                    let Some(run) = map.get_mut(&run_id) else {
                        return;
                    };
                    run.pending = Some((call_id.clone(), sender));
                    run.events.push(ChatEvent::ConfirmationRequest {
                        call_id: call_id.clone(),
                        name: name.clone(),
                        title: title.clone(),
                        arguments: truncated(&pretty_arguments, 4000),
                    });
                    run.last_touched = Instant::now();
                }
                let approved = tokio::time::timeout(CONFIRMATION_TIMEOUT, receiver)
                    .await
                    .ok()
                    .and_then(Result::ok)
                    .unwrap_or(false);
                {
                    let mut map = runs().lock().unwrap();
                    if let Some(run) = map.get_mut(&run_id) {
                        run.pending = None;
                    }
                }
                push_event(
                    &run_id,
                    ChatEvent::ConfirmationResolved {
                        call_id: call_id.clone(),
                        approved,
                    },
                );
                approved
            } else {
                true
            };

            let (ok, result_text) = if !approved {
                (
                    false,
                    "The user rejected this action in the confirmation dialog. Treat this as a \
                     decision, ask how to proceed instead, and do not retry the action unless \
                     the user explicitly asks for it."
                        .to_string(),
                )
            } else {
                push_event(
                    &run_id,
                    ChatEvent::ToolCall {
                        call_id: call_id.clone(),
                        name: name.clone(),
                        title: title.clone(),
                        arguments: truncated(&pretty_arguments, 4000),
                    },
                );
                let result = tools.0.call(actor.clone(), name.clone(), arguments).await;
                let (ok, text) = match result {
                    Ok(value) => (
                        true,
                        serde_json::to_string(&value).unwrap_or_else(|_| value.to_string()),
                    ),
                    Err(error) => (false, format!("Tool error: {error}")),
                };
                push_event(
                    &run_id,
                    ChatEvent::ToolResult {
                        call_id: call_id.clone(),
                        ok,
                        summary: truncated(&text, TOOL_RESULT_UI_LIMIT),
                    },
                );
                (ok, text)
            };
            let _ = ok;

            messages.push(json!({
                "role": "tool",
                "tool_call_id": call_id,
                "content": truncated(&result_text, TOOL_RESULT_LLM_LIMIT),
            }));
        }
    }

    mark_done(&run_id);
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::*;

    #[test]
    fn a_bare_origin_gets_the_v1_api_path_appended() {
        assert_eq!(
            normalize_chat_url("http://localhost:11434"),
            "http://localhost:11434/v1"
        );
        assert_eq!(
            normalize_chat_url("https://api.openai.com/"),
            "https://api.openai.com/v1"
        );
    }

    #[test]
    fn an_explicit_api_path_is_left_alone() {
        assert_eq!(
            normalize_chat_url("https://example.com/openai/v1/"),
            "https://example.com/openai/v1"
        );
        assert_eq!(normalize_chat_url(""), "");
    }

    #[test]
    fn truncation_is_utf8_safe_and_marked() {
        assert_eq!(truncated("kurz", 10), "kurz");
        let long = "ä".repeat(20);
        let cut = truncated(&long, 5);
        assert!(cut.starts_with("äääää"));
        assert!(cut.ends_with("…[gekürzt]"));
    }
}
