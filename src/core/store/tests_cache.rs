//! Tests for FJ-1320–FJ-1322: Binary cache and substitution protocol.

use super::cache::{
    build_inventory, parse_cache_config, resolve_substitution, ssh_command, validate_cache_config,
    verify_entry, CacheConfig, CacheEntry, CacheInventory, CacheSource, SubstitutionResult,
};
use std::collections::BTreeMap;

fn sample_ssh_source() -> CacheSource {
    CacheSource::Ssh {
        host: "cache.internal".to_string(),
        user: "forjar".to_string(),
        path: "/var/forjar/cache".to_string(),
        port: None,
    }
}

fn sample_local_source() -> CacheSource {
    CacheSource::Local {
        path: "/var/forjar/store".to_string(),
    }
}

fn sample_entry(hash: &str) -> CacheEntry {
    CacheEntry {
        store_hash: hash.to_string(),
        size_bytes: 1024,
        created_at: "2026-03-02T10:00:00Z".to_string(),
        provider: "apt".to_string(),
        arch: "x86_64".to_string(),
    }
}

#[test]
fn test_fj1320_parse_config_ssh_and_local() {
    let yaml = r#"
sources:
  - type: ssh
    host: cache.internal
    user: forjar
    path: /var/forjar/cache
  - type: local
    path: /var/forjar/store
auto_push: true
max_size_mb: 10240
"#;
    let cfg = parse_cache_config(yaml).unwrap();
    assert_eq!(cfg.sources.len(), 2);
    assert!(cfg.auto_push);
    assert_eq!(cfg.max_size_mb, 10240);
    assert!(matches!(&cfg.sources[0], CacheSource::Ssh { host, .. } if host == "cache.internal"));
    assert!(matches!(&cfg.sources[1], CacheSource::Local { path } if path == "/var/forjar/store"));
}

#[test]
fn test_fj1320_parse_config_ssh_with_port() {
    let yaml = r#"
sources:
  - type: ssh
    host: build.local
    user: builder
    path: /cache
    port: 2222
"#;
    let cfg = parse_cache_config(yaml).unwrap();
    assert!(matches!(
        &cfg.sources[0],
        CacheSource::Ssh {
            port: Some(2222),
            ..
        }
    ));
}

#[test]
fn test_fj1320_parse_config_invalid() {
    assert!(parse_cache_config("invalid: [yaml: broken").is_err());
}

#[test]
fn test_fj1320_validate_valid() {
    let cfg = CacheConfig {
        sources: vec![sample_ssh_source(), sample_local_source()],
        auto_push: false,
        max_size_mb: 0,
    };
    let errors = validate_cache_config(&cfg);
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

#[test]
fn test_fj1320_validate_no_sources() {
    let cfg = CacheConfig {
        sources: vec![],
        auto_push: false,
        max_size_mb: 0,
    };
    let errors = validate_cache_config(&cfg);
    assert!(errors.iter().any(|e| e.contains("at least one")));
}

#[test]
fn test_fj1320_validate_empty_ssh_host() {
    let cfg = CacheConfig {
        sources: vec![CacheSource::Ssh {
            host: String::new(),
            user: "forjar".to_string(),
            path: "/cache".to_string(),
            port: None,
        }],
        auto_push: false,
        max_size_mb: 0,
    };
    let errors = validate_cache_config(&cfg);
    assert!(errors.iter().any(|e| e.contains("host")));
}

#[test]
fn test_fj1320_validate_empty_local_path() {
    let cfg = CacheConfig {
        sources: vec![CacheSource::Local {
            path: String::new(),
        }],
        auto_push: false,
        max_size_mb: 0,
    };
    let errors = validate_cache_config(&cfg);
    assert!(errors.iter().any(|e| e.contains("local path")));
}

#[test]
fn test_fj1322_substitution_local_hit() {
    let hash = "blake3:abc123";
    let local = vec![hash.to_string()];
    let result = resolve_substitution(hash, &local, &[]);
    assert!(matches!(result, SubstitutionResult::LocalHit { .. }));
}

#[test]
fn test_fj1322_substitution_cache_hit() {
    let hash = "blake3:def456";
    let local: Vec<String> = vec![];
    let inv = build_inventory("remote", vec![sample_entry(hash)]);
    let result = resolve_substitution(hash, &local, &[inv]);
    assert!(matches!(
        result,
        SubstitutionResult::CacheHit {
            source_index: 0,
            ..
        }
    ));
}

#[test]
fn test_fj1322_substitution_miss() {
    let hash = "blake3:missing";
    let result = resolve_substitution(hash, &[], &[]);
    assert!(matches!(result, SubstitutionResult::CacheMiss));
}

#[test]
fn test_fj1322_substitution_prefers_local() {
    let hash = "blake3:abc123";
    let local = vec![hash.to_string()];
    let inv = build_inventory("remote", vec![sample_entry(hash)]);
    let result = resolve_substitution(hash, &local, &[inv]);
    assert!(matches!(result, SubstitutionResult::LocalHit { .. }));
}

#[test]
fn test_fj1322_substitution_checks_caches_in_order() {
    let hash = "blake3:abc123";
    let inv0 = CacheInventory {
        source_name: "first".to_string(),
        entries: BTreeMap::new(),
    };
    let inv1 = build_inventory("second", vec![sample_entry(hash)]);
    let result = resolve_substitution(hash, &[], &[inv0, inv1]);
    assert!(matches!(
        result,
        SubstitutionResult::CacheHit {
            source_index: 1,
            ..
        }
    ));
}

#[test]
fn test_fj1320_verify_entry_match() {
    let entry = sample_entry("blake3:abc123");
    assert!(verify_entry(&entry, "blake3:abc123"));
}

#[test]
fn test_fj1320_verify_entry_mismatch() {
    let entry = sample_entry("blake3:abc123");
    assert!(!verify_entry(&entry, "blake3:different"));
}

#[test]
fn test_fj1320_build_inventory() {
    let entries = vec![sample_entry("blake3:a"), sample_entry("blake3:b")];
    let inv = build_inventory("test-cache", entries);
    assert_eq!(inv.source_name, "test-cache");
    assert_eq!(inv.entries.len(), 2);
    assert!(inv.entries.contains_key("blake3:a"));
}

#[test]
fn test_fj1320_ssh_command_no_port() {
    let src = sample_ssh_source();
    let cmd = ssh_command(&src).unwrap();
    assert_eq!(cmd, "ssh forjar@cache.internal");
}

#[test]
fn test_fj1320_ssh_command_with_port() {
    let src = CacheSource::Ssh {
        host: "build.local".to_string(),
        user: "builder".to_string(),
        path: "/cache".to_string(),
        port: Some(2222),
    };
    let cmd = ssh_command(&src).unwrap();
    assert_eq!(cmd, "ssh -p 2222 builder@build.local");
}

#[test]
fn test_fj1320_ssh_command_local_returns_none() {
    let src = sample_local_source();
    assert!(ssh_command(&src).is_none());
}

#[test]
fn test_fj1320_serde_roundtrip() {
    let cfg = CacheConfig {
        sources: vec![sample_ssh_source(), sample_local_source()],
        auto_push: true,
        max_size_mb: 5120,
    };
    let yaml = serde_yaml_ng::to_string(&cfg).unwrap();
    let parsed: CacheConfig = serde_yaml_ng::from_str(&yaml).unwrap();
    assert_eq!(cfg, parsed);
}

#[test]
fn test_fj1320_cache_entry_serde() {
    let entry = sample_entry("blake3:test123");
    let json = serde_json::to_string(&entry).unwrap();
    let parsed: CacheEntry = serde_json::from_str(&json).unwrap();
    assert_eq!(entry, parsed);
}
