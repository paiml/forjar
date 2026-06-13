//! Tests for FJ-3505 promotion gate evaluation (split from promotion.rs to
//! keep that file under the 500-line health limit). Declared as a `#[path]`
//! child `mod tests` of `promotion` so it sees the private gate helpers.
use super::*;
use crate::core::types::environment::*;
use tempfile::TempDir;

fn write_valid_config(dir: &Path) -> std::path::PathBuf {
    let path = dir.join("forjar.yaml");
    std::fs::write(
        &path,
        r#"
version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 127.0.0.1
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
"#,
    )
    .unwrap();
    path
}

#[test]
fn validate_gate_passes() {
    let dir = TempDir::new().unwrap();
    let cfg = write_valid_config(dir.path());
    let result = evaluate_validate_gate(&cfg, false);
    assert!(result.passed, "gate failed: {}", result.message);
    assert_eq!(result.gate_type, "validate");
}

#[test]
fn validate_gate_fails() {
    let dir = TempDir::new().unwrap();
    let cfg = dir.path().join("forjar.yaml");
    std::fs::write(&cfg, "invalid: yaml: [").unwrap();
    let result = evaluate_validate_gate(&cfg, false);
    assert!(!result.passed);
}

#[test]
fn policy_gate_no_policies() {
    let dir = TempDir::new().unwrap();
    let cfg = write_valid_config(dir.path());
    let result = evaluate_policy_gate(&cfg);
    assert!(result.passed);
}

#[test]
fn script_gate_passes() {
    let result = evaluate_script_gate("true");
    assert!(result.passed);
    assert_eq!(result.gate_type, "script");
}

#[test]
fn script_gate_fails() {
    let result = evaluate_script_gate("false");
    assert!(!result.passed);
}

#[test]
fn coverage_gate_advisory_no_tool() {
    fn no_tool() -> CovProbe {
        CovProbe::Unavailable
    }
    let result = evaluate_coverage_gate_inner(95, no_tool);
    assert!(result.passed);
    assert!(result.message.contains("95%"));
    assert!(result.message.contains("advisory"));
}

#[test]
fn coverage_gate_passes_threshold() {
    fn mock_output() -> CovProbe {
        CovProbe::Ran("  TOTAL                          1234   1111   96.5%\n".into())
    }
    let result = evaluate_coverage_gate_inner(95, mock_output);
    assert!(result.passed);
    assert!(result.message.contains("96.5%"));
}

#[test]
fn coverage_gate_fails_threshold() {
    fn mock_output() -> CovProbe {
        CovProbe::Ran("  TOTAL                          1234   900   72.9%\n".into())
    }
    let result = evaluate_coverage_gate_inner(95, mock_output);
    assert!(!result.passed);
    assert!(result.message.contains("72.9%"));
    assert!(result.message.contains("95%"));
}

// Bug-hunt #5 (Refs #154): a present tool that exits non-zero (broken
// build / failing tests) must FAIL the gate, not vacuously pass it.
#[test]
fn coverage_gate_fails_on_tool_nonzero_exit() {
    fn tool_failed() -> CovProbe {
        CovProbe::Failed { code: Some(101) }
    }
    let result = evaluate_coverage_gate_inner(95, tool_failed);
    assert!(!result.passed, "broken build must not pass the gate");
    assert!(result.message.contains("failed"));
    assert!(result.message.contains("101"));
}

#[test]
fn coverage_gate_fails_on_tool_signal() {
    fn tool_signal() -> CovProbe {
        CovProbe::Failed { code: None }
    }
    let result = evaluate_coverage_gate_inner(95, tool_signal);
    assert!(!result.passed);
    assert!(result.message.contains("signal"));
}

// The two None-causes are distinguished: spawn failure ⇒ Unavailable
// (advisory), present-tool non-zero exit ⇒ Failed (gate failure).
// Hermetic: no subprocess is spawned — the spawn-vs-exit decision lives in
// the pure `classify_exit`, and the spawn-error arm is exercised with a
// synthetic `io::Error` (no loopback shell, safe under proxied CI).
#[test]
fn classify_llvm_cov_spawn_error_is_unavailable() {
    let err = Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "no such file",
    ));
    assert_eq!(classify_llvm_cov(err), CovProbe::Unavailable);
}

#[test]
fn classify_exit_nonzero_is_failed() {
    // Present tool exited non-zero (e.g. build/test failure).
    assert_eq!(
        classify_exit(false, Some(101), String::new()),
        CovProbe::Failed { code: Some(101) }
    );
}

#[test]
fn classify_exit_signal_is_failed_no_code() {
    // Terminated by signal: success=false, code=None.
    assert_eq!(
        classify_exit(false, None, String::new()),
        CovProbe::Failed { code: None }
    );
}

#[test]
fn classify_exit_success_is_ran() {
    match classify_exit(true, Some(0), "TOTAL 99.0%".into()) {
        CovProbe::Ran(s) => assert!(s.contains("TOTAL")),
        other => panic!("expected Ran, got {other:?}"),
    }
}

// Tool ran cleanly but produced no parseable TOTAL ⇒ advisory pass.
#[test]
fn coverage_gate_advisory_when_ran_but_unparseable() {
    fn ran_no_total() -> CovProbe {
        CovProbe::Ran("no coverage data here\n".into())
    }
    let result = evaluate_coverage_gate_inner(95, ran_no_total);
    assert!(result.passed);
    assert!(result.message.contains("advisory"));
}

#[test]
fn parse_coverage_from_llvm_output() {
    let output = r#"
Filename                      Regions    Missed Regions     Cover   Functions  Missed Functions  Executed       Lines      Missed Lines     Cover    Branches   Missed Branches     Cover
---
  TOTAL                          5432           432    92.0%        1234            56    95.5%       18234          912    95.0%         0             0         -
"#;
    let pct = parse_coverage_from_output(output);
    assert!(pct.is_some());
    // Last percentage on TOTAL line
}

#[test]
fn parse_coverage_no_total_line() {
    let pct = parse_coverage_from_output("no total here\n");
    assert!(pct.is_none());
}

#[test]
fn evaluate_all_gates() {
    let dir = TempDir::new().unwrap();
    let cfg = write_valid_config(dir.path());
    let promotion = PromotionConfig {
        from: "dev".into(),
        gates: vec![
            PromotionGate {
                validate: Some(ValidateGateOptions {
                    deep: false,
                    exhaustive: false,
                }),
                ..Default::default()
            },
            PromotionGate {
                script: Some("true".into()),
                ..Default::default()
            },
        ],
        auto_approve: false,
        rollout: None,
    };

    let result = evaluate_gates(&cfg, "staging", &promotion);
    assert_eq!(result.from, "dev");
    assert_eq!(result.to, "staging");
    assert_eq!(result.gates.len(), 2);
    assert!(result.all_passed);
    assert_eq!(result.passed_count(), 2);
    assert_eq!(result.failed_count(), 0);
}

#[test]
fn evaluate_gates_with_failure() {
    let dir = TempDir::new().unwrap();
    let cfg = write_valid_config(dir.path());
    let promotion = PromotionConfig {
        from: "dev".into(),
        gates: vec![
            PromotionGate {
                validate: Some(ValidateGateOptions {
                    deep: false,
                    exhaustive: false,
                }),
                ..Default::default()
            },
            PromotionGate {
                script: Some("false".into()),
                ..Default::default()
            },
        ],
        auto_approve: true,
        rollout: None,
    };

    let result = evaluate_gates(&cfg, "prod", &promotion);
    assert!(!result.all_passed);
    assert_eq!(result.failed_count(), 1);
    assert!(result.auto_approve);
}

#[test]
fn unknown_gate_type() {
    let dir = TempDir::new().unwrap();
    let cfg = write_valid_config(dir.path());
    let result = evaluate_single_gate(&cfg, &PromotionGate::default());
    assert!(!result.passed);
    assert_eq!(result.gate_type, "unknown");
}
