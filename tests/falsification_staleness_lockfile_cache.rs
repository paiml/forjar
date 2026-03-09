//! FJ-1310/1320: Lock files and binary cache falsification.
//! Usage: cargo test --test falsification_staleness_lockfile_cache

use forjar::core::store::cache::{
    build_inventory, parse_cache_config, resolve_substitution, ssh_command, validate_cache_config,
    verify_entry, CacheConfig, CacheEntry, CacheSource, SubstitutionResult,
};
use forjar::core::store::lockfile::{
    check_completeness, check_staleness, parse_lockfile, LockFile, Pin, StalenessEntry,
};
use std::collections::BTreeMap;

// ── helpers ──

fn pin(provider: &str, hash: &str) -> Pin {
    Pin {
        provider: provider.into(),
        version: None,
        hash: hash.into(),
        git_rev: None,
        pin_type: None,
    }
}

fn ce(hash: &str, size: u64, provider: &str) -> CacheEntry {
    CacheEntry {
        store_hash: hash.into(),
        size_bytes: size,
        created_at: "2026-03-09T10:00:00Z".into(),
        provider: provider.into(),
        arch: "x86_64".into(),
    }
}

// ── FJ-1310: parse_lockfile ──

#[test]
fn parse_lockfile_valid() {
    let yaml = r#"
schema: "1"
pins:
  nginx:
    provider: apt
    hash: blake3:abc123
    version: "1.24"
  curl:
    provider: apt
    hash: blake3:def456
"#;
    let lf = parse_lockfile(yaml).unwrap();
    assert_eq!(lf.schema, "1");
    assert_eq!(lf.pins.len(), 2);
    assert_eq!(lf.pins["nginx"].hash, "blake3:abc123");
    assert_eq!(lf.pins["curl"].provider, "apt");
}

#[test]
fn parse_lockfile_invalid() {
    assert!(parse_lockfile("not: valid: yaml: [").is_err());
}

#[test]
fn lockfile_serde_roundtrip() {
    let lf = LockFile {
        schema: "1".into(),
        pins: BTreeMap::from([("pkg".into(), pin("apt", "blake3:aaa"))]),
    };
    let yaml = serde_yaml_ng::to_string(&lf).unwrap();
    let parsed = parse_lockfile(&yaml).unwrap();
    assert_eq!(lf, parsed);
}

// ── FJ-1310: check_staleness ──

#[test]
fn staleness_no_changes() {
    let lf = LockFile {
        schema: "1".into(),
        pins: BTreeMap::from([("pkg".into(), pin("apt", "blake3:aaa"))]),
    };
    let current = BTreeMap::from([("pkg".into(), "blake3:aaa".into())]);
    assert!(check_staleness(&lf, &current).is_empty());
}

#[test]
fn staleness_hash_changed() {
    let lf = LockFile {
        schema: "1".into(),
        pins: BTreeMap::from([("pkg".into(), pin("apt", "blake3:old"))]),
    };
    let current = BTreeMap::from([("pkg".into(), "blake3:new".into())]);
    let stale = check_staleness(&lf, &current);
    assert_eq!(stale.len(), 1);
    assert_eq!(stale[0].name, "pkg");
    assert_eq!(stale[0].locked_hash, "blake3:old");
    assert_eq!(stale[0].current_hash, "blake3:new");
}

#[test]
fn staleness_missing_from_current() {
    let lf = LockFile {
        schema: "1".into(),
        pins: BTreeMap::from([("pkg".into(), pin("apt", "blake3:aaa"))]),
    };
    // If current_hashes doesn't have the pin, it's NOT stale (just missing)
    assert!(check_staleness(&lf, &BTreeMap::new()).is_empty());
}

// ── FJ-1310: check_completeness ──

#[test]
fn completeness_all_present() {
    let lf = LockFile {
        schema: "1".into(),
        pins: BTreeMap::from([
            ("a".into(), pin("apt", "h1")),
            ("b".into(), pin("apt", "h2")),
        ]),
    };
    assert!(check_completeness(&lf, &["a".into(), "b".into()]).is_empty());
}

#[test]
fn completeness_missing_inputs() {
    let lf = LockFile {
        schema: "1".into(),
        pins: BTreeMap::from([("a".into(), pin("apt", "h"))]),
    };
    let missing = check_completeness(&lf, &["a".into(), "b".into(), "c".into()]);
    assert_eq!(missing, vec!["b", "c"]);
}

// ── FJ-1320: parse_cache_config ──

#[test]
fn cache_config_ssh() {
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
    match &config.sources[0] {
        CacheSource::Ssh { host, port, .. } => {
            assert_eq!(host, "cache.example.com");
            assert_eq!(*port, Some(2222));
        }
        _ => panic!("expected SSH source"),
    }
}

#[test]
fn cache_config_local() {
    let yaml =
        "sources:\n  - type: local\n    path: /tmp/cache\nauto_push: false\nmax_size_mb: 0\n";
    let config = parse_cache_config(yaml).unwrap();
    match &config.sources[0] {
        CacheSource::Local { path } => assert_eq!(path, "/tmp/cache"),
        _ => panic!("expected Local source"),
    }
}

// ── FJ-1320: validate_cache_config ──

#[test]
fn validate_cache_valid() {
    let config = CacheConfig {
        sources: vec![CacheSource::Local {
            path: "/tmp".into(),
        }],
        auto_push: false,
        max_size_mb: 0,
    };
    assert!(validate_cache_config(&config).is_empty());
}

#[test]
fn validate_cache_empty_sources() {
    let config = CacheConfig {
        sources: vec![],
        auto_push: false,
        max_size_mb: 0,
    };
    let errs = validate_cache_config(&config);
    assert!(errs.iter().any(|e| e.contains("at least one")));
}

#[test]
fn validate_cache_empty_ssh_fields() {
    let config = CacheConfig {
        sources: vec![CacheSource::Ssh {
            host: "".into(),
            user: "".into(),
            path: "".into(),
            port: None,
        }],
        auto_push: false,
        max_size_mb: 0,
    };
    let errs = validate_cache_config(&config);
    assert!(errs.len() >= 3); // host, user, path all empty
}

// ── FJ-1320: resolve_substitution ──

#[test]
fn substitution_local_hit() {
    let result = resolve_substitution("blake3:abc", &["blake3:abc".into()], &[]);
    assert!(matches!(result, SubstitutionResult::LocalHit { .. }));
}

#[test]
fn substitution_cache_hit() {
    let inv = build_inventory("remote", vec![ce("blake3:abc", 100, "apt")]);
    let result = resolve_substitution("blake3:abc", &[], &[inv]);
    match result {
        SubstitutionResult::CacheHit { source_index, .. } => assert_eq!(source_index, 0),
        _ => panic!("expected CacheHit"),
    }
}

#[test]
fn substitution_miss() {
    let result = resolve_substitution("blake3:xyz", &[], &[]);
    assert!(matches!(result, SubstitutionResult::CacheMiss));
}

#[test]
fn substitution_prefers_local() {
    let inv = build_inventory("remote", vec![ce("blake3:abc", 100, "apt")]);
    let result = resolve_substitution("blake3:abc", &["blake3:abc".into()], &[inv]);
    assert!(matches!(result, SubstitutionResult::LocalHit { .. }));
}

// ── FJ-1320: verify_entry, build_inventory, ssh_command ──

#[test]
fn verify_entry_match_and_mismatch() {
    let entry = ce("blake3:abc", 100, "apt");
    assert!(verify_entry(&entry, "blake3:abc"));
    assert!(!verify_entry(&entry, "blake3:different"));
}

#[test]
fn build_inventory_creates_map() {
    let inv = build_inventory("test", vec![ce("h1", 10, "apt"), ce("h2", 20, "cargo")]);
    assert_eq!(inv.source_name, "test");
    assert_eq!(inv.entries.len(), 2);
    assert!(inv.entries.contains_key("h1"));
}

#[test]
fn ssh_command_with_port() {
    let src = CacheSource::Ssh {
        host: "cache.example.com".into(),
        user: "forjar".into(),
        path: "/cache".into(),
        port: Some(2222),
    };
    assert_eq!(
        ssh_command(&src).unwrap(),
        "ssh -p 2222 forjar@cache.example.com"
    );
}

#[test]
fn ssh_command_without_port() {
    let src = CacheSource::Ssh {
        host: "cache".into(),
        user: "root".into(),
        path: "/store".into(),
        port: None,
    };
    assert_eq!(ssh_command(&src).unwrap(), "ssh root@cache");
}

#[test]
fn ssh_command_local_returns_none() {
    assert!(ssh_command(&CacheSource::Local {
        path: "/tmp".into()
    })
    .is_none());
}

// ── serde roundtrips ──

#[test]
fn cache_source_serde() {
    let ssh = CacheSource::Ssh {
        host: "h".into(),
        user: "u".into(),
        path: "/p".into(),
        port: None,
    };
    let json = serde_json::to_string(&ssh).unwrap();
    let parsed: CacheSource = serde_json::from_str(&json).unwrap();
    assert_eq!(ssh, parsed);
}

#[test]
fn cache_entry_serde() {
    let e = ce("blake3:abc", 1024, "apt");
    let json = serde_json::to_string(&e).unwrap();
    let parsed: CacheEntry = serde_json::from_str(&json).unwrap();
    assert_eq!(e, parsed);
}

#[test]
fn staleness_entry_serde() {
    let s = StalenessEntry {
        name: "pkg".into(),
        locked_hash: "old".into(),
        current_hash: "new".into(),
    };
    let json = serde_json::to_string(&s).unwrap();
    let parsed: StalenessEntry = serde_json::from_str(&json).unwrap();
    assert_eq!(s, parsed);
}
