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
// ============================================================================

#[test]
fn store_path_deterministic() {
    let h1 = store_path("recipe-h", &["h1", "h2"], "x86_64", "apt");
    let h2 = store_path("recipe-h", &["h1", "h2"], "x86_64", "apt");
    assert_eq!(h1, h2);
}

#[test]
fn store_path_order_independent() {
    let h1 = store_path("rh", &["h1", "h2", "h3"], "x86_64", "apt");
    let h2 = store_path("rh", &["h3", "h1", "h2"], "x86_64", "apt");
    assert_eq!(h1, h2, "sorted inputs → same hash regardless of order");
}

#[test]
fn store_path_differs_for_different_recipe() {
    let h1 = store_path("recipe-a", &["h1"], "x86_64", "apt");
    let h2 = store_path("recipe-b", &["h1"], "x86_64", "apt");
    assert_ne!(h1, h2);
}

#[test]
fn store_path_differs_for_different_arch() {
    let h1 = store_path("rh", &["h1"], "x86_64", "apt");
    let h2 = store_path("rh", &["h1"], "aarch64", "apt");
    assert_ne!(h1, h2);
}

#[test]
fn store_path_differs_for_different_provider() {
    let h1 = store_path("rh", &["h1"], "x86_64", "apt");
    let h2 = store_path("rh", &["h1"], "x86_64", "cargo");
    assert_ne!(h1, h2);
}

// ============================================================================
// FJ-1300: store_entry_path
// ============================================================================

#[test]
fn store_entry_path_with_prefix() {
    let path = store_entry_path("blake3:abc123");
    assert_eq!(path, format!("{STORE_BASE}/abc123"));
}

#[test]
fn store_entry_path_without_prefix() {
    let path = store_entry_path("abc123");
    assert_eq!(path, format!("{STORE_BASE}/abc123"));
}

// ============================================================================
// FJ-1310: parse_lockfile
// ============================================================================

#[test]
fn parse_lockfile_valid() {
    let yaml = r#"
schema: "1.0"
pins:
  nginx:
    provider: apt
    version: "1.24.0"
    hash: "abc123"
  serde:
    provider: cargo
    hash: "def456"
    git_rev: "abc1234"
"#;
    let lockfile = parse_lockfile(yaml).unwrap();
    assert_eq!(lockfile.schema, "1.0");
    assert_eq!(lockfile.pins.len(), 2);
    assert_eq!(lockfile.pins["nginx"].provider, "apt");
    assert_eq!(lockfile.pins["nginx"].version.as_deref(), Some("1.24.0"));
    assert_eq!(lockfile.pins["serde"].hash, "def456");
    assert_eq!(lockfile.pins["serde"].git_rev.as_deref(), Some("abc1234"));
}

#[test]
fn parse_lockfile_invalid_yaml() {
    let result = parse_lockfile("{{invalid yaml");
    assert!(result.is_err());
}

// ============================================================================
// FJ-1310: write/read lockfile roundtrip
// ============================================================================

#[test]
fn write_read_lockfile_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forjar.inputs.lock.yaml");

    let lockfile = LockFile {
        schema: "1.0".into(),
        pins: BTreeMap::from([
            (
                "nginx".into(),
                Pin {
                    provider: "apt".into(),
                    version: Some("1.24".into()),
                    hash: "h1".into(),
                    git_rev: None,
                    pin_type: None,
                },
            ),
            (
                "serde".into(),
                Pin {
                    provider: "cargo".into(),
                    version: None,
                    hash: "h2".into(),
                    git_rev: Some("abc".into()),
                    pin_type: Some("git".into()),
                },
            ),
        ]),
    };

    write_lockfile(&path, &lockfile).unwrap();
    let loaded = read_lockfile(&path).unwrap();
    assert_eq!(loaded.pins.len(), 2);
    assert_eq!(loaded.pins["nginx"].hash, "h1");
    assert_eq!(loaded.pins["serde"].git_rev.as_deref(), Some("abc"));
}

// ============================================================================
// FJ-1310: check_staleness
// ============================================================================

#[test]
fn staleness_detects_changed_hash() {
    let lockfile = LockFile {
        schema: "1.0".into(),
        pins: BTreeMap::from([(
            "nginx".into(),
            Pin {
                provider: "apt".into(),
                version: None,
                hash: "old-hash".into(),
                git_rev: None,
                pin_type: None,
            },
        )]),
    };
    let mut current = BTreeMap::new();
    current.insert("nginx".into(), "new-hash".into());

    let stale = check_staleness(&lockfile, &current);
    assert_eq!(stale.len(), 1);
    assert_eq!(stale[0].name, "nginx");
    assert_eq!(stale[0].locked_hash, "old-hash");
    assert_eq!(stale[0].current_hash, "new-hash");
}

#[test]
fn staleness_no_stale_when_matching() {
    let lockfile = LockFile {
        schema: "1.0".into(),
        pins: BTreeMap::from([(
            "nginx".into(),
            Pin {
                provider: "apt".into(),
                version: None,
                hash: "same-hash".into(),
                git_rev: None,
                pin_type: None,
            },
        )]),
    };
    let mut current = BTreeMap::new();
    current.insert("nginx".into(), "same-hash".into());

    let stale = check_staleness(&lockfile, &current);
    assert!(stale.is_empty());
}

#[test]
fn staleness_ignores_missing_current() {
    let lockfile = LockFile {
        schema: "1.0".into(),
        pins: BTreeMap::from([(
            "nginx".into(),
            Pin {
                provider: "apt".into(),
                version: None,
                hash: "h".into(),
                git_rev: None,
                pin_type: None,
            },
        )]),
    };
    let current = BTreeMap::new(); // empty
    let stale = check_staleness(&lockfile, &current);
    assert!(stale.is_empty(), "missing from current shouldn't be stale");
}

// ============================================================================
// FJ-1310: check_completeness
// ============================================================================

#[test]
fn completeness_detects_missing_pin() {
    let lockfile = LockFile {
        schema: "1.0".into(),
        pins: BTreeMap::new(),
    };
    let inputs = vec!["nginx".into(), "mysql".into()];
    let missing = check_completeness(&lockfile, &inputs);
    assert_eq!(missing, vec!["mysql", "nginx"]);
}

#[test]
fn completeness_no_missing_when_all_pinned() {
    let lockfile = LockFile {
        schema: "1.0".into(),
        pins: BTreeMap::from([(
            "nginx".into(),
            Pin {
                provider: "apt".into(),
                version: None,
                hash: "h".into(),
                git_rev: None,
                pin_type: None,
            },
        )]),
    };
    let inputs = vec!["nginx".into()];
    let missing = check_completeness(&lockfile, &inputs);
    assert!(missing.is_empty());
}

// ============================================================================
// FJ-1341: validate_derivation
// ============================================================================

#[test]
fn derivation_valid() {
    let d = Derivation {
        inputs: BTreeMap::from([(
            "src".into(),
            DerivationInput::Store {
                store: "blake3:abc".into(),
            },
        )]),
        script: "make install".into(),
        sandbox: None,
        arch: "x86_64".into(),
        out_var: "$out".into(),
    };
    let errors = validate_derivation(&d);
    assert!(errors.is_empty());
}

#[test]
fn derivation_empty_inputs_error() {
    let d = Derivation {
        inputs: BTreeMap::new(),
        script: "echo ok".into(),
        sandbox: None,
        arch: "x86_64".into(),
        out_var: "$out".into(),
    };
    let errors = validate_derivation(&d);
    assert!(errors.iter().any(|e| e.contains("at least one input")));
}

#[test]
fn derivation_empty_script_error() {
    let d = Derivation {
        inputs: BTreeMap::from([("src".into(), DerivationInput::Store { store: "h".into() })]),
        script: "  ".into(),
        sandbox: None,
        arch: "x86_64".into(),
        out_var: "$out".into(),
    };
    let errors = validate_derivation(&d);
    assert!(errors.iter().any(|e| e.contains("script cannot be empty")));
}

#[test]
fn derivation_empty_store_hash_error() {
    let d = Derivation {
        inputs: BTreeMap::from([("src".into(), DerivationInput::Store { store: "".into() })]),
        script: "make".into(),
        sandbox: None,
        arch: "x86_64".into(),
        out_var: "$out".into(),
    };
    let errors = validate_derivation(&d);
    assert!(errors
        .iter()
        .any(|e| e.contains("store hash cannot be empty")));
}

// ============================================================================
// FJ-1341: validate_dag
// ============================================================================

#[test]
fn dag_linear_valid() {
    let mut graph = BTreeMap::new();
    graph.insert("A".into(), vec![]);
    graph.insert("B".into(), vec!["A".into()]);
    graph.insert("C".into(), vec!["B".into()]);
    let order = validate_dag(&graph).unwrap();
    assert_eq!(order.len(), 3);
    // A must come before B, B before C
    let pos_a = order.iter().position(|n| n == "A").unwrap();
    let pos_b = order.iter().position(|n| n == "B").unwrap();
    let pos_c = order.iter().position(|n| n == "C").unwrap();
    assert!(pos_a < pos_b);
    assert!(pos_b < pos_c);
}

#[test]
fn dag_cycle_detected() {
    let mut graph = BTreeMap::new();
    graph.insert("A".into(), vec!["B".into()]);
    graph.insert("B".into(), vec!["A".into()]);
    let result = validate_dag(&graph);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("cycle"));
}

#[test]
fn dag_diamond_valid() {
    let mut graph = BTreeMap::new();
    graph.insert("A".into(), vec![]);
    graph.insert("B".into(), vec!["A".into()]);
    graph.insert("C".into(), vec!["A".into()]);
    graph.insert("D".into(), vec!["B".into(), "C".into()]);
    let order = validate_dag(&graph).unwrap();
    assert_eq!(order.len(), 4);
}

// ============================================================================
// FJ-1341: derivation_closure_hash
// ============================================================================

#[test]
fn derivation_closure_hash_deterministic() {
    let d = Derivation {
        inputs: BTreeMap::from([("src".into(), DerivationInput::Store { store: "h1".into() })]),
        script: "make install".into(),
        sandbox: None,
        arch: "x86_64".into(),
        out_var: "$out".into(),
    };
    let hashes = BTreeMap::from([("src".into(), "h1".into())]);
    let h1 = derivation_closure_hash(&d, &hashes);
    let h2 = derivation_closure_hash(&d, &hashes);
    assert_eq!(h1, h2);
}

#[test]
fn derivation_closure_hash_differs_for_different_script() {
    let make_d = |script: &str| Derivation {
        inputs: BTreeMap::from([("src".into(), DerivationInput::Store { store: "h1".into() })]),
        script: script.into(),
        sandbox: None,
        arch: "x86_64".into(),
        out_var: "$out".into(),
    };
    let hashes = BTreeMap::from([("src".into(), "h1".into())]);
    let h1 = derivation_closure_hash(&make_d("make install"), &hashes);
    let h2 = derivation_closure_hash(&make_d("cargo build"), &hashes);
    assert_ne!(h1, h2);
}

// ============================================================================
// FJ-1341: collect_input_hashes
// ============================================================================

#[test]
fn collect_store_input() {
    let d = Derivation {
        inputs: BTreeMap::from([(
            "src".into(),
            DerivationInput::Store {
                store: "blake3:abc".into(),
            },
        )]),
        script: "echo".into(),
        sandbox: None,
        arch: "x86_64".into(),
        out_var: "$out".into(),
    };
    let resolved = BTreeMap::new();
    let hashes = collect_input_hashes(&d, &resolved).unwrap();
    assert_eq!(hashes["src"], "blake3:abc");
}

#[test]
fn collect_resource_input() {
    let d = Derivation {
        inputs: BTreeMap::from([(
            "dep".into(),
            DerivationInput::Resource {
                resource: "lib".into(),
            },
        )]),
        script: "echo".into(),
        sandbox: None,
        arch: "x86_64".into(),
        out_var: "$out".into(),
    };
    let resolved = BTreeMap::from([("lib".into(), "blake3:xyz".into())]);
    let hashes = collect_input_hashes(&d, &resolved).unwrap();
    assert_eq!(hashes["dep"], "blake3:xyz");
}

#[test]
fn collect_unresolved_resource_error() {
    let d = Derivation {
        inputs: BTreeMap::from([(
            "dep".into(),
            DerivationInput::Resource {
                resource: "missing".into(),
            },
        )]),
        script: "echo".into(),
        sandbox: None,
        arch: "x86_64".into(),
        out_var: "$out".into(),
    };
    let result = collect_input_hashes(&d, &BTreeMap::new());
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unresolved"));
}

// ============================================================================
// FJ-1341: derivation_purity and compute_depth
// ============================================================================

#[test]
fn derivation_purity_no_sandbox_impure() {
    let d = Derivation {
        inputs: BTreeMap::new(),
        script: "echo".into(),
        sandbox: None,
        arch: "x86_64".into(),
        out_var: "$out".into(),
    };
    assert_eq!(derivation_purity(&d), PurityLevel::Impure);
}
