//! Model Context Protocol endpoint, served by the main backend under `/mcp`.
//!
//! The endpoint is mounted only when `KLUBU_MCP_TOKEN` is set. It shares the
//! backend's database pool, migrations, and shutdown path, so there is no
//! second binary that could run a different schema version against the same
//! database. Authentication is a static bearer token, deliberately separate
//! from the web app's session cookies: the token is bound server-side to one
//! Klubu user (`KLUBU_MCP_USER`, or the single existing user).

mod protocol;
mod pytool;
mod sqltool;
mod tools;

use axum::{
    body::{Body, Bytes},
    extract::{DefaultBodyLimit, State},
    http::{header, HeaderMap, Request, Response, StatusCode},
    middleware::{self, Next},
    response::IntoResponse,
    routing::post,
    Router,
};
use serde_json::{json, Value};
use sqlx::Row;
use std::{collections::HashSet, sync::Arc};

const DEFAULT_BODY_LIMIT_MIB: usize = 75;

#[derive(Clone)]
struct McpState {
    repository: app::db::ActiveRepository,
    /// `KLUBU_MCP_USER`, read once at startup. `None` means "the single
    /// existing user"; the actor is re-resolved on every request so the
    /// endpoint works as soon as the admin account has been initialized.
    configured_user: Option<String>,
    bearer_token: Arc<str>,
    allowed_origins: Arc<HashSet<String>>,
}

/// Bridges the chat assistant onto the same tool table the MCP endpoint
/// serves — same business rules, same audit attribution, same confirmation
/// flags. Built unconditionally: the chat must keep working when the external
/// `/mcp` endpoint is disabled (no `KLUBU_MCP_TOKEN`).
pub fn chat_tool_backend(repository: app::db::ActiveRepository) -> app::server::chat::ChatTools {
    struct Backend {
        repository: app::db::ActiveRepository,
    }
    impl app::server::chat::ChatToolBackend for Backend {
        fn definitions(&self) -> Vec<Value> {
            tools::tool_definitions()
        }
        fn instructions(&self) -> &'static str {
            tools::OPERATING_INSTRUCTIONS
        }
        fn call(
            &self,
            actor: String,
            name: String,
            arguments: Value,
        ) -> app::server::chat::ToolFuture {
            let service = tools::ToolService::new(self.repository.clone(), actor);
            Box::pin(async move { service.call(&name, arguments).await })
        }
    }
    app::server::chat::ChatTools(Arc::new(Backend { repository }))
}

/// Builds the `/mcp` router, or `None` when `KLUBU_MCP_TOKEN` is not set.
/// A token that is set but too short is a hard startup error rather than a
/// silently disabled endpoint.
pub fn router(repository: app::db::ActiveRepository) -> Result<Option<Router>, String> {
    let Ok(bearer_token) = std::env::var("KLUBU_MCP_TOKEN") else {
        return Ok(None);
    };
    if bearer_token.len() < 32 {
        return Err(
            "KLUBU_MCP_TOKEN must contain at least 32 characters of random data".to_string(),
        );
    }

    let configured_user = std::env::var("KLUBU_MCP_USER")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let body_limit_mib = std::env::var("KLUBU_MCP_BODY_LIMIT_MIB")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(DEFAULT_BODY_LIMIT_MIB)
        .clamp(1, 512);

    let state = McpState {
        repository,
        configured_user,
        bearer_token: Arc::from(bearer_token),
        allowed_origins: Arc::new(allowed_origins()),
    };
    Ok(Some(
        Router::new()
            .route("/", post(post_mcp).get(get_mcp).delete(delete_mcp))
            .layer(DefaultBodyLimit::max(body_limit_mib * 1024 * 1024))
            .layer(middleware::from_fn_with_state(
                state.clone(),
                authorize_and_validate_origin,
            ))
            .with_state(state),
    ))
}

fn allowed_origins() -> HashSet<String> {
    std::env::var("KLUBU_MCP_ALLOWED_ORIGINS")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|origin| !origin.is_empty())
        .map(str::to_string)
        .collect()
}

/// The Klubu identity all MCP writes are attributed to. Resolved per request:
/// the token itself carries no user, and validating against the live `users`
/// table means renames and the initial admin setup take effect immediately.
async fn resolve_actor(state: &McpState) -> Result<String, String> {
    let pool = state.repository.pool();

    if let Some(username) = &state.configured_user {
        let exists: i64 =
            sqlx::query_scalar("SELECT CAST(COUNT(*) AS BIGINT) FROM users WHERE username = $1")
                .bind(username)
                .fetch_one(pool)
                .await
                .map_err(|error| format!("Could not validate KLUBU_MCP_USER: {error}"))?;
        return (exists == 1)
            .then(|| username.clone())
            .ok_or_else(|| "KLUBU_MCP_USER does not name an existing Klubu user".to_string());
    }

    let rows = sqlx::query("SELECT username FROM users ORDER BY id LIMIT 2")
        .fetch_all(pool)
        .await
        .map_err(|error| format!("Could not read Klubu users: {error}"))?;
    match rows.as_slice() {
        [row] => row
            .try_get::<String, _>("username")
            .map_err(|error| format!("Could not decode Klubu username: {error}")),
        [] => Err("Klubu has no user yet. Initialize the admin account in the web app first.".into()),
        _ => Err(
            "Klubu has multiple users. Set KLUBU_MCP_USER to the user whose identity and mailbox the MCP server should use."
                .into(),
        ),
    }
}

fn response(status: StatusCode, content_type: Option<&str>, body: Body) -> Response<Body> {
    let mut response = Response::builder().status(status);
    if let Some(content_type) = content_type {
        response = response.header(header::CONTENT_TYPE, content_type);
    }
    response
        .body(body)
        .unwrap_or_else(|_| Response::new(Body::empty()))
}

fn json_response(status: StatusCode, value: &Value) -> Response<Body> {
    match serde_json::to_vec(value) {
        Ok(encoded) => response(status, Some("application/json"), Body::from(encoded)),
        Err(error) => response(
            StatusCode::INTERNAL_SERVER_ERROR,
            Some("text/plain;charset=utf-8"),
            Body::from(format!("Could not encode MCP response: {error}")),
        ),
    }
}

/// Constant-time comparison for equal-length bearer tokens. The length check
/// itself is observable, but a 64-character random token has no useful prefix
/// structure to learn and the secret bytes are never compared with early exit.
fn token_matches(expected: &str, supplied: &str) -> bool {
    if expected.len() != supplied.len() {
        return false;
    }
    expected
        .as_bytes()
        .iter()
        .zip(supplied.as_bytes())
        .fold(0u8, |difference, (left, right)| difference | (left ^ right))
        == 0
}

fn bearer(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
}

async fn authorize_and_validate_origin(
    State(state): State<McpState>,
    request: Request<Body>,
    next: Next,
) -> Response<Body> {
    let headers = request.headers();
    if !bearer(headers)
        .map(|token| token_matches(&state.bearer_token, token))
        .unwrap_or(false)
    {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .header(header::WWW_AUTHENTICATE, "Bearer realm=\"klubu-mcp\"")
            .header(header::CONTENT_TYPE, "text/plain;charset=utf-8")
            .body(Body::from("Missing or invalid bearer token"))
            .unwrap_or_else(|_| Response::new(Body::empty()));
    }

    // Non-browser MCP clients normally send no Origin. If a browser or proxy
    // does send one, it must be explicitly allowed to prevent DNS rebinding.
    if let Some(origin) = headers.get(header::ORIGIN) {
        let origin = match origin.to_str() {
            Ok(origin) => origin,
            Err(_) => {
                return response(
                    StatusCode::FORBIDDEN,
                    Some("text/plain;charset=utf-8"),
                    Body::from("Invalid Origin header"),
                )
            }
        };
        if !state.allowed_origins.contains(origin) {
            return response(
                StatusCode::FORBIDDEN,
                Some("text/plain;charset=utf-8"),
                Body::from("Origin is not allowed"),
            );
        }
    }

    next.run(request).await
}

async fn post_mcp(
    State(state): State<McpState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response<Body> {
    let content_type_ok = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| {
            value.eq_ignore_ascii_case("application/json") || value.starts_with("application/json;")
        })
        .unwrap_or(false);
    if !content_type_ok {
        return response(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            Some("text/plain;charset=utf-8"),
            Body::from("Content-Type must be application/json"),
        );
    }

    let accepts_mcp = headers
        .get(header::ACCEPT)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.contains("application/json") && value.contains("text/event-stream"))
        .unwrap_or(false);
    if !accepts_mcp {
        return response(
            StatusCode::NOT_ACCEPTABLE,
            Some("text/plain;charset=utf-8"),
            Body::from("Accept must include application/json and text/event-stream"),
        );
    }

    let message = match serde_json::from_slice::<Value>(&body) {
        Ok(message) => message,
        Err(parse_error) => {
            return json_response(
                StatusCode::OK,
                &protocol::error(
                    Value::Null,
                    -32700,
                    "Parse error",
                    Some(json!({"detail": parse_error.to_string()})),
                ),
            )
        }
    };

    let is_initialize = message
        .get("method")
        .and_then(Value::as_str)
        .map(|method| method == "initialize")
        .unwrap_or(false);
    let requested_version = headers
        .get("mcp-protocol-version")
        .and_then(|value| value.to_str().ok());
    if !is_initialize
        && requested_version
            .map(|version| !protocol::SUPPORTED_PROTOCOLS.contains(&version))
            .unwrap_or(false)
    {
        return response(
            StatusCode::BAD_REQUEST,
            Some("text/plain;charset=utf-8"),
            Body::from("Unsupported MCP-Protocol-Version"),
        );
    }

    let service = resolve_actor(&state)
        .await
        .map(|actor| tools::ToolService::new(state.repository.clone(), actor));

    match protocol::handle_message(&service, message).await {
        Some(result) => json_response(StatusCode::OK, &result),
        None => response(StatusCode::ACCEPTED, None, Body::empty()),
    }
}

async fn get_mcp() -> impl IntoResponse {
    // This implementation has no server-initiated messages, so it does not
    // maintain a standalone SSE listening stream. Streamable HTTP permits 405.
    (
        StatusCode::METHOD_NOT_ALLOWED,
        "This Klubu MCP server does not expose a standalone SSE stream",
    )
}

async fn delete_mcp() -> impl IntoResponse {
    // Sessions are deliberately stateless; there is nothing to terminate.
    (
        StatusCode::METHOD_NOT_ALLOWED,
        "This Klubu MCP server does not allocate HTTP sessions",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bearer_comparison_requires_the_whole_exact_token() {
        let token = "0123456789abcdef0123456789abcdef";
        assert!(token_matches(token, token));
        assert!(!token_matches(token, "0123456789abcdef0123456789abcdee"));
        assert!(!token_matches(token, "short"));
    }
}
