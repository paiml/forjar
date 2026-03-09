//! FJ-1322/1348: Substitution protocol executor, conda package reader.
//! Usage: cargo test --test falsification_substitution_conda

use forjar::core::store::cache::{CacheConfig, CacheEntry, CacheInventory, CacheSource};
use forjar::core::store::conda::{parse_conda_index, CondaPackageInfo};
use forjar::core::store::substitution::{
    plan_substitution, requires_build, requires_pull, step_count, SubstitutionContext,
    SubstitutionOutcome, SubstitutionStep,
};
use std::collections::BTreeMap;
use std::path::Path;

// ── helpers ──

fn ssh_source(host: &str, user: &str, port: Option<u16>) -> CacheSource {
    CacheSource::Ssh {
        host: host.into(),
        user: user.into(),
        path: "/cache".into(),
        port,
    }
}

fn local_source(path: &str) -> CacheSource {
    CacheSource::Local { path: path.into() }
}

fn cache_config(sources: Vec<CacheSource>, auto_push: bool) -> CacheConfig {
    CacheConfig {
        sources,
        auto_push,
        max_size_mb: 1024,
    }
}

fn inventory(name: &str, hashes: &[&str]) -> CacheInventory {
    let entries = hashes
        .iter()
        .map(|h| {
            (
                h.to_string(),
                CacheEntry {
                    store_hash: h.to_string(),
                    size_bytes: 1024,
                    created_at: "2026-01-01T00:00:00Z".into(),
                    provider: "apt".into(),
                    arch: "x86_64".into(),
                },
            )
        })
        .collect();
    CacheInventory {
        source_name: name.into(),
        entries,
    }
}

// ── FJ-1322: plan_substitution — local hit ──

#[test]
fn substitution_local_hit() {
    let hash = "blake3:abc123";
    let cc = cache_config(
        vec![ssh_source("cache.example.com", "forjar", Some(2222))],
        true,
    );
    let ctx = SubstitutionContext {
        closure_hash: hash,
        input_hashes: &["blake3:in1".into()],
        local_entries: &[hash.into()],
        cache_config: &cc,
        cache_inventories: &[],
        sandbox: None,
        store_dir: Path::new("/store"),
    };
    let plan = plan_substitution(&ctx);
    assert!(matches!(plan.outcome, SubstitutionOutcome::LocalHit { .. }));
    assert!(!requires_build(&plan));
    assert!(!requires_pull(&plan));
    // Steps: ComputeClosureHash + CheckLocalStore
    assert_eq!(step_count(&plan), 2);
}

#[test]
fn substitution_local_hit_store_path() {
    let hash = "blake3:deadbeef1234567890abcdef";
    let cc = cache_config(vec![], false);
    let ctx = SubstitutionContext {
        closure_hash: hash,
        input_hashes: &[],
        local_entries: &[hash.into()],
        cache_config: &cc,
        cache_inventories: &[],
        sandbox: None,
        store_dir: Path::new("/forjar/store"),
    };
    let plan = plan_substitution(&ctx);
    if let SubstitutionOutcome::LocalHit { store_path } = &plan.outcome {
        assert!(store_path.contains("deadbeef1234567890abcdef"));
        assert!(store_path.contains("/forjar/store/"));
    } else {
        panic!("expected LocalHit");
    }
}

// ── FJ-1322: plan_substitution — cache hit ──

#[test]
fn substitution_cache_hit() {
    let hash = "blake3:remote_hash";
    let cc = cache_config(
        vec![ssh_source("cache.example.com", "forjar", Some(2222))],
        false,
    );
    let inv = inventory("forjar@cache.example.com", &[hash]);
    let ctx = SubstitutionContext {
        closure_hash: hash,
        input_hashes: &[],
        local_entries: &[],
        cache_config: &cc,
        cache_inventories: &[inv],
        sandbox: None,
        store_dir: Path::new("/store"),
    };
    let plan = plan_substitution(&ctx);
    assert!(matches!(plan.outcome, SubstitutionOutcome::CacheHit { .. }));
    assert!(!requires_build(&plan));
    assert!(requires_pull(&plan));
    // Steps: ComputeClosureHash + CheckLocalStore + CheckSshCache + PullFromCache
    assert_eq!(step_count(&plan), 4);
}

#[test]
fn substitution_cache_hit_pull_command() {
    let hash = "blake3:remote_hash";
    let cc = cache_config(
        vec![ssh_source("cache.example.com", "forjar", Some(2222))],
        false,
    );
    let inv = inventory("forjar@cache.example.com", &[hash]);
    let ctx = SubstitutionContext {
        closure_hash: hash,
        input_hashes: &[],
        local_entries: &[],
        cache_config: &cc,
        cache_inventories: &[inv],
        sandbox: None,
        store_dir: Path::new("/store"),
    };
    let plan = plan_substitution(&ctx);
    let pull_step = plan
        .steps
        .iter()
        .find(|s| matches!(s, SubstitutionStep::PullFromCache { .. }));
    assert!(pull_step.is_some());
    if let SubstitutionStep::PullFromCache { command, .. } = pull_step.unwrap() {
        assert!(command.contains("rsync"));
        assert!(command.contains("-p 2222"));
    }
}

// ── FJ-1322: plan_substitution — cache miss ──

#[test]
fn substitution_cache_miss() {
    let hash = "blake3:not_anywhere";
    let cc = cache_config(vec![ssh_source("cache.example.com", "forjar", None)], false);
    let inv = inventory("forjar@cache.example.com", &[]); // no matching hash
    let ctx = SubstitutionContext {
        closure_hash: hash,
        input_hashes: &["blake3:in1".into()],
        local_entries: &[],
        cache_config: &cc,
        cache_inventories: &[inv],
        sandbox: None,
        store_dir: Path::new("/store"),
    };
    let plan = plan_substitution(&ctx);
    assert!(matches!(
        plan.outcome,
        SubstitutionOutcome::CacheMiss { .. }
    ));
    assert!(requires_build(&plan));
    assert!(!requires_pull(&plan));
}

#[test]
fn substitution_miss_auto_push() {
    let hash = "blake3:build_me";
    let cc = cache_config(
        vec![ssh_source("cache.example.com", "forjar", Some(22))],
        true,
    );
    let inv = inventory("forjar@cache.example.com", &[]);
    let ctx = SubstitutionContext {
        closure_hash: hash,
        input_hashes: &[],
        local_entries: &[],
        cache_config: &cc,
        cache_inventories: &[inv],
        sandbox: None,
        store_dir: Path::new("/store"),
    };
    let plan = plan_substitution(&ctx);
    let push = plan
        .steps
        .iter()
        .find(|s| matches!(s, SubstitutionStep::PushToCache { .. }));
    assert!(push.is_some(), "auto_push should add PushToCache step");
}

#[test]
fn substitution_miss_no_auto_push() {
    let hash = "blake3:build_me";
    let cc = cache_config(vec![ssh_source("cache.example.com", "forjar", None)], false);
    let ctx = SubstitutionContext {
        closure_hash: hash,
        input_hashes: &[],
        local_entries: &[],
        cache_config: &cc,
        cache_inventories: &[],
        sandbox: None,
        store_dir: Path::new("/store"),
    };
    let plan = plan_substitution(&ctx);
    let push = plan
        .steps
        .iter()
        .find(|s| matches!(s, SubstitutionStep::PushToCache { .. }));
    assert!(push.is_none());
}

// ── FJ-1322: local sources skipped in SSH checks ──

#[test]
fn substitution_skips_local_sources() {
    let hash = "blake3:check_local_only";
    let cc = cache_config(vec![local_source("/var/cache")], false);
    let ctx = SubstitutionContext {
        closure_hash: hash,
        input_hashes: &[],
        local_entries: &[],
        cache_config: &cc,
        cache_inventories: &[],
        sandbox: None,
        store_dir: Path::new("/store"),
    };
    let plan = plan_substitution(&ctx);
    // No SSH checks for local sources
    let ssh_checks = plan
        .steps
        .iter()
        .filter(|s| matches!(s, SubstitutionStep::CheckSshCache { .. }))
        .count();
    assert_eq!(ssh_checks, 0);
    assert!(requires_build(&plan));
}

// ── FJ-1322: multiple SSH sources checked in order ──

#[test]
fn substitution_checks_ssh_in_order() {
    let hash = "blake3:multi_cache";
    let cc = cache_config(
        vec![
            ssh_source("cache1.example.com", "u1", None),
            ssh_source("cache2.example.com", "u2", Some(2222)),
        ],
        false,
    );
    let inv1 = inventory("u1@cache1.example.com", &[]);
    let inv2 = inventory("u2@cache2.example.com", &[hash]);
    let ctx = SubstitutionContext {
        closure_hash: hash,
        input_hashes: &[],
        local_entries: &[],
        cache_config: &cc,
        cache_inventories: &[inv1, inv2],
        sandbox: None,
        store_dir: Path::new("/store"),
    };
    let plan = plan_substitution(&ctx);
    assert!(matches!(plan.outcome, SubstitutionOutcome::CacheHit { .. }));
    if let SubstitutionOutcome::CacheHit { source, .. } = &plan.outcome {
        assert_eq!(source, "u2@cache2.example.com");
    }
}

// ── FJ-1322: step types ──

#[test]
fn substitution_step_types_complete() {
    let hash = "blake3:full_plan";
    let cc = cache_config(vec![ssh_source("cache.example.com", "forjar", None)], true);
    let inv = inventory("forjar@cache.example.com", &[]);
    let ctx = SubstitutionContext {
        closure_hash: hash,
        input_hashes: &["blake3:in1".into(), "blake3:in2".into()],
        local_entries: &[],
        cache_config: &cc,
        cache_inventories: &[inv],
        sandbox: None,
        store_dir: Path::new("/store"),
    };
    let plan = plan_substitution(&ctx);
    // Should have: ComputeClosureHash, CheckLocalStore, CheckSshCache,
    // BuildFromScratch, StoreResult, PushToCache
    assert!(plan
        .steps
        .iter()
        .any(|s| matches!(s, SubstitutionStep::ComputeClosureHash { .. })));
    assert!(plan
        .steps
        .iter()
        .any(|s| matches!(s, SubstitutionStep::CheckLocalStore { .. })));
    assert!(plan
        .steps
        .iter()
        .any(|s| matches!(s, SubstitutionStep::CheckSshCache { .. })));
    assert!(plan
        .steps
        .iter()
        .any(|s| matches!(s, SubstitutionStep::BuildFromScratch { .. })));
    assert!(plan
        .steps
        .iter()
        .any(|s| matches!(s, SubstitutionStep::StoreResult { .. })));
    assert!(plan
        .steps
        .iter()
        .any(|s| matches!(s, SubstitutionStep::PushToCache { .. })));
}

#[test]
fn substitution_compute_closure_records_inputs() {
    let hash = "blake3:h";
    let input_hashes = vec!["blake3:in1".into(), "blake3:in2".into()];
    let cc = cache_config(vec![], false);
    let ctx = SubstitutionContext {
        closure_hash: hash,
        input_hashes: &input_hashes,
        local_entries: &[hash.into()],
        cache_config: &cc,
        cache_inventories: &[],
        sandbox: None,
        store_dir: Path::new("/store"),
    };
    let plan = plan_substitution(&ctx);
    if let SubstitutionStep::ComputeClosureHash {
        input_hashes: recorded,
        ..
    } = &plan.steps[0]
    {
        assert_eq!(recorded.len(), 2);
    } else {
        panic!("first step should be ComputeClosureHash");
    }
}

// ── FJ-1348: parse_conda_index ──

#[test]
fn conda_index_full() {
    let json = r#"{"name": "numpy", "version": "1.26.4", "build": "py312h01234",
        "arch": "x86_64", "subdir": "linux-64"}"#;
    let info = parse_conda_index(json).unwrap();
    assert_eq!(info.name, "numpy");
    assert_eq!(info.version, "1.26.4");
    assert_eq!(info.build, "py312h01234");
    assert_eq!(info.arch, "x86_64");
    assert_eq!(info.subdir, "linux-64");
    assert!(info.files.is_empty());
}

#[test]
fn conda_index_minimal() {
    let json = r#"{"name": "pip", "version": "24.0"}"#;
    let info = parse_conda_index(json).unwrap();
    assert_eq!(info.name, "pip");
    assert_eq!(info.version, "24.0");
    assert_eq!(info.build, "");
    assert_eq!(info.arch, "noarch");
    assert_eq!(info.subdir, "noarch");
}

#[test]
fn conda_index_missing_name() {
    let json = r#"{"version": "1.0"}"#;
    assert!(parse_conda_index(json).is_err());
}

#[test]
fn conda_index_missing_version() {
    let json = r#"{"name": "pkg"}"#;
    assert!(parse_conda_index(json).is_err());
}

#[test]
fn conda_index_invalid_json() {
    assert!(parse_conda_index("not json").is_err());
}
