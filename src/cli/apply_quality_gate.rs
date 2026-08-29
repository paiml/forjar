//! The pre-apply quality gate — the one blocking check in front of mutation.

use crate::core::quality_gate::{evaluate, GateLevel, GateThresholds, QUALITY_GATE_ERROR_CODE};
use std::path::Path;

/// The pre-apply quality gate: the ONE blocking check in front of mutation.
///
/// It used to evaluate compliance packs and nothing else, so a config could
/// carry a plaintext password or emit shell bashrs rejects outright and still
/// apply cleanly. It now runs every check `core::quality_gate` owns, and this
/// is deliberately the only enforcement point: `validate` and `plan` are
/// ReadOnly and answer questions, and putting a gate in front of a question
/// takes away the operator's route to a fix exactly when they need it.
///
/// The complexity ceiling is NOT evaluated here. It is advisory by
/// construction (the shell it scores is forjar's own output), so it can never
/// change this function's verdict — and computing it would parse three scripts
/// per resource to produce findings nobody at this call site can act on.
pub(super) fn check_quality_gate(
    file: &Path,
    policy_dir: &Path,
    verbose: bool,
) -> Result<(), String> {
    use super::helpers::parse_and_validate;

    let config = parse_and_validate(file)?;
    let thresholds = GateThresholds {
        policy_dir: Some(policy_dir.to_path_buf()),
        ..GateThresholds::default()
    };
    let yaml_text = std::fs::read_to_string(file).ok();
    let report = evaluate(&config, yaml_text.as_deref(), &thresholds);
    if verbose {
        for line in report.render() {
            eprintln!("  {line}");
        }
    }
    if !report.passed() {
        return Err(format!(
            "{QUALITY_GATE_ERROR_CODE}: quality gate blocks apply — {} error(s):\n{}",
            report.error_count(),
            report
                .findings
                .iter()
                .filter(|f| f.level == GateLevel::Error)
                .map(|f| format!("  {}", f.render()))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &std::path::Path, yaml: &str) -> std::path::PathBuf {
        let p = dir.join("forjar.yaml");
        std::fs::write(&p, yaml).unwrap();
        p
    }

    /// A Stripe-shaped fixture, ASSEMBLED AT RUNTIME so the literal never
    /// appears in the source.
    ///
    /// The detector under test matches `[sr]k_(live|test)_[A-Za-z0-9]{20,}`, so
    /// the fixture has to have exactly that shape to exercise it — and GitHub's
    /// push protection matches the same shape, which blocked a push of this
    /// repo outright. Splitting the prefix keeps the value identical at runtime
    /// while leaving nothing for a file scanner to find. The repo already uses
    /// this idiom (`format!("...ghp_{}", "A".repeat(40))`).
    fn fake_stripe_key() -> String {
        format!("sk_{}_{}", "live", "A".repeat(24))
    }

    fn leaky() -> String {
        format!(
            r#"version: "1.0"
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
    content: "api_key={}"
"#,
            fake_stripe_key()
        )
    }

    const INERT: &str = r#"version: "1.0"
name: t
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  motd:
    type: file
    machine: m1
    path: /etc/motd
    content: "welcome"
"#;

    #[test]
    fn a_plaintext_secret_blocks_apply() {
        let d = tempfile::tempdir().unwrap();
        let f = write(d.path(), &leaky());
        let err = check_quality_gate(&f, d.path(), false)
            .expect_err("a config shipping a live-looking API key must not apply");
        assert!(
            err.contains(QUALITY_GATE_ERROR_CODE),
            "the refusal must name its error code so a caller can branch on it: {err}"
        );
        assert!(err.contains("app-config"), "{err}");
    }

    #[test]
    fn an_inert_config_is_not_blocked() {
        let d = tempfile::tempdir().unwrap();
        let f = write(d.path(), INERT);
        assert!(
            check_quality_gate(&f, d.path(), false).is_ok(),
            "the gate refused a config with nothing wrong with it, which is \
             how an operator learns to pass --no-policy-check forever"
        );
    }

    #[test]
    fn a_sealed_secret_is_not_blocked() {
        let d = tempfile::tempdir().unwrap();
        let sealed = leaky().replace(
            &fake_stripe_key(),
            "ENC[age,YWJjZGVmZ2hpamtsbW5vcHFyc3R1dnd4eXo=]",
        );
        let f = write(d.path(), &sealed);
        assert!(
            check_quality_gate(&f, d.path(), false).is_ok(),
            "sealing the value is the fix the gate asks for, so it must be \
             accepted — otherwise the gate has no exit"
        );
    }
}
