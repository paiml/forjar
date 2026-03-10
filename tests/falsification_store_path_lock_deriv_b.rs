//! FJ-1300/1310/1341/1320: Store path, lockfile, derivation, and cache falsification.
//!
//! Popperian rejection criteria for:
//! - FJ-1300: Store path derivation
//!   - store_path determinism, input order independence
//!   - store_entry_path with/without blake3 prefix
//! - FJ-1310: Input lock file
//!   - parse_lockfile, write_lockfile/read_lockfile roundtrip
//!   - check_staleness: stale/fresh detection
//!   - check_completeness: missing pin detection
//! - FJ-1341: Derivation model
//!   - validate_derivation: required fields
//!   - validate_dag: cycle detection, topological order
//!   - derivation_closure_hash determinism
//!   - collect_input_hashes: store/resource resolution
//!   - derivation_purity classification
//!   - compute_depth from input depths
//! - FJ-1320: Binary cache
//!   - parse_cache_config, validate_cache_config
//!   - resolve_substitution: local/cache/miss
//!   - verify_entry, build_inventory
//!   - ssh_command generation
//!
//! Usage: cargo test --test falsification_store_path_lock_deriv

use forjar::core::store::cache::{
    build_inventory, parse_cache_config, resolve_substitution, ssh_command, validate_cache_config,
    verify_entry, CacheEntry, CacheSource, SubstitutionResult,
};
use forjar::core::store::derivation::{
    collect_input_hashes, compute_depth, derivation_closure_hash, derivation_purity,
    parse_derivation, validate_dag, validate_derivation, Derivation, DerivationInput,
};
use forjar::core::store::lockfile::{
    check_completeness, check_staleness, parse_lockfile, read_lockfile, write_lockfile, LockFile,
    Pin,
};
use forjar::core::store::path::{store_entry_path, store_path, STORE_BASE};
use forjar::core::store::purity::PurityLevel;
use std::collections::BTreeMap;

// ============================================================================
// FJ-1300: store_path
#[test]
fn compute_depth_from_inputs() {
    assert_eq!(compute_depth(&[]), 1);
    assert_eq!(compute_depth(&[0]), 1);
    assert_eq!(compute_depth(&[1, 2, 3]), 4);
    assert_eq!(compute_depth(&[5, 3, 7, 1]), 8);
}

// ============================================================================
// FJ-1341: parse_derivation
// ============================================================================

#[test]
fn parse_derivation_yaml() {
    let yaml = r#"
inputs:
  src:
    store: "blake3:abc123"
script: "make install"
arch: "x86_64"
out_var: "$out"
"#;
    let d = parse_derivation(yaml).unwrap();
    assert_eq!(d.script, "make install");
    assert_eq!(d.arch, "x86_64");
    assert_eq!(d.inputs.len(), 1);
}

#[test]
fn parse_derivation_invalid_yaml() {
    let result = parse_derivation("{{invalid");
    assert!(result.is_err());
}

// ============================================================================
// FJ-1320: parse_cache_config
// ============================================================================

#[test]
fn parse_cache_config_ssh() {
    let yaml = r#"
sources:
  - type: ssh
    host: cache.example.com
    user: forjar
    path: /var/cache/forjar
    port: 2222
auto_push: true
max_size_mb: 1024
"#;
    let config = parse_cache_config(yaml).unwrap();
    assert_eq!(config.sources.len(), 1);
    assert!(config.auto_push);
    assert_eq!(config.max_size_mb, 1024);
}

#[test]
fn parse_cache_config_local() {
    let yaml = r#"
sources:
  - type: local
    path: /var/lib/forjar/store
"#;
    let config = parse_cache_config(yaml).unwrap();
    assert_eq!(config.sources.len(), 1);
}

// ============================================================================
// FJ-1320: validate_cache_config
// ============================================================================

#[test]
fn validate_cache_empty_sources_error() {
    let config = parse_cache_config("sources: []\n").unwrap();
    let errors = validate_cache_config(&config);
    assert!(errors.iter().any(|e| e.contains("at least one")));
}

#[test]
fn validate_cache_empty_host_error() {
    let yaml = r#"
sources:
  - type: ssh
    host: ""
    user: forjar
    path: /cache
"#;
    let config = parse_cache_config(yaml).unwrap();
    let errors = validate_cache_config(&config);
    assert!(errors.iter().any(|e| e.contains("host cannot be empty")));
}

#[test]
fn validate_cache_valid_config() {
    let yaml = r#"
sources:
  - type: ssh
    host: cache.example.com
    user: forjar
    path: /cache
"#;
    let config = parse_cache_config(yaml).unwrap();
    let errors = validate_cache_config(&config);
    assert!(errors.is_empty());
}

// ============================================================================
// FJ-1320: resolve_substitution
// ============================================================================

#[test]
fn substitution_local_hit() {
    let local = vec!["blake3:abc".into()];
    let result = resolve_substitution("blake3:abc", &local, &[]);
    assert!(matches!(result, SubstitutionResult::LocalHit { .. }));
}

#[test]
fn substitution_cache_hit() {
    let local = vec![];
    let entry = CacheEntry {
        store_hash: "blake3:abc".into(),
        size_bytes: 1024,
        created_at: "now".into(),
        provider: "apt".into(),
        arch: "x86_64".into(),
    };
    let inventory = build_inventory("remote-cache", vec![entry]);
    let result = resolve_substitution("blake3:abc", &local, &[inventory]);
    match result {
        SubstitutionResult::CacheHit {
            source_index,
            store_hash,
        } => {
            assert_eq!(source_index, 0);
            assert_eq!(store_hash, "blake3:abc");
        }
        _ => panic!("expected CacheHit"),
    }
}

#[test]
fn substitution_cache_miss() {
    let result = resolve_substitution("blake3:missing", &[], &[]);
    assert!(matches!(result, SubstitutionResult::CacheMiss));
}

#[test]
fn substitution_local_preferred_over_cache() {
    let local = vec!["blake3:abc".into()];
    let entry = CacheEntry {
        store_hash: "blake3:abc".into(),
        size_bytes: 1024,
        created_at: "now".into(),
        provider: "apt".into(),
        arch: "x86_64".into(),
    };
    let inventory = build_inventory("remote", vec![entry]);
    let result = resolve_substitution("blake3:abc", &local, &[inventory]);
    assert!(
        matches!(result, SubstitutionResult::LocalHit { .. }),
        "local should be preferred over cache"
    );
}

// ============================================================================
// FJ-1320: verify_entry and ssh_command
// ============================================================================

#[test]
fn verify_entry_matching_hash() {
    let entry = CacheEntry {
        store_hash: "blake3:abc".into(),
        size_bytes: 100,
        created_at: "now".into(),
        provider: "apt".into(),
        arch: "x86_64".into(),
    };
    assert!(verify_entry(&entry, "blake3:abc"));
    assert!(!verify_entry(&entry, "blake3:different"));
}

#[test]
fn ssh_command_with_port() {
    let source = CacheSource::Ssh {
        host: "cache.example.com".into(),
        user: "forjar".into(),
        path: "/cache".into(),
        port: Some(2222),
    };
    let cmd = ssh_command(&source).unwrap();
    assert_eq!(cmd, "ssh -p 2222 forjar@cache.example.com");
}

#[test]
fn ssh_command_default_port() {
    let source = CacheSource::Ssh {
        host: "cache.example.com".into(),
        user: "forjar".into(),
        path: "/cache".into(),
        port: None,
    };
    let cmd = ssh_command(&source).unwrap();
    assert_eq!(cmd, "ssh forjar@cache.example.com");
}

#[test]
fn ssh_command_local_returns_none() {
    let source = CacheSource::Local {
        path: "/store".into(),
    };
    assert!(ssh_command(&source).is_none());
}

// ============================================================================
// FJ-1320: build_inventory
// ============================================================================

#[test]
fn build_inventory_maps_by_hash() {
    let entries = vec![
        CacheEntry {
            store_hash: "h1".into(),
            size_bytes: 100,
            created_at: "now".into(),
            provider: "apt".into(),
            arch: "x86_64".into(),
        },
        CacheEntry {
            store_hash: "h2".into(),
            size_bytes: 200,
            created_at: "now".into(),
            provider: "cargo".into(),
            arch: "x86_64".into(),
        },
    ];
    let inv = build_inventory("my-cache", entries);
    assert_eq!(inv.source_name, "my-cache");
    assert_eq!(inv.entries.len(), 2);
    assert!(inv.entries.contains_key("h1"));
    assert!(inv.entries.contains_key("h2"));
}
