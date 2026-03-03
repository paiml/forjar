//! Tests for FJ-1362: Store diff/sync execution.

use super::meta::{Provenance, StoreMeta};
use super::store_diff::{
    build_sync_plan, compute_diff, has_diffable_provenance, upstream_check_command, DiffResult,
};
use super::sync_exec::{DiffExecResult, SyncExecResult};

fn meta_with_provenance(
    hash: &str,
    provider: &str,
    origin_ref: &str,
    origin_hash: &str,
) -> StoreMeta {
    StoreMeta {
        schema: "1.0".to_string(),
        store_hash: format!("blake3:{hash}"),
        recipe_hash: "test".to_string(),
        input_hashes: Vec::new(),
        arch: "x86_64".to_string(),
        provider: provider.to_string(),
        created_at: "2026-01-01T00:00:00Z".to_string(),
        generator: "test".to_string(),
        references: Vec::new(),
        provenance: Some(Provenance {
            origin_provider: provider.to_string(),
            origin_ref: Some(origin_ref.to_string()),
            origin_hash: Some(origin_hash.to_string()),
            derived_from: None,
            derivation_depth: 0,
        }),
    }
}

fn meta_without_provenance(hash: &str) -> StoreMeta {
    StoreMeta {
        schema: "1.0".to_string(),
        store_hash: format!("blake3:{hash}"),
        recipe_hash: "test".to_string(),
        input_hashes: Vec::new(),
        arch: "x86_64".to_string(),
        provider: "test".to_string(),
        created_at: "2026-01-01T00:00:00Z".to_string(),
        generator: "test".to_string(),
        references: Vec::new(),
        provenance: None,
    }
}

#[test]
fn diff_detects_upstream_change() {
    let meta = meta_with_provenance("aaa", "apt", "curl", "blake3:old_hash");
    let diff = compute_diff(&meta, Some("blake3:new_hash"));
    assert!(diff.upstream_changed);
    assert_eq!(diff.upstream_hash, Some("blake3:new_hash".to_string()));
}

#[test]
fn diff_no_change_when_hashes_match() {
    let meta = meta_with_provenance("aaa", "apt", "curl", "blake3:same_hash");
    let diff = compute_diff(&meta, Some("blake3:same_hash"));
    assert!(!diff.upstream_changed);
}

#[test]
fn diff_no_upstream_hash_means_no_change() {
    let meta = meta_with_provenance("aaa", "apt", "curl", "blake3:old");
    let diff = compute_diff(&meta, None);
    assert!(!diff.upstream_changed);
}

#[test]
fn diff_no_local_origin_but_upstream_exists() {
    let mut meta = meta_with_provenance("aaa", "apt", "curl", "blake3:old");
    meta.provenance.as_mut().unwrap().origin_hash = None;
    let diff = compute_diff(&meta, Some("blake3:new"));
    assert!(diff.upstream_changed);
}

#[test]
fn upstream_check_command_for_apt() {
    let meta = meta_with_provenance("aaa", "apt", "curl", "hash");
    let cmd = upstream_check_command(&meta).unwrap();
    assert_eq!(cmd, "apt-cache policy curl");
}

#[test]
fn upstream_check_command_for_cargo() {
    let meta = meta_with_provenance("aaa", "cargo", "ripgrep", "hash");
    let cmd = upstream_check_command(&meta).unwrap();
    assert_eq!(cmd, "cargo search ripgrep");
}

#[test]
fn upstream_check_command_for_docker() {
    let meta = meta_with_provenance("aaa", "docker", "alpine:3.18", "hash");
    let cmd = upstream_check_command(&meta).unwrap();
    assert!(cmd.contains("docker manifest inspect"));
}

#[test]
fn upstream_check_command_for_nix() {
    let meta = meta_with_provenance("aaa", "nix", "nixpkgs#ripgrep", "hash");
    let cmd = upstream_check_command(&meta).unwrap();
    assert!(cmd.contains("nix flake metadata"));
}

#[test]
fn upstream_check_command_none_for_unknown() {
    let meta = meta_with_provenance("aaa", "custom", "pkg", "hash");
    assert!(upstream_check_command(&meta).is_none());
}

#[test]
fn has_diffable_provenance_true() {
    let meta = meta_with_provenance("aaa", "apt", "curl", "hash");
    assert!(has_diffable_provenance(&meta));
}

#[test]
fn has_diffable_provenance_false_without() {
    let meta = meta_without_provenance("aaa");
    assert!(!has_diffable_provenance(&meta));
}

#[test]
fn build_sync_plan_with_leaf_reimport() {
    let meta = meta_with_provenance("aaa", "apt", "curl", "blake3:old");
    let entries = vec![(meta, Some("blake3:new".to_string()))];
    let plan = build_sync_plan(&entries);
    assert_eq!(plan.re_imports.len(), 1);
    assert_eq!(plan.derivation_replays.len(), 0);
    assert_eq!(plan.total_steps, 1);
    assert_eq!(plan.re_imports[0].provider, "apt");
    assert_eq!(plan.re_imports[0].origin_ref, "curl");
}

#[test]
fn build_sync_plan_with_derivation_replay() {
    let mut meta = meta_with_provenance("bbb", "apt", "curl", "blake3:old");
    meta.provenance.as_mut().unwrap().derivation_depth = 1;
    meta.provenance.as_mut().unwrap().derived_from = Some("blake3:parent".to_string());

    let entries = vec![(meta, Some("blake3:new".to_string()))];
    let plan = build_sync_plan(&entries);
    assert_eq!(plan.re_imports.len(), 0);
    assert_eq!(plan.derivation_replays.len(), 1);
    assert_eq!(plan.derivation_replays[0].derivation_depth, 1);
}

#[test]
fn build_sync_plan_no_change_is_empty() {
    let meta = meta_with_provenance("aaa", "apt", "curl", "blake3:same");
    let entries = vec![(meta, Some("blake3:same".to_string()))];
    let plan = build_sync_plan(&entries);
    assert_eq!(plan.total_steps, 0);
}

#[test]
fn build_sync_plan_mixed() {
    let meta1 = meta_with_provenance("aaa", "apt", "curl", "blake3:old1");
    let mut meta2 = meta_with_provenance("bbb", "cargo", "rg", "blake3:old2");
    meta2.provenance.as_mut().unwrap().derivation_depth = 1;
    meta2.provenance.as_mut().unwrap().derived_from = Some("blake3:aaa".to_string());
    let meta3 = meta_with_provenance("ccc", "nix", "hello", "blake3:same");

    let entries = vec![
        (meta1, Some("blake3:new1".to_string())),
        (meta2, Some("blake3:new2".to_string())),
        (meta3, Some("blake3:same".to_string())),
    ];
    let plan = build_sync_plan(&entries);
    assert_eq!(plan.re_imports.len(), 1);
    assert_eq!(plan.derivation_replays.len(), 1);
    assert_eq!(plan.total_steps, 2);
}

#[test]
fn diff_exec_result_fields() {
    let diff = DiffResult {
        store_hash: "blake3:aaa".to_string(),
        upstream_changed: true,
        local_origin_hash: Some("blake3:old".to_string()),
        upstream_hash: Some("blake3:new".to_string()),
        provider: "apt".to_string(),
        origin_ref: Some("curl".to_string()),
        derivation_chain_depth: 0,
    };
    let result = DiffExecResult {
        diff: diff.clone(),
        upstream_command: Some("apt-cache policy curl".to_string()),
        upstream_output: Some("Candidate: 7.88.1".to_string()),
    };
    assert!(result.diff.upstream_changed);
    assert!(result.upstream_command.is_some());
}

#[test]
fn sync_exec_result_fields() {
    let result = SyncExecResult {
        re_imported: Vec::new(),
        derivations_replayed: 3,
        new_profile_hash: None,
    };
    assert_eq!(result.derivations_replayed, 3);
    assert!(result.re_imported.is_empty());
}

#[test]
fn derivation_replay_sorted_by_depth() {
    let mut meta1 = meta_with_provenance("aaa", "apt", "x", "blake3:old1");
    meta1.provenance.as_mut().unwrap().derivation_depth = 2;
    meta1.provenance.as_mut().unwrap().derived_from = Some("blake3:p1".to_string());

    let mut meta2 = meta_with_provenance("bbb", "apt", "y", "blake3:old2");
    meta2.provenance.as_mut().unwrap().derivation_depth = 1;
    meta2.provenance.as_mut().unwrap().derived_from = Some("blake3:p2".to_string());

    let entries = vec![
        (meta1, Some("blake3:new1".to_string())),
        (meta2, Some("blake3:new2".to_string())),
    ];
    let plan = build_sync_plan(&entries);
    assert_eq!(plan.derivation_replays.len(), 2);
    // Should be sorted by depth (ascending)
    assert!(
        plan.derivation_replays[0].derivation_depth <= plan.derivation_replays[1].derivation_depth
    );
}

// ===== sync_exec helper function tests =====

use super::provider::ImportProvider;
use super::sync_exec::{parse_provider, tempdir_for_reimport};

#[test]
fn parse_provider_all_valid() {
    assert!(matches!(parse_provider("apt"), Ok(ImportProvider::Apt)));
    assert!(matches!(parse_provider("cargo"), Ok(ImportProvider::Cargo)));
    assert!(matches!(parse_provider("uv"), Ok(ImportProvider::Uv)));
    assert!(matches!(parse_provider("nix"), Ok(ImportProvider::Nix)));
    assert!(matches!(
        parse_provider("docker"),
        Ok(ImportProvider::Docker)
    ));
    assert!(matches!(parse_provider("tofu"), Ok(ImportProvider::Tofu)));
    assert!(matches!(
        parse_provider("terraform"),
        Ok(ImportProvider::Terraform)
    ));
    assert!(matches!(parse_provider("apr"), Ok(ImportProvider::Apr)));
}

#[test]
fn parse_provider_unknown_returns_error() {
    let result = parse_provider("pip");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unknown provider"));
}

#[test]
fn parse_provider_empty_returns_error() {
    assert!(parse_provider("").is_err());
}

#[test]
fn tempdir_for_reimport_strips_prefix() {
    let path = tempdir_for_reimport("blake3:abcdef1234567890");
    assert!(path.to_str().unwrap().contains("abcdef1234567890"));
    assert!(!path.to_str().unwrap().contains("blake3:"));
}

#[test]
fn tempdir_for_reimport_raw_hash() {
    let path = tempdir_for_reimport("rawvalue123456789");
    assert!(path.to_str().unwrap().contains("rawvalue12345678"));
}

#[test]
fn tempdir_for_reimport_short_hash() {
    let path = tempdir_for_reimport("blake3:abc");
    assert!(path.to_str().unwrap().contains("abc"));
}

#[test]
fn diff_exec_result_with_no_upstream() {
    let diff = DiffResult {
        store_hash: "blake3:aaa".to_string(),
        upstream_changed: false,
        local_origin_hash: Some("blake3:old".to_string()),
        upstream_hash: None,
        provider: "apt".to_string(),
        origin_ref: Some("curl".to_string()),
        derivation_chain_depth: 0,
    };
    let result = DiffExecResult {
        diff,
        upstream_command: None,
        upstream_output: None,
    };
    assert!(!result.diff.upstream_changed);
    assert!(result.upstream_command.is_none());
    assert!(result.upstream_output.is_none());
}

#[test]
fn upstream_check_command_for_uv_returns_none() {
    // uv is not a diffable provider in store_diff
    let meta = meta_with_provenance("aaa", "uv", "requests", "hash");
    assert!(upstream_check_command(&meta).is_none());
}

#[test]
fn upstream_check_command_for_tofu() {
    let meta = meta_with_provenance("aaa", "tofu", "./infra", "hash");
    let cmd = upstream_check_command(&meta).unwrap();
    assert!(cmd.contains("tofu plan"));
}

#[test]
fn upstream_check_command_for_apr() {
    let meta = meta_with_provenance("aaa", "apr", "mistral-7b", "hash");
    let cmd = upstream_check_command(&meta).unwrap();
    assert!(cmd.contains("apr info"));
}

#[test]
fn build_sync_plan_empty_input() {
    let entries: Vec<(StoreMeta, Option<String>)> = vec![];
    let plan = build_sync_plan(&entries);
    assert_eq!(plan.total_steps, 0);
    assert!(plan.re_imports.is_empty());
    assert!(plan.derivation_replays.is_empty());
}

// ── Individual parse_provider arm tests ────────────────────────

#[test]
fn parse_provider_apt() {
    assert_eq!(parse_provider("apt").unwrap(), ImportProvider::Apt);
}

#[test]
fn parse_provider_cargo() {
    assert_eq!(parse_provider("cargo").unwrap(), ImportProvider::Cargo);
}

#[test]
fn parse_provider_uv() {
    assert_eq!(parse_provider("uv").unwrap(), ImportProvider::Uv);
}

#[test]
fn parse_provider_nix() {
    assert_eq!(parse_provider("nix").unwrap(), ImportProvider::Nix);
}

#[test]
fn parse_provider_docker() {
    assert_eq!(parse_provider("docker").unwrap(), ImportProvider::Docker);
}

#[test]
fn parse_provider_tofu() {
    assert_eq!(parse_provider("tofu").unwrap(), ImportProvider::Tofu);
}

#[test]
fn parse_provider_terraform() {
    assert_eq!(
        parse_provider("terraform").unwrap(),
        ImportProvider::Terraform
    );
}

#[test]
fn parse_provider_apr() {
    assert_eq!(parse_provider("apr").unwrap(), ImportProvider::Apr);
}

#[test]
fn parse_provider_other_error() {
    let err = parse_provider("brew").unwrap_err();
    assert!(err.contains("unknown provider: brew"));
}
