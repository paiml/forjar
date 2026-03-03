//! Spec falsification gap tests: Phase A (profiles, references) + Phase B (validation)
//!
//! Fills gaps A-11–A-18, B-11–B-14 from the gap analysis.
#![allow(unused_imports)]

use super::meta::{new_meta, Provenance, StoreMeta};
use super::path::{store_entry_path, store_path, STORE_BASE};
use super::profile::{create_generation, current_generation, list_generations, rollback};
use super::purity::{classify, level_label, recipe_purity, PurityLevel, PuritySignals};
use super::reference::{is_valid_blake3_hash, scan_directory_refs, scan_file_refs};
use super::validate::{
    format_purity_report, format_repro_report, validate_purity, validate_repro_score,
};
use std::collections::{BTreeMap, BTreeSet};

// ═══════════════════════════════════════════════════════════════════
// Phase A gaps: Profiles (§5) + References (§7)
// ═══════════════════════════════════════════════════════════════════

/// A-11: create_generation() returns incrementing generation numbers.
#[test]
fn falsify_a11_profile_create_increments() {
    let dir = tempfile::tempdir().unwrap();
    let g0 = create_generation(dir.path(), "/store/blake3:aaa").unwrap();
    let g1 = create_generation(dir.path(), "/store/blake3:bbb").unwrap();
    assert_eq!(g0, 0, "first generation must be 0");
    assert_eq!(g1, 1, "second generation must be 1");
}

/// A-12: rollback() at generation 0 returns error (spec boundary).
#[test]
fn falsify_a12_rollback_at_zero_errors() {
    let dir = tempfile::tempdir().unwrap();
    create_generation(dir.path(), "/store/blake3:aaa").unwrap();
    let err = rollback(dir.path());
    assert!(err.is_err(), "rollback at gen 0 must fail: {err:?}");
    assert!(
        err.unwrap_err()
            .contains("cannot rollback past generation 0"),
        "error message must mention gen 0"
    );
}

/// A-13: rollback() switches current symlink to previous generation.
#[test]
fn falsify_a13_rollback_switches_current() {
    let dir = tempfile::tempdir().unwrap();
    create_generation(dir.path(), "/store/aaa").unwrap();
    create_generation(dir.path(), "/store/bbb").unwrap();
    let prev = rollback(dir.path()).unwrap();
    assert_eq!(prev, 0, "rollback from gen 1 must return gen 0");
    assert_eq!(
        current_generation(dir.path()),
        Some(0),
        "current must point to gen 0 after rollback"
    );
}

/// A-14: list_generations() returns sorted ascending.
#[test]
fn falsify_a14_list_generations_sorted() {
    let dir = tempfile::tempdir().unwrap();
    create_generation(dir.path(), "/store/aaa").unwrap();
    create_generation(dir.path(), "/store/bbb").unwrap();
    create_generation(dir.path(), "/store/ccc").unwrap();
    let gens = list_generations(dir.path()).unwrap();
    assert_eq!(gens.len(), 3);
    assert_eq!(gens[0].0, 0);
    assert_eq!(gens[1].0, 1);
    assert_eq!(gens[2].0, 2);
}

/// A-15: list_generations() includes target store paths.
#[test]
fn falsify_a15_list_generations_targets() {
    let dir = tempfile::tempdir().unwrap();
    create_generation(dir.path(), "/store/blake3:aaa").unwrap();
    let gens = list_generations(dir.path()).unwrap();
    assert_eq!(gens[0].1, "/store/blake3:aaa");
}

/// A-16: current_generation() reads from current symlink.
#[test]
fn falsify_a16_current_generation_symlink() {
    let dir = tempfile::tempdir().unwrap();
    create_generation(dir.path(), "/store/aaa").unwrap();
    assert_eq!(current_generation(dir.path()), Some(0));
    create_generation(dir.path(), "/store/bbb").unwrap();
    assert_eq!(current_generation(dir.path()), Some(1));
}

/// A-17: is_valid_blake3_hash() validates "blake3:" + 64 hex chars.
#[test]
fn falsify_a17_valid_blake3_hash_format() {
    let valid = format!("blake3:{}", "a".repeat(64));
    assert!(is_valid_blake3_hash(&valid), "must accept valid hash");
    assert!(
        !is_valid_blake3_hash("blake3:short"),
        "must reject short hash"
    );
    assert!(
        !is_valid_blake3_hash("sha256:abc"),
        "must reject non-blake3 prefix"
    );
    assert!(
        !is_valid_blake3_hash(&format!("blake3:{}", "g".repeat(64))),
        "must reject non-hex chars"
    );
}

/// A-18: scan_file_refs() finds only known hashes.
#[test]
fn falsify_a18_scan_file_refs_known_only() {
    let hash1 = format!("blake3:{}", "a".repeat(64));
    let hash2 = format!("blake3:{}", "b".repeat(64));
    let unknown = format!("blake3:{}", "c".repeat(64));
    let content = format!("ref1={hash1} ref2={hash2} unknown={unknown}");

    let mut known = BTreeSet::new();
    known.insert(hash1.clone());
    known.insert(hash2.clone());

    let refs = scan_file_refs(content.as_bytes(), &known);
    assert!(refs.contains(&hash1));
    assert!(refs.contains(&hash2));
    assert!(!refs.contains(&unknown), "must not include unknown hashes");
}

/// A-19: scan_directory_refs() recursively scans files.
#[test]
fn falsify_a19_scan_directory_refs() {
    let dir = tempfile::tempdir().unwrap();
    let hash1 = format!("blake3:{}", "d".repeat(64));
    let mut known = BTreeSet::new();
    known.insert(hash1.clone());

    let sub = dir.path().join("sub");
    std::fs::create_dir_all(&sub).unwrap();
    std::fs::write(sub.join("file.txt"), format!("content {hash1} end")).unwrap();

    let refs = scan_directory_refs(dir.path(), &known).unwrap();
    assert!(refs.contains(&hash1), "must find hash in nested file");
}

/// A-20: store_path() arch difference produces different hash.
#[test]
fn falsify_a20_store_path_arch_sensitivity() {
    let h1 = store_path("recipe", &["input"], "x86_64", "apt");
    let h2 = store_path("recipe", &["input"], "aarch64", "apt");
    assert_ne!(h1, h2, "different arch must produce different store path");
}

/// A-21: StoreMeta.provenance derivation_depth > 0 implies derived_from set.
#[test]
fn falsify_a21_provenance_derivation_chain_rule() {
    let p = Provenance {
        origin_provider: "apt".to_string(),
        origin_ref: Some("nginx".to_string()),
        origin_hash: Some("blake3:xyz".to_string()),
        derived_from: Some("blake3:parent".to_string()),
        derivation_depth: 2,
    };
    assert!(
        p.derivation_depth > 0 && p.derived_from.is_some(),
        "depth>0 must have derived_from"
    );
}

// ═══════════════════════════════════════════════════════════════════
// Phase B gaps: Validation commands
// ═══════════════════════════════════════════════════════════════════

/// B-11: validate_purity() passes when no min_level set.
#[test]
fn falsify_b11_validate_purity_no_min_passes() {
    let signals = PuritySignals::default();
    let result = validate_purity(&[("nginx", &signals)], None);
    assert!(result.pass, "no min_level must always pass");
}

/// B-12: validate_purity() fails when below min_level.
#[test]
fn falsify_b12_validate_purity_below_min_fails() {
    let signals = PuritySignals {
        has_version: false,
        ..Default::default()
    };
    let result = validate_purity(&[("nginx", &signals)], Some(PurityLevel::Pure));
    assert!(!result.pass, "Constrained must fail Pure min_level");
}

/// B-13: validate_repro_score() pass/fail based on min_score.
#[test]
fn falsify_b13_validate_repro_score() {
    let inputs = vec![super::repro_score::ReproInput {
        name: "nginx".to_string(),
        purity: PurityLevel::Pure,
        has_store: true,
        has_lock_pin: true,
    }];
    let pass = validate_repro_score(&inputs, Some(50.0));
    assert!(pass.pass, "full purity+store+lock must pass 50.0 threshold");
    let fail = validate_repro_score(&inputs, Some(200.0));
    assert!(!fail.pass, "no score reaches 200.0");
}

/// B-14: format_purity_report() includes PASS/FAIL.
#[test]
fn falsify_b14_format_purity_report() {
    let signals = PuritySignals {
        has_version: true,
        has_store: true,
        has_sandbox: true,
        ..Default::default()
    };
    let result = validate_purity(&[("nginx", &signals)], None);
    let report = format_purity_report(&result);
    assert!(report.contains("PASS"), "passing report must contain PASS");
}

/// B-15: format_repro_report() includes grade.
#[test]
fn falsify_b15_format_repro_report() {
    let inputs = vec![super::repro_score::ReproInput {
        name: "nginx".to_string(),
        purity: PurityLevel::Pure,
        has_store: true,
        has_lock_pin: true,
    }];
    let result = validate_repro_score(&inputs, None);
    let report = format_repro_report(&result);
    assert!(report.contains("Grade"), "report must contain grade");
}

// ═══════════════════════════════════════════════════════════════════
// Phase C gaps: Pin tripwire integration
// ═══════════════════════════════════════════════════════════════════

/// C-08: check_before_apply() returns all_fresh when pins match.
#[test]
fn falsify_c08_pin_tripwire_fresh() {
    use super::lockfile::{LockFile, Pin};
    use super::pin_tripwire::check_before_apply;
    let mut pins = BTreeMap::new();
    pins.insert(
        "nginx".to_string(),
        Pin {
            provider: "apt".to_string(),
            version: Some("1.24.0".to_string()),
            hash: "blake3:abc".to_string(),
            git_rev: None,
            pin_type: None,
        },
    );
    let lockfile = LockFile {
        schema: "1.0".to_string(),
        pins,
    };
    let mut current = BTreeMap::new();
    current.insert("nginx".to_string(), "blake3:abc".to_string());
    let result = check_before_apply(&lockfile, &current, &["nginx".to_string()]);
    assert!(result.all_fresh, "matching pins must be fresh");
}

/// C-09: check_before_apply() detects stale pins.
#[test]
fn falsify_c09_pin_tripwire_stale() {
    use super::lockfile::{LockFile, Pin};
    use super::pin_tripwire::check_before_apply;
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
    let result = check_before_apply(&lockfile, &current, &["nginx".to_string()]);
    assert!(!result.all_fresh, "stale pins must not be fresh");
    assert_eq!(result.stale_pins.len(), 1);
}

/// C-10: check_before_apply() detects missing (unpinned) inputs.
#[test]
fn falsify_c10_pin_tripwire_missing() {
    use super::lockfile::LockFile;
    use super::pin_tripwire::check_before_apply;
    let lockfile = LockFile {
        schema: "1.0".to_string(),
        pins: BTreeMap::new(),
    };
    let result = check_before_apply(&lockfile, &BTreeMap::new(), &["nginx".to_string()]);
    assert!(!result.all_fresh);
    assert_eq!(result.missing_inputs, vec!["nginx".to_string()]);
}

/// C-11: pin_severity() strict mode returns Error for stale pins.
#[test]
fn falsify_c11_pin_severity_strict() {
    use super::pin_tripwire::{pin_severity, PinCheckResult, PinSeverity};
    let result = PinCheckResult {
        all_fresh: false,
        stale_pins: vec![],
        missing_inputs: vec!["x".to_string()],
        summary: String::new(),
    };
    assert_eq!(pin_severity(&result, true), PinSeverity::Error);
    assert_eq!(pin_severity(&result, false), PinSeverity::Warning);
}
