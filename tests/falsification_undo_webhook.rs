//! FJ-2003/3104: Undo plan types and webhook event source.
//! Usage: cargo test --test falsification_undo_webhook

use forjar::core::types::*;
use forjar::core::webhook_source::*;
use std::collections::HashMap;

// ============================================================================
// FJ-2003: UndoPlan
// ============================================================================

fn sample_plan() -> UndoPlan {
    UndoPlan {
        generation_from: 12,
        generation_to: 10,
        machines: vec!["intel".into(), "jetson".into()],
        actions: vec![
            UndoResourceAction {
                resource_id: "new-pkg".into(),
                machine: "intel".into(),
                action: UndoAction::Destroy,
                reversible: true,
            },
            UndoResourceAction {
                resource_id: "old-config".into(),
                machine: "intel".into(),
                action: UndoAction::Create,
                reversible: true,
            },
            UndoResourceAction {
                resource_id: "bash-aliases".into(),
                machine: "jetson".into(),
                action: UndoAction::Update,
                reversible: true,
            },
        ],
        dry_run: false,
    }
}

#[test]
fn undo_plan_counts() {
    let plan = sample_plan();
    assert_eq!(plan.action_count(), 3);
    assert_eq!(plan.destroy_count(), 1);
    assert_eq!(plan.create_count(), 1);
    assert_eq!(plan.update_count(), 1);
}

#[test]
fn undo_plan_no_irreversible() {
    assert!(!sample_plan().has_irreversible());
}

#[test]
fn undo_plan_has_irreversible() {
    let mut plan = sample_plan();
    plan.actions[0].reversible = false;
    assert!(plan.has_irreversible());
}

#[test]
fn undo_plan_format_summary_content() {
    let summary = sample_plan().format_summary();
    assert!(summary.contains("generation 12 → 10"));
    assert!(summary.contains("intel, jetson"));
    assert!(summary.contains("1 destroy"));
    assert!(summary.contains("1 create"));
    assert!(summary.contains("1 update"));
    assert!(summary.contains("[DESTROY]"));
    assert!(summary.contains("[CREATE]"));
    assert!(summary.contains("[UPDATE]"));
}

#[test]
fn undo_plan_dry_run_label() {
    let mut plan = sample_plan();
    plan.dry_run = true;
    assert!(plan.format_summary().contains("dry-run"));
}

#[test]
fn undo_plan_irreversible_warning() {
    let mut plan = sample_plan();
    plan.actions[0].reversible = false;
    assert!(plan.format_summary().contains("IRREVERSIBLE"));
}

#[test]
fn undo_plan_serde_roundtrip() {
    let plan = sample_plan();
    let json = serde_json::to_string(&plan).unwrap();
    let parsed: UndoPlan = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.generation_from, 12);
    assert_eq!(parsed.generation_to, 10);
    assert_eq!(parsed.actions.len(), 3);
}

#[test]
fn undo_plan_empty() {
    let plan = UndoPlan {
        generation_from: 5,
        generation_to: 5,
        machines: vec![],
        actions: vec![],
        dry_run: false,
    };
    assert_eq!(plan.action_count(), 0);
    assert_eq!(plan.destroy_count(), 0);
    assert!(!plan.has_irreversible());
}

// ============================================================================
// FJ-2003: UndoProgress
// ============================================================================

fn sample_progress() -> UndoProgress {
    let mut resources = HashMap::new();
    resources.insert(
        "a".into(),
        ResourceProgress {
            status: ResourceProgressStatus::Completed,
            at: Some("t1".into()),
        },
    );
    resources.insert(
        "b".into(),
        ResourceProgress {
            status: ResourceProgressStatus::Failed {
                error: "timeout".into(),
            },
            at: Some("t2".into()),
        },
    );
    resources.insert(
        "c".into(),
        ResourceProgress {
            status: ResourceProgressStatus::Pending,
            at: None,
        },
    );
    UndoProgress {
        generation_from: 12,
        generation_to: 10,
        started_at: "2026-03-09T12:00:00Z".into(),
        status: UndoStatus::Partial,
        resources,
    }
}

#[test]
fn undo_progress_counts() {
    let p = sample_progress();
    assert_eq!(p.completed_count(), 1);
    assert_eq!(p.failed_count(), 1);
    assert_eq!(p.pending_count(), 1);
}

#[test]
fn undo_progress_partial() {
    let p = sample_progress();
    assert!(!p.is_complete());
    assert!(p.needs_resume());
}

#[test]
fn undo_progress_completed() {
    let p = UndoProgress {
        generation_from: 5,
        generation_to: 3,
        started_at: "t".into(),
        status: UndoStatus::Completed,
        resources: HashMap::new(),
    };
    assert!(p.is_complete());
    assert!(!p.needs_resume());
}

#[test]
fn undo_progress_serde() {
    let p = sample_progress();
    let json = serde_json::to_string(&p).unwrap();
    let parsed: UndoProgress = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.completed_count(), 1);
}

#[test]
fn undo_action_serde() {
    for action in [UndoAction::Destroy, UndoAction::Create, UndoAction::Update] {
        let json = serde_json::to_string(&action).unwrap();
        let parsed: UndoAction = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, action);
    }
}

// ============================================================================
// FJ-3104: WebhookConfig
// ============================================================================

#[test]
fn webhook_config_defaults() {
    let config = WebhookConfig::default();
    assert_eq!(config.port, 8484);
    assert!(config.secret.is_none());
    assert_eq!(config.max_body_bytes, 64 * 1024);
    assert_eq!(config.allowed_paths, vec!["/webhook"]);
}

// ============================================================================
// FJ-3104: validate_request
// ============================================================================

fn post_request(path: &str, body: &str) -> WebhookRequest {
    WebhookRequest {
        method: "POST".into(),
        path: path.into(),
        headers: HashMap::new(),
        body: body.into(),
        source_ip: Some("127.0.0.1".into()),
    }
}

#[test]
fn webhook_valid_post() {
    let config = WebhookConfig::default();
    let req = post_request("/webhook", r#"{"action":"deploy"}"#);
    assert!(validate_request(&config, &req).is_valid());
}

#[test]
fn webhook_method_not_allowed() {
    let config = WebhookConfig::default();
    let req = WebhookRequest {
        method: "GET".into(),
        path: "/webhook".into(),
        headers: HashMap::new(),
        body: String::new(),
        source_ip: None,
    };
    assert_eq!(
        validate_request(&config, &req),
        ValidationResult::MethodNotAllowed {
            method: "GET".into()
        }
    );
}

#[test]
fn webhook_body_too_large() {
    let config = WebhookConfig {
        max_body_bytes: 10,
        ..Default::default()
    };
    let req = post_request("/webhook", "a]long body that exceeds limit");
    match validate_request(&config, &req) {
        ValidationResult::BodyTooLarge { size, max } => assert!(size > max),
        other => panic!("expected BodyTooLarge, got {other:?}"),
    }
}

#[test]
fn webhook_path_not_allowed() {
    let config = WebhookConfig::default();
    let req = post_request("/admin/hack", "{}");
    assert_eq!(
        validate_request(&config, &req),
        ValidationResult::PathNotAllowed {
            path: "/admin/hack".into()
        }
    );
}

#[test]
fn webhook_signature_missing() {
    let config = WebhookConfig {
        secret: Some("secret".into()),
        ..Default::default()
    };
    let req = post_request("/webhook", "{}");
    assert_eq!(
        validate_request(&config, &req),
        ValidationResult::SignatureMissing
    );
}

#[test]
fn webhook_signature_valid() {
    let secret = "test-secret";
    let body = r#"{"deploy":true}"#;
    let sig = compute_hmac_hex(secret, body);
    let config = WebhookConfig {
        secret: Some(secret.into()),
        ..Default::default()
    };
    let mut req = post_request("/webhook", body);
    req.headers.insert("x-forjar-signature".into(), sig);
    assert!(validate_request(&config, &req).is_valid());
}

#[test]
fn webhook_signature_invalid() {
    let config = WebhookConfig {
        secret: Some("real".into()),
        ..Default::default()
    };
    let mut req = post_request("/webhook", "{}");
    req.headers
        .insert("x-forjar-signature".into(), "bad".into());
    assert_eq!(
        validate_request(&config, &req),
        ValidationResult::SignatureInvalid
    );
}

// ============================================================================
// FJ-3104: parse_json_payload
// ============================================================================

#[test]
fn parse_json_object() {
    let payload = parse_json_payload(r#"{"action":"deploy","env":"prod"}"#).unwrap();
    assert_eq!(payload["action"], "deploy");
    assert_eq!(payload["env"], "prod");
}

#[test]
fn parse_json_nested_stringified() {
    let payload = parse_json_payload(r#"{"count":42}"#).unwrap();
    assert_eq!(payload["count"], "42");
}

#[test]
fn parse_json_invalid() {
    assert!(parse_json_payload("not json").is_err());
}

#[test]
fn parse_json_array_rejected() {
    assert!(parse_json_payload("[1,2,3]").is_err());
}

// ============================================================================
// FJ-3104: request_to_event
// ============================================================================

#[test]
fn request_to_event_valid() {
    let req = post_request("/webhook", r#"{"action":"restart"}"#);
    let event = request_to_event(&req).unwrap();
    assert_eq!(event.event_type, EventType::WebhookReceived);
    assert_eq!(event.payload["action"], "restart");
    assert_eq!(event.payload["_path"], "/webhook");
    assert_eq!(event.payload["_source_ip"], "127.0.0.1");
}

#[test]
fn request_to_event_invalid() {
    let req = post_request("/webhook", "not json");
    assert!(request_to_event(&req).is_err());
}

#[test]
fn request_to_event_no_source_ip() {
    let req = WebhookRequest {
        method: "POST".into(),
        path: "/webhook".into(),
        headers: HashMap::new(),
        body: r#"{"key":"val"}"#.into(),
        source_ip: None,
    };
    let event = request_to_event(&req).unwrap();
    assert!(!event.payload.contains_key("_source_ip"));
}

// ============================================================================
// FJ-3104: compute_hmac_hex
// ============================================================================

#[test]
fn hmac_deterministic() {
    let h1 = compute_hmac_hex("key", "data");
    let h2 = compute_hmac_hex("key", "data");
    assert_eq!(h1, h2);
    assert_eq!(h1.len(), 64);
}

#[test]
fn hmac_different_keys() {
    assert_ne!(
        compute_hmac_hex("k1", "data"),
        compute_hmac_hex("k2", "data")
    );
}

#[test]
fn hmac_different_data() {
    assert_ne!(compute_hmac_hex("key", "d1"), compute_hmac_hex("key", "d2"));
}

// ============================================================================
// FJ-3104: ack_response
// ============================================================================

#[test]
fn ack_response_200() {
    let resp = ack_response(200, "accepted");
    assert!(resp.starts_with("HTTP/1.1 200 OK"));
    assert!(resp.contains("application/json"));
    assert!(resp.contains("accepted"));
}

#[test]
fn ack_response_400() {
    let resp = ack_response(400, "bad");
    assert!(resp.contains("400 Bad Request"));
}

#[test]
fn ack_response_401() {
    let resp = ack_response(401, "unauthorized");
    assert!(resp.contains("401 Unauthorized"));
}

// ============================================================================
// FJ-3104: ValidationResult
// ============================================================================

#[test]
fn validation_result_is_valid() {
    assert!(ValidationResult::Valid.is_valid());
    assert!(!ValidationResult::SignatureMissing.is_valid());
    assert!(!ValidationResult::SignatureInvalid.is_valid());
}
