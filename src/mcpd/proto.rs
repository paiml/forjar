//! JSON-RPC 2.0 method handling for the derived MCP server.
//!
//! Three methods are implemented — `initialize`, `tools/list`, `tools/call` —
//! and every tool they describe comes from [`crate::verb`]. There is no tool
//! table in this file.

use crate::verb::{self, VerbCtx, VerbError};
use serde_json::{json, Value};

/// The MCP protocol revision this server speaks.
pub const PROTOCOL_VERSION: &str = "2024-11-05";

/// Handle one JSON-RPC request, returning the response body.
///
/// Returns `None` for a notification (a request with no `id`), which per
/// JSON-RPC 2.0 §4.1 must not be answered.
#[must_use]
pub fn handle(req: &Value, ctx: &VerbCtx) -> Option<Value> {
    let id = req.get("id").cloned();
    let method = req.get("method").and_then(Value::as_str).unwrap_or("");

    // A notification gets no reply, even a failing one.
    id.as_ref()?;

    let result = match method {
        "initialize" => Ok(initialize()),
        "tools/list" => Ok(tools_list()),
        "tools/call" => tools_call(req.get("params"), ctx),
        "ping" => Ok(json!({})),
        _ => Err(VerbError::UnknownVerb(method.to_string())),
    };

    Some(match result {
        Ok(v) => json!({ "jsonrpc": "2.0", "id": id, "result": v }),
        Err(e) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": e.jsonrpc_code(), "message": e.to_string() }
        }),
    })
}

fn initialize() -> Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        "capabilities": { "tools": {} },
        "serverInfo": { "name": "forjar", "version": env!("CARGO_PKG_VERSION") },
    })
}

/// Every invocable verb, as an MCP tool.
///
/// Transport verbs are omitted rather than listed-and-refused: an agent that
/// can see `serve` in `tools/list` will eventually call it, and the honest
/// answer is that it was never available.
fn tools_list() -> Value {
    let tools: Vec<Value> = verb::registry()
        .iter()
        .filter(|v| v.effects.is_invocable())
        .map(|v| {
            json!({
                "name": v.name,
                "description": format!("[{}] {}", v.effects.as_str(), v.description),
                "inputSchema": v.params_schema,
            })
        })
        .collect();
    json!({ "tools": tools })
}

fn tools_call(params: Option<&Value>, ctx: &VerbCtx) -> Result<Value, VerbError> {
    let params = params.ok_or_else(|| VerbError::InvalidParams("missing params".into()))?;
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| VerbError::InvalidParams("missing tool name".into()))?;
    let args = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));

    let envelope = verb::dispatch(name, &args, ctx)?;

    // MCP wants human-readable content plus a failure flag. The envelope is
    // carried verbatim in structuredContent so a client loses nothing.
    let text = envelope["stdout"].as_str().unwrap_or("").to_string();
    let stderr = envelope["stderr"].as_str().unwrap_or("");
    let ok = envelope["ok"].as_bool().unwrap_or(false);
    let body = if text.is_empty() && !stderr.is_empty() {
        stderr.to_string()
    } else {
        text
    };

    Ok(json!({
        "content": [{ "type": "text", "text": body }],
        "isError": !ok,
        "structuredContent": envelope,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn ctx() -> VerbCtx {
        VerbCtx::new(PathBuf::from("/nonexistent/forjar"), PathBuf::from("."))
    }

    fn call(method: &str, params: Value) -> Option<Value> {
        handle(
            &json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params}),
            &ctx(),
        )
    }

    #[test]
    fn initialize_advertises_tools_and_the_protocol_version() {
        let r = call("initialize", json!({})).unwrap();
        assert_eq!(r["result"]["protocolVersion"], PROTOCOL_VERSION);
        assert!(r["result"]["capabilities"]["tools"].is_object());
        assert_eq!(r["result"]["serverInfo"]["name"], "forjar");
    }

    #[test]
    fn tools_list_exposes_every_invocable_verb() {
        let r = call("tools/list", json!({})).unwrap();
        let tools = r["result"]["tools"].as_array().unwrap();
        let expected = verb::registry()
            .iter()
            .filter(|v| v.effects.is_invocable())
            .count();
        assert_eq!(tools.len(), expected);
        assert!(
            tools.len() > 140,
            "expected the full surface, got {}",
            tools.len()
        );
    }

    #[test]
    fn tools_list_omits_transport_verbs_entirely() {
        let r = call("tools/list", json!({})).unwrap();
        let names: Vec<&str> = r["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        for v in ["serve", "mcp", "lsp"] {
            assert!(!names.contains(&v), "{v} must not be listed as a tool");
        }
        assert!(names.contains(&"plan"));
    }

    #[test]
    fn every_listed_tool_carries_an_object_input_schema() {
        let r = call("tools/list", json!({})).unwrap();
        for t in r["result"]["tools"].as_array().unwrap() {
            assert_eq!(t["inputSchema"]["type"], "object", "{}", t["name"]);
            assert!(!t["description"].as_str().unwrap().is_empty());
        }
    }

    #[test]
    fn tool_descriptions_state_the_effect_class() {
        // An agent choosing a tool must be able to see that `destroy` is not
        // like `status` without invoking it.
        let r = call("tools/list", json!({})).unwrap();
        let tools = r["result"]["tools"].as_array().unwrap();
        let destroy = tools.iter().find(|t| t["name"] == "destroy").unwrap();
        assert!(destroy["description"]
            .as_str()
            .unwrap()
            .starts_with("[mutating]"));
        let plan = tools.iter().find(|t| t["name"] == "plan").unwrap();
        assert!(plan["description"]
            .as_str()
            .unwrap()
            .starts_with("[read-only]"));
    }

    #[test]
    fn an_unknown_method_is_method_not_found() {
        let r = call("nope/nope", json!({})).unwrap();
        assert_eq!(r["error"]["code"], -32601);
    }

    #[test]
    fn calling_an_unknown_tool_is_method_not_found() {
        let r = call("tools/call", json!({"name": "no-such-verb"})).unwrap();
        assert_eq!(r["error"]["code"], -32601);
    }

    #[test]
    fn calling_a_transport_verb_is_refused() {
        let r = call("tools/call", json!({"name": "serve"})).unwrap();
        assert_eq!(r["error"]["code"], -32601);
    }

    #[test]
    fn bad_arguments_are_invalid_params_not_internal_error() {
        let r = call(
            "tools/call",
            json!({"name": "plan", "arguments": {"zz": 1}}),
        )
        .unwrap();
        assert_eq!(r["error"]["code"], -32602);
    }

    #[test]
    fn tools_call_without_params_or_name_is_invalid_params() {
        let r = handle(
            &json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call"}),
            &ctx(),
        )
        .unwrap();
        assert_eq!(r["error"]["code"], -32602);
        let r = call("tools/call", json!({})).unwrap();
        assert_eq!(r["error"]["code"], -32602);
    }

    #[test]
    fn a_notification_gets_no_response() {
        // JSON-RPC 2.0 §4.1. Replying to a notification desynchronises a client
        // that is not expecting a message.
        assert!(handle(&json!({"jsonrpc": "2.0", "method": "tools/list"}), &ctx()).is_none());
        assert!(handle(&json!({"jsonrpc": "2.0", "method": "nope"}), &ctx()).is_none());
    }

    #[test]
    fn responses_echo_the_request_id_including_string_ids() {
        for id in [json!(7), json!("abc")] {
            let r = handle(
                &json!({"jsonrpc": "2.0", "id": id, "method": "tools/list"}),
                &ctx(),
            )
            .unwrap();
            assert_eq!(r["id"], id);
            assert_eq!(r["jsonrpc"], "2.0");
        }
    }

    #[test]
    fn ping_is_answered_with_an_empty_result() {
        let r = call("ping", json!({})).unwrap();
        assert!(r["result"].is_object());
        assert!(r.get("error").is_none());
    }
}
