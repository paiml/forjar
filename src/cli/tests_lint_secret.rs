//! #685: `forjar lint` must lint the apply script of a resource that carries a
//! `{{secrets.*}}` reference. `apply_script` refuses such a resource (I8), and
//! lint used to drop that refusal silently, so only check and state_query were
//! ever linted while I8 gates apply.

use super::lint_scripts;
use crate::core::parser::parse_config;

/// A real-looking OpenAI key, ASSEMBLED AT RUNTIME so the literal never sits in
/// the source (GitHub push protection matches the same shape).
fn fake_openai_key() -> String {
    format!("sk-proj-{}", "abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGH")
}

fn config_with(command: &str) -> crate::core::types::ForjarConfig {
    parse_config(&format!(
        r#"
version: "1.0"
name: secret-lint
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
fn secret_bearing_task_apply_script_is_linted() {
    let key = fake_openai_key();
    let cfg = config_with(&format!(
        "curl -H 'X-Token: {{{{secrets.deploy-token}}}}' https://x.invalid && KEY=\"{key}\""
    ));
    let warnings = lint_scripts(&cfg);
    assert!(
        warnings
            .iter()
            .any(|w| w.starts_with("bashrs: deploy/apply [")),
        "the apply script of a secret-bearing task must be linted: {warnings:?}"
    );
}

#[test]
fn secret_bearing_task_with_clean_apply_script_stays_clean() {
    let cfg = config_with("curl -H 'X-Token: {{secrets.deploy-token}}' https://x.invalid");
    let warnings = lint_scripts(&cfg);
    assert!(
        !warnings.iter().any(|w| w.contains("deploy/apply")),
        "a placeholder must not manufacture a finding: {warnings:?}"
    );
    assert!(
        !warnings.iter().any(|w| w.contains("not linted")),
        "{warnings:?}"
    );
}
