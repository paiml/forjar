//! Two policy rules with no `id:` and the same `message:` are one rule
//! (paiml/forjar#369).
//!
//! `display_id_of(None, message)` derives a rule's identity from a slug of its
//! prose, and that string was used AS an identity by both consumers:
//!
//! * `policy_coverage::trigger_split` intersected two sets of it, so N co-firing
//!   siblings counted as ONE and an idle rule whose sibling fired was neither
//!   counted nor listed. Measured: `total_rules: 2, rules_triggered: 1,
//!   untriggered_rules: []`. Two is not one plus zero, in the one report whose
//!   job is to say what did NOT run.
//! * `remediate` keyed `selected()`, the reason map and every reported
//!   `policy_id` on it — so `--policy-id RULE-baseline-hardening` edited fields
//!   belonging to a rule the caller did not select, and a rule's "why I could
//!   not fix this" was overwritten by its sibling's. That half ships on the MCP
//!   surface as `forjar_remediate`, whose input schema advertises the generated
//!   id as the selector.
//!
//! The fix is `PolicyRule::display_id_at(index)`: the explicit `id:` when there
//! is one, else `RULE-<index>-<slug>`. The index is a total function of the
//! declaration, so no two rules can share an identity, and every call site
//! already holds it (`violating_pairs` yields it; `Candidate.rule_index` stores
//! it; `policies.iter().enumerate()` produces it).
//!
//! Usage: cargo test --test falsification_policy_rule_identity

use forjar::core::policy_coverage::compute_coverage;
use forjar::core::remediate::{remediate, Report};
use forjar::core::types::ForjarConfig;
use std::path::Path;

fn config(yaml: &str) -> ForjarConfig {
    serde_yaml_ng::from_str(yaml).expect("fixture parses")
}

// ── policy-coverage: the arithmetic must close ──────────────────────

/// One rule fires, its un-id'd sibling is satisfied. Both slugify identically.
const ONE_FIRES_ONE_IDLE: &str = r#"
version: "1.0"
name: collapsed-ids
machines:
  local:
    hostname: localhost
    addr: localhost
resources:
  conf:
    type: file
    machine: local
    path: /etc/app.conf
    content: "k=v"
  pkg:
    type: package
    machine: local
    provider: apt
    packages: [git]
policies:
  - type: require
    message: resources need a field
    field: owner
    resource_type: file
  - type: require
    message: resources need a field
    field: provider
    resource_type: package
"#;

/// Both un-id'd rules fire, on the same resource, sharing a message.
const BOTH_FIRE: &str = r#"
version: "1.0"
name: cofire
machines:
  local:
    hostname: localhost
    addr: localhost
resources:
  conf:
    type: file
    machine: local
    path: /etc/app.conf
    content: "k=v"
policies:
  - type: require
    message: resources need a field
    field: owner
    resource_type: file
  - type: require
    message: resources need a field
    field: group
    resource_type: file
"#;

#[test]
fn an_idle_rule_is_listed_even_when_its_twin_fired() {
    let cov = compute_coverage(&config(ONE_FIRES_ONE_IDLE));

    assert_eq!(cov.total_rules, 2, "the config declares two rules: {cov:?}");
    assert_eq!(
        cov.untriggered_rules.len(),
        1,
        "the `require: provider` rule is satisfied — it never fired, so it belongs in \
         `untriggered_rules`. It is missing because it slugifies to the same \
         `RULE-resources-need-a-field` as the rule that DID fire (paiml/forjar#369): {cov:?}"
    );
    assert_eq!(
        cov.total_rules,
        cov.rules_triggered + cov.untriggered_rules.len(),
        "every rule is either fired or idle; a report where the two halves do not add up to \
         `total_rules` is counting them in different id-spaces: {cov:?}"
    );
}

#[test]
fn two_rules_that_both_fire_count_as_two() {
    let cov = compute_coverage(&config(BOTH_FIRE));

    assert_eq!(cov.total_rules, 2);
    assert_eq!(
        cov.rules_triggered, 2,
        "both rules fired against `conf`, which has neither an owner nor a group. Counting \
         distinct id STRINGS collapses them into one (paiml/forjar#369): {cov:?}"
    );
    assert!(cov.untriggered_rules.is_empty(), "{cov:?}");
}

/// Vacuity guard: the fix must not pass by making every rule look idle, or by
/// giving explicitly-id'd rules a new spelling.
#[test]
fn an_explicit_id_is_still_the_identity() {
    let with_ids = ONE_FIRES_ONE_IDLE
        .replacen("  - type: require\n", "  - type: require\n    id: P-file\n", 1)
        .replacen(
            "  - type: require\n    message: resources need a field\n    field: provider",
            "  - type: require\n    id: P-package\n    message: resources need a field\n    field: provider",
            1,
        );
    let cov = compute_coverage(&config(&with_ids));

    assert_eq!(cov.rules_triggered, 1, "{cov:?}");
    assert_eq!(
        cov.untriggered_rules,
        vec!["P-package"],
        "a rule that declares an `id:` keeps it verbatim — the generated identity is only for \
         rules that declare none: {cov:?}"
    );
}

// ── the same report, through the shipped binary ─────────────────────

fn cli_coverage_json(cfg: &Path) -> serde_json::Value {
    let run = std::process::Command::new(env!("CARGO_BIN_EXE_forjar"))
        .args(["policy-coverage", "--file"])
        .arg(cfg)
        .arg("--json")
        .output()
        .expect("forjar policy-coverage runs");
    assert!(
        run.status.success(),
        "forjar policy-coverage --json failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    serde_json::from_slice(&run.stdout).expect("--json prints JSON")
}

#[test]
fn the_printed_report_accounts_for_every_rule() {
    let d = tempfile::tempdir().unwrap();
    let cfg = d.path().join("forjar.yaml");
    std::fs::write(&cfg, ONE_FIRES_ONE_IDLE).unwrap();

    let out = cli_coverage_json(&cfg);
    let total = out["total_rules"].as_u64().unwrap();
    let fired = out["rules_triggered"].as_u64().unwrap();
    let idle = out["untriggered_rules"].as_array().unwrap().len() as u64;

    assert_eq!(
        total,
        fired + idle,
        "`forjar policy-coverage --json` printed {total} rules as {fired} fired plus {idle} \
         idle. The satisfied rule ran nowhere and is reported nowhere: {out}"
    );
    assert_eq!(idle, 1, "{out}");
}

// ── remediate: the selector must select ONE rule ────────────────────

/// Two un-id'd `assert` rules sharing a message, each demanding a different
/// field of one resource.
const TWO_ASSERTS: &str = "version: \"1.0\"\n\
     name: remediate-ids\n\
     machines:\n\
     \x20 box:\n\
     \x20   hostname: box\n\
     \x20   addr: 127.0.0.1\n\
     resources:\n\
     \x20 web-conf:\n\
     \x20   type: file\n\
     \x20   machine: box\n\
     \x20   path: /etc/web.conf\n\
     \x20   mode: \"0777\"\n\
     \x20   owner: nobody\n\
     \x20   content: |\n\
     \x20     listen 80;\n\
     policies:\n\
     \x20 - type: assert\n\
     \x20   message: baseline hardening\n\
     \x20   resource_type: file\n\
     \x20   condition_field: mode\n\
     \x20   condition_value: \"0640\"\n\
     \x20 - type: assert\n\
     \x20   message: baseline hardening\n\
     \x20   resource_type: file\n\
     \x20   condition_field: owner\n\
     \x20   condition_value: root\n";

/// An UNFIXABLE assert (`path` is not a settable field) beside a `deny`, both
/// un-id'd and sharing a message, both violated by the same resource.
const UNFIXABLE_PAIR: &str = "version: \"1.0\"\n\
     name: remediate-reasons\n\
     machines:\n\
     \x20 box:\n\
     \x20   hostname: box\n\
     \x20   addr: 127.0.0.1\n\
     resources:\n\
     \x20 web-conf:\n\
     \x20   type: file\n\
     \x20   machine: box\n\
     \x20   path: /etc/web.conf\n\
     \x20   owner: nobody\n\
     \x20   content: |\n\
     \x20     listen 80;\n\
     policies:\n\
     \x20 - type: assert\n\
     \x20   message: baseline hardening\n\
     \x20   resource_type: file\n\
     \x20   condition_field: path\n\
     \x20   condition_value: /srv/web.conf\n\
     \x20 - type: deny\n\
     \x20   message: baseline hardening\n\
     \x20   resource_type: file\n\
     \x20   condition_field: owner\n\
     \x20   condition_value: nobody\n";

fn run(source: &str, ids: Option<&[String]>) -> Report {
    remediate(source, &config(source), ids).expect("remediation ran")
}

/// Spelling-independent: the tool's OWN reported id is fed straight back as the
/// filter. Whatever `policy_id` means, selecting by it must select one rule.
#[test]
fn a_reported_policy_id_selects_exactly_the_rule_that_reported_it() {
    let all = run(TWO_ASSERTS, None);
    assert_eq!(
        all.applied.len(),
        2,
        "both asserts are fixable and both fire: {:?}",
        all.applied
    );

    let first = all.applied[0].policy_id.clone();
    let filtered = run(TWO_ASSERTS, Some(std::slice::from_ref(&first)));

    assert_eq!(
        filtered.applied.len(),
        1,
        "`policy_ids: [{first}]` named ONE rule and remediate rewrote {} fields: {:?}. The two \
         rules declare no `id:` and share a `message:`, so they generate the same identity and \
         no string can select between them — the caller's filter silently edits a rule it did \
         not choose (paiml/forjar#369)",
        filtered.applied.len(),
        filtered.applied
    );
    assert_eq!(
        filtered.applied[0].policy_id, first,
        "the surviving fix must be the one whose id was passed: {:?}",
        filtered.applied
    );
    assert_eq!(
        filtered.applied[0].field, all.applied[0].field,
        "and it must be the same FIELD the unfiltered run attributed to that id: {:?}",
        filtered.applied
    );
}

/// Vacuity guard for the test above: an id that names nothing selects nothing.
#[test]
fn an_unknown_policy_id_selects_nothing() {
    let none = run(TWO_ASSERTS, Some(&["NOPE".to_string()]));
    assert!(none.applied.is_empty(), "{:?}", none.applied);
    assert!(!none.changed);
}

/// Each rule must be told why IT could not be fixed. The reason map was keyed
/// on the shared identity, so the second rule's reason overwrote the first's.
#[test]
fn each_unfixable_rule_reports_its_own_reason() {
    let report = run(UNFIXABLE_PAIR, None);
    assert!(report.applied.is_empty(), "{:?}", report.applied);
    assert_eq!(report.remaining.len(), 2, "{:?}", report.remaining);

    let assert_rule = report
        .remaining
        .iter()
        .find(|u| u.rule_type == "assert")
        .expect("the assert rule is still violated");
    assert!(
        assert_rule
            .reason
            .contains("scalar fields forjar will rewrite"),
        "the assert on `path` is unfixable because `path` is not settable, but it reports the \
         DENY rule's reason: `{}`. Both rules key the reason map on the same generated id, so \
         the later `record()` overwrote the earlier one (paiml/forjar#369)",
        assert_rule.reason
    );

    let deny_rule = report
        .remaining
        .iter()
        .find(|u| u.rule_type == "deny")
        .expect("the deny rule is still violated");
    assert!(
        deny_rule.reason.contains("FORBIDDEN"),
        "the deny rule's own reason went missing too: `{}`",
        deny_rule.reason
    );

    assert_ne!(
        assert_rule.policy_id, deny_rule.policy_id,
        "two distinct rules reported the SAME id ({}), which is what made the reasons \
         collide: {:?}",
        assert_rule.policy_id, report.remaining
    );
}
