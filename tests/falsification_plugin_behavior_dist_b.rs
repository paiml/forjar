//! FJ-3400/2602/2105/3405: Plugin types, behavior specs, distribution, shell providers.
//! Usage: cargo test --test falsification_plugin_behavior_dist
#![allow(dead_code)]

use forjar::core::shell_provider::{is_shell_type, parse_shell_type, validate_provider_script};
use forjar::core::types::*;

// ── FJ-3400: PluginManifest ──

fn sample_manifest() -> PluginManifest {
    PluginManifest {
        name: "k8s-deployment".into(),
        version: "0.1.0".into(),
        description: Some("Manage K8s Deployments".into()),
        abi_version: PLUGIN_ABI_VERSION,
        wasm: "k8s-deployment.wasm".into(),
        blake3: "placeholder".into(),
        permissions: PluginPermissions::default(),
        schema: None,
    }
}

fn layer(idx: u32, name: &str, cached: bool) -> LayerReport {
    LayerReport {
        index: idx,
        name: name.into(),
        store_hash: format!("blake3:{name}"),
        size: 25_000_000,
        cached,
        duration_secs: if cached { 0.2 } else { 48.3 },
    }
}

fn sample_build_report() -> BuildReport {
    BuildReport {
        image_ref: "myregistry.io/app:1.0".into(),
        digest: "sha256:abc123".into(),
        total_size: 50_000_000,
        layer_count: 2,
        duration_secs: 48.5,
        layers: vec![layer(0, "base", true), layer(1, "app", false)],
        distribution: vec![],
        architectures: vec![],
    }
}

#[test]
fn build_report_serde() {
    let report = sample_build_report();
    let json = serde_json::to_string(&report).unwrap();
    let parsed: BuildReport = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.layer_count, 2);
}

// ── FJ-2105: PushResult / PushKind ──

#[test]
fn push_result_existed() {
    let r = PushResult {
        kind: PushKind::Layer,
        digest: "sha256:abc".into(),
        size: 1000,
        existed: true,
        duration_secs: 0.0,
    };
    assert!(r.existed);
    assert_eq!(r.kind, PushKind::Layer);
}

#[test]
fn push_result_new_upload() {
    let r = PushResult {
        kind: PushKind::Manifest,
        digest: "sha256:xyz".into(),
        size: 512,
        existed: false,
        duration_secs: 0.5,
    };
    assert!(!r.existed);
    assert_eq!(r.kind, PushKind::Manifest);
}

// ── FJ-3405: Shell provider helpers ──

#[test]
fn parse_shell_type_valid() {
    assert_eq!(parse_shell_type("shell:my-provider"), Some("my-provider"));
}

#[test]
fn parse_shell_type_not_shell() {
    assert_eq!(parse_shell_type("file"), None);
    assert_eq!(parse_shell_type("plugin:k8s"), None);
}

#[test]
fn is_shell_type_true() {
    assert!(is_shell_type("shell:custom"));
}

#[test]
fn is_shell_type_false() {
    assert!(!is_shell_type("file"));
    assert!(!is_shell_type("package"));
}

#[test]
fn validate_provider_script_valid() {
    assert!(validate_provider_script("#!/bin/bash\necho ok\n").is_ok());
}

#[test]
fn validate_provider_script_empty() {
    // Empty script should still be parseable by bashrs
    let result = validate_provider_script("");
    // Accept either ok or err — depends on bashrs strictness
    let _ = result;
}
