# Secret Namespaces, Plugins, Behavior Specs & Distribution

Falsification coverage for FJ-3306, FJ-3108, FJ-3400, FJ-2602, FJ-2105, and FJ-3405.

## Secret Namespace Isolation (FJ-3306)

Process-level isolation for ephemeral secret injection:

```rust
use forjar::core::secret_namespace::*;
use forjar::core::ephemeral::ResolvedEphemeral;

let config = NamespaceConfig::default(); // inherit PATH, HOME, etc.
let secrets = vec![ResolvedEphemeral { key: "DB_PASS".into(), /* ... */ }];
let env = build_isolated_env(&config, &secrets);
// env contains only allowlisted vars + secrets + FORJAR_NAMESPACE

let result = execute_isolated(&config, &secrets, "sh", &["-c", "echo $DB_PASS"])?;
assert!(result.success);
assert!(verify_no_leak("DB_PASS")); // secret not in parent
```

## Rulebook Validation (FJ-3108)

YAML validation with semantic checks (event patterns, action completeness, cooldown bounds):

```rust
use forjar::core::rules_engine::*;

let issues = validate_rulebook_yaml(yaml_content)?;
let summary = ValidationSummary::new(rulebook_count, issues);
assert!(summary.passed()); // no errors (warnings ok)

let coverage = event_type_coverage(&config);
// [(FileChanged, 2), (Manual, 1), (CronFired, 0), ...]
```

Checks: duplicate names, empty events/actions, apply.file, notify.channel, cooldown=0, high retries.

## WASM Plugin Manifests (FJ-3400)

Content-addressed plugin verification with BLAKE3:

```rust
use forjar::core::types::*;

let manifest: PluginManifest = serde_yaml_ng::from_str(yaml)?;
assert!(manifest.verify_hash(&wasm_bytes));  // BLAKE3 check
assert!(manifest.is_abi_compatible());        // ABI v1
assert_eq!(manifest.resource_type(), "plugin:k8s-deployment");
```

Schema validation (`PluginSchema`) checks required properties and types.
Permission model: `FsPermissions`, `NetPermissions`, `ExecPermissions`, `EnvPermissions`.

## Behavior Specs (FJ-2602)

Infrastructure behavior assertions with convergence testing:

```rust
use forjar::core::types::*;

let spec: BehaviorSpec = serde_yaml_ng::from_str(yaml)?;
assert_eq!(spec.behavior_count(), 3);
assert_eq!(spec.referenced_resources(), vec!["nginx-pkg", "nginx-svc"]);

let report = BehaviorReport::from_results("nginx".into(), results);
assert!(report.all_passed());
println!("{}", report.format_summary());
```

## Distribution Targets (FJ-2105)

Load, push, and FAR archive distribution:

```rust
use forjar::core::types::*;

let target = DistTarget::Push { registry: "ghcr.io".into(), name: "app".into(), tag: "v1".into() };
assert_eq!(target.description(), "ghcr.io/app:v1");

let report = BuildReport { /* ... */ };
println!("{:.1} MB, {} layers", report.size_mb(), report.layer_count);
```

## Shell Providers (FJ-3405)

Shell scripts as resource providers (on-ramp to WASM):

```rust
use forjar::core::shell_provider::*;

assert_eq!(parse_shell_type("shell:custom"), Some("custom"));
assert!(is_shell_type("shell:custom"));
validate_provider_script("#!/bin/bash\necho ok")?; // bashrs + secret lint
```

## Test Coverage

| File | Tests | Lines |
|------|-------|-------|
| `falsification_namespace_rules.rs` | 33 | ~470 |
| `falsification_plugin_behavior_dist.rs` | 47 | ~500 |
