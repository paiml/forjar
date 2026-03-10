//! FJ-3104: Webhook source falsification (split from falsification_event_sources).
//!
//! Popperian rejection criteria for:
//! - FJ-3104: Webhook request validation (method, body, path, HMAC)
//! - FJ-3104: JSON payload parsing
//! - FJ-3104: Request-to-event conversion
//!
//! Usage: cargo test --test falsification_event_sources_b

use forjar::core::types::EventType;
use forjar::core::webhook_source::{
    ack_response, compute_hmac_hex, parse_json_payload, request_to_event, validate_request,
    ValidationResult, WebhookConfig, WebhookRequest,
};
use std::collections::HashMap;

// ============================================================================
// Helpers
// ============================================================================

fn webhook_config() -> WebhookConfig {
    WebhookConfig::default()
}

fn post_request(path: &str, body: &str) -> WebhookRequest {
    WebhookRequest {
        method: "POST".into(),
        path: path.into(),
        headers: HashMap::new(),
        body: body.into(),
        source_ip: Some("127.0.0.1".into()),
    }
}

// ============================================================================
// FJ-3104: Webhook Request Validation
// ============================================================================

#[test]
fn webhook_valid_post() {
    let config = webhook_config();
    let req = post_request("/webhook", r#"{"action":"deploy"}"#);
    assert!(validate_request(&config, &req).is_valid());
}

#[test]
fn webhook_method_not_allowed() {
    let config = webhook_config();
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
        ..webhook_config()
    };
    let req = post_request("/webhook", "a long body that exceeds limit");
    match validate_request(&config, &req) {
        ValidationResult::BodyTooLarge { size, max } => {
            assert!(size > max);
        }
        other => panic!("expected BodyTooLarge, got {other:?}"),
    }
}

#[test]
fn webhook_path_not_allowed() {
    let config = webhook_config();
    let req = post_request("/admin/hack", r#"{}"#);
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
        secret: Some("mysecret".into()),
        ..webhook_config()
    };
    let req = post_request("/webhook", r#"{}"#);
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
        ..webhook_config()
    };
    let mut req = post_request("/webhook", body);
    req.headers.insert("x-forjar-signature".into(), sig);
    assert!(validate_request(&config, &req).is_valid());
}

#[test]
fn webhook_signature_invalid() {
    let config = WebhookConfig {
        secret: Some("real-secret".into()),
        ..webhook_config()
    };
    let mut req = post_request("/webhook", r#"{}"#);
    req.headers
        .insert("x-forjar-signature".into(), "bad-sig".into());
    assert_eq!(
        validate_request(&config, &req),
        ValidationResult::SignatureInvalid
    );
}

// ============================================================================
// FJ-3104: HMAC
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
    let h1 = compute_hmac_hex("key1", "data");
    let h2 = compute_hmac_hex("key2", "data");
    assert_ne!(h1, h2);
}

#[test]
fn hmac_different_data() {
    let h1 = compute_hmac_hex("key", "data1");
    let h2 = compute_hmac_hex("key", "data2");
    assert_ne!(h1, h2);
}

// ============================================================================
// FJ-3104: JSON Payload Parsing
// ============================================================================

#[test]
fn parse_json_object() {
    let payload = parse_json_payload(r#"{"action":"deploy","env":"prod"}"#).unwrap();
    assert_eq!(payload.get("action").unwrap(), "deploy");
    assert_eq!(payload.get("env").unwrap(), "prod");
}

#[test]
fn parse_json_nested_stringified() {
    let payload = parse_json_payload(r#"{"count":42,"nested":{"a":1}}"#).unwrap();
    assert_eq!(payload.get("count").unwrap(), "42");
    assert!(payload.get("nested").unwrap().contains("\"a\":1"));
}

#[test]
fn parse_json_invalid() {
    assert!(parse_json_payload("not json").is_err());
}

#[test]
fn parse_json_non_object() {
    assert!(parse_json_payload("[1,2,3]").is_err());
}

#[test]
fn parse_json_empty_object() {
    let payload = parse_json_payload("{}").unwrap();
    assert!(payload.is_empty());
}

// ============================================================================
// FJ-3104: Request to Event
// ============================================================================

#[test]
fn request_to_event_valid() {
    let req = post_request("/webhook", r#"{"action":"restart"}"#);
    let event = request_to_event(&req).unwrap();
    assert_eq!(event.event_type, EventType::WebhookReceived);
    assert_eq!(event.payload.get("action").unwrap(), "restart");
    assert_eq!(event.payload.get("_path").unwrap(), "/webhook");
    assert_eq!(event.payload.get("_source_ip").unwrap(), "127.0.0.1");
}

#[test]
fn request_to_event_no_source_ip() {
    let req = WebhookRequest {
        method: "POST".into(),
        path: "/webhook".into(),
        headers: HashMap::new(),
        body: r#"{"action":"deploy"}"#.into(),
        source_ip: None,
    };
    let event = request_to_event(&req).unwrap();
    assert!(!event.payload.contains_key("_source_ip"));
}

#[test]
fn request_to_event_invalid_body() {
    let req = post_request("/webhook", "not json");
    assert!(request_to_event(&req).is_err());
}

// ============================================================================
// FJ-3104: Ack Response
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
    let resp = ack_response(400, "bad request");
    assert!(resp.contains("400 Bad Request"));
}

#[test]
fn ack_response_401() {
    let resp = ack_response(401, "unauthorized");
    assert!(resp.contains("401 Unauthorized"));
}

// ============================================================================
// FJ-3104: WebhookConfig Default
// ============================================================================

#[test]
fn webhook_config_default() {
    let config = WebhookConfig::default();
    assert_eq!(config.port, 8484);
    assert!(config.secret.is_none());
    assert_eq!(config.max_body_bytes, 64 * 1024);
    assert_eq!(config.allowed_paths, vec!["/webhook"]);
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
