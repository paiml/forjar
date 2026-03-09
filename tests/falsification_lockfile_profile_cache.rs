//! FJ-1310/1302/1304/1320: Lock files, profile generations, reference scanning, cache config.
//! Usage: cargo test --test falsification_lockfile_profile_cache

use forjar::core::store::cache::{
    build_inventory, parse_cache_config, resolve_substitution, ssh_command, validate_cache_config,
    verify_entry, CacheConfig, CacheEntry, CacheSource, SubstitutionResult,
};
use forjar::core::store::lockfile::{
    check_completeness, check_staleness, parse_lockfile, write_lockfile, LockFile, Pin,
};
use forjar::core::store::profile::{
    create_generation, current_generation, list_generations, rollback,
};
use forjar::core::store::reference::{is_valid_blake3_hash, scan_directory_refs, scan_file_refs};
use std::collections::{BTreeMap, BTreeSet};

// ── helpers ──

fn pin(provider: &str, hash: &str, version: Option<&str>) -> Pin {
    Pin {
        provider: provider.into(),
        version: version.map(|v| v.into()),
        hash: hash.into(),
        git_rev: None,
        pin_type: None,
    }
}

fn lock(pins: &[(&str, &str, &str)]) -> LockFile {
    let mut map = BTreeMap::new();
    for (name, provider, hash) in pins {
        map.insert(name.to_string(), pin(provider, hash, None));
    }
    LockFile {
        schema: "1.0".into(),
        pins: map,
    }
}

// ── FJ-1310: Lock file ──

#[test]
fn lockfile_parse_yaml() {
    let yaml = "schema: '1.0'\npins:\n  nginx:\n    provider: apt\n    hash: blake3:abc\n";
    let lf = parse_lockfile(yaml).unwrap();
    assert_eq!(lf.schema, "1.0");
    assert_eq!(lf.pins["nginx"].provider, "apt");
    assert_eq!(lf.pins["nginx"].hash, "blake3:abc");
}

#[test]
fn lockfile_parse_invalid() {
    assert!(parse_lockfile("invalid: [[[").is_err());
}

#[test]
fn lockfile_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("lock.yaml");
    let lf = lock(&[
        ("nginx", "apt", "blake3:abc"),
        ("curl", "apt", "blake3:def"),
    ]);
    write_lockfile(&path, &lf).unwrap();
    let read = parse_lockfile(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(lf, read);
}

#[test]
fn lockfile_serde_roundtrip() {
    let lf = lock(&[("serde", "cargo", "blake3:xyz")]);
    let yaml = serde_yaml_ng::to_string(&lf).unwrap();
    let parsed: LockFile = serde_yaml_ng::from_str(&yaml).unwrap();
    assert_eq!(lf, parsed);
}

#[test]
fn staleness_fresh() {
    let lf = lock(&[("nginx", "apt", "blake3:abc")]);
    let mut h = BTreeMap::new();
    h.insert("nginx".into(), "blake3:abc".into());
    assert!(check_staleness(&lf, &h).is_empty());
}

#[test]
fn staleness_stale() {
    let lf = lock(&[("nginx", "apt", "blake3:old")]);
    let mut h = BTreeMap::new();
    h.insert("nginx".into(), "blake3:new".into());
    let stale = check_staleness(&lf, &h);
    assert_eq!(stale.len(), 1);
    assert_eq!(stale[0].locked_hash, "blake3:old");
    assert_eq!(stale[0].current_hash, "blake3:new");
}

#[test]
fn staleness_missing_current() {
    let lf = lock(&[("nginx", "apt", "blake3:abc")]);
    let h = BTreeMap::new();
    assert!(check_staleness(&lf, &h).is_empty());
}

#[test]
fn completeness_all_present() {
    let lf = lock(&[("nginx", "apt", "h1"), ("curl", "apt", "h2")]);
    assert!(check_completeness(&lf, &["nginx".into(), "curl".into()]).is_empty());
}

#[test]
fn completeness_missing() {
    let lf = lock(&[("nginx", "apt", "h1")]);
    let missing = check_completeness(&lf, &["nginx".into(), "curl".into()]);
    assert_eq!(missing, vec!["curl"]);
}

// ── FJ-1302: Profile generations ──

#[test]
fn profile_create_and_list() {
    let tmp = tempfile::tempdir().unwrap();
    let profiles = tmp.path().join("profiles");
    let gen0 = create_generation(&profiles, "/store/hash-a").unwrap();
    assert_eq!(gen0, 0);
    let gen1 = create_generation(&profiles, "/store/hash-b").unwrap();
    assert_eq!(gen1, 1);

    let gens = list_generations(&profiles).unwrap();
    assert_eq!(gens.len(), 2);
    assert_eq!(gens[0].0, 0);
    assert!(gens[0].1.contains("/store/hash-a"));
    assert_eq!(gens[1].0, 1);
}

#[test]
fn profile_current_generation() {
    let tmp = tempfile::tempdir().unwrap();
    let profiles = tmp.path().join("profiles");
    create_generation(&profiles, "/store/hash-a").unwrap();
    create_generation(&profiles, "/store/hash-b").unwrap();
    assert_eq!(current_generation(&profiles), Some(1));
}

#[test]
fn profile_rollback() {
    let tmp = tempfile::tempdir().unwrap();
    let profiles = tmp.path().join("profiles");
    create_generation(&profiles, "/store/hash-a").unwrap();
    create_generation(&profiles, "/store/hash-b").unwrap();
    let rolled = rollback(&profiles).unwrap();
    assert_eq!(rolled, 0);
    assert_eq!(current_generation(&profiles), Some(0));
}

#[test]
fn profile_rollback_at_zero_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let profiles = tmp.path().join("profiles");
    create_generation(&profiles, "/store/hash-a").unwrap();
    assert!(rollback(&profiles).is_err());
}

#[test]
fn profile_list_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let gens = list_generations(&tmp.path().join("nonexistent")).unwrap();
    assert!(gens.is_empty());
}

#[test]
fn profile_current_no_generations() {
    let tmp = tempfile::tempdir().unwrap();
    assert_eq!(current_generation(tmp.path()), None);
}

// ── FJ-1304: Reference scanning ──

#[test]
fn blake3_hash_valid() {
    let hash = format!("blake3:{}", "a".repeat(64));
    assert!(is_valid_blake3_hash(&hash));
}

#[test]
fn blake3_hash_invalid_prefix() {
    assert!(!is_valid_blake3_hash("sha256:abc"));
}

#[test]
fn blake3_hash_invalid_length() {
    assert!(!is_valid_blake3_hash("blake3:abc"));
}

#[test]
fn blake3_hash_invalid_chars() {
    let hash = format!("blake3:{}", "g".repeat(64));
    assert!(!is_valid_blake3_hash(&hash));
}

#[test]
fn scan_file_refs_finds_known() {
    let hash = format!("blake3:{}", "a".repeat(64));
    let content = format!("config: {hash}\nother: data\n");
    let mut known = BTreeSet::new();
    known.insert(hash.clone());
    let refs = scan_file_refs(content.as_bytes(), &known);
    assert_eq!(refs.len(), 1);
    assert!(refs.contains(&hash));
}

#[test]
fn scan_file_refs_ignores_unknown() {
    let hash = format!("blake3:{}", "b".repeat(64));
    let content = format!("ref: {hash}\n");
    let known = BTreeSet::new();
    let refs = scan_file_refs(content.as_bytes(), &known);
    assert!(refs.is_empty());
}

#[test]
fn scan_file_refs_multiple() {
    let h1 = format!("blake3:{}", "a".repeat(64));
    let h2 = format!("blake3:{}", "b".repeat(64));
    let content = format!("dep1: {h1}\ndep2: {h2}\n");
    let known: BTreeSet<String> = [h1.clone(), h2.clone()].into();
    let refs = scan_file_refs(content.as_bytes(), &known);
    assert_eq!(refs.len(), 2);
}

#[test]
fn scan_directory_refs_walks() {
    let tmp = tempfile::tempdir().unwrap();
    let h1 = format!("blake3:{}", "c".repeat(64));
    std::fs::create_dir_all(tmp.path().join("sub")).unwrap();
    std::fs::write(tmp.path().join("sub/ref.txt"), format!("ref: {h1}")).unwrap();
    let known: BTreeSet<String> = [h1.clone()].into();
    let refs = scan_directory_refs(tmp.path(), &known).unwrap();
    assert_eq!(refs.len(), 1);
    assert!(refs.contains(&h1));
}

#[test]
fn scan_directory_refs_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let refs = scan_directory_refs(tmp.path(), &BTreeSet::new()).unwrap();
    assert!(refs.is_empty());
}

// ── FJ-1320: Cache config ──

#[test]
fn cache_parse_ssh() {
    let yaml = "sources:\n  - type: ssh\n    host: cache.example.com\n    user: forjar\n    path: /var/lib/forjar/cache\nauto_push: true\n";
    let cfg = parse_cache_config(yaml).unwrap();
    assert_eq!(cfg.sources.len(), 1);
    assert!(cfg.auto_push);
    matches!(&cfg.sources[0], CacheSource::Ssh { host, .. } if host == "cache.example.com");
}

#[test]
fn cache_parse_local() {
    let yaml = "sources:\n  - type: local\n    path: /tmp/cache\n";
    let cfg = parse_cache_config(yaml).unwrap();
    matches!(&cfg.sources[0], CacheSource::Local { path } if path == "/tmp/cache");
}

#[test]
fn cache_validate_good() {
    let cfg = CacheConfig {
        sources: vec![CacheSource::Local {
            path: "/tmp".into(),
        }],
        auto_push: false,
        max_size_mb: 0,
    };
    assert!(validate_cache_config(&cfg).is_empty());
}

#[test]
fn cache_validate_empty_sources() {
    let cfg = CacheConfig {
        sources: vec![],
        auto_push: false,
        max_size_mb: 0,
    };
    assert!(!validate_cache_config(&cfg).is_empty());
}

#[test]
fn cache_validate_empty_ssh_fields() {
    let cfg = CacheConfig {
        sources: vec![CacheSource::Ssh {
            host: String::new(),
            user: String::new(),
            path: String::new(),
            port: None,
        }],
        auto_push: false,
        max_size_mb: 0,
    };
    let errors = validate_cache_config(&cfg);
    assert!(errors.len() >= 3);
}

#[test]
fn cache_resolve_local_hit() {
    let result = resolve_substitution("blake3:abc", &["blake3:abc".into()], &[]);
    assert!(matches!(result, SubstitutionResult::LocalHit { .. }));
}

#[test]
fn cache_resolve_cache_hit() {
    let entry = CacheEntry {
        store_hash: "blake3:abc".into(),
        size_bytes: 1024,
        created_at: "2026-01-01T00:00:00Z".into(),
        provider: "apt".into(),
        arch: "x86_64".into(),
    };
    let inv = build_inventory("cache1", vec![entry]);
    let result = resolve_substitution("blake3:abc", &[], &[inv]);
    assert!(matches!(
        result,
        SubstitutionResult::CacheHit {
            source_index: 0,
            ..
        }
    ));
}

#[test]
fn cache_resolve_miss() {
    let result = resolve_substitution("blake3:missing", &[], &[]);
    assert!(matches!(result, SubstitutionResult::CacheMiss));
}

#[test]
fn cache_verify_entry_match() {
    let entry = CacheEntry {
        store_hash: "blake3:abc".into(),
        size_bytes: 0,
        created_at: String::new(),
        provider: String::new(),
        arch: String::new(),
    };
    assert!(verify_entry(&entry, "blake3:abc"));
    assert!(!verify_entry(&entry, "blake3:def"));
}

#[test]
fn cache_ssh_command() {
    let src = CacheSource::Ssh {
        host: "cache.example.com".into(),
        user: "forjar".into(),
        path: "/store".into(),
        port: Some(2222),
    };
    let cmd = ssh_command(&src).unwrap();
    assert!(cmd.contains("-p 2222"));
    assert!(cmd.contains("forjar@cache.example.com"));
}

#[test]
fn cache_ssh_command_no_port() {
    let src = CacheSource::Ssh {
        host: "cache.example.com".into(),
        user: "forjar".into(),
        path: "/store".into(),
        port: None,
    };
    let cmd = ssh_command(&src).unwrap();
    assert!(!cmd.contains("-p"));
}

#[test]
fn cache_ssh_command_local_none() {
    let src = CacheSource::Local {
        path: "/tmp".into(),
    };
    assert!(ssh_command(&src).is_none());
}

#[test]
fn cache_build_inventory() {
    let entries = vec![CacheEntry {
        store_hash: "blake3:h1".into(),
        size_bytes: 100,
        created_at: String::new(),
        provider: "apt".into(),
        arch: "x86_64".into(),
    }];
    let inv = build_inventory("my-cache", entries);
    assert_eq!(inv.source_name, "my-cache");
    assert!(inv.entries.contains_key("blake3:h1"));
}
