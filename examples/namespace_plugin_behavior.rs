//! FJ-3306/3108/3400/2602/2105/3405: Namespaces, rulebook validation, plugins,
//! behavior specs, distribution, shell providers.
//!
//! Demonstrates:
//! - Secret namespace isolation (build_isolated_env, format_result)
//! - Rulebook YAML validation and event type coverage
//! - WASM plugin manifests, schema validation, status types
//! - Behavior-driven infrastructure specs and reports
//! - Distribution targets and build reports
//! - Shell provider type parsing
//!
//! Usage: cargo run --example namespace_plugin_behavior

use forjar::core::ephemeral::ResolvedEphemeral;
use forjar::core::rules_engine::{event_type_coverage, validate_rulebook_yaml, ValidationSummary};
use forjar::core::secret_namespace::*;
use forjar::core::shell_provider::{is_shell_type, parse_shell_type};
use forjar::core::types::*;

fn main() {
    println!("Forjar: Namespace, Plugins, Behavior & Distribution");
    println!("{}", "=".repeat(55));

    // ── Secret Namespace Isolation ──
    println!("\n[FJ-3306] Secret Namespace:");
    let config = NamespaceConfig {
        namespace_id: "ns-demo-42".into(),
        inherit_env: vec![],
        audit_enabled: false,
        state_dir: None,
    };
    let secrets = vec![
        ResolvedEphemeral {
            key: "DB_PASS".into(),
            value: "s3cret".into(),
            hash: blake3::hash(b"s3cret").to_hex().to_string(),
        },
        ResolvedEphemeral {
            key: "API_KEY".into(),
            value: "tok-abc".into(),
            hash: blake3::hash(b"tok-abc").to_hex().to_string(),
        },
    ];
    let env = build_isolated_env(&config, &secrets);
    println!("  Env vars: {} (secrets + namespace marker)", env.len());
    println!("  DB_PASS present: {}", env.contains_key("DB_PASS"));
    println!("  No leak check: {}", verify_no_leak("DB_PASS"));

    let result = NamespaceResult {
        namespace_id: "ns-demo-42".into(),
        success: true,
        exit_code: Some(0),
        stdout: "ok\n".into(),
        stderr: String::new(),
        secrets_injected: 2,
        secrets_discarded: 2,
    };
    println!("  {}", format_result(&result));

    // ── Rulebook Validation ──
    println!("\n[FJ-3108] Rulebook Validation:");
    let valid_yaml = r#"
rulebooks:
  - name: config-repair
    events:
      - type: file_changed
        match:
          path: /etc/nginx/nginx.conf
    actions:
      - apply:
          file: forjar.yaml
          tags: [config]
    cooldown_secs: 60
"#;
    let issues = validate_rulebook_yaml(valid_yaml).unwrap();
    println!("  Valid config: {} issues", issues.len());

    let bad_yaml = r#"
rulebooks:
  - name: bad
    events: []
    actions: []
    cooldown_secs: 0
    max_retries: 50
"#;
    let issues = validate_rulebook_yaml(bad_yaml).unwrap();
    let summary = ValidationSummary::new(1, issues);
    println!(
        "  Bad config: {} errors, {} warnings, passed={}",
        summary.error_count(),
        summary.warning_count(),
        summary.passed()
    );

    let config: RulebookConfig = serde_yaml_ng::from_str(valid_yaml).unwrap();
    let coverage = event_type_coverage(&config);
    for (et, count) in &coverage {
        if *count > 0 {
            println!("  EventType {et}: {count} patterns");
        }
    }

    // ── WASM Plugin Manifest ──
    println!("\n[FJ-3400] Plugin Manifest:");
    let manifest = PluginManifest {
        name: "k8s-deployment".into(),
        version: "0.1.0".into(),
        description: Some("Manage K8s Deployments".into()),
        abi_version: PLUGIN_ABI_VERSION,
        wasm: "k8s-deployment.wasm".into(),
        blake3: blake3::hash(b"fake-wasm").to_hex().to_string(),
        permissions: PluginPermissions::default(),
        schema: None,
    };
    println!("  {manifest}");
    println!("  ABI compatible: {}", manifest.is_abi_compatible());
    println!("  Resource type: {}", manifest.resource_type());
    println!("  Hash valid: {}", manifest.verify_hash(b"fake-wasm"));

    // ── Plugin Schema Validation ──
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
        required: vec!["name".into()],
        properties,
    };
    let mut props = indexmap::IndexMap::new();
    props.insert("name".into(), serde_yaml_ng::Value::String("app".into()));
    println!("  Schema errors: {}", schema.validate(&props).len());

    // ── Behavior Spec ──
    println!("\n[FJ-2602] Behavior Spec:");
    let spec = BehaviorSpec {
        name: "nginx infra".into(),
        config: "examples/nginx.yaml".into(),
        machine: Some("web-1".into()),
        behaviors: vec![
            BehaviorEntry {
                name: "nginx installed".into(),
                resource: Some("nginx-pkg".into()),
                behavior_type: None,
                assert_state: Some("present".into()),
                verify: Some(VerifyCommand {
                    command: "dpkg -l nginx".into(),
                    exit_code: Some(0),
                    ..Default::default()
                }),
                convergence: None,
            },
            BehaviorEntry {
                name: "idempotency".into(),
                resource: None,
                behavior_type: Some("convergence".into()),
                assert_state: None,
                verify: None,
                convergence: Some(ConvergenceAssert {
                    second_apply: Some("noop".into()),
                    state_unchanged: Some(true),
                }),
            },
        ],
    };
    println!("  Behaviors: {}", spec.behavior_count());
    println!("  Resources: {:?}", spec.referenced_resources());

    let results = vec![
        BehaviorResult {
            name: "nginx installed".into(),
            passed: true,
            failure: None,
            actual_exit_code: Some(0),
            actual_stdout: None,
            duration_ms: 42,
        },
        BehaviorResult {
            name: "idempotency".into(),
            passed: true,
            failure: None,
            actual_exit_code: None,
            actual_stdout: None,
            duration_ms: 80,
        },
    ];
    let report = BehaviorReport::from_results("nginx infra".into(), results);
    println!("  Report: {}/{} passed", report.passed, report.total);

    // ── Distribution ──
    println!("\n[FJ-2105] Distribution:");
    for target in [
        DistTarget::Load {
            runtime: "docker".into(),
        },
        DistTarget::Push {
            registry: "ghcr.io".into(),
            name: "myapp".into(),
            tag: "v1".into(),
        },
        DistTarget::Far {
            output_path: "/tmp/out.far".into(),
        },
    ] {
        println!("  {}", target.description());
    }
    let a = ArchBuild::linux_amd64();
    println!("  Arch: {} ({})", a.platform, a.architecture);

    // ── Shell Provider ──
    println!("\n[FJ-3405] Shell Provider:");
    println!(
        "  parse 'shell:custom': {:?}",
        parse_shell_type("shell:custom")
    );
    println!("  parse 'file': {:?}", parse_shell_type("file"));
    println!("  is_shell 'shell:x': {}", is_shell_type("shell:x"));

    println!("\n{}", "=".repeat(55));
    println!("All namespace/plugin/behavior/dist criteria survived.");
}
