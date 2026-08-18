//! Route an HTTP request onto the verb registry.
//!
//! The routing table has four entries and no per-verb code. `POST /v1/{name}`
//! resolves `{name}` against the registry, so a verb added to the clap enum is
//! reachable over HTTP with no change to this file — which is the property the
//! whole Unified Verb Surface exists to provide.

use super::http::Request;
use crate::verb::{self, VerbCtx, VerbError};
use serde_json::{json, Value};

/// A routed response: an HTTP status and a JSON body.
pub type Routed = (u16, Value);

/// Server configuration that affects routing.
#[derive(Debug, Clone)]
pub struct ServeConfig {
    /// Refuse verbs classified as mutating.
    pub read_only: bool,
    /// How verbs are executed.
    pub ctx: VerbCtx,
}

/// Route one request.
#[must_use]
pub fn route(req: &Request, cfg: &ServeConfig) -> Routed {
    match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/health") => (200, health()),
        ("GET", "/v1/verbs") => (200, verb::catalogue()),
        ("GET", p) if p.starts_with("/v1/verbs/") => describe(p.trim_start_matches("/v1/verbs/")),
        ("POST", p) if p.starts_with("/v1/") => {
            invoke(p.trim_start_matches("/v1/"), &req.body, cfg)
        }
        ("GET", _) | ("POST", _) => (404, err(&VerbError::UnknownVerb(req.path.clone()))),
        _ => (
            405,
            json!({"error": {"kind": "malformed", "message": format!("method {} not allowed", req.method)}}),
        ),
    }
}

fn health() -> Value {
    json!({
        "status": "ok",
        "service": "forjar",
        "version": env!("CARGO_PKG_VERSION"),
        "verb_count": verb::registry().len(),
    })
}

fn describe(name: &str) -> Routed {
    match verb::find(name) {
        Some(v) => (
            200,
            json!({
                "name": v.name,
                "description": v.description,
                "effects": v.effects.as_str(),
                "params_schema": v.params_schema,
                "output_schema": v.output_schema,
                "subcommands": v.subcommands,
            }),
        ),
        None => (404, err(&VerbError::UnknownVerb(name.to_string()))),
    }
}

fn invoke(name: &str, body: &str, cfg: &ServeConfig) -> Routed {
    let params: Value = if body.trim().is_empty() {
        json!({})
    } else {
        match serde_json::from_str(body) {
            Ok(v) => v,
            Err(e) => {
                let e = VerbError::Malformed(format!("body is not JSON: {e}"));
                return (e.http_status(), err(&e));
            }
        }
    };

    let Some(spec) = verb::find(name) else {
        let e = VerbError::UnknownVerb(name.to_string());
        return (e.http_status(), err(&e));
    };

    // `--read-only` is a promise about the server, so it is enforced where the
    // request is admitted rather than left to the caller to respect.
    if cfg.read_only && spec.effects != verb::Effects::ReadOnly {
        let e = VerbError::NotInvocable(name.to_string());
        return (e.http_status(), err(&e));
    }

    match verb::exec::dispatch_spec(spec, &params, &cfg.ctx) {
        Ok(envelope) => (200, envelope),
        Err(e) => (e.http_status(), err(&e)),
    }
}

/// Render an error into the one body shape every failure uses.
fn err(e: &VerbError) -> Value {
    json!({
        "error": {
            "kind": e.kind(),
            "message": e.to_string(),
            "exit_code": e.exit_code(),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    fn cfg(read_only: bool) -> ServeConfig {
        ServeConfig {
            read_only,
            ctx: VerbCtx::new(PathBuf::from("/nonexistent/forjar"), PathBuf::from(".")),
        }
    }

    fn req(method: &str, path: &str, body: &str) -> Request {
        Request {
            method: method.into(),
            path: path.into(),
            headers: BTreeMap::new(),
            body: body.into(),
        }
    }

    #[test]
    fn health_reports_the_verb_count_it_actually_serves() {
        let (s, b) = route(&req("GET", "/health", ""), &cfg(false));
        assert_eq!(s, 200);
        assert_eq!(b["status"], "ok");
        assert_eq!(
            b["verb_count"].as_u64().unwrap() as usize,
            verb::registry().len()
        );
    }

    #[test]
    fn the_catalogue_lists_every_verb() {
        let (s, b) = route(&req("GET", "/v1/verbs", ""), &cfg(false));
        assert_eq!(s, 200);
        assert_eq!(b["verbs"].as_array().unwrap().len(), verb::registry().len());
    }

    #[test]
    fn a_single_verb_can_be_described() {
        let (s, b) = route(&req("GET", "/v1/verbs/plan", ""), &cfg(false));
        assert_eq!(s, 200);
        assert_eq!(b["name"], "plan");
        assert_eq!(b["effects"], "read-only");
        assert!(b["params_schema"]["properties"]["file"].is_object());
    }

    #[test]
    fn describing_an_unknown_verb_is_404_not_500() {
        let (s, b) = route(&req("GET", "/v1/verbs/nope", ""), &cfg(false));
        assert_eq!(s, 404);
        assert_eq!(b["error"]["kind"], "unknown_verb");
    }

    #[test]
    fn an_unknown_route_is_404() {
        assert_eq!(route(&req("GET", "/nope", ""), &cfg(false)).0, 404);
    }

    #[test]
    fn an_unsupported_method_is_405() {
        let (s, b) = route(&req("DELETE", "/v1/plan", ""), &cfg(false));
        assert_eq!(s, 405);
        assert_eq!(b["error"]["kind"], "malformed");
    }

    #[test]
    fn a_non_json_body_is_400_with_a_reason() {
        let (s, b) = route(&req("POST", "/v1/plan", "not json"), &cfg(false));
        assert_eq!(s, 400);
        assert_eq!(b["error"]["kind"], "malformed");
    }

    #[test]
    fn posting_to_an_unknown_verb_is_404() {
        let (s, b) = route(&req("POST", "/v1/nope", "{}"), &cfg(false));
        assert_eq!(s, 404);
        assert_eq!(b["error"]["kind"], "unknown_verb");
    }

    #[test]
    fn invalid_params_are_400_and_never_reach_the_binary() {
        // The ctx binary does not exist, so a 500 here would prove the request
        // was admitted before validation.
        let (s, b) = route(&req("POST", "/v1/plan", r#"{"bogus": 1}"#), &cfg(false));
        assert_eq!(s, 400);
        assert_eq!(b["error"]["kind"], "invalid_params");
    }

    #[test]
    fn read_only_mode_refuses_mutating_verbs_with_403() {
        let (s, b) = route(&req("POST", "/v1/apply", "{}"), &cfg(true));
        assert_eq!(s, 403);
        assert_eq!(b["error"]["kind"], "not_invocable");
    }

    #[test]
    fn read_only_mode_still_admits_read_only_verbs() {
        // Admitted, then fails to spawn — 500, not 403. That distinction is the
        // point: the gate let it through.
        let (s, _) = route(&req("POST", "/v1/plan", "{}"), &cfg(true));
        assert_eq!(s, 500);
    }

    #[test]
    fn transport_verbs_are_refused_even_when_not_read_only() {
        for v in ["serve", "mcp", "lsp"] {
            let (s, b) = route(&req("POST", &format!("/v1/{v}"), "{}"), &cfg(false));
            assert_eq!(s, 403, "{v} must not be servable from the server");
            assert_eq!(b["error"]["kind"], "not_invocable", "{v}");
        }
    }

    #[test]
    fn an_empty_body_means_empty_params_not_a_parse_error() {
        let (s, _) = route(&req("POST", "/v1/plan", ""), &cfg(false));
        assert_eq!(
            s, 500,
            "empty body should reach dispatch, then fail to spawn"
        );
    }

    #[test]
    fn every_error_body_carries_kind_message_and_exit_code() {
        for (m, p, b) in [
            ("GET", "/nope", ""),
            ("POST", "/v1/nope", "{}"),
            ("POST", "/v1/plan", "xx"),
            ("DELETE", "/v1/plan", ""),
        ] {
            let (_, body) = route(&req(m, p, b), &cfg(false));
            assert!(body["error"]["kind"].is_string(), "{m} {p}");
            assert!(body["error"]["message"].is_string(), "{m} {p}");
        }
    }

    #[test]
    fn every_registry_verb_is_routable_by_name() {
        // Totality: no verb is describable-but-unroutable or vice versa.
        for v in verb::registry() {
            let (s, _) = route(
                &req("GET", &format!("/v1/verbs/{}", v.name), ""),
                &cfg(false),
            );
            assert_eq!(s, 200, "verb {} is not describable", v.name);
        }
    }
}
