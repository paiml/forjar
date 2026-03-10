//! FJ-1305/1329/1345/1356: Purity, reproducibility, store diff, and secret scan falsification.
//!
//! Popperian rejection criteria for:
//! - FJ-1305: Purity classification (Pure/Pinned/Constrained/Impure)
//!   - classify for each level
//!   - Monotonicity invariant (dependency elevation)
//!   - recipe_purity aggregation
//!   - level_label formatting
//! - FJ-1329: Reproducibility scoring
//!   - compute_score with various purity mixes
//!   - Grade thresholds (A/B/C/D/F)
//!   - Empty inputs, store/lock coverage
//! - FJ-1345: Store diff and sync model
//!   - compute_diff: no change, upstream changed, no provenance
//!   - build_sync_plan: re-imports and derivation replays
//!   - has_diffable_provenance predicate
//!   - upstream_check_command per provider
//! - FJ-1356: Secret scanning
//!   - is_encrypted: age-encrypted detection
//!   - scan_text: AWS keys, PEM, GitHub tokens, JWT
//!   - scan_yaml_str: recursive YAML scanning
//!   - Clean config passes
//!
//! Usage: cargo test --test falsification_store_purity_repro
#![allow(dead_code)]

use forjar::core::store::meta::{Provenance, StoreMeta};
use forjar::core::store::purity::{
    classify, level_label, recipe_purity, PurityLevel, PuritySignals,
};
use forjar::core::store::repro_score::{compute_score, grade, ReproInput};
use forjar::core::store::secret_scan::{is_encrypted, scan_text, scan_yaml_str};
use forjar::core::store::store_diff::{
    build_sync_plan, compute_diff, has_diffable_provenance, upstream_check_command,
};

// ============================================================================
// Helpers
// ============================================================================

fn meta_with_provenance(
    hash: &str,
    provider: &str,
    origin_ref: Option<&str>,
    origin_hash: Option<&str>,
    depth: u32,
) -> StoreMeta {
    StoreMeta {
        schema: "1.0".into(),
        store_hash: hash.into(),
        recipe_hash: "rh".into(),
        input_hashes: vec![],
        arch: "x86_64".into(),
        provider: provider.into(),
        created_at: "2026-03-09T12:00:00Z".into(),
        generator: "test".into(),
        references: vec![],
        provenance: Some(Provenance {
            origin_provider: provider.into(),
            origin_ref: origin_ref.map(|s| s.into()),
            origin_hash: origin_hash.map(|s| s.into()),
            derived_from: None,
            derivation_depth: depth,
        }),
    }
}

fn meta_no_provenance(hash: &str) -> StoreMeta {
    StoreMeta {
        schema: "1.0".into(),
        store_hash: hash.into(),
        recipe_hash: "rh".into(),
        input_hashes: vec![],
        arch: "x86_64".into(),
        provider: "apt".into(),
        created_at: "2026-03-09T12:00:00Z".into(),
        generator: "test".into(),
        references: vec![],
        provenance: None,
    }
}

// ============================================================================
// FJ-1305: Purity — classify
#[test]
fn not_encrypted_plain_text() {
    assert!(!is_encrypted("plain text"));
    assert!(!is_encrypted("ENC[gpg,data]"));
    assert!(!is_encrypted(""));
}

// ============================================================================
// FJ-1356: scan_text
// ============================================================================

#[test]
fn scan_detects_aws_access_key() {
    let findings = scan_text("AKIAIOSFODNN7EXAMPLE");
    assert!(!findings.is_empty());
    assert!(findings.iter().any(|(name, _)| name == "aws_access_key"));
}

#[test]
fn scan_detects_pem_private_key() {
    let findings = scan_text("-----BEGIN RSA PRIVATE KEY-----");
    assert!(!findings.is_empty());
    assert!(findings.iter().any(|(name, _)| name == "private_key_pem"));
}

#[test]
fn scan_skips_encrypted_values() {
    let findings = scan_text("ENC[age,AKIAIOSFODNN7EXAMPLE]");
    assert!(findings.is_empty(), "encrypted values should be skipped");
}

#[test]
fn scan_clean_text_no_findings() {
    let findings = scan_text("just a normal config value");
    assert!(findings.is_empty());
}

#[test]
fn scan_detects_database_url_with_password() {
    let findings = scan_text("postgres://user:password123@host:5432/db");
    assert!(!findings.is_empty());
    assert!(findings.iter().any(|(name, _)| name == "database_url_pass"));
}

// ============================================================================
// FJ-1356: scan_yaml_str
// ============================================================================

#[test]
fn scan_yaml_clean_config() {
    let yaml = r#"
name: my-app
version: "1.0"
resources:
  nginx:
    type: package
"#;
    let result = scan_yaml_str(yaml);
    assert!(result.clean);
    assert!(result.findings.is_empty());
    assert!(result.scanned_fields > 0);
}

#[test]
fn scan_yaml_detects_secret_in_field() {
    let yaml = r#"
db_password: "postgres://admin:supersecret@db.example.com:5432/app"
"#;
    let result = scan_yaml_str(yaml);
    assert!(!result.clean);
    assert!(!result.findings.is_empty());
    assert!(result
        .findings
        .iter()
        .any(|f| f.location.contains("db_password")));
}

#[test]
fn scan_yaml_nested_secret() {
    let yaml = r#"
resources:
  web:
    env:
      AWS_KEY: "AKIAIOSFODNN7EXAMPLE"
"#;
    let result = scan_yaml_str(yaml);
    assert!(!result.clean);
    assert!(result
        .findings
        .iter()
        .any(|f| f.pattern_name == "aws_access_key"));
}

#[test]
fn scan_yaml_encrypted_field_clean() {
    let yaml = r#"
secret_key: "ENC[age,YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOQ==]"
"#;
    let result = scan_yaml_str(yaml);
    assert!(result.clean, "encrypted fields should not trigger findings");
}
