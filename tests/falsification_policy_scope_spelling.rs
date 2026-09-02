//! A policy rule scoped to a multi-word resource type is inert (paiml/forjar#366).
//!
//! `matches_scope` compared `format!("{:?}", resource.resource_type)
//! .to_lowercase()` — the DEBUG spelling — against `rule.resource_type`, which
//! is the free-form string an operator writes in YAML. For the fifteen
//! single-word variants the two spellings agree. For the six multi-word ones
//! they do not, and the one that `matches_scope` demanded (`githubrelease`) is
//! the one spelling serde REFUSES everywhere else:
//!
//! ```text
//! $ forjar validate -f badtype.yaml
//! error: resources.release.type: unknown variant `githubrelease`, expected one
//! of `package`, ..., `github_release`, `overlay_interface`, `disk_budget`,
//! `backup_sync`, `nas_archive`
//! ```
//!
//! So a rule scoped `resource_type: github_release` — the only spelling a user
//! can write — matched nothing, `forjar policy` printed "All 1 policy rules
//! passed" and exited 0, and `apply_preflight::check_policy_violations` let the
//! apply through. The gate failed OPEN.
//!
//! The same Debug-as-identity backed `condition_field: type`, so a `deny` rule
//! comparing `type == github_release` never fired either, and
//! `remediate::apply_one` refused a `type`-keyed candidate with the misleading
//! "the value is produced by a recipe or a {{template}} expansion".
//!
//! Usage: cargo test --test falsification_policy_scope_spelling

use forjar::core::parser::{evaluate_policies, resource_field_value};
use forjar::core::types::{ForjarConfig, PolicyRule, PolicyRuleType, Resource, ResourceType};
use std::path::Path;

/// Every `ResourceType` whose serde spelling differs from its `Debug` spelling,
/// paired with the serde spelling — the ONLY one `type:` accepts in a document.
const MULTI_WORD: &[(ResourceType, &str)] = &[
    (ResourceType::WasmBundle, "wasm_bundle"),
    (ResourceType::GithubRelease, "github_release"),
    (ResourceType::OverlayInterface, "overlay_interface"),
    (ResourceType::DiskBudget, "disk_budget"),
    (ResourceType::BackupSync, "backup_sync"),
    (ResourceType::NasArchive, "nas_archive"),
];

fn rule(rule_type: PolicyRuleType, scope: Option<&str>) -> PolicyRule {
    PolicyRule {
        rule_type,
        message: "resources must declare an owner".into(),
        id: Some("SCOPE".into()),
        resource_type: scope.map(str::to_string),
        tag: None,
        field: Some("owner".into()),
        condition_field: None,
        condition_value: None,
        max_count: None,
        min_count: None,
        severity: None,
        remediation: None,
        compliance: vec![],
    }
}

/// One resource of `rt` with no `owner`, and one `require: owner` rule scoped
/// to `scope`.
fn scoped(rt: ResourceType, scope: &str) -> ForjarConfig {
    let mut cfg = ForjarConfig::default();
    cfg.resources.insert(
        "release".into(),
        Resource {
            resource_type: rt,
            ..Default::default()
        },
    );
    cfg.policies = vec![rule(PolicyRuleType::Require, Some(scope))];
    cfg
}

/// The Debug spelling of a variant, lowercased — what `matches_scope` used to
/// compare against, derived here rather than hard-coded so the table cannot
/// drift from the enum.
fn debug_spelling(rt: &ResourceType) -> String {
    format!("{rt:?}").to_lowercase()
}

/// THE DEFECT. A rule scoped with the spelling the schema accepts must fire.
#[test]
fn a_rule_scoped_to_a_multi_word_type_is_enforced() {
    for (rt, serde_name) in MULTI_WORD {
        // Vacuity guard: this table is only meaningful where the two spellings
        // actually diverge.
        assert_ne!(
            &debug_spelling(rt),
            serde_name,
            "{rt:?} is not a multi-word variant — it does not belong in this table"
        );

        let violations = evaluate_policies(&scoped(rt.clone(), serde_name));
        assert_eq!(
            violations.len(),
            1,
            "a `require: owner` rule scoped `resource_type: {serde_name}` did not fire against a \
             {rt:?} resource with no owner. `{serde_name}` is the ONLY spelling `type:` accepts, \
             so this rule cannot be written in a form that works — the policy gate fails open \
             (paiml/forjar#366)"
        );
    }
}

/// The other half: the Debug spelling must NOT be a working scope. It is not a
/// legal `type:` anywhere in the schema, so a rule naming it scopes to nothing.
#[test]
fn the_debug_spelling_is_not_a_scope() {
    for (rt, _) in MULTI_WORD {
        let debug = debug_spelling(rt);
        let violations = evaluate_policies(&scoped(rt.clone(), &debug));
        assert!(
            violations.is_empty(),
            "`resource_type: {debug}` matched a {rt:?} resource. That string is rejected by \
             `type:` (serde emits `unknown variant`), so accepting it as a scope makes the one \
             working spelling the one no document can declare"
        );
    }
}

/// `Display` is the serde spelling, for EVERY variant — the contract the fix
/// leans on. Asserted against serde itself rather than against a second
/// hand-written table, so a new variant that forgets a `Display` arm fails here.
#[test]
fn display_is_the_serde_spelling_for_every_variant() {
    let all = [
        ResourceType::Package,
        ResourceType::File,
        ResourceType::Service,
        ResourceType::Mount,
        ResourceType::User,
        ResourceType::Docker,
        ResourceType::Pepita,
        ResourceType::Network,
        ResourceType::Cron,
        ResourceType::Recipe,
        ResourceType::Model,
        ResourceType::Gpu,
        ResourceType::Task,
        ResourceType::WasmBundle,
        ResourceType::Image,
        ResourceType::Build,
        ResourceType::GithubRelease,
        ResourceType::OverlayInterface,
        ResourceType::DiskBudget,
        ResourceType::BackupSync,
        ResourceType::NasArchive,
    ];
    for rt in &all {
        let wire = serde_json::to_value(rt).expect("a unit variant serialises");
        assert_eq!(
            wire.as_str(),
            Some(rt.to_string().as_str()),
            "{rt:?} serialises as {wire} but Displays as `{rt}` — the two spellings are the same \
             identity and every surface that names a resource type depends on it"
        );
    }
}

/// `condition_field: type` reads the same identity, and it was the same Debug
/// spelling — so a `deny` on a multi-word type never fired.
#[test]
fn the_type_field_reads_the_serde_spelling() {
    for (rt, serde_name) in MULTI_WORD {
        let resource = Resource {
            resource_type: rt.clone(),
            ..Default::default()
        };
        assert_eq!(
            resource_field_value(&resource, "type").as_deref(),
            Some(*serde_name),
            "`condition_field: type` on a {rt:?} resource reports a spelling the document cannot \
             write, so `condition_value: {serde_name}` never matches"
        );
    }
}

// ── through the shipped binary ──────────────────────────────────────

/// A `github_release` resource with no `owner`, and a rule that demands one.
const GATE_FIXTURE: &str = r#"
version: "1.0"
name: scope-gate
machines:
  local:
    hostname: localhost
    addr: localhost
resources:
  release:
    type: github_release
    machine: local
    repo: paiml/forjar
    binary: forjar
policies:
  - type: require
    id: OWNER
    message: github_release resources must declare an owner
    field: owner
    resource_type: github_release
"#;

fn run_policy(cfg: &Path) -> std::process::Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_forjar"))
        .args(["policy", "--file"])
        .arg(cfg)
        .arg("--json")
        .output()
        .expect("forjar policy runs")
}

/// The end-to-end consequence: `forjar policy` exits 0 over a violating
/// resource, and `apply_preflight` reads exactly this result.
#[test]
fn forjar_policy_blocks_on_a_multi_word_scope() {
    let d = tempfile::tempdir().unwrap();
    let cfg = d.path().join("forjar.yaml");
    std::fs::write(&cfg, GATE_FIXTURE).unwrap();

    let out = run_policy(&cfg);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let report: serde_json::Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("--json did not print JSON ({e}): {stdout}"));

    assert_eq!(
        report["violations"].as_array().map(Vec::len),
        Some(1),
        "the rule is scoped `github_release` and the resource IS one, with no owner: {report}"
    );
    assert!(
        !out.status.success(),
        "`forjar policy` exited 0 over a violating resource. That exit code is the apply gate \
         (`apply_preflight::check_policy_violations`), so a rule scoped to any multi-word type \
         let the apply through (paiml/forjar#366). stdout: {stdout}"
    );
}
