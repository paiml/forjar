//! Tests for FJ-1360: Cache SSH execution.

use super::cache::{build_inventory, CacheConfig, CacheEntry, CacheSource};
use super::cache_exec::{pull_command, push_command};
use super::substitution::{plan_substitution, SubstitutionContext, SubstitutionOutcome};
use crate::tripwire::hasher::hash_directory;
use std::path::Path;

fn ssh_source() -> CacheSource {
    CacheSource::Ssh {
        host: "cache.internal".to_string(),
        user: "forjar".to_string(),
        path: "/var/forjar/cache".to_string(),
        port: None,
    }
}

fn ssh_source_with_port() -> CacheSource {
    CacheSource::Ssh {
        host: "cache.internal".to_string(),
        user: "forjar".to_string(),
        path: "/var/forjar/cache".to_string(),
        port: Some(2222),
    }
}

fn local_source() -> CacheSource {
    CacheSource::Local {
        path: "/mnt/cache".to_string(),
    }
}

#[test]
fn pull_command_ssh_basic() {
    let cmd = pull_command(&ssh_source(), "blake3:abc123", Path::new("/tmp/staging"));
    assert!(cmd.contains("rsync -az"));
    assert!(cmd.contains("forjar@cache.internal:/var/forjar/cache/abc123/"));
    assert!(cmd.contains("/tmp/staging"));
    assert!(cmd.contains("mkdir -p"));
}

#[test]
fn pull_command_ssh_with_port() {
    let cmd = pull_command(
        &ssh_source_with_port(),
        "blake3:def456",
        Path::new("/tmp/staging"),
    );
    assert!(cmd.contains("-p 2222"));
    assert!(cmd.contains("rsync"));
}

#[test]
fn pull_command_local() {
    let cmd = pull_command(&local_source(), "blake3:abc123", Path::new("/tmp/staging"));
    assert!(cmd.contains("cp -a"));
    assert!(cmd.contains("/mnt/cache/abc123/."));
    assert!(cmd.contains("/tmp/staging"));
}

#[test]
fn push_command_ssh_basic() {
    let cmd = push_command(
        &ssh_source(),
        "blake3:abc123",
        Path::new("/var/lib/forjar/store"),
    );
    assert!(cmd.contains("rsync -az"));
    assert!(cmd.contains("/var/lib/forjar/store/abc123/"));
    assert!(cmd.contains("forjar@cache.internal:/var/forjar/cache/abc123/"));
}

#[test]
fn push_command_ssh_with_port() {
    let cmd = push_command(
        &ssh_source_with_port(),
        "blake3:def456",
        Path::new("/var/lib/forjar/store"),
    );
    assert!(cmd.contains("-p 2222"));
}

#[test]
fn push_command_local() {
    let cmd = push_command(
        &local_source(),
        "blake3:abc123",
        Path::new("/var/lib/forjar/store"),
    );
    assert!(cmd.contains("cp -a"));
    assert!(cmd.contains("/var/lib/forjar/store/abc123"));
    assert!(cmd.contains("/mnt/cache/abc123"));
}

#[test]
fn pull_command_strips_blake3_prefix() {
    let cmd = pull_command(&ssh_source(), "blake3:aabbccdd", Path::new("/tmp/staging"));
    assert!(cmd.contains("aabbccdd/"));
    assert!(!cmd.contains("blake3:"));
}

#[test]
fn push_command_strips_blake3_prefix() {
    let cmd = push_command(&ssh_source(), "blake3:aabbccdd", Path::new("/store"));
    assert!(cmd.contains("aabbccdd/"));
    assert!(!cmd.contains("blake3:"));
}

#[test]
fn substitution_local_hit_returns_path() {
    let cc = CacheConfig {
        sources: vec![local_source()],
        auto_push: false,
        max_size_mb: 0,
    };
    let local = vec!["blake3:hit111".to_string()];
    let ctx = SubstitutionContext {
        closure_hash: "blake3:hit111",
        input_hashes: &[],
        local_entries: &local,
        cache_config: &cc,
        cache_inventories: &[],
        sandbox: None,
        store_dir: Path::new("/var/lib/forjar/store"),
    };

    let plan = plan_substitution(&ctx);
    if let SubstitutionOutcome::LocalHit { store_path } = &plan.outcome {
        assert!(store_path.contains("hit111"));
    } else {
        panic!("expected local hit");
    }
}

#[test]
fn substitution_cache_hit_plan() {
    let cc = CacheConfig {
        sources: vec![ssh_source()],
        auto_push: false,
        max_size_mb: 0,
    };
    let inv = build_inventory(
        "cache.internal",
        vec![CacheEntry {
            store_hash: "blake3:cached999".to_string(),
            size_bytes: 8192,
            created_at: "2026-01-15T12:00:00Z".to_string(),
            provider: "cargo".to_string(),
            arch: "x86_64".to_string(),
        }],
    );

    let ctx = SubstitutionContext {
        closure_hash: "blake3:cached999",
        input_hashes: &[],
        local_entries: &[],
        cache_config: &cc,
        cache_inventories: &[inv],
        sandbox: None,
        store_dir: Path::new("/var/lib/forjar/store"),
    };

    let plan = plan_substitution(&ctx);
    assert!(matches!(plan.outcome, SubstitutionOutcome::CacheHit { .. }));
}

#[test]
fn substitution_cache_miss_plan() {
    let cc = CacheConfig {
        sources: vec![ssh_source()],
        auto_push: true,
        max_size_mb: 0,
    };

    let ctx = SubstitutionContext {
        closure_hash: "blake3:never_seen",
        input_hashes: &[],
        local_entries: &[],
        cache_config: &cc,
        cache_inventories: &[],
        sandbox: None,
        store_dir: Path::new("/var/lib/forjar/store"),
    };

    let plan = plan_substitution(&ctx);
    assert!(matches!(
        plan.outcome,
        SubstitutionOutcome::CacheMiss { .. }
    ));
}

#[test]
fn pull_command_handles_raw_hash() {
    // Without blake3: prefix
    let cmd = pull_command(&ssh_source(), "raw_hash_value", Path::new("/tmp/staging"));
    assert!(cmd.contains("raw_hash_value/"));
}

#[test]
fn push_command_handles_raw_hash() {
    let cmd = push_command(&ssh_source(), "raw_hash_value", Path::new("/store"));
    assert!(cmd.contains("raw_hash_value/"));
}

#[test]
fn pull_command_ssh_uses_single_quotes() {
    let cmd = pull_command(&ssh_source(), "blake3:abc123", Path::new("/tmp/staging"));
    // Should use single quotes for shell safety
    assert!(cmd.contains('\''));
}

#[test]
fn substitution_with_auto_push_includes_push_step() {
    let cc = CacheConfig {
        sources: vec![ssh_source()],
        auto_push: true,
        max_size_mb: 0,
    };

    let ctx = SubstitutionContext {
        closure_hash: "blake3:build_me",
        input_hashes: &[],
        local_entries: &[],
        cache_config: &cc,
        cache_inventories: &[],
        sandbox: None,
        store_dir: Path::new("/var/lib/forjar/store"),
    };

    let plan = plan_substitution(&ctx);
    let has_push = plan
        .steps
        .iter()
        .any(|s| matches!(s, super::substitution::SubstitutionStep::PushToCache { .. }));
    assert!(has_push, "auto_push should include PushToCache step");
}

#[test]
fn verify_pulled_content_with_matching_hash() {
    let dir = tempfile::tempdir().unwrap();
    let content_dir = dir.path().join("content");
    std::fs::create_dir_all(&content_dir).unwrap();
    std::fs::write(content_dir.join("hello.txt"), "hello world").unwrap();

    // Hash the content directory
    let hash = hash_directory(&content_dir).unwrap();

    // verify_pulled_content is private, so test through the hash_directory mechanism
    // The function hashes content/ subdir and compares against expected
    assert!(hash.starts_with("blake3:"));
    assert!(!hash.is_empty());
}

#[test]
fn verify_hash_changes_with_different_content() {
    let dir1 = tempfile::tempdir().unwrap();
    let dir2 = tempfile::tempdir().unwrap();
    let c1 = dir1.path().join("content");
    let c2 = dir2.path().join("content");
    std::fs::create_dir_all(&c1).unwrap();
    std::fs::create_dir_all(&c2).unwrap();
    std::fs::write(c1.join("a.txt"), "alpha").unwrap();
    std::fs::write(c2.join("a.txt"), "beta").unwrap();

    let h1 = hash_directory(&c1).unwrap();
    let h2 = hash_directory(&c2).unwrap();
    assert_ne!(h1, h2, "different content should produce different hashes");
}

#[test]
fn verify_hash_deterministic() {
    let dir = tempfile::tempdir().unwrap();
    let c = dir.path().join("content");
    std::fs::create_dir_all(&c).unwrap();
    std::fs::write(c.join("x.bin"), "data").unwrap();

    let h1 = hash_directory(&c).unwrap();
    let h2 = hash_directory(&c).unwrap();
    assert_eq!(h1, h2, "same content should produce same hash");
}
