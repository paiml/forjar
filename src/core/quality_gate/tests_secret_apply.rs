//! #685: the quality gate analyses the apply script of a resource that carries
//! a `{{secrets.*}}` reference, with the reference masked by a placeholder.

use super::*;
use crate::core::parser::parse_config;

fn cfg(command: &str) -> crate::core::types::ForjarConfig {
    parse_config(&format!(
        r#"
version: "1.0"
name: t
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
resources:
  deploy:
    type: task
    machine: m1
    command: |
      {command}
"#
    ))
    .expect("fixture must parse")
}

#[test]
fn generate_scripts_includes_apply_of_a_secret_bearing_resource() {
    let config = cfg("curl -H 'X-Token: {{secrets.deploy-token}}' https://x.invalid");
    let scripts = checks::generate_scripts(&config);
    let apply = scripts
        .iter()
        .find(|s| s.resource_id == "deploy" && s.kind == "apply")
        .expect("apply script must be generated for linting (#685)");
    assert!(
        !apply.text.contains("{{secrets."),
        "the reference must be masked"
    );
    assert!(apply
        .text
        .contains(crate::core::resolver::LINT_SECRET_PLACEHOLDER));
}

#[test]
fn gate_reports_a_lint_error_in_a_secret_bearing_apply_script() {
    let key = format!("sk-proj-{}", "abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGH");
    let config = cfg(&format!(
        "curl -H 'X-Token: {{{{secrets.deploy-token}}}}' https://x.invalid && KEY=\"{key}\""
    ));
    let report = evaluate(&config, None, &GateThresholds::default());
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.rule_id.starts_with("FJQ-SH-") && f.script_kind == Some("apply")),
        "gate must lint the apply script: {:?}",
        report.findings
    );
}

#[test]
fn masking_leaves_a_resource_without_a_secret_untouched() {
    let config = cfg("echo hi");
    let r = &config.resources["deploy"];
    let script = crate::core::codegen::apply_script_for_lint(r).unwrap();
    assert_eq!(script, crate::core::codegen::apply_script(r).unwrap());
}
