use crate::{protocol, tools::ToolService};
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
use std::{collections::HashSet, net::SocketAddr, sync::Arc};

const DEFAULT_BIND: &str = "127.0.0.1:8090";
const DEFAULT_BODY_LIMIT_MIB: usize = 75;

#[derive(Clone)]
struct HttpState {
    service: ToolService,
    bearer_token: Arc<str>,
    allowed_origins: Arc<HashSet<String>>,
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
    State(state): State<HttpState>,
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
    State(state): State<HttpState>,
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
            .map(|version| !matches!(version, "2025-03-26" | "2025-06-18" | "2025-11-25"))
            .unwrap_or(false)
    {
        return response(
            StatusCode::BAD_REQUEST,
            Some("text/plain;charset=utf-8"),
            Body::from("Unsupported MCP-Protocol-Version"),
        );
    }
    if is_initialize
        && message
            .get("params")
            .and_then(|params| params.get("protocolVersion"))
            .and_then(Value::as_str)
            == Some("2024-11-05")
    {
        return response(
            StatusCode::BAD_REQUEST,
            Some("text/plain;charset=utf-8"),
            Body::from("HTTP mode requires MCP protocol 2025-03-26 or newer"),
        );
    }

    match protocol::handle_message(&state.service, message).await {
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

fn allowed_origins() -> HashSet<String> {
    std::env::var("KLUBU_MCP_ALLOWED_ORIGINS")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|origin| !origin.is_empty())
        .map(str::to_string)
        .collect()
}

pub async fn serve(service: ToolService) -> Result<(), String> {
    let bearer_token = std::env::var("KLUBU_MCP_TOKEN")
        .map_err(|_| "KLUBU_MCP_TOKEN is required in HTTP mode".to_string())?;
    if bearer_token.len() < 32 {
        return Err(
            "KLUBU_MCP_TOKEN must contain at least 32 characters of random data".to_string(),
        );
    }

    let bind = std::env::var("KLUBU_MCP_BIND").unwrap_or_else(|_| DEFAULT_BIND.to_string());
    let address: SocketAddr = bind
        .parse()
        .map_err(|error| format!("Invalid KLUBU_MCP_BIND '{bind}': {error}"))?;
    if !address.ip().is_loopback()
        && !std::env::var("KLUBU_MCP_ALLOW_NON_LOOPBACK")
            .map(|value| matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
            .unwrap_or(false)
    {
        return Err(format!(
            "Refusing non-loopback KLUBU_MCP_BIND '{address}'. Put the server behind a local HTTPS reverse proxy, or set KLUBU_MCP_ALLOW_NON_LOOPBACK=true only when a trusted private network or TLS proxy protects the connection."
        ));
    }

    let body_limit_mib = std::env::var("KLUBU_MCP_BODY_LIMIT_MIB")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(DEFAULT_BODY_LIMIT_MIB)
        .clamp(1, 512);
    let state = HttpState {
        service,
        bearer_token: Arc::from(bearer_token),
        allowed_origins: Arc::new(allowed_origins()),
    };
    let router = Router::new()
        .route("/mcp", post(post_mcp).get(get_mcp).delete(delete_mcp))
        .layer(DefaultBodyLimit::max(body_limit_mib * 1024 * 1024))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            authorize_and_validate_origin,
        ))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(address)
        .await
        .map_err(|error| format!("Could not bind Klubu MCP HTTP server to {address}: {error}"))?;
    eprintln!("Klubu MCP Streamable HTTP endpoint listening on http://{address}/mcp");
    axum::serve(listener, router)
        .await
        .map_err(|error| format!("Klubu MCP HTTP server failed: {error}"))
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
