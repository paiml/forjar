//! Tests for FJ-1345: Upstream diff and sync.

use super::meta::{Provenance, StoreMeta};
use super::store_diff::{
    build_sync_plan, compute_diff, has_diffable_provenance, upstream_check_command,
};

fn sample_meta(hash: &str, provider: &str, origin_hash: &str, depth: u32) -> StoreMeta {
    StoreMeta {
        schema: "1.0".to_string(),
        store_hash: hash.to_string(),
        recipe_hash: "blake3:recipe".to_string(),
        input_hashes: vec![],
        arch: "x86_64".to_string(),
        provider: provider.to_string(),
        created_at: "2026-03-02T10:00:00Z".to_string(),
        generator: "forjar test".to_string(),
        references: vec![],
        provenance: Some(Provenance {
            origin_provider: provider.to_string(),
            origin_ref: Some(format!("{provider}:pkg")),
            origin_hash: Some(origin_hash.to_string()),
            derived_from: if depth > 0 {
                Some("blake3:parent".to_string())
            } else {
                None
            },
            derivation_depth: depth,
        }),
    }
}

#[test]
fn test_fj1345_diff_unchanged() {
    let meta = sample_meta("blake3:aaa", "apt", "blake3:upstream1", 0);
    let diff = compute_diff(&meta, Some("blake3:upstream1"));
    assert!(!diff.upstream_changed);
    assert_eq!(diff.store_hash, "blake3:aaa");
}

#[test]
fn test_fj1345_diff_changed() {
    let meta = sample_meta("blake3:aaa", "apt", "blake3:upstream1", 0);
    let diff = compute_diff(&meta, Some("blake3:upstream_NEW"));
    assert!(diff.upstream_changed);
}

#[test]
fn test_fj1345_diff_no_upstream_hash() {
    let meta = sample_meta("blake3:aaa", "apt", "blake3:upstream1", 0);
    let diff = compute_diff(&meta, None);
    assert!(!diff.upstream_changed);
}

#[test]
fn test_fj1345_diff_no_local_origin() {
    let mut meta = sample_meta("blake3:aaa", "apt", "blake3:x", 0);
    meta.provenance.as_mut().unwrap().origin_hash = None;
    let diff = compute_diff(&meta, Some("blake3:upstream"));
    assert!(diff.upstream_changed);
}

#[test]
fn test_fj1345_diff_no_provenance() {
    let mut meta = sample_meta("blake3:aaa", "apt", "blake3:x", 0);
    meta.provenance = None;
    let diff = compute_diff(&meta, Some("blake3:upstream"));
    assert!(diff.upstream_changed);
    assert_eq!(diff.provider, "unknown");
}

#[test]
fn test_fj1345_diff_derivation_depth() {
    let meta = sample_meta("blake3:aaa", "nix", "blake3:x", 2);
    let diff = compute_diff(&meta, Some("blake3:new"));
    assert!(diff.upstream_changed);
    assert_eq!(diff.derivation_chain_depth, 2);
}

#[test]
fn test_fj1345_sync_plan_empty() {
    let plan = build_sync_plan(&[]);
    assert_eq!(plan.total_steps, 0);
    assert!(plan.re_imports.is_empty());
    assert!(plan.derivation_replays.is_empty());
}

#[test]
fn test_fj1345_sync_plan_no_changes() {
    let meta = sample_meta("blake3:aaa", "apt", "blake3:same", 0);
    let plan = build_sync_plan(&[(meta, Some("blake3:same".to_string()))]);
    assert_eq!(plan.total_steps, 0);
}

#[test]
fn test_fj1345_sync_plan_reimport() {
    let meta = sample_meta("blake3:aaa", "apt", "blake3:old", 0);
    let plan = build_sync_plan(&[(meta, Some("blake3:new".to_string()))]);
    assert_eq!(plan.re_imports.len(), 1);
    assert_eq!(plan.re_imports[0].provider, "apt");
    assert_eq!(plan.total_steps, 1);
}

#[test]
fn test_fj1345_sync_plan_replay() {
    let meta = sample_meta("blake3:derived", "nix", "blake3:old", 1);
    let plan = build_sync_plan(&[(meta, Some("blake3:new".to_string()))]);
    assert_eq!(plan.derivation_replays.len(), 1);
    assert_eq!(plan.derivation_replays[0].derivation_depth, 1);
    assert_eq!(plan.derivation_replays[0].derived_from, "blake3:parent");
}

#[test]
fn test_fj1345_sync_plan_mixed() {
    let leaf = sample_meta("blake3:leaf", "apt", "blake3:old1", 0);
    let derived = sample_meta("blake3:derived", "apt", "blake3:old2", 1);
    let unchanged = sample_meta("blake3:same", "cargo", "blake3:match", 0);
    let plan = build_sync_plan(&[
        (leaf, Some("blake3:new1".to_string())),
        (derived, Some("blake3:new2".to_string())),
        (unchanged, Some("blake3:match".to_string())),
    ]);
    assert_eq!(plan.re_imports.len(), 1);
    assert_eq!(plan.derivation_replays.len(), 1);
    assert_eq!(plan.total_steps, 2);
}

#[test]
fn test_fj1345_has_diffable_provenance() {
    let meta = sample_meta("blake3:aaa", "apt", "blake3:x", 0);
    assert!(has_diffable_provenance(&meta));
}

#[test]
fn test_fj1345_no_diffable_provenance() {
    let mut meta = sample_meta("blake3:aaa", "apt", "blake3:x", 0);
    meta.provenance = None;
    assert!(!has_diffable_provenance(&meta));
}

#[test]
fn test_fj1345_upstream_check_apt() {
    let meta = sample_meta("blake3:aaa", "apt", "blake3:x", 0);
    let cmd = upstream_check_command(&meta).unwrap();
    assert!(cmd.contains("apt-cache policy"));
}

#[test]
fn test_fj1345_upstream_check_nix() {
    let meta = sample_meta("blake3:aaa", "nix", "blake3:x", 0);
    let cmd = upstream_check_command(&meta).unwrap();
    assert!(cmd.contains("nix flake metadata"));
}

#[test]
fn test_fj1345_upstream_check_docker() {
    let meta = sample_meta("blake3:aaa", "docker", "blake3:x", 0);
    let cmd = upstream_check_command(&meta).unwrap();
    assert!(cmd.contains("docker manifest inspect"));
}

#[test]
fn test_fj1345_upstream_check_cargo() {
    let meta = sample_meta("blake3:aaa", "cargo", "blake3:x", 0);
    let cmd = upstream_check_command(&meta).unwrap();
    assert!(cmd.contains("cargo search"));
}

#[test]
fn test_fj1345_upstream_check_no_provenance() {
    let mut meta = sample_meta("blake3:aaa", "apt", "blake3:x", 0);
    meta.provenance = None;
    assert!(upstream_check_command(&meta).is_none());
}

#[test]
fn test_fj1345_sync_plan_replays_sorted_by_depth() {
    let d1 = sample_meta("blake3:d1", "nix", "blake3:old1", 3);
    let d2 = sample_meta("blake3:d2", "nix", "blake3:old2", 1);
    let d3 = sample_meta("blake3:d3", "nix", "blake3:old3", 2);
    let plan = build_sync_plan(&[
        (d1, Some("blake3:new1".to_string())),
        (d2, Some("blake3:new2".to_string())),
        (d3, Some("blake3:new3".to_string())),
    ]);
    let depths: Vec<u32> = plan
        .derivation_replays
        .iter()
        .map(|r| r.derivation_depth)
        .collect();
    assert_eq!(depths, vec![1, 2, 3]);
}
