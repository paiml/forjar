//! FJ-1302/1315/1346: Profile generation, sandbox config, and FAR archive falsification.
//!
//! Popperian rejection criteria for:
//! - FJ-1302: Profile generation management
//!   - create_generation: atomic symlink switching
//!   - rollback: previous generation restoration
//!   - list_generations: sorted generation listing
//!   - current_generation: symlink target reading
//! - FJ-1315: Build sandbox configuration
//!   - validate_config: constraint enforcement
//!   - preset_profile: named presets
//!   - parse_sandbox_config: YAML deserialization
//!   - blocks_network / enforces_fs_isolation: level predicates
//!   - cgroup_path: deterministic path construction
//! - FJ-1346: FAR (Forjar ARchive) binary format
//!   - encode_far / decode_far_manifest: roundtrip fidelity
//!   - ChunkEntry table reconstruction
//!   - Magic validation
//!
//! Usage: cargo test --test falsification_store_profile_sandbox_far

use forjar::core::store::far::{
    decode_far_manifest, encode_far, FarFileEntry, FarManifest, FarProvenance, FAR_MAGIC,
};
use forjar::core::store::profile::{
    create_generation, current_generation, list_generations, rollback,
};
use forjar::core::store::sandbox::{
    blocks_network, cgroup_path, enforces_fs_isolation, parse_sandbox_config, preset_profile,
    validate_config, BindMount, EnvVar, SandboxConfig, SandboxLevel,
};

// ============================================================================
// FJ-1302: create_generation
// ============================================================================

#[test]
fn profile_create_first_generation() {
    let dir = tempfile::tempdir().unwrap();
    let profiles = dir.path().join("profiles");
    let gen = create_generation(&profiles, "/store/abc123/content").unwrap();
    assert_eq!(gen, 0);
    assert_eq!(current_generation(&profiles), Some(0));
}

#[test]
fn profile_create_multiple_generations() {
    let dir = tempfile::tempdir().unwrap();
    let profiles = dir.path().join("profiles");
    let g0 = create_generation(&profiles, "/store/hash0/content").unwrap();
    let g1 = create_generation(&profiles, "/store/hash1/content").unwrap();
    let g2 = create_generation(&profiles, "/store/hash2/content").unwrap();
    assert_eq!(g0, 0);
    assert_eq!(g1, 1);
    assert_eq!(g2, 2);
    assert_eq!(current_generation(&profiles), Some(2));
}

#[test]
fn profile_current_points_to_latest() {
    let dir = tempfile::tempdir().unwrap();
    let profiles = dir.path().join("profiles");
    create_generation(&profiles, "/store/first").unwrap();
    create_generation(&profiles, "/store/second").unwrap();
    assert_eq!(current_generation(&profiles), Some(1));
}

// ============================================================================
// FJ-1302: list_generations
// ============================================================================

#[test]
fn profile_list_empty() {
    let dir = tempfile::tempdir().unwrap();
    let profiles = dir.path().join("nonexistent");
    let gens = list_generations(&profiles).unwrap();
    assert!(gens.is_empty());
}

#[test]
fn profile_list_returns_sorted() {
    let dir = tempfile::tempdir().unwrap();
    let profiles = dir.path().join("profiles");
    create_generation(&profiles, "/store/a").unwrap();
    create_generation(&profiles, "/store/b").unwrap();
    create_generation(&profiles, "/store/c").unwrap();

    let gens = list_generations(&profiles).unwrap();
    assert_eq!(gens.len(), 3);
    assert_eq!(gens[0].0, 0);
    assert_eq!(gens[1].0, 1);
    assert_eq!(gens[2].0, 2);
    assert_eq!(gens[0].1, "/store/a");
    assert_eq!(gens[2].1, "/store/c");
}

// ============================================================================
// FJ-1302: rollback
// ============================================================================

#[test]
fn profile_rollback_to_previous() {
    let dir = tempfile::tempdir().unwrap();
    let profiles = dir.path().join("profiles");
    create_generation(&profiles, "/store/v1").unwrap();
    create_generation(&profiles, "/store/v2").unwrap();
    assert_eq!(current_generation(&profiles), Some(1));

    let rolled = rollback(&profiles).unwrap();
    assert_eq!(rolled, 0);
    assert_eq!(current_generation(&profiles), Some(0));
}

#[test]
fn profile_rollback_at_zero_fails() {
    let dir = tempfile::tempdir().unwrap();
    let profiles = dir.path().join("profiles");
    create_generation(&profiles, "/store/only").unwrap();
    assert_eq!(current_generation(&profiles), Some(0));

    let result = rollback(&profiles);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .contains("cannot rollback past generation 0"));
}

#[test]
fn profile_rollback_no_current_fails() {
    let dir = tempfile::tempdir().unwrap();
    let profiles = dir.path().join("empty");
    std::fs::create_dir_all(&profiles).unwrap();
    let result = rollback(&profiles);
    assert!(result.is_err());
}

// ============================================================================
// FJ-1302: current_generation edge cases
// ============================================================================

#[test]
fn profile_current_no_profiles_dir() {
    let dir = tempfile::tempdir().unwrap();
    let profiles = dir.path().join("no_such_dir");
    assert_eq!(current_generation(&profiles), None);
}

// ============================================================================
// FJ-1315: validate_config
// ============================================================================

#[test]
fn sandbox_valid_config_no_errors() {
    let config = preset_profile("full").unwrap();
    let errors = validate_config(&config);
    assert!(
        errors.is_empty(),
        "full preset should validate: {:?}",
        errors
    );
}

#[test]
fn sandbox_zero_memory_rejected() {
    let mut config = preset_profile("full").unwrap();
    config.memory_mb = 0;
    let errors = validate_config(&config);
    assert!(errors.iter().any(|e| e.contains("memory_mb")));
}

#[test]
fn sandbox_zero_cpus_rejected() {
    let mut config = preset_profile("full").unwrap();
    config.cpus = 0.0;
    let errors = validate_config(&config);
    assert!(errors.iter().any(|e| e.contains("cpus")));
}

#[test]
fn sandbox_zero_timeout_rejected() {
    let mut config = preset_profile("full").unwrap();
    config.timeout = 0;
    let errors = validate_config(&config);
    assert!(errors.iter().any(|e| e.contains("timeout")));
}

#[test]
fn sandbox_excessive_memory_rejected() {
    let mut config = preset_profile("full").unwrap();
    config.memory_mb = 2_000_000; // > 1 TiB
    let errors = validate_config(&config);
    assert!(errors.iter().any(|e| e.contains("1 TiB")));
}

#[test]
fn sandbox_excessive_cpus_rejected() {
    let mut config = preset_profile("full").unwrap();
    config.cpus = 2000.0;
    let errors = validate_config(&config);
    assert!(errors.iter().any(|e| e.contains("1024")));
}

#[test]
fn sandbox_empty_bind_mount_source_rejected() {
    let mut config = preset_profile("full").unwrap();
    config.bind_mounts.push(BindMount {
        source: "".to_string(),
        target: "/mnt".to_string(),
        readonly: true,
    });
    let errors = validate_config(&config);
    assert!(errors.iter().any(|e| e.contains("source")));
}

#[test]
fn sandbox_empty_bind_mount_target_rejected() {
    let mut config = preset_profile("full").unwrap();
    config.bind_mounts.push(BindMount {
        source: "/host".to_string(),
        target: "".to_string(),
        readonly: true,
    });
    let errors = validate_config(&config);
    assert!(errors.iter().any(|e| e.contains("target")));
}

// ============================================================================
// FJ-1315: preset_profile
// ============================================================================

#[test]
fn sandbox_preset_full() {
    let config = preset_profile("full").unwrap();
    assert_eq!(config.level, SandboxLevel::Full);
    assert_eq!(config.memory_mb, 2048);
    assert_eq!(config.cpus, 4.0);
    assert_eq!(config.timeout, 600);
}

#[test]
fn sandbox_preset_network_only() {
    let config = preset_profile("network-only").unwrap();
    assert_eq!(config.level, SandboxLevel::NetworkOnly);
    assert_eq!(config.memory_mb, 4096);
}

#[test]
fn sandbox_preset_minimal() {
    let config = preset_profile("minimal").unwrap();
    assert_eq!(config.level, SandboxLevel::Minimal);
    assert_eq!(config.memory_mb, 1024);
}

#[test]
fn sandbox_preset_gpu() {
    let config = preset_profile("gpu").unwrap();
    assert_eq!(config.level, SandboxLevel::NetworkOnly);
    assert_eq!(config.memory_mb, 16384);
    assert!(!config.bind_mounts.is_empty());
    assert!(!config.env.is_empty());
    assert_eq!(config.env[0].name, "NVIDIA_VISIBLE_DEVICES");
}

#[test]
fn sandbox_preset_unknown_returns_none() {
    assert!(preset_profile("nonexistent").is_none());
}

// ============================================================================
// FJ-1315: parse_sandbox_config
// ============================================================================

#[test]
fn sandbox_parse_yaml() {
    let yaml = r#"
level: full
memory_mb: 4096
cpus: 8.0
timeout: 1200
bind_mounts: []
env: []
"#;
    let config = parse_sandbox_config(yaml).unwrap();
    assert_eq!(config.level, SandboxLevel::Full);
    assert_eq!(config.memory_mb, 4096);
    assert_eq!(config.cpus, 8.0);
}

#[test]
fn sandbox_parse_yaml_with_mounts() {
    let yaml = r#"
level: network-only
bind_mounts:
  - source: /data
    target: /mnt/data
    readonly: true
env:
  - name: HOME
    value: /root
"#;
    let config = parse_sandbox_config(yaml).unwrap();
    assert_eq!(config.bind_mounts.len(), 1);
    assert_eq!(config.bind_mounts[0].source, "/data");
    assert!(config.bind_mounts[0].readonly);
    assert_eq!(config.env.len(), 1);
}

#[test]
fn sandbox_parse_yaml_defaults() {
    let yaml = "level: minimal\n";
    let config = parse_sandbox_config(yaml).unwrap();
    assert_eq!(config.memory_mb, 2048); // default
    assert_eq!(config.cpus, 4.0); // default
    assert_eq!(config.timeout, 600); // default
}

#[test]
fn sandbox_parse_invalid_yaml() {
    let result = parse_sandbox_config("not: [valid: yaml: {{");
    assert!(result.is_err());
}

// ============================================================================
// FJ-1315: blocks_network / enforces_fs_isolation
// ============================================================================

#[test]
fn sandbox_blocks_network_full_only() {
    assert!(blocks_network(SandboxLevel::Full));
    assert!(!blocks_network(SandboxLevel::NetworkOnly));
    assert!(!blocks_network(SandboxLevel::Minimal));
    assert!(!blocks_network(SandboxLevel::None));
}

#[test]
fn sandbox_enforces_fs_isolation() {
    assert!(enforces_fs_isolation(SandboxLevel::Full));
    assert!(enforces_fs_isolation(SandboxLevel::NetworkOnly));
    assert!(enforces_fs_isolation(SandboxLevel::Minimal));
    assert!(!enforces_fs_isolation(SandboxLevel::None));
}

// ============================================================================
// FJ-1315: cgroup_path
// ============================================================================

#[test]
fn sandbox_cgroup_path_deterministic() {
    let p1 = cgroup_path("blake3:abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789");
    let p2 = cgroup_path("blake3:abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789");
    assert_eq!(p1, p2);
    assert!(p1.starts_with("/sys/fs/cgroup/forjar-build-"));
    assert!(p1.contains("abcdef0123456789"));
}

#[test]
fn sandbox_cgroup_path_strips_prefix() {
    // With prefix: strips "blake3:", uses first 16 chars of hex
    let p1 = cgroup_path("blake3:abcdef0123456789extra");
    assert!(p1.contains("abcdef0123456789"));
    // Without prefix: uses the raw string's first 16 chars
    let p2 = cgroup_path("abcdef0123456789extra");
    assert!(p2.contains("abcdef0123456789"));
    // Both should produce valid cgroup paths
    assert!(p1.starts_with("/sys/fs/cgroup/forjar-build-"));
    assert!(p2.starts_with("/sys/fs/cgroup/forjar-build-"));
}

#[test]
fn sandbox_cgroup_path_short_hash() {
    let p = cgroup_path("abc");
    assert!(p.starts_with("/sys/fs/cgroup/forjar-build-"));
    assert!(p.contains("abc"));
}

// ============================================================================
// FJ-1315: SandboxConfig serde roundtrip
// ============================================================================

#[test]
fn sandbox_config_yaml_roundtrip() {
    let config = SandboxConfig {
        level: SandboxLevel::Full,
        memory_mb: 8192,
        cpus: 16.0,
        timeout: 3600,
        bind_mounts: vec![BindMount {
            source: "/src".to_string(),
            target: "/mnt/src".to_string(),
            readonly: true,
        }],
        env: vec![EnvVar {
            name: "CC".to_string(),
            value: "gcc".to_string(),
        }],
    };
    let yaml = serde_yaml_ng::to_string(&config).unwrap();
    let parsed: SandboxConfig = serde_yaml_ng::from_str(&yaml).unwrap();
    assert_eq!(parsed.level, SandboxLevel::Full);
    assert_eq!(parsed.memory_mb, 8192);
    assert_eq!(parsed.bind_mounts.len(), 1);
    assert_eq!(parsed.env[0].name, "CC");
}

#[test]
fn sandbox_config_json_roundtrip() {
    let config = preset_profile("minimal").unwrap();
    let json = serde_json::to_string(&config).unwrap();
    let parsed: SandboxConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.level, SandboxLevel::Minimal);
}

// ============================================================================
// FJ-1346: FAR encode/decode roundtrip
// ============================================================================

fn test_manifest() -> FarManifest {
    FarManifest {
        name: "test-pkg".to_string(),
        version: "1.0.0".to_string(),
        arch: "x86_64".to_string(),
        store_hash: "blake3:abc123".to_string(),
        tree_hash: "blake3:def456".to_string(),
        file_count: 2,
        total_size: 1024,
        files: vec![
            FarFileEntry {
                path: "bin/app".to_string(),
                size: 512,
                blake3: "blake3:aaa".to_string(),
            },
            FarFileEntry {
                path: "lib/core.so".to_string(),
                size: 512,
                blake3: "blake3:bbb".to_string(),
            },
        ],
        provenance: FarProvenance {
            origin_provider: "apt".to_string(),
            origin_ref: Some("nginx=1.24".to_string()),
            origin_hash: Some("upstream-hash".to_string()),
            created_at: "2026-03-09T00:00:00Z".to_string(),
            generator: "forjar 1.0".to_string(),
        },
        kernel_contracts: None,
    }
}

#[test]
fn far_encode_decode_manifest_roundtrip() {
    let manifest = test_manifest();
    let data = b"hello world chunk data";
    let chunk_hash = blake3::hash(data);
    let chunks = vec![(*chunk_hash.as_bytes(), data.to_vec())];

    let mut buf = Vec::new();
    encode_far(&manifest, &chunks, &mut buf).unwrap();

    let (decoded_manifest, decoded_chunks) =
        decode_far_manifest(std::io::Cursor::new(&buf)).unwrap();

    assert_eq!(decoded_manifest.name, "test-pkg");
    assert_eq!(decoded_manifest.version, "1.0.0");
    assert_eq!(decoded_manifest.arch, "x86_64");
    assert_eq!(decoded_manifest.store_hash, "blake3:abc123");
    assert_eq!(decoded_manifest.files.len(), 2);
    assert_eq!(decoded_manifest.provenance.origin_provider, "apt");
    assert_eq!(decoded_chunks.len(), 1);
    assert_eq!(decoded_chunks[0].hash, *chunk_hash.as_bytes());
}

#[test]
fn far_encode_decode_multiple_chunks() {
    let manifest = test_manifest();
    let d1 = b"chunk one data bytes";
    let d2 = b"chunk two different content here";
    let h1 = blake3::hash(d1);
    let h2 = blake3::hash(d2);
    let chunks = vec![(*h1.as_bytes(), d1.to_vec()), (*h2.as_bytes(), d2.to_vec())];

    let mut buf = Vec::new();
    encode_far(&manifest, &chunks, &mut buf).unwrap();

    let (_, decoded_chunks) = decode_far_manifest(std::io::Cursor::new(&buf)).unwrap();
    assert_eq!(decoded_chunks.len(), 2);
    assert_eq!(decoded_chunks[0].hash, *h1.as_bytes());
    assert_eq!(decoded_chunks[1].hash, *h2.as_bytes());
    // Second chunk offset should be after first chunk's compressed data
    assert!(decoded_chunks[1].offset > 0);
}

#[test]
fn far_encode_decode_zero_chunks() {
    let manifest = test_manifest();
    let chunks: Vec<([u8; 32], Vec<u8>)> = vec![];

    let mut buf = Vec::new();
    encode_far(&manifest, &chunks, &mut buf).unwrap();

    let (decoded, decoded_chunks) = decode_far_manifest(std::io::Cursor::new(&buf)).unwrap();
    assert_eq!(decoded.name, "test-pkg");
    assert!(decoded_chunks.is_empty());
}

#[test]
fn far_magic_is_12_bytes() {
    assert_eq!(FAR_MAGIC.len(), 12);
    assert!(FAR_MAGIC.starts_with(b"FORJAR-FAR"));
}

#[test]
fn far_decode_invalid_magic_rejected() {
    let buf = b"NOT-FAR-MAGIC-HEADER-AND-MORE-BYTES";
    let result = decode_far_manifest(std::io::Cursor::new(buf));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("magic"));
}

#[test]
fn far_decode_truncated_rejected() {
    let result = decode_far_manifest(std::io::Cursor::new(b"FORJAR"));
    assert!(result.is_err());
}

#[test]
fn far_manifest_provenance_optional_fields() {
    let manifest = FarManifest {
        name: "minimal".to_string(),
        version: "0.1".to_string(),
        arch: "aarch64".to_string(),
        store_hash: "blake3:min".to_string(),
        tree_hash: "blake3:tree".to_string(),
        file_count: 0,
        total_size: 0,
        files: vec![],
        provenance: FarProvenance {
            origin_provider: "cargo".to_string(),
            origin_ref: None,
            origin_hash: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            generator: "forjar".to_string(),
        },
        kernel_contracts: None,
    };

    let mut buf = Vec::new();
    encode_far(&manifest, &[], &mut buf).unwrap();

    let (decoded, _) = decode_far_manifest(std::io::Cursor::new(&buf)).unwrap();
    assert!(decoded.provenance.origin_ref.is_none());
    assert!(decoded.provenance.origin_hash.is_none());
    assert!(decoded.kernel_contracts.is_none());
}

#[test]
fn far_manifest_with_kernel_contracts() {
    let manifest = FarManifest {
        name: "model-pkg".to_string(),
        version: "2.0".to_string(),
        arch: "x86_64".to_string(),
        store_hash: "blake3:model".to_string(),
        tree_hash: "blake3:tree".to_string(),
        file_count: 1,
        total_size: 1000,
        files: vec![FarFileEntry {
            path: "model.safetensors".to_string(),
            size: 1000,
            blake3: "blake3:weights".to_string(),
        }],
        provenance: FarProvenance {
            origin_provider: "hf".to_string(),
            origin_ref: Some("meta-llama/Llama-3.1-8B".to_string()),
            origin_hash: None,
            created_at: "2026-03-09T00:00:00Z".to_string(),
            generator: "forjar".to_string(),
        },
        kernel_contracts: Some(forjar::core::store::far::KernelContractInfo {
            model_type: "llama".to_string(),
            required_ops: vec!["matmul".to_string(), "rms_norm".to_string()],
            coverage_pct: 95.5,
        }),
    };

    let mut buf = Vec::new();
    encode_far(&manifest, &[], &mut buf).unwrap();

    let (decoded, _) = decode_far_manifest(std::io::Cursor::new(&buf)).unwrap();
    let kc = decoded.kernel_contracts.unwrap();
    assert_eq!(kc.model_type, "llama");
    assert_eq!(kc.required_ops.len(), 2);
    assert!((kc.coverage_pct - 95.5).abs() < 0.01);
}

#[test]
fn far_manifest_serde_yaml_roundtrip() {
    let manifest = test_manifest();
    let yaml = serde_yaml_ng::to_string(&manifest).unwrap();
    let parsed: FarManifest = serde_yaml_ng::from_str(&yaml).unwrap();
    assert_eq!(parsed, manifest);
}

#[test]
fn far_manifest_serde_json_roundtrip() {
    let manifest = test_manifest();
    let json = serde_json::to_string(&manifest).unwrap();
    let parsed: FarManifest = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, manifest);
}

#[test]
fn far_large_chunk_roundtrip() {
    let manifest = test_manifest();
    // 1 MB chunk to test zstd compression with larger data
    let data = vec![0xABu8; 1024 * 1024];
    let hash = blake3::hash(&data);
    let chunks = vec![(*hash.as_bytes(), data)];

    let mut buf = Vec::new();
    encode_far(&manifest, &chunks, &mut buf).unwrap();

    // Compressed size should be less than 1 MB + overhead
    assert!(buf.len() < 1024 * 1024);

    let (decoded, entries) = decode_far_manifest(std::io::Cursor::new(&buf)).unwrap();
    assert_eq!(decoded.name, "test-pkg");
    assert_eq!(entries.len(), 1);
}

#[test]
fn far_chunk_table_offsets_consistent() {
    let manifest = test_manifest();
    let d1 = vec![1u8; 100];
    let d2 = vec![2u8; 200];
    let d3 = vec![3u8; 300];
    let h1 = blake3::hash(&d1);
    let h2 = blake3::hash(&d2);
    let h3 = blake3::hash(&d3);
    let chunks = vec![
        (*h1.as_bytes(), d1),
        (*h2.as_bytes(), d2),
        (*h3.as_bytes(), d3),
    ];

    let mut buf = Vec::new();
    encode_far(&manifest, &chunks, &mut buf).unwrap();

    let (_, entries) = decode_far_manifest(std::io::Cursor::new(&buf)).unwrap();
    assert_eq!(entries.len(), 3);
    // First chunk starts at offset 0
    assert_eq!(entries[0].offset, 0);
    // Second chunk starts after first chunk's compressed data
    assert_eq!(entries[1].offset, entries[0].length);
    // Third chunk starts after second
    assert_eq!(entries[2].offset, entries[0].length + entries[1].length);
}
