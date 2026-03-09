# Security Types, Container Builds & Generation Metadata

Falsification coverage for FJ-2300, FJ-2101, and FJ-2002.

## Security Model (FJ-2300)

Secret providers, path deny policies, authorization, secret scanning, and operator identity:

```rust
use forjar::core::types::*;

// Secret providers
assert_eq!(SecretProvider::default(), SecretProvider::Env);

// Path deny policy with glob support
let policy = PathPolicy { deny_paths: vec!["/etc/shadow".into(), "/root/*".into()] };
assert!(policy.is_denied("/etc/shadow"));
assert!(policy.is_denied("/root/.ssh/id_rsa"));
assert!(!policy.is_denied("/etc/nginx.conf"));

// Authorization
let authz = AuthzResult::Denied { operator: "eve".into(), machine: "prod".into() };
assert!(!authz.is_allowed());

// Secret scanning
let result = SecretScanResult::from_findings(vec![], 15);
assert!(result.clean);

// Operator identity
let op = OperatorIdentity::resolve(Some("deploy-bot"));
assert_eq!(op.source, OperatorSource::CliFlag);
```

## Container Builds (FJ-2101)

Dual digests, image build plans, base image refs, OCI results, and whiteout entries:

```rust
use forjar::core::types::*;

// Dual digest — BLAKE3 + SHA-256 in a single pass
let d = DualDigest { blake3: "abc".into(), sha256: "def".into(), size_bytes: 1024 };
assert_eq!(d.oci_digest(), "sha256:def");
assert_eq!(d.forjar_digest(), "blake3:abc");

// Image build plan with tier assignment
let plan = ImageBuildPlan {
    tag: "app:v1".into(), base_image: Some("ubuntu:22.04".into()),
    layers: vec![
        LayerStrategy::Packages { names: vec!["nginx".into()] },
        LayerStrategy::Files { paths: vec!["/etc/app.conf".into()] },
    ],
    labels: vec![], entrypoint: None,
};
assert_eq!(plan.layer_count(), 2);
assert!(!plan.is_scratch());

// Base image registry parsing
assert_eq!(BaseImageRef::new("ghcr.io/owner/img:v1").registry(), "ghcr.io");
assert_eq!(BaseImageRef::new("ubuntu:22.04").registry(), "docker.io");

// Whiteout entries for OCI layers
let w = WhiteoutEntry::FileDelete { path: "/etc/old.conf".into() };
assert_eq!(w.oci_path(), "/etc/.wh.old.conf");
```

## Generation Metadata (FJ-2002)

Extended generation tracking with config hashes, git refs, undo chains, and per-machine deltas:

```rust
use forjar::core::types::*;

let mut meta = GenerationMeta::new(42, "2026-03-09T10:00:00Z".into());
meta = meta.with_config_hash("blake3:deadbeef".into()).with_git_ref("a1b2c3d".into());
meta.record_machine("intel", MachineDelta {
    created: vec!["new-pkg".into()], updated: vec!["config".into()],
    destroyed: vec![], unchanged: 15,
});
assert_eq!(meta.total_changes(), 2);
assert!(!meta.is_undo());

// Undo generation
let undo = GenerationMeta::new_undo(43, "ts".into(), 42);
assert!(undo.is_undo());

// YAML roundtrip
let yaml = meta.to_yaml().unwrap();
let parsed = GenerationMeta::from_yaml(&yaml).unwrap();
assert_eq!(parsed.generation, 42);

// Destroy log for undo-destroy recovery
let entry = DestroyLogEntry { /* ... */ };
let jsonl = entry.to_jsonl().unwrap();
let parsed = DestroyLogEntry::from_jsonl(&jsonl).unwrap();
```

## Test Coverage

| File | Tests | Lines |
|------|-------|-------|
| `falsification_security_container_gen.rs` | 53 | ~490 |
