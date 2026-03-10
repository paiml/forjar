//! FJ-3400/2602/2105/3405: Plugin types, behavior specs, distribution, shell providers.
//! Usage: cargo test --test falsification_plugin_behavior_dist

use forjar::core::shell_provider::{is_shell_type, parse_shell_type, validate_provider_script};
use forjar::core::types::*;

// ── FJ-3400: PluginManifest ──

fn sample_manifest() -> PluginManifest {
    PluginManifest {
        name: "k8s-deployment".into(),
        version: "0.1.0".into(),
        description: Some("Manage K8s Deployments".into()),
        abi_version: PLUGIN_ABI_VERSION,
        wasm: "k8s-deployment.wasm".into(),
        blake3: "placeholder".into(),
        permissions: PluginPermissions::default(),
        schema: None,
    }
}

#[test]
fn manifest_resource_type() {
    assert_eq!(sample_manifest().resource_type(), "plugin:k8s-deployment");
}

#[test]
fn manifest_abi_compatible() {
    assert!(sample_manifest().is_abi_compatible());
}

#[test]
fn manifest_abi_incompatible() {
    let mut m = sample_manifest();
    m.abi_version = 99;
    assert!(!m.is_abi_compatible());
}

#[test]
fn manifest_verify_hash_correct() {
    let data = b"fake wasm module bytes";
    let hash = blake3::hash(data).to_hex().to_string();
    let mut m = sample_manifest();
    m.blake3 = hash;
    assert!(m.verify_hash(data));
}

#[test]
fn manifest_verify_hash_tampered() {
    let mut m = sample_manifest();
    m.blake3 = blake3::hash(b"original").to_hex().to_string();
    assert!(!m.verify_hash(b"tampered"));
}

#[test]
fn manifest_display() {
    let m = sample_manifest();
    let display = format!("{m}");
    assert!(display.contains("k8s-deployment"));
    assert!(display.contains("0.1.0"));
    assert!(display.contains("ABI v"));
}

#[test]
fn manifest_serde_roundtrip() {
    let yaml = r#"
name: test-plugin
version: "1.0.0"
abi_version: 1
wasm: test.wasm
blake3: "abc123"
permissions:
  fs:
    read: ["/etc/config"]
  exec:
    allow: ["kubectl"]
  env:
    read: ["KUBECONFIG"]
"#;
    let m: PluginManifest = serde_yaml_ng::from_str(yaml).unwrap();
    assert_eq!(m.name, "test-plugin");
    assert_eq!(m.permissions.fs.read, vec!["/etc/config"]);
    assert_eq!(m.permissions.exec.allow, vec!["kubectl"]);
    assert_eq!(m.permissions.env.read, vec!["KUBECONFIG"]);
}

// ── FJ-3400: PluginPermissions ──

#[test]
fn permissions_empty_default() {
    assert!(PluginPermissions::default().is_empty());
}

#[test]
fn permissions_not_empty_fs() {
    let mut p = PluginPermissions::default();
    p.fs.read.push("/etc".into());
    assert!(!p.is_empty());
}

#[test]
fn permissions_not_empty_net() {
    let mut p = PluginPermissions::default();
    p.net.connect.push("api.example.com:443".into());
    assert!(!p.is_empty());
}

#[test]
fn permissions_not_empty_exec() {
    let mut p = PluginPermissions::default();
    p.exec.allow.push("curl".into());
    assert!(!p.is_empty());
}

// ── FJ-3408: PluginSchema validation ──

#[test]
fn schema_validate_required_missing() {
    let schema = PluginSchema {
        required: vec!["name".into(), "image".into()],
        properties: indexmap::IndexMap::new(),
    };
    let mut props = indexmap::IndexMap::new();
    props.insert("name".into(), serde_yaml_ng::Value::String("app".into()));
    let errors = schema.validate(&props);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("image"));
}

#[test]
fn schema_validate_type_mismatch() {
    let mut properties = indexmap::IndexMap::new();
    properties.insert(
        "replicas".into(),
        SchemaProperty {
            prop_type: Some("integer".into()),
            default: None,
            items: None,
        },
    );
    let schema = PluginSchema {
        required: vec![],
        properties,
    };
    let mut props = indexmap::IndexMap::new();
    props.insert(
        "replicas".into(),
        serde_yaml_ng::Value::String("three".into()),
    );
    let errors = schema.validate(&props);
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("expected type 'integer'"));
}

#[test]
fn schema_validate_all_pass() {
    let schema = PluginSchema {
        required: vec!["name".into()],
        properties: indexmap::IndexMap::new(),
    };
    let mut props = indexmap::IndexMap::new();
    props.insert("name".into(), serde_yaml_ng::Value::String("ok".into()));
    assert!(schema.validate(&props).is_empty());
}

// ── FJ-3400: PluginStatus ──

#[test]
fn plugin_status_display() {
    assert_eq!(PluginStatus::Converged.to_string(), "converged");
    assert_eq!(PluginStatus::Drifted.to_string(), "drifted");
    assert_eq!(PluginStatus::Missing.to_string(), "missing");
    assert_eq!(PluginStatus::Error.to_string(), "error");
}

#[test]
fn plugin_status_serde() {
    for status in [
        PluginStatus::Converged,
        PluginStatus::Drifted,
        PluginStatus::Missing,
        PluginStatus::Error,
    ] {
        let json = serde_json::to_string(&status).unwrap();
        let parsed: PluginStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, status);
    }
}

// ── FJ-3400: PluginApplyOutcome ──

#[test]
fn plugin_apply_outcome_serde() {
    let outcome = PluginApplyOutcome {
        success: true,
        status: PluginStatus::Converged,
        changes: vec!["created deployment".into()],
        error: None,
    };
    let json = serde_json::to_string(&outcome).unwrap();
    let parsed: PluginApplyOutcome = serde_json::from_str(&json).unwrap();
    assert!(parsed.success);
    assert_eq!(parsed.status, PluginStatus::Converged);
    assert_eq!(parsed.changes.len(), 1);
}

// ── FJ-2602: BehaviorSpec ──

fn entry(name: &str, resource: Option<&str>, btype: Option<&str>, verify: bool) -> BehaviorEntry {
    BehaviorEntry {
        name: name.into(),
        resource: resource.map(Into::into),
        behavior_type: btype.map(Into::into),
        assert_state: if verify { Some("present".into()) } else { None },
        verify: if verify {
            Some(VerifyCommand {
                command: "dpkg -l nginx".into(),
                exit_code: Some(0),
                ..Default::default()
            })
        } else {
            None
        },
        convergence: if btype == Some("convergence") {
            Some(ConvergenceAssert {
                second_apply: Some("noop".into()),
                state_unchanged: Some(true),
            })
        } else {
            None
        },
    }
}

fn sample_behavior_spec() -> BehaviorSpec {
    BehaviorSpec {
        name: "nginx test".into(),
        config: "examples/nginx.yaml".into(),
        machine: Some("web-1".into()),
        behaviors: vec![
            entry("nginx installed", Some("nginx-pkg"), None, true),
            entry("idempotency", None, Some("convergence"), false),
            entry("service running", Some("nginx-svc"), None, false),
        ],
    }
}

#[test]
fn behavior_spec_count() {
    assert_eq!(sample_behavior_spec().behavior_count(), 3);
}

#[test]
fn behavior_spec_referenced_resources() {
    let spec = sample_behavior_spec();
    let refs = spec.referenced_resources();
    assert_eq!(refs, vec!["nginx-pkg", "nginx-svc"]);
}

#[test]
fn behavior_spec_serde_roundtrip() {
    let yaml = r#"
name: nginx test
config: examples/nginx.yaml
machine: web-1
behaviors:
  - name: nginx installed
    resource: nginx-pkg
    state: present
    verify:
      command: "dpkg -l nginx"
      exit_code: 0
  - name: idempotency
    type: convergence
    convergence:
      second_apply: noop
      state_unchanged: true
"#;
    let spec: BehaviorSpec = serde_yaml_ng::from_str(yaml).unwrap();
    assert_eq!(spec.name, "nginx test");
    assert_eq!(spec.behavior_count(), 2);
    assert!(spec.behaviors[1].is_convergence());
}

// ── FJ-2602: BehaviorEntry ──

#[test]
fn behavior_entry_is_convergence() {
    let entry = &sample_behavior_spec().behaviors[1];
    assert!(entry.is_convergence());
    assert!(!entry.has_verify());
}

#[test]
fn behavior_entry_has_verify() {
    let entry = &sample_behavior_spec().behaviors[0];
    assert!(entry.has_verify());
    assert!(!entry.is_convergence());
}

#[test]
fn behavior_entry_neither() {
    let entry = &sample_behavior_spec().behaviors[2];
    assert!(!entry.is_convergence());
    assert!(!entry.has_verify());
}

// ── FJ-2602: BehaviorReport ──

fn br(name: &str, passed: bool, failure: Option<&str>) -> BehaviorResult {
    BehaviorResult {
        name: name.into(),
        passed,
        failure: failure.map(Into::into),
        actual_exit_code: None,
        actual_stdout: None,
        duration_ms: 50,
    }
}

#[test]
fn behavior_report_all_passed() {
    let report = BehaviorReport::from_results(
        "nginx".into(),
        vec![br("a", true, None), br("b", true, None)],
    );
    assert!(report.all_passed());
    assert_eq!(report.total, 2);
    assert_eq!(report.passed, 2);
}

#[test]
fn behavior_report_with_failures() {
    let report = BehaviorReport::from_results(
        "nginx".into(),
        vec![br("ok", true, None), br("fail", false, Some("exit 1"))],
    );
    assert!(!report.all_passed());
    assert_eq!(report.passed, 1);
    assert_eq!(report.failed, 1);
}

#[test]
fn behavior_report_format_summary() {
    let report = BehaviorReport::from_results(
        "test".into(),
        vec![
            br("pkg ok", true, None),
            br("svc fail", false, Some("not active")),
        ],
    );
    let summary = report.format_summary();
    assert!(summary.contains("[PASS] pkg ok"));
    assert!(summary.contains("[FAIL] svc fail"));
    assert!(summary.contains("not active"));
    assert!(summary.contains("1/2 passed"));
}

#[test]
fn behavior_report_empty() {
    let report = BehaviorReport::from_results("empty".into(), vec![]);
    assert!(report.all_passed());
    assert_eq!(report.total, 0);
}

#[test]
fn behavior_report_display() {
    let report = BehaviorReport::from_results("spec".into(), vec![]);
    let display = format!("{report}");
    assert!(display.contains("Behavior Spec: spec"));
}

// ── FJ-2602: VerifyCommand / ConvergenceAssert ──

#[test]
fn verify_command_defaults() {
    let vc = VerifyCommand::default();
    assert!(vc.command.is_empty());
    assert!(vc.exit_code.is_none());
    assert!(vc.stdout.is_none());
    assert!(vc.port_open.is_none());
    assert!(vc.retries.is_none());
}

#[test]
fn convergence_assert_defaults() {
    let ca = ConvergenceAssert::default();
    assert!(ca.second_apply.is_none());
    assert!(ca.state_unchanged.is_none());
}

// ── FJ-2105: DistTarget ──

#[test]
fn dist_target_load() {
    let t = DistTarget::Load {
        runtime: "docker".into(),
    };
    assert_eq!(t.description(), "docker load");
}

#[test]
fn dist_target_push() {
    let t = DistTarget::Push {
        registry: "ghcr.io".into(),
        name: "myorg/myapp".into(),
        tag: "v1".into(),
    };
    assert_eq!(t.description(), "ghcr.io/myorg/myapp:v1");
}

#[test]
fn dist_target_far() {
    let t = DistTarget::Far {
        output_path: "/tmp/out.far".into(),
    };
    assert_eq!(t.description(), "FAR → /tmp/out.far");
}

#[test]
fn dist_target_serde() {
    let t = DistTarget::Push {
        registry: "r".into(),
        name: "n".into(),
        tag: "t".into(),
    };
    let json = serde_json::to_string(&t).unwrap();
    let parsed: DistTarget = serde_json::from_str(&json).unwrap();
    assert!(matches!(parsed, DistTarget::Push { .. }));
}

// ── FJ-2105: ArchBuild ──

#[test]
fn arch_build_amd64() {
    let a = ArchBuild::linux_amd64();
    assert_eq!(a.platform, "linux/amd64");
    assert_eq!(a.os, "linux");
    assert_eq!(a.architecture, "amd64");
    assert!(a.variant.is_none());
}

#[test]
fn arch_build_arm64() {
    let a = ArchBuild::linux_arm64();
    assert_eq!(a.platform, "linux/arm64");
    assert_eq!(a.architecture, "arm64");
    assert_eq!(a.variant.as_deref(), Some("v8"));
}

// ── FJ-2105: BuildReport ──

fn layer(idx: u32, name: &str, cached: bool) -> LayerReport {
    LayerReport {
        index: idx,
        name: name.into(),
        store_hash: format!("blake3:{name}"),
        size: 25_000_000,
        cached,
        duration_secs: if cached { 0.2 } else { 48.3 },
    }
}

fn sample_build_report() -> BuildReport {
    BuildReport {
        image_ref: "myregistry.io/app:1.0".into(),
        digest: "sha256:abc123".into(),
        total_size: 50_000_000,
        layer_count: 2,
        duration_secs: 48.5,
        layers: vec![layer(0, "base", true), layer(1, "app", false)],
        distribution: vec![],
        architectures: vec![],
    }
}

#[test]
fn build_report_size_mb() {
    let report = sample_build_report();
    assert!((report.size_mb() - 47.7).abs() < 0.1);
}

#[test]
fn build_report_format_summary() {
    let s = sample_build_report().format_summary();
    assert!(s.contains("myregistry.io/app:1.0"));
    assert!(s.contains("2 layers"));
    assert!(s.contains("cached"));
    assert!(s.contains("new"));
}
