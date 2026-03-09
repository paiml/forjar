//! FJ-1333–1336/1346/1345: Provider import, FAR archives, store diff/sync.
//! Usage: cargo test --test falsification_provider_far_diff

use forjar::core::store::far::{
    decode_far_manifest, encode_far, FarFileEntry, FarManifest, FarProvenance, FAR_MAGIC,
};
use forjar::core::store::meta::{new_meta, Provenance};
use forjar::core::store::provider::{
    all_providers, capture_method, import_command, origin_ref_string, parse_import_config,
    validate_import, ImportConfig, ImportProvider,
};
use forjar::core::store::store_diff::{
    build_sync_plan, compute_diff, has_diffable_provenance, upstream_check_command,
};
use std::collections::BTreeMap;

// ── helpers ──

fn config(prov: ImportProvider, reference: &str, ver: Option<&str>) -> ImportConfig {
    ImportConfig {
        provider: prov,
        reference: reference.into(),
        version: ver.map(|v| v.into()),
        arch: "x86_64".into(),
        options: BTreeMap::new(),
    }
}

fn test_manifest() -> FarManifest {
    FarManifest {
        name: "testpkg".into(),
        version: "1.0.0".into(),
        arch: "x86_64".into(),
        store_hash: "blake3:abc".into(),
        tree_hash: "blake3:def".into(),
        file_count: 1,
        total_size: 5,
        files: vec![FarFileEntry {
            path: "hello.txt".into(),
            size: 5,
            blake3: "blake3:file1".into(),
        }],
        provenance: FarProvenance {
            origin_provider: "apt".into(),
            origin_ref: Some("nginx".into()),
            origin_hash: None,
            created_at: "2026-01-01T00:00:00Z".into(),
            generator: "forjar 1.0.0".into(),
        },
        kernel_contracts: None,
    }
}

// ── FJ-1333–1336: Provider import commands ──

#[test]
fn import_apt_no_version() {
    let cmd = import_command(&config(ImportProvider::Apt, "nginx", None));
    assert!(cmd.contains("apt-get install"));
    assert!(cmd.contains("nginx"));
}

#[test]
fn import_apt_with_version() {
    let cmd = import_command(&config(ImportProvider::Apt, "nginx", Some("1.24")));
    assert!(cmd.contains("=1.24"));
}

#[test]
fn import_cargo_with_version() {
    let cmd = import_command(&config(ImportProvider::Cargo, "ripgrep", Some("14.0")));
    assert!(cmd.contains("cargo install"));
    assert!(cmd.contains("--version 14.0"));
}

#[test]
fn import_uv_with_version() {
    let cmd = import_command(&config(ImportProvider::Uv, "numpy", Some("1.26")));
    assert!(cmd.contains("uv pip install"));
    assert!(cmd.contains("numpy==1.26"));
}

#[test]
fn import_nix() {
    let cmd = import_command(&config(ImportProvider::Nix, "nixpkgs#ripgrep", None));
    assert!(cmd.contains("nix build"));
}

#[test]
fn import_docker() {
    let cmd = import_command(&config(ImportProvider::Docker, "alpine", Some("3.19")));
    assert!(cmd.contains("docker create"));
    assert!(cmd.contains(":3.19"));
}

#[test]
fn import_tofu() {
    let cmd = import_command(&config(ImportProvider::Tofu, "./infra", None));
    assert!(cmd.contains("tofu -chdir=./infra"));
}

#[test]
fn import_apr() {
    let cmd = import_command(&config(ImportProvider::Apr, "llama-7b", None));
    assert!(cmd.contains("apr pull llama-7b"));
}

// ── origin_ref_string ──

#[test]
fn origin_ref_apt() {
    let r = origin_ref_string(&config(ImportProvider::Apt, "nginx", Some("1.24")));
    assert_eq!(r, "apt:nginx@1.24");
}

#[test]
fn origin_ref_cargo() {
    let r = origin_ref_string(&config(ImportProvider::Cargo, "serde", None));
    assert_eq!(r, "cargo:serde");
}

#[test]
fn origin_ref_nix() {
    let r = origin_ref_string(&config(ImportProvider::Nix, "nixpkgs#rg", None));
    assert_eq!(r, "nixpkgs#rg");
}

// ── validate_import ──

#[test]
fn validate_import_good() {
    let errors = validate_import(&config(ImportProvider::Apt, "nginx", None));
    assert!(errors.is_empty());
}

#[test]
fn validate_import_empty_ref() {
    let errors = validate_import(&config(ImportProvider::Apt, "", None));
    assert!(errors.iter().any(|e| e.contains("reference")));
}

#[test]
fn validate_import_nix_bad_format() {
    let errors = validate_import(&config(ImportProvider::Nix, "ripgrep", None));
    assert!(errors.iter().any(|e| e.contains("flake")));
}

#[test]
fn validate_import_docker_spaces() {
    let errors = validate_import(&config(ImportProvider::Docker, "bad image", None));
    assert!(errors.iter().any(|e| e.contains("spaces")));
}

// ── parse_import_config ──

#[test]
fn parse_import_yaml() {
    let yaml = "provider: apt\nreference: nginx\nversion: '1.24'\narch: x86_64\n";
    let cfg = parse_import_config(yaml).unwrap();
    assert_eq!(cfg.provider, ImportProvider::Apt);
    assert_eq!(cfg.reference, "nginx");
    assert_eq!(cfg.version.as_deref(), Some("1.24"));
}

#[test]
fn parse_import_yaml_invalid() {
    assert!(parse_import_config("invalid: [[[").is_err());
}

// ── capture_method ──

#[test]
fn capture_methods_all_non_empty() {
    for p in all_providers() {
        assert!(!capture_method(p).is_empty());
    }
}

// ── all_providers ──

#[test]
fn all_providers_count() {
    assert_eq!(all_providers().len(), 8);
}

// ── FJ-1346: FAR encode/decode ──

#[test]
fn far_magic_is_12_bytes() {
    assert_eq!(FAR_MAGIC.len(), 12);
    assert!(FAR_MAGIC.starts_with(b"FORJAR-FAR"));
}

#[test]
fn far_roundtrip_single_chunk() {
    let manifest = test_manifest();
    let chunk_hash = [42u8; 32];
    let chunk_data = b"hello forjar".to_vec();

    let mut buf = Vec::new();
    encode_far(&manifest, &[(chunk_hash, chunk_data.clone())], &mut buf).unwrap();

    let cursor = std::io::Cursor::new(buf);
    let (decoded_manifest, chunk_table) = decode_far_manifest(cursor).unwrap();
    assert_eq!(decoded_manifest, manifest);
    assert_eq!(chunk_table.len(), 1);
    assert_eq!(chunk_table[0].hash, chunk_hash);
}

#[test]
fn far_roundtrip_multiple_chunks() {
    let manifest = test_manifest();
    let chunks: Vec<([u8; 32], Vec<u8>)> = (0..3)
        .map(|i| {
            let mut hash = [0u8; 32];
            hash[0] = i;
            (hash, vec![i; 100])
        })
        .collect();

    let mut buf = Vec::new();
    encode_far(&manifest, &chunks, &mut buf).unwrap();

    let (decoded, table) = decode_far_manifest(std::io::Cursor::new(buf)).unwrap();
    assert_eq!(decoded, manifest);
    assert_eq!(table.len(), 3);
    // Offsets should be sequential
    assert_eq!(table[0].offset, 0);
    assert!(table[1].offset > 0);
    assert!(table[2].offset > table[1].offset);
}

#[test]
fn far_roundtrip_empty_chunks() {
    let manifest = test_manifest();
    let mut buf = Vec::new();
    encode_far(&manifest, &[], &mut buf).unwrap();
    let (decoded, table) = decode_far_manifest(std::io::Cursor::new(buf)).unwrap();
    assert_eq!(decoded, manifest);
    assert!(table.is_empty());
}

#[test]
fn far_decode_bad_magic() {
    let result = decode_far_manifest(std::io::Cursor::new(b"NOT-FAR-MAGIC"));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("magic"));
}

#[test]
fn far_manifest_serde() {
    let m = test_manifest();
    let yaml = serde_yaml_ng::to_string(&m).unwrap();
    let parsed: FarManifest = serde_yaml_ng::from_str(&yaml).unwrap();
    assert_eq!(m, parsed);
}

// ── FJ-1345: store diff ──

#[test]
fn diff_no_change() {
    let meta = new_meta("blake3:store1", "blake3:r", &[], "x86_64", "apt");
    let diff = compute_diff(&meta, None);
    assert!(!diff.upstream_changed);
}

#[test]
fn diff_upstream_changed() {
    let mut meta = new_meta("blake3:store1", "blake3:r", &[], "x86_64", "apt");
    meta.provenance = Some(Provenance {
        origin_provider: "apt".into(),
        origin_ref: Some("nginx".into()),
        origin_hash: Some("blake3:old".into()),
        derived_from: None,
        derivation_depth: 0,
    });
    let diff = compute_diff(&meta, Some("blake3:new"));
    assert!(diff.upstream_changed);
    assert_eq!(diff.local_origin_hash, Some("blake3:old".into()));
    assert_eq!(diff.upstream_hash, Some("blake3:new".into()));
}

#[test]
fn diff_upstream_same() {
    let mut meta = new_meta("blake3:store1", "blake3:r", &[], "x86_64", "apt");
    meta.provenance = Some(Provenance {
        origin_provider: "apt".into(),
        origin_ref: Some("nginx".into()),
        origin_hash: Some("blake3:same".into()),
        derived_from: None,
        derivation_depth: 0,
    });
    let diff = compute_diff(&meta, Some("blake3:same"));
    assert!(!diff.upstream_changed);
}

#[test]
fn diff_no_local_hash_upstream_present() {
    let mut meta = new_meta("blake3:store1", "blake3:r", &[], "x86_64", "apt");
    meta.provenance = Some(Provenance {
        origin_provider: "apt".into(),
        origin_ref: Some("nginx".into()),
        origin_hash: None,
        derived_from: None,
        derivation_depth: 0,
    });
    let diff = compute_diff(&meta, Some("blake3:new"));
    assert!(diff.upstream_changed);
}

// ── FJ-1345: build_sync_plan ──

#[test]
fn sync_plan_no_changes() {
    let mut meta = new_meta("blake3:a", "blake3:r", &[], "x86_64", "apt");
    meta.provenance = Some(Provenance {
        origin_provider: "apt".into(),
        origin_ref: Some("nginx".into()),
        origin_hash: Some("blake3:same".into()),
        derived_from: None,
        derivation_depth: 0,
    });
    let plan = build_sync_plan(&[(meta, Some("blake3:same".into()))]);
    assert_eq!(plan.total_steps, 0);
}

#[test]
fn sync_plan_re_import() {
    let mut meta = new_meta("blake3:a", "blake3:r", &[], "x86_64", "apt");
    meta.provenance = Some(Provenance {
        origin_provider: "apt".into(),
        origin_ref: Some("nginx".into()),
        origin_hash: Some("blake3:old".into()),
        derived_from: None,
        derivation_depth: 0,
    });
    let plan = build_sync_plan(&[(meta, Some("blake3:new".into()))]);
    assert_eq!(plan.re_imports.len(), 1);
    assert_eq!(plan.re_imports[0].provider, "apt");
}

#[test]
fn sync_plan_derivation_replay() {
    let mut meta = new_meta("blake3:derived", "blake3:r", &[], "x86_64", "apt");
    meta.provenance = Some(Provenance {
        origin_provider: "apt".into(),
        origin_ref: Some("nginx".into()),
        origin_hash: Some("blake3:old".into()),
        derived_from: Some("blake3:base".into()),
        derivation_depth: 2,
    });
    let plan = build_sync_plan(&[(meta, Some("blake3:new".into()))]);
    assert_eq!(plan.derivation_replays.len(), 1);
    assert_eq!(plan.derivation_replays[0].derivation_depth, 2);
}

// ── has_diffable_provenance ──

#[test]
fn diffable_with_origin_hash() {
    let mut meta = new_meta("blake3:a", "blake3:r", &[], "x86_64", "apt");
    meta.provenance = Some(Provenance {
        origin_provider: "apt".into(),
        origin_ref: None,
        origin_hash: Some("blake3:h".into()),
        derived_from: None,
        derivation_depth: 0,
    });
    assert!(has_diffable_provenance(&meta));
}

#[test]
fn diffable_with_origin_ref() {
    let mut meta = new_meta("blake3:a", "blake3:r", &[], "x86_64", "apt");
    meta.provenance = Some(Provenance {
        origin_provider: "apt".into(),
        origin_ref: Some("nginx".into()),
        origin_hash: None,
        derived_from: None,
        derivation_depth: 0,
    });
    assert!(has_diffable_provenance(&meta));
}

#[test]
fn not_diffable_no_provenance() {
    let meta = new_meta("blake3:a", "blake3:r", &[], "x86_64", "apt");
    assert!(!has_diffable_provenance(&meta));
}

// ── upstream_check_command ──

#[test]
fn upstream_cmd_apt() {
    let mut meta = new_meta("blake3:a", "blake3:r", &[], "x86_64", "apt");
    meta.provenance = Some(Provenance {
        origin_provider: "apt".into(),
        origin_ref: Some("nginx".into()),
        origin_hash: None,
        derived_from: None,
        derivation_depth: 0,
    });
    let cmd = upstream_check_command(&meta).unwrap();
    assert!(cmd.contains("apt-cache policy nginx"));
}

#[test]
fn upstream_cmd_cargo() {
    let mut meta = new_meta("blake3:a", "blake3:r", &[], "x86_64", "cargo");
    meta.provenance = Some(Provenance {
        origin_provider: "cargo".into(),
        origin_ref: Some("serde".into()),
        origin_hash: None,
        derived_from: None,
        derivation_depth: 0,
    });
    let cmd = upstream_check_command(&meta).unwrap();
    assert!(cmd.contains("cargo search serde"));
}

#[test]
fn upstream_cmd_no_provenance() {
    let meta = new_meta("blake3:a", "blake3:r", &[], "x86_64", "apt");
    assert!(upstream_check_command(&meta).is_none());
}
