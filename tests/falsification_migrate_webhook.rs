//! FJ-044/3104: Docker→pepita migration and webhook event source.
//!
//! Popperian rejection criteria for:
//! - FJ-044: Docker→pepita resource migration (state mapping, field clearing, warnings)
//! - FJ-044: Config-level migration (selective Docker conversion)
//! - FJ-3104: Webhook request validation (HMAC, body size, path, method)
//! - FJ-3104: JSON payload parsing and event conversion
//! - FJ-3104: HMAC computation and ACK response formatting
//!
//! Usage: cargo test --test falsification_migrate_webhook

use forjar::core::migrate::{docker_to_pepita, migrate_config};
use forjar::core::parser::parse_config;
use forjar::core::types::*;
use forjar::core::webhook_source::{
    ack_response, compute_hmac_hex, parse_json_payload, request_to_event, validate_request,
    ValidationResult, WebhookConfig, WebhookRequest,
};
use std::collections::HashMap;

// ============================================================================
// FJ-044: docker_to_pepita — basic conversion
// ============================================================================

fn docker_resource() -> Resource {
    let yaml = r#"
version: "1.0"
name: test
resources:
  web:
    type: docker
    name: nginx
    image: nginx:latest
    state: running
"#;
    parse_config(yaml).unwrap().resources["web"].clone()
}

#[test]
fn migrate_basic_docker_to_pepita() {
    let docker = docker_resource();
    let result = docker_to_pepita("web", &docker);
    assert_eq!(result.resource.resource_type, ResourceType::Pepita);
    assert_eq!(result.resource.name.as_deref(), Some("nginx"));
    assert_eq!(result.resource.state.as_deref(), Some("present"));
    assert!(result.resource.image.is_none());
}

#[test]
fn migrate_running_maps_to_present() {
    let docker = docker_resource();
    let result = docker_to_pepita("web", &docker);
    assert_eq!(result.resource.state.as_deref(), Some("present"));
}

#[test]
fn migrate_absent_preserved() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  web:
    type: docker
    name: nginx
    image: nginx:latest
    state: absent
"#;
    let docker = parse_config(yaml).unwrap().resources["web"].clone();
    let result = docker_to_pepita("web", &docker);
    assert_eq!(result.resource.state.as_deref(), Some("absent"));
}

#[test]
fn migrate_stopped_maps_to_absent() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  web:
    type: docker
    name: nginx
    image: nginx:latest
    state: stopped
"#;
    let docker = parse_config(yaml).unwrap().resources["web"].clone();
    let result = docker_to_pepita("web", &docker);
    assert_eq!(result.resource.state.as_deref(), Some("absent"));
    assert!(result.warnings.iter().any(|w| w.contains("stopped")));
}

#[test]
fn migrate_unknown_state_defaults_to_present() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  web:
    type: docker
    name: nginx
    image: nginx:latest
    state: restarting
"#;
    let docker = parse_config(yaml).unwrap().resources["web"].clone();
    let result = docker_to_pepita("web", &docker);
    assert_eq!(result.resource.state.as_deref(), Some("present"));
    assert!(result.warnings.iter().any(|w| w.contains("restarting")));
}

// ============================================================================
// FJ-044: docker_to_pepita — ports, volumes, env, restart
// ============================================================================

#[test]
fn migrate_ports_enable_netns() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  web:
    type: docker
    name: nginx
    image: nginx:latest
    ports: ["8080:80"]
"#;
    let docker = parse_config(yaml).unwrap().resources["web"].clone();
    let result = docker_to_pepita("web", &docker);
    assert!(result.resource.netns);
    assert!(result.resource.ports.is_empty());
    assert!(result.warnings.iter().any(|w| w.contains("iptables")));
}

#[test]
fn migrate_image_warning() {
    let docker = docker_resource();
    let result = docker_to_pepita("web", &docker);
    assert!(result.warnings.iter().any(|w| w.contains("nginx:latest")));
    assert!(result.warnings.iter().any(|w| w.contains("overlay_lower")));
}

#[test]
fn migrate_volumes_cleared_with_warning() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  db:
    type: docker
    name: postgres
    image: postgres:16
    volumes: ["/data:/var/lib/postgresql"]
"#;
    let docker = parse_config(yaml).unwrap().resources["db"].clone();
    let result = docker_to_pepita("db", &docker);
    assert!(result.resource.volumes.is_empty());
    assert!(result.warnings.iter().any(|w| w.contains("volumes")));
}

#[test]
fn migrate_env_cleared_with_warning() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  app:
    type: docker
    name: app
    image: app:v1
    environment: ["NODE_ENV=production"]
"#;
    let docker = parse_config(yaml).unwrap().resources["app"].clone();
    let result = docker_to_pepita("app", &docker);
    assert!(result.resource.environment.is_empty());
    assert!(result.warnings.iter().any(|w| w.contains("environment")));
}

#[test]
fn migrate_restart_cleared_with_warning() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  web:
    type: docker
    name: nginx
    image: nginx:latest
    restart: unless-stopped
"#;
    let docker = parse_config(yaml).unwrap().resources["web"].clone();
    let result = docker_to_pepita("web", &docker);
    assert!(result.resource.restart.is_none());
    assert!(result.warnings.iter().any(|w| w.contains("restart")));
}

// ============================================================================
// FJ-044: migrate_config — full config migration
// ============================================================================

#[test]
fn migrate_config_converts_docker_only() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  web:
    type: docker
    name: nginx
    image: nginx:latest
  pkg:
    type: package
    packages: [curl]
"#;
    let cfg = parse_config(yaml).unwrap();
    let (migrated, warnings) = migrate_config(&cfg);
    assert_eq!(
        migrated.resources["web"].resource_type,
        ResourceType::Pepita
    );
    assert_eq!(
        migrated.resources["pkg"].resource_type,
        ResourceType::Package
    );
    assert!(!warnings.is_empty());
}

#[test]
fn migrate_config_no_docker_no_warnings() {
    let yaml = r#"
version: "1.0"
name: test
resources:
  pkg:
    type: package
    packages: [curl]
"#;
    let cfg = parse_config(yaml).unwrap();
    let (migrated, warnings) = migrate_config(&cfg);
    assert!(warnings.is_empty());
    assert_eq!(
        migrated.resources["pkg"].resource_type,
        ResourceType::Package
    );
}

// ============================================================================
// FJ-3104: validate_request — method
// ============================================================================

fn webhook_request(method: &str, path: &str, body: &str) -> WebhookRequest {
    WebhookRequest {
        method: method.into(),
        path: path.into(),
        headers: HashMap::new(),
        body: body.into(),
        source_ip: None,
    }
}

#[test]
fn webhook_valid_post() {
    let config = WebhookConfig::default();
    let req = webhook_request("POST", "/webhook", "{}");
    assert!(validate_request(&config, &req).is_valid());
}

#[test]
fn webhook_get_rejected() {
    let config = WebhookConfig::default();
    let req = webhook_request("GET", "/webhook", "");
    let result = validate_request(&config, &req);
    assert!(matches!(result, ValidationResult::MethodNotAllowed { .. }));
}

// ============================================================================
// FJ-3104: validate_request — body size
// ============================================================================

#[test]
fn webhook_body_too_large() {
    let config = WebhookConfig {
        max_body_bytes: 10,
        ..WebhookConfig::default()
    };
    let req = webhook_request("POST", "/webhook", "a]b".repeat(100).as_str());
    let result = validate_request(&config, &req);
    assert!(matches!(result, ValidationResult::BodyTooLarge { .. }));
}

// ============================================================================
// FJ-3104: validate_request — path
// ============================================================================

#[test]
fn webhook_path_not_allowed() {
    let config = WebhookConfig {
        allowed_paths: vec!["/hooks/deploy".into()],
        ..WebhookConfig::default()
    };
    let req = webhook_request("POST", "/wrong-path", "{}");
    let result = validate_request(&config, &req);
    assert!(matches!(result, ValidationResult::PathNotAllowed { .. }));
}

#[test]
fn webhook_path_allowed() {
    let config = WebhookConfig {
        allowed_paths: vec!["/hooks/deploy".into()],
        ..WebhookConfig::default()
    };
    let req = webhook_request("POST", "/hooks/deploy", "{}");
    assert!(validate_request(&config, &req).is_valid());
}

// ============================================================================
// FJ-3104: validate_request — HMAC signature
// ============================================================================

#[test]
fn webhook_hmac_missing() {
    let config = WebhookConfig {
        secret: Some("mysecret".into()),
        ..WebhookConfig::default()
    };
    let req = webhook_request("POST", "/webhook", "{}");
    assert!(matches!(
        validate_request(&config, &req),
        ValidationResult::SignatureMissing
    ));
}

#[test]
fn webhook_hmac_invalid() {
    let config = WebhookConfig {
        secret: Some("mysecret".into()),
        ..WebhookConfig::default()
    };
    let mut headers = HashMap::new();
    headers.insert("x-forjar-signature".into(), "bad-signature".into());
    let req = WebhookRequest {
        method: "POST".into(),
        path: "/webhook".into(),
        headers,
        body: "{}".into(),
        source_ip: None,
    };
    assert!(matches!(
        validate_request(&config, &req),
        ValidationResult::SignatureInvalid
    ));
}

#[test]
fn webhook_hmac_valid() {
    let secret = "mysecret";
    let body = r#"{"action":"deploy"}"#;
    let sig = compute_hmac_hex(secret, body);

    let config = WebhookConfig {
        secret: Some(secret.into()),
        ..WebhookConfig::default()
    };
    let mut headers = HashMap::new();
    headers.insert("x-forjar-signature".into(), sig);
    let req = WebhookRequest {
        method: "POST".into(),
        path: "/webhook".into(),
        headers,
        body: body.into(),
        source_ip: None,
    };
    assert!(validate_request(&config, &req).is_valid());
}

// ============================================================================
// FJ-3104: compute_hmac_hex — determinism
// ============================================================================

#[test]
fn hmac_deterministic() {
    let h1 = compute_hmac_hex("key", "data");
    let h2 = compute_hmac_hex("key", "data");
    assert_eq!(h1, h2);
}

#[test]
fn hmac_different_keys_different_hashes() {
    let h1 = compute_hmac_hex("key1", "data");
    let h2 = compute_hmac_hex("key2", "data");
    assert_ne!(h1, h2);
}

// ============================================================================
// FJ-3104: parse_json_payload
// ============================================================================

#[test]
fn payload_parse_object() {
    let payload = parse_json_payload(r#"{"action":"deploy","env":"prod"}"#).unwrap();
    assert_eq!(payload["action"], "deploy");
    assert_eq!(payload["env"], "prod");
}

#[test]
fn payload_parse_non_string_values() {
    let payload = parse_json_payload(r#"{"count":42,"active":true}"#).unwrap();
    assert_eq!(payload["count"], "42");
    assert_eq!(payload["active"], "true");
}

#[test]
fn payload_reject_non_object() {
    let result = parse_json_payload("[1,2,3]");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("JSON object"));
}

#[test]
fn payload_reject_invalid_json() {
    let result = parse_json_payload("not json");
    assert!(result.is_err());
}

// ============================================================================
// FJ-3104: request_to_event
// ============================================================================

#[test]
fn request_to_event_adds_metadata() {
    let req = WebhookRequest {
        method: "POST".into(),
        path: "/hooks/deploy".into(),
        headers: HashMap::new(),
        body: r#"{"action":"deploy"}"#.into(),
        source_ip: Some("10.0.0.1".into()),
    };
    let event = request_to_event(&req).unwrap();
    assert_eq!(event.event_type, EventType::WebhookReceived);
    assert_eq!(event.payload["action"], "deploy");
    assert_eq!(event.payload["_path"], "/hooks/deploy");
    assert_eq!(event.payload["_source_ip"], "10.0.0.1");
}

// ============================================================================
// FJ-3104: ack_response
// ============================================================================

#[test]
fn ack_response_format() {
    let resp = ack_response(200, "accepted");
    assert!(resp.contains("HTTP/1.1 200 OK"));
    assert!(resp.contains("application/json"));
    assert!(resp.contains("accepted"));
}

#[test]
fn ack_response_error() {
    let resp = ack_response(401, "unauthorized");
    assert!(resp.contains("401 Unauthorized"));
}
