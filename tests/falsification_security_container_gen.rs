//! FJ-2300/2101/2002: Security types, container builds, generation metadata.
//! Usage: cargo test --test falsification_security_container_gen

use forjar::core::types::*;

// ── helpers ──

fn dd(b3: &str, sha: &str, sz: u64) -> DualDigest {
    DualDigest {
        blake3: b3.into(),
        sha256: sha.into(),
        size_bytes: sz,
    }
}

fn md(created: &[&str], updated: &[&str], destroyed: &[&str], unchanged: u32) -> MachineDelta {
    MachineDelta {
        created: created.iter().map(|s| s.to_string()).collect(),
        updated: updated.iter().map(|s| s.to_string()).collect(),
        destroyed: destroyed.iter().map(|s| s.to_string()).collect(),
        unchanged,
    }
}

fn dle(id: &str, rtype: &str, gen: u32, reliable: bool) -> DestroyLogEntry {
    DestroyLogEntry {
        timestamp: "2026-03-05T14:30:00Z".into(),
        machine: "intel".into(),
        resource_id: id.into(),
        resource_type: rtype.into(),
        pre_hash: "blake3:aaa".into(),
        generation: gen,
        config_fragment: Some("state: present".into()),
        reliable_recreate: reliable,
    }
}

fn finding(rid: &str, field: &str, pat: &str) -> SecretScanFinding {
    SecretScanFinding {
        resource_id: rid.into(),
        field: field.into(),
        pattern: pat.into(),
        preview: format!("{pat} s3c***"),
    }
}

// ── FJ-2300: SecretProvider ──

#[test]
fn secret_provider_display_and_default() {
    assert_eq!(SecretProvider::Env.to_string(), "env");
    assert_eq!(SecretProvider::File.to_string(), "file");
    assert_eq!(SecretProvider::Sops.to_string(), "sops");
    assert_eq!(SecretProvider::Op.to_string(), "op");
    assert_eq!(SecretProvider::default(), SecretProvider::Env);
}

#[test]
fn secret_provider_serde() {
    for p in [
        SecretProvider::Env,
        SecretProvider::File,
        SecretProvider::Sops,
        SecretProvider::Op,
    ] {
        let json = serde_json::to_string(&p).unwrap();
        let parsed: SecretProvider = serde_json::from_str(&json).unwrap();
        assert_eq!(p, parsed);
    }
}

// ── FJ-2300: PathPolicy ──

#[test]
fn path_policy_exact_match() {
    let policy = PathPolicy {
        deny_paths: vec!["/etc/shadow".into(), "/etc/sudoers".into()],
    };
    assert!(policy.is_denied("/etc/shadow"));
    assert!(policy.is_denied("/etc/sudoers"));
    assert!(!policy.is_denied("/etc/nginx.conf"));
}

#[test]
fn path_policy_glob_match() {
    let policy = PathPolicy {
        deny_paths: vec!["/etc/sudoers.d/*".into()],
    };
    assert!(policy.is_denied("/etc/sudoers.d/custom"));
    assert!(!policy.is_denied("/etc/sudoers"));
}

#[test]
fn path_policy_no_restrictions() {
    let policy = PathPolicy::default();
    assert!(!policy.has_restrictions());
    assert!(!policy.is_denied("/anything"));
}

#[test]
fn path_policy_multiple_patterns() {
    let policy = PathPolicy {
        deny_paths: vec!["/etc/shadow".into(), "/root/*".into(), "/proc/*".into()],
    };
    assert!(policy.has_restrictions());
    assert!(policy.is_denied("/etc/shadow"));
    assert!(policy.is_denied("/root/.ssh/id_rsa"));
    assert!(policy.is_denied("/proc/1/cmdline"));
    assert!(!policy.is_denied("/etc/nginx.conf"));
}

// ── FJ-2300: AuthzResult ──

#[test]
fn authz_allowed_and_denied() {
    assert!(AuthzResult::Allowed.is_allowed());
    assert_eq!(AuthzResult::Allowed.to_string(), "allowed");
    let denied = AuthzResult::Denied {
        operator: "eve".into(),
        machine: "prod-db".into(),
    };
    assert!(!denied.is_allowed());
    assert!(denied.to_string().contains("eve"));
    assert!(denied.to_string().contains("prod-db"));
}

#[test]
fn authz_serde() {
    let json = serde_json::to_string(&AuthzResult::Allowed).unwrap();
    let parsed: AuthzResult = serde_json::from_str(&json).unwrap();
    assert!(parsed.is_allowed());
}

// ── FJ-2300: SecretScanResult ──

#[test]
fn secret_scan_clean() {
    let r = SecretScanResult::from_findings(vec![], 10);
    assert!(r.clean);
    assert_eq!(r.scanned_fields, 10);
    assert!(r.findings.is_empty());
}

#[test]
fn secret_scan_with_findings() {
    let r = SecretScanResult::from_findings(vec![finding("db", "content", "password:")], 5);
    assert!(!r.clean);
    assert_eq!(r.findings.len(), 1);
    assert_eq!(r.scanned_fields, 5);
}

#[test]
fn secret_scan_finding_display() {
    let s = finding("app", "content", "api_key:").to_string();
    assert!(s.contains("app.content"));
    assert!(s.contains("api_key:"));
}

// ── FJ-2300: OperatorIdentity ──

#[test]
fn operator_from_flag() {
    let op = OperatorIdentity::from_flag("deploy-bot");
    assert_eq!(op.name, "deploy-bot");
    assert_eq!(op.source, OperatorSource::CliFlag);
    assert_eq!(op.to_string(), "deploy-bot");
}

#[test]
fn operator_from_env() {
    let op = OperatorIdentity::from_env();
    assert!(!op.name.is_empty());
    assert!(op.name.contains('@'));
    assert_eq!(op.source, OperatorSource::Environment);
}

#[test]
fn operator_resolve() {
    let with = OperatorIdentity::resolve(Some("ci-bot"));
    assert_eq!(with.name, "ci-bot");
    assert_eq!(with.source, OperatorSource::CliFlag);
    let without = OperatorIdentity::resolve(None);
    assert!(without.name.contains('@'));
    assert_eq!(without.source, OperatorSource::Environment);
}

#[test]
fn operator_serde() {
    let op = OperatorIdentity::from_flag("admin");
    let json = serde_json::to_string(&op).unwrap();
    let parsed: OperatorIdentity = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.name, "admin");
    assert_eq!(parsed.source, OperatorSource::CliFlag);
}

// ── FJ-2101: DualDigest ──

#[test]
fn dual_digest_oci_and_forjar() {
    let d = dd("abc123", "def456", 1024);
    assert_eq!(d.oci_digest(), "sha256:def456");
    assert_eq!(d.forjar_digest(), "blake3:abc123");
}

#[test]
fn dual_digest_display() {
    let s = dd("abcdef0123456789", "fedcba9876543210", 4096).to_string();
    assert!(s.contains("blake3:abcdef01"));
    assert!(s.contains("sha256:fedcba98"));
    assert!(s.contains("4096B"));
}

#[test]
fn dual_digest_serde() {
    let d = dd("abc", "def", 100);
    let json = serde_json::to_string(&d).unwrap();
    let parsed: DualDigest = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.blake3, "abc");
    assert_eq!(parsed.size_bytes, 100);
}

// ── FJ-2101: ImageBuildPlan ──

fn sample_build_plan() -> ImageBuildPlan {
    ImageBuildPlan {
        tag: "myapp:latest".into(),
        base_image: Some("ubuntu:22.04".into()),
        layers: vec![
            LayerStrategy::Packages {
                names: vec!["nginx".into()],
            },
            LayerStrategy::Files {
                paths: vec!["/etc/nginx/nginx.conf".into()],
            },
            LayerStrategy::Build {
                command: "make".into(),
                workdir: None,
            },
            LayerStrategy::Derivation {
                store_path: "/forjar/store/abc".into(),
            },
        ],
        labels: vec![("maintainer".into(), "team@example.com".into())],
        entrypoint: None,
    }
}

#[test]
fn build_plan_basics() {
    assert_eq!(sample_build_plan().layer_count(), 4);
    assert!(!sample_build_plan().is_scratch());
}

#[test]
fn build_plan_scratch() {
    let p = ImageBuildPlan {
        tag: "s:v1".into(),
        base_image: None,
        layers: vec![],
        labels: vec![],
        entrypoint: None,
    };
    assert!(p.is_scratch());
    assert_eq!(p.layer_count(), 0);
}

#[test]
fn build_plan_tier_plan() {
    let plan = sample_build_plan();
    let tiers = plan.tier_plan();
    assert_eq!(tiers.len(), 4);
    assert_eq!(tiers[0].0, 0); // Packages = tier 0
    assert_eq!(tiers[1].0, 2); // Files = tier 2
    assert_eq!(tiers[2].0, 1); // Build = tier 1
    assert_eq!(tiers[3].0, 3); // Derivation = tier 3
}

// ── FJ-2101: BaseImageRef ──

#[test]
fn base_image_ref_new() {
    let r = BaseImageRef::new("ubuntu:22.04");
    assert_eq!(r.reference, "ubuntu:22.04");
    assert!(!r.resolved);
    assert!(r.manifest_digest.is_none());
}

#[test]
fn base_image_ref_registry_default() {
    assert_eq!(BaseImageRef::new("ubuntu:22.04").registry(), "docker.io");
}

#[test]
fn base_image_ref_registries() {
    assert_eq!(
        BaseImageRef::new("ghcr.io/owner/image:v1").registry(),
        "ghcr.io"
    );
    assert_eq!(
        BaseImageRef::new("localhost:5000/myimage:latest").registry(),
        "localhost:5000"
    );
}

// ── FJ-2101: OciBuildResult ──

fn sample_oci_result() -> OciBuildResult {
    OciBuildResult {
        tag: "myapp:v1".into(),
        manifest_digest: "sha256:abc".into(),
        layer_count: 3,
        total_size: 50 * 1024 * 1024,
        duration_secs: 12.5,
        layout_path: "/tmp/oci".into(),
    }
}

#[test]
fn oci_build_result_size_mb() {
    assert!((sample_oci_result().size_mb() - 50.0).abs() < 0.01);
}

#[test]
fn oci_build_result_display() {
    let s = sample_oci_result().to_string();
    assert!(s.contains("myapp:v1"));
    assert!(s.contains("3 layers"));
    assert!(s.contains("50.0 MB"));
}

// ── FJ-2101: WhiteoutEntry ──

#[test]
fn whiteout_file_delete() {
    let w = WhiteoutEntry::FileDelete {
        path: "/etc/old.conf".into(),
    };
    assert_eq!(w.oci_path(), "/etc/.wh.old.conf");
}

#[test]
fn whiteout_file_delete_no_dir() {
    let w = WhiteoutEntry::FileDelete {
        path: "orphan".into(),
    };
    assert_eq!(w.oci_path(), ".wh.orphan");
}

#[test]
fn whiteout_opaque_dir() {
    let w = WhiteoutEntry::OpaqueDir {
        path: "/var/cache".into(),
    };
    assert_eq!(w.oci_path(), "/var/cache/.wh..wh..opq");
}

// ── FJ-2101: OciLayerConfig & Compression ──

#[test]
fn oci_layer_config_default() {
    let c = OciLayerConfig::default();
    assert_eq!(c.compression, OciCompression::Gzip);
    assert!(c.deterministic);
    assert_eq!(c.epoch_mtime, 0);
    assert_eq!(c.sort_order, TarSortOrder::Lexicographic);
}

#[test]
fn oci_compression_display() {
    assert_eq!(OciCompression::None.to_string(), "none");
    assert_eq!(OciCompression::Gzip.to_string(), "gzip");
    assert_eq!(OciCompression::Zstd.to_string(), "zstd");
}

#[test]
fn oci_compression_serde() {
    for c in [
        OciCompression::None,
        OciCompression::Gzip,
        OciCompression::Zstd,
    ] {
        let json = serde_json::to_string(&c).unwrap();
        let parsed: OciCompression = serde_json::from_str(&json).unwrap();
        assert_eq!(c, parsed);
    }
}

// ── FJ-2002: GenerationMeta ──

#[test]
fn gen_meta_new() {
    let m = GenerationMeta::new(5, "2026-03-01T00:00:00Z".into());
    assert_eq!(m.generation, 5);
    assert_eq!(m.action, "apply");
    assert!(m.config_hash.is_none());
    assert!(!m.is_undo());
}

#[test]
fn gen_meta_undo() {
    let m = GenerationMeta::new_undo(3, "ts".into(), 1);
    assert_eq!(m.action, "undo");
    assert_eq!(m.parent_generation, Some(1));
    assert!(m.is_undo());
}

#[test]
fn gen_meta_builder_methods() {
    let m = GenerationMeta::new(1, "ts".into())
        .with_config_hash("blake3:abc".into())
        .with_git_ref("a1b2c3d".into());
    assert_eq!(m.config_hash.as_deref(), Some("blake3:abc"));
    assert_eq!(m.git_ref.as_deref(), Some("a1b2c3d"));
}

#[test]
fn gen_meta_yaml_roundtrip() {
    let mut m = GenerationMeta::new(5, "2026-03-05T14:30:00Z".into());
    m.config_hash = Some("blake3:abc".into());
    m.operator = Some("noah@host".into());
    m.record_machine("intel", md(&["pkg-a"], &["config-b"], &[], 10));
    let yaml = m.to_yaml().unwrap();
    let parsed = GenerationMeta::from_yaml(&yaml).unwrap();
    assert_eq!(parsed.generation, 5);
    assert_eq!(parsed.config_hash.as_deref(), Some("blake3:abc"));
    assert_eq!(parsed.machines["intel"].created, vec!["pkg-a"]);
}

#[test]
fn gen_meta_total_changes() {
    let mut m = GenerationMeta::new(1, "ts".into());
    m.record_machine("a", md(&["x"], &["y"], &[], 5));
    m.record_machine("b", md(&[], &[], &["z"], 3));
    assert_eq!(m.total_changes(), 3);
}

#[test]
fn gen_meta_no_changes() {
    assert_eq!(GenerationMeta::new(0, "ts".into()).total_changes(), 0);
}

#[test]
fn gen_meta_compat_and_skip_empty() {
    let m =
        GenerationMeta::from_yaml("generation: 3\ncreated_at: '2026-01-01T00:00:00Z'\n").unwrap();
    assert_eq!(m.generation, 3);
    assert_eq!(m.action, "apply");
    assert!(m.machines.is_empty());
    let yaml = GenerationMeta::new(0, "ts".into()).to_yaml().unwrap();
    assert!(!yaml.contains("config_hash"));
    assert!(!yaml.contains("git_ref"));
    assert!(!yaml.contains("machines"));
}

// ── FJ-2002: MachineDelta ──

#[test]
fn machine_delta() {
    let d = md(&["a", "b"], &["c"], &["d"], 10);
    assert_eq!(d.total_changes(), 4);
    assert!(d.has_changes());
    let empty = MachineDelta::default();
    assert_eq!(empty.total_changes(), 0);
    assert!(!empty.has_changes());
}

// ── FJ-2002: DestroyLogEntry ──

#[test]
fn destroy_log_jsonl_roundtrip() {
    let entry = dle("nginx-pkg", "package", 5, false);
    let line = entry.to_jsonl().unwrap();
    let parsed = DestroyLogEntry::from_jsonl(&line).unwrap();
    assert_eq!(parsed.resource_id, "nginx-pkg");
    assert_eq!(parsed.generation, 5);
    assert!(!parsed.reliable_recreate);
}

#[test]
fn destroy_log_reliable() {
    assert!(dle("cfg", "file", 2, true).reliable_recreate);
}

#[test]
fn destroy_log_from_jsonl_invalid() {
    assert!(DestroyLogEntry::from_jsonl("not json").is_err());
}

// ── FJ-2002: git helpers ──

#[test]
fn git_helpers_callable() {
    if let Some(ref r) = get_git_ref() {
        assert!(!r.is_empty());
    }
    let _ = git_is_dirty(); // just verify no panic
}
