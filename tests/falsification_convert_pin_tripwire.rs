//! FJ-1328/1314/1364: Recipe conversion, pin tripwire, and pin resolution.
//!
//! Popperian rejection criteria for:
//! - FJ-1328: analyze_conversion (5-step ladder, purity classification, projections)
//! - FJ-1314: check_before_apply (stale/missing/fresh detection)
//! - FJ-1314: pin_severity (Info, Warning, Error modes)
//! - FJ-1314: format_pin_report (human-readable output)
//! - FJ-1314: needs_pin_update (boolean gate)
//! - FJ-1364: resolution_command (provider-specific CLI commands)
//! - FJ-1364: parse_resolved_version (apt, cargo, pip, nix, docker, apr)
//! - FJ-1364: pin_hash (determinism, sensitivity)
//!
//! Usage: cargo test --test falsification_convert_pin_tripwire

use forjar::core::store::convert::{analyze_conversion, ChangeType, ConversionSignals};
use forjar::core::store::lockfile::{LockFile, Pin};
use forjar::core::store::pin_resolve::{parse_resolved_version, pin_hash, resolution_command};
use forjar::core::store::pin_tripwire::{
    check_before_apply, format_pin_report, needs_pin_update, pin_severity, PinSeverity,
};
use forjar::core::store::purity::PurityLevel;
use std::collections::BTreeMap;

// ============================================================================
// FJ-1328: analyze_conversion — 5-step conversion ladder
// ============================================================================

#[test]
fn conversion_unpinned_resource_gets_pin_and_store() {
    let signals = vec![ConversionSignals {
        name: "curl".into(),
        has_version: false,
        has_store: false,
        has_sandbox: false,
        has_curl_pipe: false,
        provider: "apt".into(),
        current_version: None,
    }];
    let report = analyze_conversion(&signals);
    assert_eq!(report.resources.len(), 1);
    let changes = &report.resources[0].auto_changes;
    assert!(changes
        .iter()
        .any(|c| c.change_type == ChangeType::AddVersionPin));
    assert!(changes
        .iter()
        .any(|c| c.change_type == ChangeType::EnableStore));
    assert!(changes
        .iter()
        .any(|c| c.change_type == ChangeType::GenerateLockPin));
}

#[test]
fn conversion_pinned_with_store_already_pinned() {
    let signals = vec![ConversionSignals {
        name: "serde".into(),
        has_version: true,
        has_store: true,
        has_sandbox: false,
        has_curl_pipe: false,
        provider: "cargo".into(),
        current_version: Some("1.0.215".into()),
    }];
    let report = analyze_conversion(&signals);
    let r = &report.resources[0];
    assert!(r.auto_changes.is_empty());
    assert_eq!(r.current_purity, PurityLevel::Pinned);
    // Manual changes suggest sandbox
    assert!(!r.manual_changes.is_empty());
}

#[test]
fn conversion_fully_pure_no_changes() {
    let signals = vec![ConversionSignals {
        name: "pure-pkg".into(),
        has_version: true,
        has_store: true,
        has_sandbox: true,
        has_curl_pipe: false,
        provider: "nix".into(),
        current_version: Some("23.11".into()),
    }];
    let report = analyze_conversion(&signals);
    let r = &report.resources[0];
    assert!(r.auto_changes.is_empty());
    assert!(r.manual_changes.is_empty());
    assert_eq!(r.current_purity, PurityLevel::Pure);
    assert_eq!(r.target_purity, PurityLevel::Pure);
}

#[test]
fn conversion_curl_pipe_stays_impure() {
    let signals = vec![ConversionSignals {
        name: "rustup".into(),
        has_version: false,
        has_store: false,
        has_sandbox: false,
        has_curl_pipe: true,
        provider: "shell".into(),
        current_version: None,
    }];
    let report = analyze_conversion(&signals);
    let r = &report.resources[0];
    assert_eq!(r.current_purity, PurityLevel::Impure);
    assert_eq!(r.target_purity, PurityLevel::Impure);
    assert!(r.manual_changes.iter().any(|m| m.contains("curl|bash")));
}

#[test]
fn conversion_report_counts() {
    let signals = vec![
        ConversionSignals {
            name: "a".into(),
            has_version: false,
            has_store: false,
            has_sandbox: false,
            has_curl_pipe: false,
            provider: "apt".into(),
            current_version: None,
        },
        ConversionSignals {
            name: "b".into(),
            has_version: true,
            has_store: true,
            has_sandbox: true,
            has_curl_pipe: false,
            provider: "cargo".into(),
            current_version: Some("1.0".into()),
        },
    ];
    let report = analyze_conversion(&signals);
    assert!(report.auto_change_count > 0);
    assert_eq!(report.current_purity, PurityLevel::Constrained);
}

#[test]
fn conversion_non_cacheable_provider_no_store() {
    let signals = vec![ConversionSignals {
        name: "custom".into(),
        has_version: false,
        has_store: false,
        has_sandbox: false,
        has_curl_pipe: false,
        provider: "shell".into(), // not cacheable
        current_version: None,
    }];
    let report = analyze_conversion(&signals);
    let changes = &report.resources[0].auto_changes;
    assert!(!changes
        .iter()
        .any(|c| c.change_type == ChangeType::EnableStore));
}

// ============================================================================
// FJ-1314: check_before_apply — pin freshness
// ============================================================================

fn make_lockfile(pins: &[(&str, &str)]) -> LockFile {
    let mut map = BTreeMap::new();
    for (name, hash) in pins {
        map.insert(
            name.to_string(),
            Pin {
                provider: "apt".into(),
                version: Some("1.0".into()),
                hash: hash.to_string(),
                git_rev: None,
                pin_type: None,
            },
        );
    }
    LockFile {
        schema: "1.0".into(),
        pins: map,
    }
}

#[test]
fn all_pins_fresh() {
    let lock = make_lockfile(&[("curl", "blake3:aaa"), ("jq", "blake3:bbb")]);
    let mut current = BTreeMap::new();
    current.insert("curl".into(), "blake3:aaa".into());
    current.insert("jq".into(), "blake3:bbb".into());
    let inputs = vec!["curl".into(), "jq".into()];

    let result = check_before_apply(&lock, &current, &inputs);
    assert!(result.all_fresh);
    assert!(result.stale_pins.is_empty());
    assert!(result.missing_inputs.is_empty());
}

#[test]
fn stale_pin_detected() {
    let lock = make_lockfile(&[("curl", "blake3:old")]);
    let mut current = BTreeMap::new();
    current.insert("curl".into(), "blake3:new".into());
    let inputs = vec!["curl".into()];

    let result = check_before_apply(&lock, &current, &inputs);
    assert!(!result.all_fresh);
    assert_eq!(result.stale_pins.len(), 1);
    assert_eq!(result.stale_pins[0].name, "curl");
    assert_eq!(result.stale_pins[0].locked_hash, "blake3:old");
    assert_eq!(result.stale_pins[0].current_hash, "blake3:new");
}

#[test]
fn missing_input_detected() {
    let lock = make_lockfile(&[("curl", "blake3:aaa")]);
    let current = BTreeMap::new();
    let inputs = vec!["curl".into(), "jq".into()];

    let result = check_before_apply(&lock, &current, &inputs);
    assert!(!result.all_fresh);
    assert!(result.missing_inputs.contains(&"jq".to_string()));
}

// ============================================================================
// FJ-1314: pin_severity
// ============================================================================

#[test]
fn severity_info_when_fresh() {
    let lock = make_lockfile(&[("curl", "blake3:aaa")]);
    let mut current = BTreeMap::new();
    current.insert("curl".into(), "blake3:aaa".into());
    let result = check_before_apply(&lock, &current, &["curl".into()]);
    assert_eq!(pin_severity(&result, false), PinSeverity::Info);
    assert_eq!(pin_severity(&result, true), PinSeverity::Info);
}

#[test]
fn severity_warning_when_stale_non_strict() {
    let lock = make_lockfile(&[("curl", "blake3:old")]);
    let mut current = BTreeMap::new();
    current.insert("curl".into(), "blake3:new".into());
    let result = check_before_apply(&lock, &current, &["curl".into()]);
    assert_eq!(pin_severity(&result, false), PinSeverity::Warning);
}

#[test]
fn severity_error_when_stale_strict() {
    let lock = make_lockfile(&[("curl", "blake3:old")]);
    let mut current = BTreeMap::new();
    current.insert("curl".into(), "blake3:new".into());
    let result = check_before_apply(&lock, &current, &["curl".into()]);
    assert_eq!(pin_severity(&result, true), PinSeverity::Error);
}

// ============================================================================
// FJ-1314: format_pin_report & needs_pin_update
// ============================================================================

#[test]
fn format_pin_report_shows_stale_and_missing() {
    let lock = make_lockfile(&[("curl", "blake3:old")]);
    let mut current = BTreeMap::new();
    current.insert("curl".into(), "blake3:new".into());
    let inputs = vec!["curl".into(), "jq".into()];
    let result = check_before_apply(&lock, &current, &inputs);
    let report = format_pin_report(&result);
    assert!(report.contains("STALE"));
    assert!(report.contains("curl"));
    assert!(report.contains("MISSING"));
    assert!(report.contains("jq"));
}

#[test]
fn needs_pin_update_true_when_stale() {
    let lock = make_lockfile(&[("curl", "blake3:old")]);
    let mut current = BTreeMap::new();
    current.insert("curl".into(), "blake3:new".into());
    assert!(needs_pin_update(&lock, &current, &["curl".into()]));
}

#[test]
fn needs_pin_update_false_when_fresh() {
    let lock = make_lockfile(&[("curl", "blake3:aaa")]);
    let mut current = BTreeMap::new();
    current.insert("curl".into(), "blake3:aaa".into());
    assert!(!needs_pin_update(&lock, &current, &["curl".into()]));
}

// ============================================================================
// FJ-1364: resolution_command — provider CLI commands
// ============================================================================

#[test]
fn resolution_cmd_apt() {
    assert_eq!(
        resolution_command("apt", "curl"),
        Some("apt-cache policy curl".into())
    );
}

#[test]
fn resolution_cmd_cargo() {
    let cmd = resolution_command("cargo", "serde").unwrap();
    assert!(cmd.contains("cargo search serde"));
}

#[test]
fn resolution_cmd_nix() {
    let cmd = resolution_command("nix", "hello").unwrap();
    assert!(cmd.contains("nix eval"));
}

#[test]
fn resolution_cmd_pip_and_uv() {
    assert!(resolution_command("pip", "requests").is_some());
    assert!(resolution_command("uv", "numpy").is_some());
}

#[test]
fn resolution_cmd_docker() {
    let cmd = resolution_command("docker", "nginx").unwrap();
    assert!(cmd.contains("docker image inspect"));
}

#[test]
fn resolution_cmd_unknown_returns_none() {
    assert!(resolution_command("brew", "pkg").is_none());
}

// ============================================================================
// FJ-1364: parse_resolved_version — provider output parsing
// ============================================================================

#[test]
fn parse_version_apt() {
    let output = "curl:\n  Installed: 7.88.1-10\n  Candidate: 7.88.1-10+deb12u7\n  Version table:";
    assert_eq!(
        parse_resolved_version("apt", output),
        Some("7.88.1-10+deb12u7".into())
    );
}

#[test]
fn parse_version_cargo() {
    let output = r#"serde = "1.0.215"    # A serialization framework"#;
    assert_eq!(
        parse_resolved_version("cargo", output),
        Some("1.0.215".into())
    );
}

#[test]
fn parse_version_nix_raw() {
    assert_eq!(parse_resolved_version("nix", "23.11"), Some("23.11".into()));
}

#[test]
fn parse_version_docker_digest() {
    assert_eq!(
        parse_resolved_version("docker", "sha256:abc123"),
        Some("sha256:abc123".into())
    );
}

#[test]
fn parse_version_pip_available() {
    let output = "Available versions: 2.31.0, 2.30.0, 2.29.0";
    assert_eq!(parse_resolved_version("pip", output), Some("2.31.0".into()));
}

#[test]
fn parse_version_empty_input() {
    assert_eq!(parse_resolved_version("apt", ""), None);
    assert_eq!(parse_resolved_version("cargo", "   "), None);
}

#[test]
fn parse_version_unknown_provider() {
    assert_eq!(parse_resolved_version("brew", "1.0"), None);
}

// ============================================================================
// FJ-1364: pin_hash — determinism and sensitivity
// ============================================================================

#[test]
fn pin_hash_deterministic() {
    let h1 = pin_hash("apt", "curl", "7.88.1");
    let h2 = pin_hash("apt", "curl", "7.88.1");
    assert_eq!(h1, h2);
    assert!(h1.starts_with("blake3:"));
}

#[test]
fn pin_hash_sensitive_to_version() {
    let h1 = pin_hash("apt", "curl", "7.88.1");
    let h2 = pin_hash("apt", "curl", "7.99.0");
    assert_ne!(h1, h2);
}

#[test]
fn pin_hash_sensitive_to_provider() {
    let h1 = pin_hash("apt", "curl", "7.88.1");
    let h2 = pin_hash("cargo", "curl", "7.88.1");
    assert_ne!(h1, h2);
}

#[test]
fn pin_hash_sensitive_to_name() {
    let h1 = pin_hash("apt", "curl", "1.0");
    let h2 = pin_hash("apt", "wget", "1.0");
    assert_ne!(h1, h2);
}
