//! Unit coverage for the remediation pipeline.

use super::*;
use crate::core::types::ForjarConfig;

fn config(yaml: &str) -> ForjarConfig {
    serde_yaml_ng::from_str(yaml).expect("fixture parses")
}

fn base(policies: &str) -> String {
    format!(
        "version: \"1.0\"\n\
         name: remediate-fixture\n\
         # this comment must survive\n\
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
         \x20   content: |\n\
         \x20     listen 80;\n\
         policies:\n{policies}"
    )
}

const ASSERT_0644: &str = "  - type: assert\n\
                           \x20   id: SEC-MODE\n\
                           \x20   message: files must be 0644\n\
                           \x20   resource_type: file\n\
                           \x20   condition_field: mode\n\
                           \x20   condition_value: \"0644\"\n";

const DENY_0777: &str = "  - type: deny\n\
                         \x20   id: SEC-NO-0777\n\
                         \x20   message: 0777 is forbidden\n\
                         \x20   resource_type: file\n\
                         \x20   condition_field: mode\n\
                         \x20   condition_value: \"0777\"\n";

fn run(source: &str) -> Report {
    let cfg = config(source);
    remediate(source, &cfg, None).expect("remediation ran")
}

#[test]
fn the_target_value_comes_from_the_rule() {
    let source = base(ASSERT_0644);
    let report = run(&source);
    assert_eq!(report.applied.len(), 1);
    assert_eq!(report.applied[0].to, "0644");
    assert_eq!(report.applied[0].from.as_deref(), Some("0777"));
    assert_eq!(report.applied[0].policy_id, "SEC-MODE");
    assert!(report.updated_yaml.contains("mode: \"0644\""));
    assert!(report.changed);
}

#[test]
fn a_different_rule_yields_a_different_value() {
    let source = base(&ASSERT_0644.replace("\"0644\"", "\"0600\""));
    let report = run(&source);
    assert_eq!(report.applied[0].to, "0600");
    assert!(report.updated_yaml.contains("mode: \"0600\""));
}

#[test]
fn every_untouched_byte_survives() {
    let source = base(ASSERT_0644);
    let report = run(&source);
    assert_eq!(report.updated_yaml.lines().count(), source.lines().count());
    assert!(report.updated_yaml.contains("# this comment must survive"));
    assert!(report.updated_yaml.contains("    content: |"));
    assert!(report.updated_yaml.contains("      listen 80;"));
    let differing = source
        .lines()
        .zip(report.updated_yaml.lines())
        .filter(|(a, b)| a != b)
        .count();
    assert_eq!(differing, 1);
}

#[test]
fn a_deny_rule_is_reported_unfixable_never_guessed() {
    let source = base(DENY_0777);
    let report = run(&source);
    assert!(report.applied.is_empty());
    assert!(!report.changed);
    assert_eq!(report.updated_yaml, source);
    assert_eq!(report.remaining.len(), 1);
    assert!(report.remaining[0].reason.contains("FORBIDDEN"));
    assert_eq!(report.hash_before, report.hash_after);
}

#[test]
fn a_require_rule_is_reported_unfixable() {
    let policies = "  - type: require\n\
                    \x20   id: OWN-001\n\
                    \x20   message: files must declare an owner\n\
                    \x20   resource_type: file\n\
                    \x20   field: owner\n";
    let report = run(&base(policies));
    assert!(report.applied.is_empty());
    assert_eq!(report.remaining.len(), 1);
    assert!(report.remaining[0].reason.contains("not the value"));
}

#[test]
fn a_limit_rule_is_reported_unfixable() {
    let policies = "  - type: limit\n\
                    \x20   id: TAG-001\n\
                    \x20   message: at least one tag\n\
                    \x20   resource_type: file\n\
                    \x20   field: tags\n\
                    \x20   min_count: 1\n";
    let report = run(&base(policies));
    assert!(report.applied.is_empty());
    assert!(report.remaining[0]
        .reason
        .contains("bounds the size of a list"));
}

#[test]
fn an_unsettable_field_is_refused_with_the_settable_list() {
    let policies = ASSERT_0644
        .replace("condition_field: mode", "condition_field: content")
        .replace("condition_value: \"0644\"", "condition_value: \"x\"");
    let report = run(&base(&policies));
    assert!(report.applied.is_empty());
    assert!(report.remaining[0]
        .reason
        .contains("scalar fields forjar will rewrite"));
}

#[test]
fn remaining_is_re_evaluated_not_bookkept() {
    let source = base(&format!("{ASSERT_0644}{DENY_0777}"));
    let report = run(&source);
    // SEC-MODE was satisfied by writing 0644; SEC-NO-0777 was satisfied as a
    // side effect (0644 is not 0777). Both disappear because the rules were
    // re-run against the corrected config, not because anything was removed
    // from a list.
    assert_eq!(report.applied.len(), 1);
    assert!(
        report.remaining.is_empty(),
        "unexpected remaining: {:?}",
        report.remaining
    );
}

#[test]
fn remediation_is_idempotent() {
    let source = base(ASSERT_0644);
    let once = run(&source);
    let twice = run(&once.updated_yaml);
    assert!(!twice.changed);
    assert!(twice.applied.is_empty());
    assert_eq!(twice.updated_yaml, once.updated_yaml);
    assert_eq!(twice.hash_before, twice.hash_after);
}

#[test]
fn a_flow_style_resource_fails_closed() {
    let source = "version: \"1.0\"\n\
                  name: flow\n\
                  machines:\n\
                  \x20 box:\n\
                  \x20   hostname: box\n\
                  \x20   addr: 127.0.0.1\n\
                  resources:\n\
                  \x20 web-conf: {type: file, machine: box, path: /etc/w, mode: \"0777\"}\n\
                  policies:\n"
        .to_string()
        + ASSERT_0644;
    let report = run(&source);
    assert!(report.applied.is_empty());
    assert_eq!(report.updated_yaml, source);
    assert!(report.remaining[0].reason.contains("flow style"));
}

#[test]
fn a_value_the_parser_resolved_differently_is_refused() {
    // Stand-in for a `{{template}}` or recipe expansion: the document says one
    // thing, the resolved config another. Editing the literal would not change
    // the resolved value, so the anchor is refused.
    let source = base(ASSERT_0644);
    let mut cfg = config(&source);
    if let Some(r) = cfg.resources.get_mut("web-conf") {
        r.mode = Some("0755".to_string());
    }
    let report = remediate(&source, &cfg, None).expect("ran");
    assert!(report.applied.is_empty());
    assert_eq!(report.updated_yaml, source);
    assert!(report.remaining[0].reason.contains("expansion"));
}

#[test]
fn a_resource_from_an_include_names_the_file() {
    let source = base(ASSERT_0644);
    let mut cfg = config(&source);
    cfg.resources.shift_remove("web-conf");
    let mut extra: crate::core::types::Resource = serde_yaml_ng::from_str(
        "type: file\nmachine: box\npath: /etc/other.conf\nmode: \"0777\"\n",
    )
    .expect("resource parses");
    extra.mode = Some("0777".to_string());
    cfg.resources.insert("other-conf".to_string(), extra);
    cfg.include_provenance
        .insert("resource:other-conf".to_string(), "extra.yaml".to_string());
    let report = remediate(&source, &cfg, None).expect("ran");
    assert!(report.applied.is_empty());
    assert!(report.remaining[0].reason.contains("extra.yaml"));
}

#[test]
fn policy_ids_filter_which_rules_run() {
    let source = base(&format!("{ASSERT_0644}{DENY_0777}"));
    let cfg = config(&source);
    let only_deny = vec!["SEC-NO-0777".to_string()];
    let report = remediate(&source, &cfg, Some(&only_deny)).expect("ran");
    assert!(report.applied.is_empty(), "SEC-MODE was not selected");
    let by_id: Vec<&str> = report
        .remaining
        .iter()
        .map(|v| v.policy_id.as_str())
        .collect();
    assert!(by_id.contains(&"SEC-MODE"));
    assert!(by_id.contains(&"SEC-NO-0777"));
    let sec_mode = report
        .remaining
        .iter()
        .find(|v| v.policy_id == "SEC-MODE")
        .expect("present");
    assert_eq!(sec_mode.reason, "not selected by policy_ids");
}

#[test]
fn a_config_with_no_policies_says_what_it_did_not_look_at() {
    let source = base("");
    let cfg = config(&source);
    let report = remediate(&source, &cfg, None).expect("ran");
    assert!(report.applied.is_empty());
    let note = report.scope_note.expect("a scope note");
    assert!(note.contains("compliance packs"));
}

#[test]
fn a_rule_without_an_id_still_reports_under_its_generated_id() {
    let policies = ASSERT_0644.replace("    id: SEC-MODE\n", "");
    let report = run(&base(&policies));
    assert_eq!(report.applied.len(), 1);
    assert!(report.applied[0].policy_id.starts_with("RULE-"));
}

#[test]
fn the_documents_own_quote_style_survives_the_edit() {
    // The fixture writes `mode: "0777"`. An operator reading the diff should
    // see one value change, not a value change plus a quote-style change.
    let double = base(ASSERT_0644);
    assert!(run(&double).updated_yaml.contains("mode: \"0644\""));

    let single = double.replace("mode: \"0777\"", "mode: '0777'");
    assert!(run(&single).updated_yaml.contains("mode: '0644'"));
}

#[test]
fn a_value_that_would_need_escaping_is_not_hand_quoted() {
    // `emit_in_style` may only add the document's own double quotes when the
    // result parses back to exactly the value. A value carrying a quote and a
    // backslash does not, so serde's rendering stands.
    let policies = ASSERT_0644
        .replace("condition_field: mode", "condition_field: owner")
        .replace("condition_value: \"0644\"", "condition_value: 'a\"b\\\\c'");
    let source = base(&policies).replace(
        "    mode: \"0777\"\n",
        "    mode: \"0777\"\n    owner: \"root\"\n",
    );
    let report = run(&source);
    assert_eq!(report.applied.len(), 1, "{:?}", report.remaining);
    let want = &report.applied[0].to;
    let round_tripped = config(&report.updated_yaml);
    assert_eq!(
        round_tripped
            .resources
            .get("web-conf")
            .and_then(|r| r.owner.as_deref()),
        Some(want.as_str()),
        "the emitted scalar did not parse back to the value the rule named"
    );
}

#[test]
fn two_rules_that_demand_different_values_are_both_refused() {
    let other = ASSERT_0644
        .replace("id: SEC-MODE", "id: SEC-MODE-B")
        .replace("condition_value: \"0644\"", "condition_value: \"0600\"");
    let source = base(&format!("{ASSERT_0644}{other}"));
    let report = run(&source);
    assert!(
        report.applied.is_empty(),
        "sort order picked a winner between two contradictory rules: {:?}",
        report.applied
    );
    assert_eq!(report.updated_yaml, source);
    assert_eq!(report.remaining.len(), 2);
    for v in &report.remaining {
        assert!(
            v.reason.contains("will not choose between them"),
            "the reason blames something other than the contradiction: {}",
            v.reason
        );
        assert!(
            v.reason.contains("`0600`") && v.reason.contains("`0644`"),
            "{}",
            v.reason
        );
    }
}
