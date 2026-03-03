//! Spec falsification tests: Phases A–D
//!
//! Every test targets a specific, falsifiable claim from the nix-compatible
//! reproducible package manager specification. If the implementation diverges
//! from the spec, the corresponding test fails.
//!
//! Phase A: Store model (types, paths, metadata, provenance)
//! Phase B: Purity model (4 levels, monotonicity, classification)
//! Phase C: Closures & locking (input closure, lock file format)
//! Phase D: Sandboxing & scoring (presets, repro score weights, grades)
#![allow(unused_imports)]

use super::closure::{closure_hash, input_closure, ResourceInputs};
use super::derivation::{
    compute_depth, derivation_closure_hash, derivation_purity, validate_dag, validate_derivation,
    Derivation, DerivationInput,
};
use super::far::{ChunkEntry, FarFileEntry, FarManifest, FarProvenance, FAR_MAGIC};
use super::lockfile::{check_completeness, check_staleness, LockFile, Pin, StalenessEntry};
use super::meta::{new_meta, Provenance, StoreMeta};
use super::path::{store_entry_path, store_path, STORE_BASE};
use super::provider::{
    all_providers, capture_method, import_command, origin_ref_string, validate_import,
    ImportConfig, ImportProvider, ImportResult,
};
use super::purity::{classify, level_label, recipe_purity, PurityLevel, PuritySignals};
use super::repro_score::{compute_score, grade, ReproInput, ReproScore};
use super::sandbox::{
    blocks_network, cgroup_path, enforces_fs_isolation, preset_profile, validate_config,
    SandboxConfig, SandboxLevel,
};
use std::collections::BTreeMap;

// ═══════════════════════════════════════════════════════════════════
// Phase A: Store Model
// ═══════════════════════════════════════════════════════════════════

/// A-01: Store base path matches spec ("/var/lib/forjar/store" or "/var/forjar/store").
#[test]
fn falsify_a01_store_base_path() {
    assert!(
        STORE_BASE.contains("forjar/store"),
        "STORE_BASE must contain 'forjar/store': got {STORE_BASE}"
    );
}

/// A-02: store_path() uses BLAKE3 — output must start with "blake3:".
#[test]
fn falsify_a02_store_path_blake3() {
    let hash = store_path("recipe:abc", &["input:111"], "x86_64", "apt");
    assert!(
        hash.starts_with("blake3:"),
        "store_path must return blake3 hash: got {hash}"
    );
}

/// A-03: store_path() is deterministic — same inputs → same hash.
#[test]
fn falsify_a03_store_path_deterministic() {
    let h1 = store_path("r", &["a", "b"], "x86_64", "apt");
    let h2 = store_path("r", &["a", "b"], "x86_64", "apt");
    assert_eq!(h1, h2, "store_path must be deterministic");
}

/// A-04: store_path() input ordering doesn't matter (sorted internally).
#[test]
fn falsify_a04_store_path_order_invariant() {
    let h1 = store_path("r", &["a", "b", "c"], "x86_64", "apt");
    let h2 = store_path("r", &["c", "a", "b"], "x86_64", "apt");
    assert_eq!(h1, h2, "input order must not affect store hash");
}

/// A-05: Different inputs → different hashes.
#[test]
fn falsify_a05_store_path_collision_resistance() {
    let h1 = store_path("r1", &["a"], "x86_64", "apt");
    let h2 = store_path("r2", &["a"], "x86_64", "apt");
    assert_ne!(
        h1, h2,
        "different recipe hashes must produce different store paths"
    );
}

/// A-06: store_entry_path() strips "blake3:" prefix and joins with STORE_BASE.
#[test]
fn falsify_a06_store_entry_path_format() {
    let path = store_entry_path("blake3:abc123");
    assert!(
        path.starts_with(STORE_BASE),
        "entry path must start with STORE_BASE"
    );
    assert!(
        path.ends_with("abc123"),
        "entry path must end with hex hash"
    );
    assert!(
        !path.contains("blake3:"),
        "entry path must not contain 'blake3:' prefix"
    );
}

/// A-07: StoreMeta has all spec-required fields.
#[test]
fn falsify_a07_store_meta_fields() {
    let meta = new_meta("blake3:h", "blake3:r", &[], "x86_64", "apt");
    assert_eq!(meta.schema, "1.0", "schema must be '1.0'");
    assert_eq!(meta.store_hash, "blake3:h");
    assert_eq!(meta.recipe_hash, "blake3:r");
    assert_eq!(meta.arch, "x86_64");
    assert_eq!(meta.provider, "apt");
    assert!(!meta.created_at.is_empty(), "created_at must be set");
    assert!(
        meta.generator.starts_with("forjar"),
        "generator must start with 'forjar': got {}",
        meta.generator
    );
}

/// A-08: Provenance struct has required fields.
#[test]
fn falsify_a08_provenance_struct() {
    let p = Provenance {
        origin_provider: "nix".to_string(),
        origin_ref: Some("nixpkgs#ripgrep@14.1.0".to_string()),
        origin_hash: Some("sha256:def456".to_string()),
        derived_from: Some("blake3:aaa".to_string()),
        derivation_depth: 1,
    };
    assert_eq!(p.origin_provider, "nix");
    assert_eq!(p.derivation_depth, 1);
}

/// A-09: StoreMeta schema defaults to "1.0".
#[test]
fn falsify_a09_meta_schema_version() {
    let meta = new_meta("h", "r", &[], "x86_64", "apt");
    assert_eq!(meta.schema, "1.0", "spec requires schema '1.0'");
}

/// A-10: StoreMeta references defaults to empty vec.
#[test]
fn falsify_a10_meta_references_default_empty() {
    let meta = new_meta("h", "r", &[], "x86_64", "apt");
    assert!(
        meta.references.is_empty(),
        "references must default to empty"
    );
}

// ═══════════════════════════════════════════════════════════════════
// Phase B: Purity Model
// ═══════════════════════════════════════════════════════════════════

/// B-01: PurityLevel enum has exactly 4 variants at correct ordinal values.
#[test]
fn falsify_b01_purity_level_ordinals() {
    assert_eq!(PurityLevel::Pure as u8, 0);
    assert_eq!(PurityLevel::Pinned as u8, 1);
    assert_eq!(PurityLevel::Constrained as u8, 2);
    assert_eq!(PurityLevel::Impure as u8, 3);
}

/// B-02: PurityLevel ordering — Pure < Pinned < Constrained < Impure.
#[test]
fn falsify_b02_purity_ordering() {
    assert!(PurityLevel::Pure < PurityLevel::Pinned);
    assert!(PurityLevel::Pinned < PurityLevel::Constrained);
    assert!(PurityLevel::Constrained < PurityLevel::Impure);
}

/// B-03: curl|bash → Impure classification.
#[test]
fn falsify_b03_curl_pipe_impure() {
    let signals = PuritySignals {
        has_curl_pipe: true,
        has_version: true,
        has_store: true,
        has_sandbox: true,
        dep_levels: vec![],
    };
    let result = classify("test", &signals);
    assert_eq!(
        result.level,
        PurityLevel::Impure,
        "curl|bash must be Impure"
    );
}

/// B-04: No version → Constrained.
#[test]
fn falsify_b04_no_version_constrained() {
    let signals = PuritySignals {
        has_version: false,
        ..Default::default()
    };
    let result = classify("test", &signals);
    assert_eq!(result.level, PurityLevel::Constrained);
}

/// B-05: Version + store + no sandbox → Pinned.
#[test]
fn falsify_b05_no_sandbox_pinned() {
    let signals = PuritySignals {
        has_version: true,
        has_store: true,
        has_sandbox: false,
        ..Default::default()
    };
    let result = classify("test", &signals);
    assert_eq!(result.level, PurityLevel::Pinned);
}

/// B-06: Version + store + sandbox → Pure.
#[test]
fn falsify_b06_full_pure() {
    let signals = PuritySignals {
        has_version: true,
        has_store: true,
        has_sandbox: true,
        ..Default::default()
    };
    let result = classify("test", &signals);
    assert_eq!(result.level, PurityLevel::Pure);
}

/// B-07: Monotonicity — recipe purity = max(deps).
#[test]
fn falsify_b07_monotonicity() {
    let signals = PuritySignals {
        has_version: true,
        has_store: true,
        has_sandbox: true,
        dep_levels: vec![PurityLevel::Impure],
        ..Default::default()
    };
    let result = classify("test", &signals);
    assert_eq!(
        result.level,
        PurityLevel::Impure,
        "monotonicity: impure dep must elevate resource purity"
    );
}

/// B-08: recipe_purity() returns max of all resource levels.
#[test]
fn falsify_b08_recipe_purity_max() {
    let levels = [
        PurityLevel::Pure,
        PurityLevel::Pinned,
        PurityLevel::Constrained,
    ];
    assert_eq!(recipe_purity(&levels), PurityLevel::Constrained);
}

/// B-09: recipe_purity() on empty is Pure.
#[test]
fn falsify_b09_empty_recipe_pure() {
    assert_eq!(recipe_purity(&[]), PurityLevel::Pure);
}

/// B-10: level_label() returns human-readable label with ordinal.
#[test]
fn falsify_b10_level_labels() {
    assert!(level_label(PurityLevel::Pure).contains("0"));
    assert!(level_label(PurityLevel::Pinned).contains("1"));
    assert!(level_label(PurityLevel::Constrained).contains("2"));
    assert!(level_label(PurityLevel::Impure).contains("3"));
}

// ═══════════════════════════════════════════════════════════════════
// Phase C: Closures & Locking
// ═══════════════════════════════════════════════════════════════════

/// C-01: input_closure() collects transitive hashes.
#[test]
fn falsify_c01_transitive_closure() {
    let mut graph = BTreeMap::new();
    graph.insert(
        "a".to_string(),
        ResourceInputs {
            input_hashes: vec!["h1".to_string()],
            depends_on: vec!["b".to_string()],
        },
    );
    graph.insert(
        "b".to_string(),
        ResourceInputs {
            input_hashes: vec!["h2".to_string()],
            depends_on: vec![],
        },
    );
    let closure = input_closure("a", &graph);
    assert!(closure.contains(&"h1".to_string()), "must include own hash");
    assert!(closure.contains(&"h2".to_string()), "must include dep hash");
}

/// C-02: closure_hash() is deterministic.
#[test]
fn falsify_c02_closure_hash_deterministic() {
    let c1 = closure_hash(&["a".to_string(), "b".to_string()]);
    let c2 = closure_hash(&["a".to_string(), "b".to_string()]);
    assert_eq!(c1, c2);
}

/// C-03: closure_hash() returns blake3 prefix.
#[test]
fn falsify_c03_closure_hash_blake3() {
    let h = closure_hash(&["a".to_string()]);
    assert!(
        h.starts_with("blake3:"),
        "closure hash must use blake3: {h}"
    );
}

/// C-04: LockFile has schema "1.0" and BTreeMap pins.
#[test]
fn falsify_c04_lockfile_schema() {
    let lockfile = LockFile {
        schema: "1.0".to_string(),
        pins: BTreeMap::new(),
    };
    assert_eq!(lockfile.schema, "1.0");
}

/// C-05: Pin struct has provider, version, hash fields.
#[test]
fn falsify_c05_pin_fields() {
    let pin = Pin {
        provider: "apt".to_string(),
        version: Some("1.24.0".to_string()),
        hash: "blake3:abc".to_string(),
        git_rev: None,
        pin_type: Some("package".to_string()),
    };
    assert_eq!(pin.provider, "apt");
    assert_eq!(pin.hash, "blake3:abc");
}

/// C-06: check_staleness detects hash mismatch.
#[test]
fn falsify_c06_staleness_detection() {
    let mut pins = BTreeMap::new();
    pins.insert(
        "nginx".to_string(),
        Pin {
            provider: "apt".to_string(),
            version: Some("1.24.0".to_string()),
            hash: "blake3:old".to_string(),
            git_rev: None,
            pin_type: None,
        },
    );
    let lockfile = LockFile {
        schema: "1.0".to_string(),
        pins,
    };
    let mut current = BTreeMap::new();
    current.insert("nginx".to_string(), "blake3:new".to_string());
    let stale = check_staleness(&lockfile, &current);
    assert_eq!(stale.len(), 1, "must detect one stale pin");
    assert_eq!(stale[0].name, "nginx");
}

/// C-07: check_completeness detects missing pins.
#[test]
fn falsify_c07_completeness() {
    let lockfile = LockFile {
        schema: "1.0".to_string(),
        pins: BTreeMap::new(),
    };
    let missing = check_completeness(&lockfile, &["nginx".to_string()]);
    assert_eq!(missing, vec!["nginx".to_string()]);
}

// Phase D tests moved to tests_falsify_spec_d.rs (500-line limit)
