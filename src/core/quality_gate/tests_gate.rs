//! Unit tests for the gate's parts, in isolation from any transport.

use super::locate::resource_line;
use super::*;
use crate::core::parser::parse_config;

/// A Stripe-shaped fixture, ASSEMBLED AT RUNTIME so the literal never appears
/// in the source. The detector under test matches
/// `[sr]k_(live|test)_[A-Za-z0-9]{20,}`, so the fixture must have that exact
/// shape — and GitHub push protection matches the same shape, which blocked a
/// push of this repo outright.
fn fake_stripe_key() -> String {
    format!("sk_{}_{}", "live", "A".repeat(24))
}

fn cfg(yaml: &str) -> crate::core::types::ForjarConfig {
    parse_config(yaml).expect("fixture must parse")
}

const SEALED: &str = "ENC[age,YWJjZGVmZ2hpamtsbW5vcHFyc3R1dnd4eXo=]";

fn secret_config(value: &str) -> String {
    format!(
        r#"
version: "1.0"
name: t
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  app-config:
    type: file
    machine: m1
    path: /etc/app.conf
    content: "api_key={value}"
"#
    )
}

// ── GateReport::passed at the level boundary ────────────────────────

#[test]
fn only_errors_block() {
    let mut report = GateReport {
        findings: vec![GateFinding::new("X", GateLevel::Warning, "r", "m")],
        scripts_analysed: 0,
        resources_checked: 1,
    };
    assert!(report.passed(), "a Warning must not block");
    report
        .findings
        .push(GateFinding::new("Y", GateLevel::Note, "r", "m"));
    assert!(report.passed(), "a Note must not block");
    report
        .findings
        .push(GateFinding::new("Z", GateLevel::Error, "r", "m"));
    assert!(!report.passed(), "an Error must block");
    assert_eq!(report.error_count(), 1);
    assert_eq!(report.advisory_count(), 2);
}

// ── locate.rs ───────────────────────────────────────────────────────

#[test]
fn locate_finds_a_resource_key() {
    let yaml = "version: \"1.0\"\nname: t\nresources:\n  first:\n    type: file\n  second:\n    type: file\n";
    assert_eq!(resource_line(yaml, "first"), Some(4));
    assert_eq!(resource_line(yaml, "second"), Some(6));
}

#[test]
fn locate_answers_none_for_an_absent_resource() {
    let yaml = "resources:\n  first:\n    type: file\n";
    assert_eq!(
        resource_line(yaml, "elsewhere"),
        None,
        "a resource that arrived via `includes:` is not in this file, and \
         inventing a line for it points a reviewer at the wrong place"
    );
}

#[test]
fn locate_ignores_a_resources_key_inside_content() {
    // `resources:` at column 0 is the only one that counts; the nested one is
    // file DATA being written to the target host.
    let yaml = "resources:\n  writer:\n    type: file\n    content: |\n      resources:\n        fake:\n          type: file\n";
    assert_eq!(resource_line(yaml, "writer"), Some(2));
    assert_eq!(
        resource_line(yaml, "fake"),
        None,
        "a key inside a block scalar is not a resource declaration"
    );
}

#[test]
fn locate_stops_at_the_end_of_the_resources_block() {
    let yaml = "resources:\n  a:\n    type: file\npolicies:\n  - type: require\n    field: owner\n";
    assert_eq!(resource_line(yaml, "owner"), None);
}

#[test]
fn locate_answers_none_when_there_is_no_resources_key() {
    assert_eq!(resource_line("version: \"1.0\"\n", "a"), None);
}

// ── check 2: the discrimination that makes the verdict honest ───────

#[test]
fn a_plaintext_secret_in_content_is_an_error() {
    let text = secret_config(&fake_stripe_key());
    let report = evaluate(&cfg(&text), Some(&text), &GateThresholds::default());
    let sec: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.rule_id == "FJQ-SEC-002")
        .collect();
    assert_eq!(sec.len(), 1, "findings: {:?}", report.findings);
    assert_eq!(sec[0].level, GateLevel::Error);
    assert_eq!(sec[0].resource_id, "app-config");
    assert!(!report.passed());
}

/// A task whose command matches `sshpass_inline` WHATEVER the password is.
///
/// The pattern fires on `sshpass -p <anything>`, so a sealed value here is
/// suppressed by the ENC discrimination and by nothing else. A fixture the
/// pattern cannot match either way would pass this test vacuously.
fn sshpass_config(value: &str) -> String {
    format!(
        r#"
version: "1.0"
name: t
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  deploy:
    type: task
    machine: m1
    phony: true
    command: "sshpass -p {value} ssh deploy@host"
"#
    )
}

#[test]
fn the_same_field_fails_plaintext_and_passes_sealed() {
    let plain = sshpass_config("hunter2");
    let sealed = sshpass_config(SEALED);

    let bad = evaluate(&cfg(&plain), Some(&plain), &GateThresholds::default());
    assert!(
        bad.findings.iter().any(|f| f.rule_id == "FJQ-SEC-002"),
        "a plaintext sshpass password was not reported: {:?}",
        bad.findings
    );
    assert!(bad.findings.iter().any(|f| f.rule_id == "FJQ-SEC-001"));

    let good = evaluate(&cfg(&sealed), Some(&sealed), &GateThresholds::default());
    assert!(
        good.passed(),
        "an ENC[age,…] value is ciphertext, not a plaintext secret — and the \
         `sshpass -p <anything>` pattern matches both, so only the ENC \
         discrimination can tell them apart: {:?}",
        good.findings
    );
}

#[test]
fn a_sealed_secret_in_content_is_not_a_finding() {
    let text = secret_config(SEALED);
    let report = evaluate(&cfg(&text), Some(&text), &GateThresholds::default());
    assert!(
        !report.findings.iter().any(|f| f.rule_id == "FJQ-SEC-002"),
        "an ENC[age,…] value is ciphertext, not a plaintext secret: {:?}",
        report.findings
    );
}

#[test]
fn prose_about_enc_markers_does_not_seal_a_real_secret() {
    // `has_encrypted_markers` requires >= 20 chars of decodable base64, so a
    // comment mentioning the marker cannot be used to smuggle a secret past
    // the check on the same line.
    let text = secret_config(&format!("{} # use ENC[age,...] here", fake_stripe_key()));
    let report = evaluate(&cfg(&text), Some(&text), &GateThresholds::default());
    assert!(
        report.findings.iter().any(|f| f.rule_id == "FJQ-SEC-002"),
        "prose suppressed a real secret: {:?}",
        report.findings
    );
}

#[test]
fn a_finding_carries_the_line_its_resource_is_declared_on() {
    let text = secret_config(&fake_stripe_key());
    let expected = text
        .lines()
        .position(|l| l.trim_start().starts_with("app-config:"))
        .map(|i| i + 1);
    let report = evaluate(&cfg(&text), Some(&text), &GateThresholds::default());
    let f = report
        .findings
        .iter()
        .find(|f| f.rule_id == "FJQ-SEC-002")
        .unwrap();
    assert_eq!(f.yaml_line, expected);
}

#[test]
fn without_yaml_text_a_finding_carries_no_line() {
    let text = secret_config(&fake_stripe_key());
    let report = evaluate(&cfg(&text), None, &GateThresholds::default());
    let f = report
        .findings
        .iter()
        .find(|f| f.rule_id == "FJQ-SEC-002")
        .unwrap();
    assert_eq!(f.yaml_line, None);
}

// ── check 1: the ceiling is read, not decorative ────────────────────

const BRANCHY: &str = r#"
version: "1.0"
name: t
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  branchy:
    type: task
    machine: m1
    phony: true
    command: |
      for f in a b c d e f g h; do
        case "$f" in
          a) echo a ;;
          b) echo b ;;
          c) echo c ;;
          d) echo d ;;
          e) echo e ;;
          f) echo f ;;
          g) echo g ;;
          h) echo h ;;
          i) echo i ;;
          j) echo j ;;
          k) echo k ;;
          l) echo l ;;
          m) echo m ;;
          n) echo n ;;
          o) echo o ;;
          p) echo p ;;
          q) echo q ;;
          r) echo r ;;
          s) echo s ;;
          t) echo t ;;
          u) echo u ;;
          v) echo v ;;
          w) echo w ;;
          x) echo x ;;
          *) echo other ;;
        esac
      done
"#;

#[test]
fn a_ceiling_of_one_flags_a_branchy_script() {
    let t = GateThresholds {
        max_cyclomatic: Some(1),
        ..Default::default()
    };
    let report = evaluate(&cfg(BRANCHY), None, &t);
    assert!(
        report.findings.iter().any(|f| f.rule_id == "FJQ-CPX-001"),
        "a for/if/case chain did not exceed a ceiling of 1: {:?}",
        report.findings
    );
}

#[test]
fn a_none_ceiling_disables_the_check_entirely() {
    let t = GateThresholds {
        max_cyclomatic: None,
        ..Default::default()
    };
    let report = evaluate(&cfg(BRANCHY), None, &t);
    assert!(
        !report.findings.iter().any(|f| f.rule_id == "FJQ-CPX-001"),
        "None must SKIP the check, not fall back to some default ceiling. The \
         fixture's apply script scores ~28, so any silent default under that \
         would show up here: {:?}",
        report.findings
    );
}

#[test]
fn a_ceiling_above_the_score_is_silent() {
    let t = GateThresholds {
        max_cyclomatic: Some(1000),
        ..Default::default()
    };
    let report = evaluate(&cfg(BRANCHY), None, &t);
    assert!(
        !report.findings.iter().any(|f| f.rule_id == "FJQ-CPX-001"),
        "a ceiling of 1000 fired, so the comparison is not against the number \
         that was passed: {:?}",
        report.findings
    );
}

#[test]
fn complexity_is_advisory_until_opted_in() {
    let advisory = GateThresholds {
        max_cyclomatic: Some(1),
        ..Default::default()
    };
    let report = evaluate(&cfg(BRANCHY), None, &advisory);
    let f = report
        .findings
        .iter()
        .find(|f| f.rule_id == "FJQ-CPX-001")
        .unwrap();
    assert_eq!(
        f.level,
        GateLevel::Warning,
        "the shell scored here is EMITTED by forjar, so blocking an operator's \
         apply for its complexity punishes the wrong party"
    );

    let blocking = GateThresholds {
        max_cyclomatic: Some(1),
        complexity_is_error: true,
        ..Default::default()
    };
    let report = evaluate(&cfg(BRANCHY), None, &blocking);
    let f = report
        .findings
        .iter()
        .find(|f| f.rule_id == "FJQ-CPX-001")
        .unwrap();
    assert_eq!(f.level, GateLevel::Error);
}

// ── check 4: in-config policies reach the gate ──────────────────────

#[test]
fn an_in_config_policy_violation_becomes_a_finding() {
    let yaml = r#"
version: "1.0"
name: t
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  cfg:
    type: file
    machine: m1
    path: /etc/app.conf
policies:
  - type: require
    message: "files must have owner"
    id: "SEC-001"
    field: owner
    remediation: "Add owner field"
"#;
    let report = evaluate(&cfg(yaml), Some(yaml), &GateThresholds::default());
    let f = report
        .findings
        .iter()
        .find(|f| f.rule_id == "SEC-001")
        .expect("policy violation missing from the gate");
    assert_eq!(f.level, GateLevel::Error);
    assert_eq!(f.remediation.as_deref(), Some("Add owner field"));
    assert!(!report.passed());
}

// ── the SARIF projection ────────────────────────────────────────────

#[test]
fn sarif_carries_a_region_only_when_the_line_is_known() {
    let findings = vec![
        GateFinding {
            yaml_line: Some(7),
            ..GateFinding::new("A", GateLevel::Error, "r1", "m1")
        },
        GateFinding::new("B", GateLevel::Warning, "r2", "m2"),
    ];
    let sarif = sarif::findings_to_sarif(&findings, "some/forjar.yaml");
    let results = sarif["runs"][0]["results"].as_array().unwrap();
    assert_eq!(results.len(), 2);
    let loc = &results[0]["locations"][0]["physicalLocation"];
    assert_eq!(loc["artifactLocation"]["uri"], "some/forjar.yaml");
    assert_eq!(loc["region"]["startLine"], 7);
    assert!(
        results[1]["locations"][0]["physicalLocation"]["region"].is_null(),
        "a finding with no known line must carry no region at all"
    );
    assert_eq!(results[0]["level"], "error");
    assert_eq!(results[1]["level"], "warning");
    assert_eq!(results[0]["message"]["text"], "r1: m1");
}

#[test]
fn sarif_rules_are_deduped() {
    let findings = vec![
        GateFinding::new("SAME", GateLevel::Error, "a", "first"),
        GateFinding::new("SAME", GateLevel::Error, "b", "second"),
    ];
    let sarif = sarif::findings_to_sarif(&findings, "forjar.yaml");
    let rules = sarif["runs"][0]["tool"]["driver"]["rules"]
        .as_array()
        .unwrap();
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0]["shortDescription"]["text"], "first");
    assert_eq!(sarif["runs"][0]["results"].as_array().unwrap().len(), 2);
}

// ── rendering is shared, so two surfaces cannot disagree ────────────

#[test]
fn render_lists_errors_and_tallies_the_rest() {
    let report = GateReport {
        findings: vec![
            GateFinding::new("FJQ-SEC-002", GateLevel::Error, "app", "leak"),
            GateFinding::new("FJQ-SH-SC2086", GateLevel::Warning, "app", "quote it")
                .in_script("apply", 3),
        ],
        scripts_analysed: 3,
        resources_checked: 1,
    };
    let lines = report.render();
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert_eq!(lines[0], "FJQ-SEC-002 app: leak");
    assert!(lines[1].contains("1 error(s), 1 advisory"));
}

#[test]
fn render_is_empty_for_a_clean_report() {
    let report = GateReport {
        findings: vec![],
        scripts_analysed: 0,
        resources_checked: 0,
    };
    assert!(report.render().is_empty());
}

// ── the gate judges the text the EXECUTOR will judge ────────────────

#[test]
fn a_file_payload_is_not_linted_as_shell() {
    // `content:` becomes a heredoc payload (or a base64 blob) in the generated
    // script. bashrs reads it as shell if nobody strips it, so a gate that
    // lints the RAW script refuses an apply the transport would run without
    // comment. This content is deliberately shell-shaped nonsense.
    let yaml = r#"
version: "1.0"
name: t
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  payload:
    type: file
    machine: m1
    path: /etc/payload.conf
    content: |
      for x in a; do
      then ]] fi esac
      done
"#;
    let report = evaluate(&cfg(yaml), Some(yaml), &GateThresholds::default());
    assert!(
        report.passed(),
        "file DATA was linted as shell, so the gate refuses what the executor \
         accepts: {:?}",
        report.findings
    );
}
