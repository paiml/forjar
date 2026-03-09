//! FJ-044/3104: Docker→pepita migration and webhook event source.
//!
//! Demonstrates:
//! - Docker→pepita migration with field mapping and warnings
//! - Config-level migration (selective Docker conversion)
//! - Webhook request validation (HMAC, body size, path, method)
//! - JSON payload parsing and event conversion
//!
//! Usage: cargo run --example migrate_webhook

use forjar::core::migrate::{docker_to_pepita, migrate_config};
use forjar::core::parser::parse_config;
use forjar::core::types::*;
use forjar::core::webhook_source::{
    compute_hmac_hex, parse_json_payload, request_to_event, validate_request, WebhookConfig,
    WebhookRequest,
};
use std::collections::HashMap;

fn main() {
    println!("Forjar: Docker Migration & Webhook Events");
    println!("{}", "=".repeat(50));

    // ── FJ-044: Docker → pepita migration ──
    println!("\n[FJ-044] Docker → Pepita Migration:");
    let yaml = r#"
version: "1.0"
name: migration-demo
resources:
  web:
    type: docker
    name: nginx
    image: nginx:latest
    ports: ["8080:80"]
    environment: ["NODE_ENV=production"]
    volumes: ["/data:/app/data"]
    restart: unless-stopped
  tools:
    type: package
    packages: [curl, jq]
"#;
    let cfg = parse_config(yaml).unwrap();
    let (migrated, warnings) = migrate_config(&cfg);

    println!("  Before: web={:?}", cfg.resources["web"].resource_type);
    println!(
        "  After:  web={:?}",
        migrated.resources["web"].resource_type
    );
    println!(
        "  tools unchanged: {:?}",
        migrated.resources["tools"].resource_type
    );
    assert_eq!(
        migrated.resources["web"].resource_type,
        ResourceType::Pepita
    );
    assert_eq!(
        migrated.resources["tools"].resource_type,
        ResourceType::Package
    );

    println!("  Migration warnings ({}):", warnings.len());
    for w in &warnings {
        println!("    - {w}");
    }
    assert!(warnings.len() >= 4);

    // Single resource migration
    let docker = &cfg.resources["web"];
    let result = docker_to_pepita("web", docker);
    assert!(result.resource.netns, "ports → netns");
    assert!(result.resource.ports.is_empty());
    assert!(result.resource.image.is_none());

    // ── FJ-3104: Webhook Validation ──
    println!("\n[FJ-3104] Webhook Request Validation:");
    let config = WebhookConfig {
        port: 8484,
        secret: Some("deploy-secret".into()),
        max_body_bytes: 1024,
        allowed_paths: vec!["/hooks/deploy".into()],
    };

    // Valid request with HMAC
    let body = r#"{"action":"deploy","env":"production"}"#;
    let sig = compute_hmac_hex("deploy-secret", body);
    let mut headers = HashMap::new();
    headers.insert("x-forjar-signature".into(), sig);
    let req = WebhookRequest {
        method: "POST".into(),
        path: "/hooks/deploy".into(),
        headers,
        body: body.into(),
        source_ip: Some("10.0.0.1".into()),
    };
    let vr = validate_request(&config, &req);
    println!("  Valid POST with HMAC: {:?}", vr);
    assert!(vr.is_valid());

    // Invalid method
    let bad_req = WebhookRequest {
        method: "GET".into(),
        path: "/hooks/deploy".into(),
        headers: HashMap::new(),
        body: "".into(),
        source_ip: None,
    };
    let vr = validate_request(&config, &bad_req);
    println!("  GET rejected: {:?}", vr);
    assert!(!vr.is_valid());

    // ── FJ-3104: Payload parsing ──
    println!("\n[FJ-3104] Webhook Payload Parsing:");
    let payload = parse_json_payload(body).unwrap();
    println!("  action={}, env={}", payload["action"], payload["env"]);
    assert_eq!(payload["action"], "deploy");

    let event = request_to_event(&req).unwrap();
    println!("  Event type: {:?}", event.event_type);
    println!(
        "  Payload keys: {:?}",
        event.payload.keys().collect::<Vec<_>>()
    );
    assert_eq!(event.event_type, EventType::WebhookReceived);
    assert_eq!(event.payload["_source_ip"], "10.0.0.1");

    println!("\n{}", "=".repeat(50));
    println!("All migration/webhook criteria survived.");
}
