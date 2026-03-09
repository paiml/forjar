//! FJ-3303/051/2003/3104: Encryption, MC/DC, undo plans, webhooks.
//!
//! Demonstrates:
//! - BLAKE3 hashing, keyed HMAC, key derivation, metadata
//! - MC/DC test pair generation (AND/OR decisions)
//! - Undo plan creation and formatting
//! - Webhook request validation and event conversion
//!
//! Usage: cargo run --example crypto_mcdc_undo_webhook

use forjar::core::mcdc::{build_decision, generate_mcdc_and, generate_mcdc_or};
use forjar::core::state_encryption::*;
use forjar::core::types::*;
use forjar::core::webhook_source::*;
use std::collections::HashMap;

fn main() {
    println!("Forjar: Crypto, MC/DC, Undo & Webhooks");
    println!("{}", "=".repeat(50));

    // ── State Encryption ──
    println!("\n[FJ-3303] State Encryption:");
    let h = hash_data(b"resource state yaml");
    println!("  BLAKE3 hash: {}...{}", &h[..8], &h[56..]);

    let key = derive_key("my-secret-passphrase");
    let hmac = keyed_hash(b"ciphertext data", &key);
    println!("  HMAC: {}...{}", &hmac[..8], &hmac[56..]);
    println!(
        "  Verify: {}",
        verify_keyed_hash(b"ciphertext data", &key, &hmac)
    );

    let meta = create_metadata(b"plaintext", b"ciphertext", &key);
    println!(
        "  Metadata v{}, verified={}",
        meta.version,
        verify_metadata(&meta, b"ciphertext", &key)
    );

    // ── MC/DC ──
    println!("\n[FJ-051] MC/DC Analysis:");
    let and_decision = build_decision(
        "ready && approved && tested",
        &["ready", "approved", "tested"],
    );
    let and_report = generate_mcdc_and(&and_decision);
    println!(
        "  AND '{}': {} pairs, {} min tests",
        and_report.decision,
        and_report.pairs.len(),
        and_report.min_tests_needed
    );
    for pair in &and_report.pairs {
        println!(
            "    {}: T={:?} F={:?}",
            pair.condition, pair.true_case, pair.false_case
        );
    }

    let or_decision = build_decision("error || timeout", &["error", "timeout"]);
    let or_report = generate_mcdc_or(&or_decision);
    println!(
        "  OR '{}': {} pairs, achievable={}",
        or_report.decision,
        or_report.pairs.len(),
        or_report.coverage_achievable
    );

    // ── Undo Plans ──
    println!("\n[FJ-2003] Undo Plan:");
    let plan = UndoPlan {
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
                machine: "jetson".into(),
                action: UndoAction::Create,
                reversible: true,
            },
            UndoResourceAction {
                resource_id: "sshd".into(),
                machine: "intel".into(),
                action: UndoAction::Update,
                reversible: false,
            },
        ],
        dry_run: false,
    };
    println!("{}", plan.format_summary());

    // ── Webhooks ──
    println!("[FJ-3104] Webhook Validation:");
    let config = WebhookConfig::default();
    let req = WebhookRequest {
        method: "POST".into(),
        path: "/webhook".into(),
        headers: HashMap::new(),
        body: r#"{"action":"deploy","env":"prod"}"#.into(),
        source_ip: Some("10.0.0.5".into()),
    };
    println!("  Valid POST: {:?}", validate_request(&config, &req));

    let event = request_to_event(&req).unwrap();
    println!("  Event type: {}", event.event_type);
    println!("  Payload action: {}", event.payload["action"]);
    println!("  HMAC: {}...", &compute_hmac_hex("secret", "data")[..16]);
    println!("  ACK: {}", ack_response(200, "ok").lines().next().unwrap());

    println!("\n{}", "=".repeat(50));
    println!("All crypto/mcdc/undo/webhook criteria survived.");
}
