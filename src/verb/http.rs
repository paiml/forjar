//! The HTTP leg of the unified verb surface.
//!
//! Every verb in [`super::registry`] is reachable here, and it is the SAME
//! declaration the CLI and MCP render — there is no third list. Routes:
//!
//! | route | meaning |
//! |---|---|
//! | `GET  /healthz` | liveness |
//! | `GET  /v1/verbs` | the surface, identical to `forjar verb list --json` |
//! | `GET  /v1/verbs/{name}/schema` | that verb's input and output schema |
//! | `POST /v1/verbs/{name}` | invoke with a JSON body |
//!
//! Request framing is [`crate::core::webhook_http::read_request`], not a second
//! parser written for this module. That code already handles the parts that are
//! easy to get wrong — an 8 KiB head cap answered with 431, `Transfer-Encoding`
//! refused with 501 rather than silently mis-framed, `Content-Length` checked
//! against the body cap BEFORE the body is buffered, and a whole-connection
//! deadline. Writing a second parser would mean getting all of that right
//! twice, and only one of them would be exercised by the webhook tests.
//!
//! This is a LOCAL surface. It binds 127.0.0.1 by default and carries no
//! authentication, so `--bind` on a routable address is an explicit choice with
//! a warning attached. Every verb is `Effects::ReadOnly`, so exposure leaks
//! configuration rather than granting mutation — that is a real distinction,
//! not a reason to relax.

use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

use crate::core::webhook_http::{json_response, read_request, ReadOutcome};

use super::{registry, render_result};

/// Largest request body accepted. Verb params are small; a generous cap here
/// only buys an attacker memory.
const MAX_BODY_BYTES: usize = 256 * 1024;

/// Whole-connection deadline for reading one request.
const READ_DEADLINE: Duration = Duration::from_secs(15);

/// Where to listen.
pub struct HttpConfig {
    /// Interface to bind. 127.0.0.1 unless deliberately changed.
    pub bind: String,
    /// TCP port.
    pub port: u16,
}

/// Serve the verb surface until the process is stopped.
pub fn serve(cfg: HttpConfig) -> Result<(), String> {
    let addr = format!("{}:{}", cfg.bind, cfg.port);
    let listener = TcpListener::bind(&addr).map_err(|e| format!("cannot bind {addr}: {e}"))?;

    if cfg.bind != "127.0.0.1" && cfg.bind != "localhost" {
        eprintln!(
            "warning: serving the verb surface on {} — it has NO authentication. \
             Every forjar verb is read-only, so this exposes configuration, not control.",
            cfg.bind
        );
    }
    eprintln!("forjar verb surface listening on http://{addr}");

    for stream in listener.incoming() {
        match stream {
            // One connection at a time is deliberate: this is a local
            // introspection surface, and a thread per connection would be an
            // unbounded spawn on an unauthenticated port.
            Ok(s) => handle_connection(s),
            Err(e) => eprintln!("accept failed: {e}"),
        }
    }
    Ok(())
}

fn handle_connection(mut stream: TcpStream) {
    let _ = stream.set_read_timeout(Some(READ_DEADLINE));
    let (status, body) = match read_request(&mut stream, MAX_BODY_BYTES, READ_DEADLINE) {
        ReadOutcome::Complete {
            method, path, body, ..
        } => route(&method, &path, &body),
        // The client closed before sending anything; no response is owed.
        ReadOutcome::Empty => return,
        ReadOutcome::Rejected { status, code } => {
            (status, serde_json::json!({ "error": code }).to_string())
        }
    };
    let _ = stream.write_all(&json_response(status, &body));
    let _ = stream.flush();
}

/// Route one request. Split out from the socket so it is testable without one.
pub fn route(method: &str, path: &str, body: &[u8]) -> (u16, String) {
    // Strip a query string; no route here reads one.
    let path = path.split('?').next().unwrap_or(path);

    match (method, path) {
        ("GET", "/healthz") => (200, serde_json::json!({ "status": "ok" }).to_string()),
        ("GET", "/v1/verbs") => (200, render_result(&surface())),
        _ => match path.strip_prefix("/v1/verbs/") {
            Some(rest) => verb_route(method, rest, body),
            None => error(404, "not_found"),
        },
    }
}

fn verb_route(method: &str, rest: &str, body: &[u8]) -> (u16, String) {
    if let Some(name) = rest.strip_suffix("/schema") {
        return match (method, registry::find(name)) {
            ("GET", Some(v)) => (
                200,
                render_result(&serde_json::json!({
                    "name": v.name,
                    "input_schema": (v.input_schema)(),
                    "output_schema": (v.output_schema)(),
                })),
            ),
            // 404 before 405: whether the verb exists is not a secret, and
            // answering 405 for a name that does not exist is misleading.
            (_, None) => error(404, "unknown_verb"),
            _ => error(405, "method_not_allowed"),
        };
    }

    let Some(verb) = registry::find(rest) else {
        return error(404, "unknown_verb");
    };
    if method != "POST" {
        return error(405, "method_not_allowed");
    }

    // An empty body means "no params", matching the CLI's `--json {}` default.
    let raw = if body.is_empty() {
        b"{}".as_slice()
    } else {
        body
    };
    let params: serde_json::Value = match serde_json::from_slice(raw) {
        Ok(v) => v,
        Err(_) => return error(400, "body_is_not_json"),
    };

    match (verb.invoke)(params) {
        Ok(out) => (200, render_result(&out)),
        // FVS-2: a params rejection is the CLIENT's error, so 400. Returning
        // 500 here would tell a caller to retry something that can only fail
        // the same way — the exact confusion the exit-code taxonomy fixes on
        // the CLI side.
        Err(e) if e.starts_with("invalid params") => (
            400,
            serde_json::json!({ "error": "invalid_params" }).to_string(),
        ),
        Err(_) => (
            500,
            serde_json::json!({ "error": "verb_failed" }).to_string(),
        ),
    }
}

/// The surface, in exactly the shape `forjar verb list --json` prints.
fn surface() -> serde_json::Value {
    let rows: Vec<_> = registry::verbs()
        .iter()
        .map(|v| {
            serde_json::json!({
                "name": v.name,
                "mcp_name": v.mcp_name(),
                "description": v.description,
                "read_only": v.effects.read_only(),
                "timeout_ms": v.timeout_ms,
            })
        })
        .collect();
    serde_json::json!({ "verbs": rows })
}

/// Errors carry a fixed code and NEVER echo the request. A 404 that repeats the
/// path back is a reflection primitive on an unauthenticated port.
fn error(status: u16, code: &'static str) -> (u16, String) {
    (status, serde_json::json!({ "error": code }).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_verb_is_404_and_does_not_echo_input() {
        let (status, body) = route("POST", "/v1/verbs/<script>alert(1)</script>", b"{}");
        assert_eq!(status, 404);
        assert!(!body.contains("script"), "the error echoes input: {body}");
    }

    #[test]
    fn get_on_an_invoke_endpoint_is_405() {
        assert_eq!(route("GET", "/v1/verbs/validate", b"").0, 405);
    }

    #[test]
    fn unknown_verb_beats_method_check() {
        // 404, not 405: answering "wrong method" for a name that does not
        // exist tells the caller the route is real when it is not.
        assert_eq!(route("GET", "/v1/verbs/nope", b"").0, 404);
    }

    #[test]
    fn a_body_that_is_not_json_is_400() {
        assert_eq!(route("POST", "/v1/verbs/validate", b"not json").0, 400);
    }

    #[test]
    fn invalid_params_are_400_not_500() {
        let (status, _) = route("POST", "/v1/verbs/validate", br#"{"wrong":1}"#);
        assert_eq!(status, 400, "a params rejection is the client's error");
    }

    #[test]
    fn the_surface_matches_the_registry() {
        let (status, body) = route("GET", "/v1/verbs", b"");
        assert_eq!(status, 200);
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        let names: Vec<&str> = v["verbs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["name"].as_str().unwrap())
            .collect();
        let expected: Vec<&str> = registry::verbs().iter().map(|v| v.name).collect();
        assert_eq!(names, expected);
    }

    #[test]
    fn a_query_string_does_not_break_routing() {
        assert_eq!(route("GET", "/v1/verbs?limit=1", b"").0, 200);
    }
}
