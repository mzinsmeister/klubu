use super::tools::{tool_definitions, ToolService, OPERATING_INSTRUCTIONS};
use serde_json::{json, Value};

const LATEST_PROTOCOL: &str = "2025-11-25";
// Streamable HTTP is the only transport, and it entered the spec with
// 2025-03-26. A client requesting an older version gets the newest one
// offered back per the negotiation rules and can disconnect if incompatible.
pub(crate) const SUPPORTED_PROTOCOLS: &[&str] = &["2025-11-25", "2025-06-18", "2025-03-26"];

fn success(id: Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

pub(crate) fn error(
    id: Value,
    code: i64,
    message: impl Into<String>,
    data: Option<Value>,
) -> Value {
    let mut body = json!({"code": code, "message": message.into()});
    if let Some(data) = data {
        body["data"] = data;
    }
    json!({"jsonrpc": "2.0", "id": id, "error": body})
}

fn initialize(id: Value, params: &Value) -> Value {
    let requested = params
        .get("protocolVersion")
        .and_then(Value::as_str)
        .unwrap_or(LATEST_PROTOCOL);
    let protocol = if SUPPORTED_PROTOCOLS.contains(&requested) {
        requested
    } else {
        LATEST_PROTOCOL
    };
    success(
        id,
        json!({
            "protocolVersion": protocol,
            "capabilities": {
                "tools": {"listChanged": false},
                "resources": {"subscribe": false, "listChanged": false}
            },
            "serverInfo": {"name": "klubu", "title": "Klubu autonomous operations", "version": env!("CARGO_PKG_VERSION")},
            "instructions": OPERATING_INSTRUCTIONS
        }),
    )
}

fn resource_list() -> Value {
    json!({
        "resources": [
            {
                "uri": "klubu://operating-guide",
                "name": "Klubu operating guide",
                "title": "Autonomous operation rules and workflows",
                "description": "Stable rules an agent must follow when operating Klubu.",
                "mimeType": "text/markdown"
            },
            {
                "uri": "klubu://current-session",
                "name": "Current MCP session",
                "title": "Current actor and database session",
                "description": "The authenticated Klubu identity used for audit and mailbox scoping.",
                "mimeType": "application/json"
            },
            {
                "uri": "klubu://dashboard",
                "name": "Dashboard",
                "title": "Current business dashboard",
                "description": "Live Klubu dashboard totals.",
                "mimeType": "application/json"
            }
        ]
    })
}

/// `service` is `Err` when no Klubu actor could be resolved yet (for example
/// before the admin account exists). The handshake and discovery methods must
/// still work then; only the methods that execute as a user report the error.
async fn handle_request(service: &Result<ToolService, String>, request: Value) -> Option<Value> {
    let Some(object) = request.as_object() else {
        return Some(error(Value::Null, -32600, "Invalid Request", None));
    };
    if object.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return Some(error(Value::Null, -32600, "Invalid Request", None));
    }
    let method = object.get("method").and_then(Value::as_str);
    let id = object.get("id").cloned();

    // JSON-RPC notifications never receive a response.
    id.as_ref()?;
    let id = id.unwrap_or(Value::Null);
    let params = object.get("params").cloned().unwrap_or_else(|| json!({}));

    Some(match method {
        Some("initialize") => initialize(id, &params),
        Some("ping") => success(id, json!({})),
        Some("tools/list") => success(id, json!({"tools": tool_definitions()})),
        Some("tools/call") => {
            let service = match service {
                Ok(service) => service,
                Err(message) => return Some(error(id, -32000, message.clone(), None)),
            };
            let Some(name) = params.get("name").and_then(Value::as_str) else {
                return Some(error(id, -32602, "Missing tool name", None));
            };
            let arguments = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            match service.call(name, arguments).await {
                Ok(value) => {
                    let text =
                        serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string());
                    success(
                        id,
                        json!({
                            "content": [{"type": "text", "text": text}],
                            "structuredContent": {"result": value},
                            "isError": false
                        }),
                    )
                }
                Err(message) => success(
                    id,
                    json!({
                        "content": [{"type": "text", "text": message}],
                        "isError": true
                    }),
                ),
            }
        }
        Some("resources/list") => success(id, resource_list()),
        Some("resources/read") => {
            let service = match service {
                Ok(service) => service,
                Err(message) => return Some(error(id, -32000, message.clone(), None)),
            };
            let uri = params.get("uri").and_then(Value::as_str).unwrap_or("");
            match service.read_resource(uri).await {
                Ok((mime_type, text)) => success(
                    id,
                    json!({"contents": [{"uri": uri, "mimeType": mime_type, "text": text}]}),
                ),
                Err(message) => error(id, -32002, message, None),
            }
        }
        Some(method) => error(id, -32601, format!("Method not found: {method}"), None),
        None => error(id, -32600, "Invalid Request", None),
    })
}

pub(crate) async fn handle_message(
    service: &Result<ToolService, String>,
    message: Value,
) -> Option<Value> {
    if let Some(batch) = message.as_array() {
        if batch.is_empty() {
            return Some(error(Value::Null, -32600, "Invalid Request", None));
        }
        let mut responses = Vec::new();
        for request in batch.iter().cloned() {
            if let Some(response) = handle_request(service, request).await {
                responses.push(response);
            }
        }
        (!responses.is_empty()).then_some(Value::Array(responses))
    } else {
        handle_request(service, message).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negotiates_current_and_legacy_protocols() {
        for version in SUPPORTED_PROTOCOLS {
            let response = initialize(json!(1), &json!({"protocolVersion": version}));
            assert_eq!(response["result"]["protocolVersion"], *version);
        }
        let response = initialize(json!(1), &json!({"protocolVersion": "unknown"}));
        assert_eq!(response["result"]["protocolVersion"], LATEST_PROTOCOL);
    }

    #[test]
    fn every_tool_name_is_unique_and_has_an_object_schema() {
        let definitions = tool_definitions();
        let mut names = std::collections::HashSet::new();
        for tool in definitions {
            assert!(names.insert(tool["name"].as_str().unwrap().to_string()));
            assert_eq!(tool["inputSchema"]["type"], "object");
            assert!(tool["description"].as_str().unwrap().len() > 20);
        }
    }
}
