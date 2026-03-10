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
// ============================================================================

#[test]
fn purity_pure_all_signals() {
    let signals = PuritySignals {
        has_version: true,
        has_store: true,
        has_sandbox: true,
        has_curl_pipe: false,
        dep_levels: vec![],
    };
    let result = classify("pkg", &signals);
    assert_eq!(result.level, PurityLevel::Pure);
    assert_eq!(result.name, "pkg");
}

#[test]
fn purity_pinned_no_sandbox() {
    let signals = PuritySignals {
        has_version: true,
        has_store: true,
        has_sandbox: false,
        has_curl_pipe: false,
        dep_levels: vec![],
    };
    let result = classify("pkg", &signals);
    assert_eq!(result.level, PurityLevel::Pinned);
}

#[test]
fn purity_pinned_no_store() {
    let signals = PuritySignals {
        has_version: true,
        has_store: false,
        has_sandbox: true,
        has_curl_pipe: false,
        dep_levels: vec![],
    };
    let result = classify("pkg", &signals);
    assert_eq!(result.level, PurityLevel::Pinned);
}

#[test]
fn purity_constrained_no_version() {
    let signals = PuritySignals {
        has_version: false,
        has_store: true,
        has_sandbox: true,
        has_curl_pipe: false,
        dep_levels: vec![],
    };
    let result = classify("pkg", &signals);
    assert_eq!(result.level, PurityLevel::Constrained);
}

#[test]
fn purity_impure_curl_pipe() {
    let signals = PuritySignals {
        has_version: true,
        has_store: true,
        has_sandbox: true,
        has_curl_pipe: true,
        dep_levels: vec![],
    };
    let result = classify("pkg", &signals);
    assert_eq!(result.level, PurityLevel::Impure);
}

// ============================================================================
// FJ-1305: Purity — monotonicity invariant
// ============================================================================

#[test]
fn purity_dep_elevation() {
    let signals = PuritySignals {
        has_version: true,
        has_store: true,
        has_sandbox: true,
        has_curl_pipe: false,
        dep_levels: vec![PurityLevel::Impure],
    };
    let result = classify("pkg", &signals);
    assert_eq!(
        result.level,
        PurityLevel::Impure,
        "monotonicity: impure dep elevates pure resource"
    );
}

#[test]
fn purity_dep_no_elevation_when_own_is_worse() {
    let signals = PuritySignals {
        has_version: false,
        has_store: false,
        has_sandbox: false,
        has_curl_pipe: false,
        dep_levels: vec![PurityLevel::Pure],
    };
    let result = classify("pkg", &signals);
    assert_eq!(result.level, PurityLevel::Constrained);
}

#[test]
fn purity_dep_elevation_reason() {
    let signals = PuritySignals {
        has_version: true,
        has_store: true,
        has_sandbox: true,
        has_curl_pipe: false,
        dep_levels: vec![PurityLevel::Constrained],
    };
    let result = classify("pkg", &signals);
    assert_eq!(result.level, PurityLevel::Constrained);
    assert!(result.reasons.iter().any(|r| r.contains("elevates")));
}

// ============================================================================
// FJ-1305: recipe_purity and level_label
// ============================================================================

#[test]
fn recipe_purity_max_wins() {
    let levels = vec![PurityLevel::Pure, PurityLevel::Pinned, PurityLevel::Impure];
    assert_eq!(recipe_purity(&levels), PurityLevel::Impure);
}

#[test]
fn recipe_purity_empty_is_pure() {
    assert_eq!(recipe_purity(&[]), PurityLevel::Pure);
}

#[test]
fn recipe_purity_all_pure() {
    let levels = vec![PurityLevel::Pure, PurityLevel::Pure];
    assert_eq!(recipe_purity(&levels), PurityLevel::Pure);
}

#[test]
fn level_label_all_levels() {
    assert_eq!(level_label(PurityLevel::Pure), "Pure (0)");
    assert_eq!(level_label(PurityLevel::Pinned), "Pinned (1)");
    assert_eq!(level_label(PurityLevel::Constrained), "Constrained (2)");
    assert_eq!(level_label(PurityLevel::Impure), "Impure (3)");
}

// ============================================================================
// FJ-1329: Reproducibility — compute_score
// ============================================================================

#[test]
fn repro_empty_inputs_perfect_score() {
    let score = compute_score(&[]);
    assert_eq!(score.composite, 100.0);
    assert!(score.resources.is_empty());
}

#[test]
fn repro_all_pure_with_store_and_lock() {
    let inputs = vec![ReproInput {
        name: "pkg".into(),
        purity: PurityLevel::Pure,
        has_store: true,
        has_lock_pin: true,
    }];
    let score = compute_score(&inputs);
    assert_eq!(score.composite, 100.0);
    assert_eq!(score.purity_score, 100.0);
    assert_eq!(score.store_score, 100.0);
    assert_eq!(score.lock_score, 100.0);
}

#[test]
fn repro_impure_no_store_no_lock() {
    let inputs = vec![ReproInput {
        name: "pkg".into(),
        purity: PurityLevel::Impure,
        has_store: false,
        has_lock_pin: false,
    }];
    let score = compute_score(&inputs);
    assert_eq!(score.composite, 0.0);
}

#[test]
fn repro_mixed_purity() {
    let inputs = vec![
        ReproInput {
            name: "a".into(),
            purity: PurityLevel::Pure,
            has_store: true,
            has_lock_pin: true,
        },
        ReproInput {
            name: "b".into(),
            purity: PurityLevel::Impure,
            has_store: false,
            has_lock_pin: false,
        },
    ];
    let score = compute_score(&inputs);
    // purity: (100+0)/2=50, store: 50, lock: 50
    // composite = 50*0.5 + 50*0.3 + 50*0.2 = 50
    assert_eq!(score.composite, 50.0);
}

#[test]
fn repro_per_resource_breakdown() {
    let inputs = vec![
        ReproInput {
            name: "nginx".into(),
            purity: PurityLevel::Pure,
            has_store: true,
            has_lock_pin: true,
        },
        ReproInput {
            name: "mysql".into(),
            purity: PurityLevel::Pinned,
            has_store: true,
            has_lock_pin: false,
        },
    ];
    let score = compute_score(&inputs);
    assert_eq!(score.resources.len(), 2);
    assert_eq!(score.resources[0].name, "nginx");
    assert_eq!(score.resources[1].name, "mysql");
}

// ============================================================================
// FJ-1329: grade thresholds
// ============================================================================

#[test]
fn repro_grade_thresholds() {
    assert_eq!(grade(100.0), "A");
    assert_eq!(grade(90.0), "A");
    assert_eq!(grade(89.9), "B");
    assert_eq!(grade(75.0), "B");
    assert_eq!(grade(74.9), "C");
    assert_eq!(grade(50.0), "C");
    assert_eq!(grade(49.9), "D");
    assert_eq!(grade(25.0), "D");
    assert_eq!(grade(24.9), "F");
    assert_eq!(grade(0.0), "F");
}

// ============================================================================
// FJ-1345: store_diff — compute_diff
// ============================================================================

#[test]
fn diff_no_change_when_hashes_match() {
    let meta = meta_with_provenance("sh1", "apt", Some("nginx"), Some("h1"), 0);
    let diff = compute_diff(&meta, Some("h1"));
    assert!(!diff.upstream_changed);
    assert_eq!(diff.store_hash, "sh1");
}

#[test]
fn diff_changed_when_hashes_differ() {
    let meta = meta_with_provenance("sh1", "apt", Some("nginx"), Some("h1"), 0);
    let diff = compute_diff(&meta, Some("h2-new"));
    assert!(diff.upstream_changed);
    assert_eq!(diff.upstream_hash.as_deref(), Some("h2-new"));
}

#[test]
fn diff_changed_when_no_local_origin_hash() {
    let meta = meta_with_provenance("sh1", "apt", Some("nginx"), None, 0);
    let diff = compute_diff(&meta, Some("h1"));
    assert!(diff.upstream_changed);
}

#[test]
fn diff_not_changed_when_no_upstream_hash() {
    let meta = meta_with_provenance("sh1", "apt", Some("nginx"), Some("h1"), 0);
    let diff = compute_diff(&meta, None);
    assert!(!diff.upstream_changed);
}

#[test]
fn diff_no_provenance() {
    let meta = meta_no_provenance("sh1");
    let diff = compute_diff(&meta, Some("h1"));
    assert!(diff.upstream_changed); // no local origin → changed
    assert_eq!(diff.provider, "unknown");
}

#[test]
fn diff_derivation_chain_depth() {
    let meta = meta_with_provenance("sh1", "cargo", Some("crate"), Some("h1"), 3);
    let diff = compute_diff(&meta, Some("h1"));
    assert_eq!(diff.derivation_chain_depth, 3);
}

// ============================================================================
// FJ-1345: build_sync_plan
// ============================================================================

#[test]
fn sync_plan_no_changes_empty() {
    let entries = vec![(
        meta_with_provenance("sh1", "apt", Some("nginx"), Some("h1"), 0),
        Some("h1".to_string()),
    )];
    let plan = build_sync_plan(&entries);
    assert_eq!(plan.total_steps, 0);
    assert!(plan.re_imports.is_empty());
}

#[test]
fn sync_plan_single_re_import() {
    let entries = vec![(
        meta_with_provenance("sh1", "apt", Some("nginx"), Some("h-old"), 0),
        Some("h-new".to_string()),
    )];
    let plan = build_sync_plan(&entries);
    assert_eq!(plan.total_steps, 1);
    assert_eq!(plan.re_imports.len(), 1);
    assert_eq!(plan.re_imports[0].provider, "apt");
    assert_eq!(plan.re_imports[0].origin_ref, "nginx");
}

#[test]
fn sync_plan_derivation_replay() {
    let mut meta = meta_with_provenance("sh2", "cargo", Some("crate"), Some("h-old"), 2);
    meta.provenance.as_mut().unwrap().derived_from = Some("sh1".into());
    let entries = vec![(meta, Some("h-new".to_string()))];
    let plan = build_sync_plan(&entries);
    assert_eq!(plan.derivation_replays.len(), 1);
    assert_eq!(plan.derivation_replays[0].derived_from, "sh1");
    assert_eq!(plan.derivation_replays[0].derivation_depth, 2);
}

// ============================================================================
// FJ-1345: has_diffable_provenance
// ============================================================================

#[test]
fn diffable_with_origin_hash() {
    let meta = meta_with_provenance("sh1", "apt", None, Some("h1"), 0);
    assert!(has_diffable_provenance(&meta));
}

#[test]
fn diffable_with_origin_ref() {
    let meta = meta_with_provenance("sh1", "apt", Some("nginx"), None, 0);
    assert!(has_diffable_provenance(&meta));
}

#[test]
fn not_diffable_without_provenance() {
    let meta = meta_no_provenance("sh1");
    assert!(!has_diffable_provenance(&meta));
}

// ============================================================================
// FJ-1345: upstream_check_command
// ============================================================================

#[test]
fn upstream_cmd_apt() {
    let meta = meta_with_provenance("sh1", "apt", Some("nginx"), Some("h"), 0);
    let cmd = upstream_check_command(&meta);
    assert_eq!(cmd.as_deref(), Some("apt-cache policy nginx"));
}

#[test]
fn upstream_cmd_cargo() {
    let meta = meta_with_provenance("sh1", "cargo", Some("serde"), Some("h"), 0);
    let cmd = upstream_check_command(&meta);
    assert_eq!(cmd.as_deref(), Some("cargo search serde"));
}

#[test]
fn upstream_cmd_nix() {
    let meta = meta_with_provenance("sh1", "nix", Some("nixpkgs#nginx"), Some("h"), 0);
    let cmd = upstream_check_command(&meta);
    assert_eq!(cmd.as_deref(), Some("nix flake metadata nixpkgs#nginx"));
}

#[test]
fn upstream_cmd_docker() {
    let meta = meta_with_provenance("sh1", "docker", Some("nginx:latest"), Some("h"), 0);
    let cmd = upstream_check_command(&meta);
    assert_eq!(cmd.as_deref(), Some("docker manifest inspect nginx:latest"));
}

#[test]
fn upstream_cmd_unknown_provider() {
    let meta = meta_with_provenance("sh1", "unknown", Some("ref"), Some("h"), 0);
    assert!(upstream_check_command(&meta).is_none());
}

#[test]
fn upstream_cmd_no_origin_ref() {
    let meta = meta_with_provenance("sh1", "apt", None, Some("h"), 0);
    assert!(upstream_check_command(&meta).is_none());
}

// ============================================================================
// FJ-1356: is_encrypted
// ============================================================================

#[test]
fn encrypted_age_marker() {
    assert!(is_encrypted("ENC[age,abc123]"));
    assert!(is_encrypted("prefix ENC[age,data] suffix"));
}
