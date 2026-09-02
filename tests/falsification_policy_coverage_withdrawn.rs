//! `policy-coverage` was withdrawn from the unified surface (paiml/forjar#369).
//!
//! It shipped as a verb earlier on this branch, on the strength of one
//! calculation serving both surfaces. The brief that commissioned it set a
//! condition: *if unifying them is too large, do NOT ship the verb — leave the
//! row in Pending and say why, rather than publishing a tool that answers
//! differently from the CLI.* The condition was met, by measurement.
//!
//! `PolicyRule::display_id` and `PolicyViolation::display_id` both derive an
//! identity from the rule's `message:` when it declares no `id:`
//! (`RULE-<slugified message>`), and `policy-coverage` used that string AS an
//! identity — it decided which rules fired by intersecting two sets of it. Two
//! rules with no `id:` that shared a `message:` were therefore ONE rule to this
//! report. Measured on the built binary, one violated rule and one satisfied
//! rule sharing a message:
//!
//! ```text
//!   "total_rules": 2, "rules_triggered": 1, "untriggered_rules": []
//! ```
//!
//! Two is not one plus zero. The satisfied rule never ran and the report could
//! not say so — in the one report whose entire job is to say what is NOT
//! covered.
//!
//! THAT DEFECT IS FIXED. `trigger_split` splits by rule INDEX and names an idle
//! rule with `PolicyRule::display_id_at`, so `total_rules == rules_triggered +
//! untriggered_rules.len()` holds structurally;
//! `tests/falsification_policy_rule_identity.rs` is the falsifier, and the pin
//! below is inverted to measure the right answer through the same binary.
//!
//! The ROW is still `Bucket::Pending`, and this file still guards that: putting
//! a tool back on the unified surface publishes a new schema on every transport
//! and is a decision with its own test surface (`falsification_verb_pending_
//! discharge.rs`, `falsification_readonly_surface_executes_nothing.rs`,
//! `e2e_verb_surface_t.rs`), not a side effect of a bugfix. What changed is the
//! REASON: the answer is no longer wrong, only unpublished.
//!
//! Nothing here reverts #356's merge: one calculation is still the right end
//! state, and `the_cli_leaf_prints_the_one_calculation_verbatim` below is what
//! holds it to that.
//!
//! `the_verb_and_the_cli_leaf_print_the_same_document` could never have caught
//! this: both surfaces were wrong identically. Its surviving half is here.
//!
//! Usage: cargo test --test falsification_policy_coverage_withdrawn

use forjar::core::{parser, policy_coverage};
use forjar::verb::{partition, Bucket};
use std::path::{Path, PathBuf};

/// Run `forjar policy-coverage --json` on the SHIPPED binary and parse it.
///
/// Spawned, never called in-process. The claim under test is about what the
/// command prints, and a library function compared with itself proves the
/// library agrees with itself.
fn cli_json(cfg: &Path) -> serde_json::Value {
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
    serde_json::from_slice(&run.stdout)
        .unwrap_or_else(|e| panic!("--json did not print JSON ({e}): {:?}", run.stdout))
}

fn write_cfg(dir: &Path, body: &str) -> PathBuf {
    let cfg = dir.join("forjar.yaml");
    std::fs::write(&cfg, body).unwrap();
    cfg
}

/// A project whose policies cover the file resource and not the package one.
fn policy_project(dir: &Path) -> PathBuf {
    write_cfg(
        dir,
        r#"
version: "1.0"
name: coverage-fixture
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
    message: files need an owner
    field: owner
    resource_type: file
    compliance:
      - framework: soc2
        control: CC6.1
"#,
    )
}

// ── the withdrawal itself ───────────────────────────────────────────

/// REJECTION CRITERION: `policy-coverage` re-added to the unified surface as a
/// side effect, rather than as a decision.
///
/// Checked on every transport-facing table at once, because the failure this
/// guards against is a row being re-added to one of them: the verb registry,
/// the MCP handler map and the published `tools/list` schema each have to agree
/// that the tool is not there.
///
/// The #369 defect that forced the withdrawal is fixed. Re-shipping is now a
/// deliberate edit — two type declarations, one handler, one `register_all`
/// line, one `verb_table!` row — that publishes a new tool schema on every
/// transport and has to answer to the verb-surface suites. Making it happen is
/// editing this test; letting it happen is not.
#[test]
fn policy_coverage_is_not_on_the_unified_surface() {
    assert!(
        forjar::verb::find("policy-coverage").is_none(),
        "`policy-coverage` is a verb again. That publishes a new tool on every \
         transport at once; do it as its own change, with `falsification_verb_\
         pending_discharge.rs` and the read-only surface suites run against it, \
         and edit this test in the same commit"
    );

    let reg = forjar::mcp::build_registry();
    assert!(
        !reg.has_handler("forjar_policy_coverage"),
        "a handler is registered for a tool no verb table declares — \
         `register_all_matches_the_verb_table` should have caught this"
    );

    let schema = forjar::mcp::export_schema();
    let names: Vec<&str> = schema["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();
    assert!(
        !names.contains(&"forjar_policy_coverage"),
        "tools/list still advertises forjar_policy_coverage: {names:?}"
    );
}

/// The row is DEBT, not a gap. `Bucket::Pending` is the ledger and its reason
/// has to name the defect — "not done yet" would read as work never started,
/// when this is work that shipped and was taken back. The reason now also says
/// the defect is FIXED — a ledger that still describes a repaired answer as
/// wrong is a ledger nobody can act on — but only the issue reference is
/// asserted here: the wording of a debt note is prose, and pinning prose is how
/// a ledger becomes unmaintainable.
#[test]
fn the_withdrawal_is_recorded_as_debt_naming_the_defect() {
    let row = partition()
        .iter()
        .find(|l| l.path == ["policy-coverage"])
        .expect("`policy-coverage` has no row in the partition");

    match &row.bucket {
        Bucket::Pending(reason) => {
            assert!(
                reason.contains("#369"),
                "the ledger row must cite the defect that put it back, not a \
                 generic backlog issue: `{reason}`"
            );
        }
        other => panic!(
            "`policy-coverage` is {other:?}; a capability withdrawn for being \
             wrong belongs in the debt ledger, not in CliOnly (which asserts it \
             has no transport-neutral meaning) and not in Unified"
        ),
    }
}

// ── the defect that justifies the withdrawal ────────────────────────

/// #369, FIXED — this used to pin the WRONG answer on purpose.
///
/// The fixture is the minimum that triggered it: two `require` rules, neither
/// declaring an `id:`, sharing one `message:`. One is violated (`conf` has no
/// owner) and one is satisfied (`pkg` declares a provider). The satisfied rule
/// never fired, so it must appear in `untriggered_rules` — and did not, because
/// both rules slugified to the same `RULE-resources-need-a-field` and the
/// report intersected those strings.
///
/// `trigger_split` now splits by rule INDEX and names an idle rule with
/// `display_id_at`, so the arithmetic below closes structurally rather than by
/// assertion. The test is kept, inverted, because the numbers it measures are
/// the ones that were wrong.
#[test]
fn two_unnamed_rules_sharing_a_message_are_two_rules() {
    let d = tempfile::tempdir().unwrap();
    let cfg = write_cfg(
        d.path(),
        r#"
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
"#,
    );

    let out = cli_json(&cfg);

    assert_eq!(
        out["total_rules"], 2,
        "the config declares two rules: {out}"
    );
    assert_eq!(
        out["rules_triggered"], 1,
        "only the file rule is violated: {out}"
    );
    assert_eq!(
        out["untriggered_rules"],
        serde_json::json!(["RULE-1-resources-need-a-field"]),
        "the `require: provider` rule is satisfied — it never fired — so it \
         belongs here, named by its INDEX rather than by the message slug it \
         shares with the rule that did fire: {out}"
    );

    // The arithmetic the defect broke, stated without naming an
    // implementation. A report that cannot say what did not run is the one
    // report that must.
    let total = out["total_rules"].as_u64().unwrap();
    let fired = out["rules_triggered"].as_u64().unwrap();
    let idle = out["untriggered_rules"].as_array().unwrap().len() as u64;
    assert_eq!(
        total,
        fired + idle,
        "every rule is accounted for as fired or idle: {out}"
    );
}

// ── what #356 built, which survives the withdrawal ──────────────────

/// The surviving half of `the_verb_and_the_cli_leaf_print_the_same_document`.
///
/// Before #356 there were two policy-coverage calculations answering different
/// questions under one name, and the CLI leaf routed to the one with no
/// documentation while the verb was wired to the one with no caller. That merge
/// is NOT reverted here: withdrawing the verb removes a transport, not the
/// single calculation, and this asserts the single calculation is still what
/// the command prints — byte for byte, from the shipped binary against the
/// library it claims to render.
///
/// It is also what makes re-shipping the verb cheap once #369 is fixed: one
/// row, no second answer to reconcile.
#[test]
fn the_cli_leaf_prints_the_one_calculation_verbatim() {
    let d = tempfile::tempdir().unwrap();
    let cfg = policy_project(d.path());

    let from_cli = cli_json(&cfg);

    let config = parser::parse_and_validate(&cfg).expect("fixture parses");
    let from_lib = serde_json::to_value(policy_coverage::compute_coverage(&config))
        .expect("the report serialises");

    assert_eq!(
        from_cli,
        from_lib,
        "`forjar policy-coverage --json` and `core::policy_coverage::\
         compute_coverage` returned DIFFERENT documents. The renderer is \
         supposed to print the calculation verbatim; a field that appears on \
         one and not the other, or the same field with a different value, means \
         a hand-written projection is back.\n\ncli: {}\n\nlib: {}",
        serde_json::to_string_pretty(&from_cli).unwrap_or_default(),
        serde_json::to_string_pretty(&from_lib).unwrap_or_default(),
    );

    // Vacuity guard: an empty object equals an empty object.
    assert!(
        from_cli.as_object().is_some_and(|o| o.len() >= 10),
        "the document has {} fields — the equality above is close to vacuous",
        from_cli.as_object().map(serde_json::Map::len).unwrap_or(0)
    );
}

/// COVERED is not CLEAN, and the report has to carry both halves.
///
/// `conf` is the covered resource and it VIOLATES; `pkg` is the clean one and
/// no rule scopes to it. Both halves report "1 of 2" and mean opposite
/// resources — the divergence that shipped while these were two calculations.
#[test]
fn policy_coverage_names_the_resources_no_policy_covers() {
    let d = tempfile::tempdir().unwrap();
    let cfg = policy_project(d.path());
    let out = cli_json(&cfg);

    assert_eq!(out["total_resources"], 2);
    assert_eq!(out["covered_resources"], 1);
    assert_eq!(out["fully_covered"], false);
    assert_eq!(
        out["uncovered"],
        serde_json::json!(["pkg"]),
        "the package resource is matched by no policy, and saying which one is \
         uncovered is the entire point of the report: {out}"
    );
    assert_eq!(out["by_type"]["require"], 1);
    assert_eq!(out["compliance_frameworks"]["soc2"], 1);

    assert_eq!(out["clean_resources"], 1, "{out}");
    assert_eq!(out["rules_triggered"], 1, "{out}");
    assert_eq!(out["untriggered_rules"], serde_json::json!([]), "{out}");
}

/// The guard against "covered" being a constant.
#[test]
fn policy_coverage_reports_a_config_with_no_policies_as_uncovered() {
    let d = tempfile::tempdir().unwrap();
    let cfg = write_cfg(
        d.path(),
        "version: \"1.0\"\nname: bare\nmachines:\n  local:\n    hostname: localhost\n    \
         addr: localhost\nresources:\n  conf:\n    type: file\n    machine: local\n    \
         path: /etc/a.conf\n    content: x\n",
    );
    let out = cli_json(&cfg);

    assert_eq!(out["covered_resources"], 0);
    assert_eq!(out["coverage_percent"], 0.0);
    assert_eq!(out["uncovered"], serde_json::json!(["conf"]));
    // ...and it is clean, because nothing looked at it. A report that printed
    // only `clean_resources` would call this config compliant.
    assert_eq!(out["clean_resources"], 1, "{out}");
}
