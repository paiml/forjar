//! FJ-2602: Behavior-driven infrastructure spec types.
//!
//! Describes expected system state after convergence using `.spec.yaml` files
//! with verifiable assertions (commands, exit codes, content checks).

use serde::{Deserialize, Serialize};
use std::fmt;

/// FJ-2602: A behavior specification file.
///
/// # Examples
///
/// ```
/// use forjar::core::types::{BehaviorSpec, BehaviorEntry, VerifyCommand};
///
/// let spec = BehaviorSpec {
///     name: "nginx web server".into(),
///     config: "examples/nginx.yaml".into(),
///     machine: Some("web-1".into()),
///     behaviors: vec![
///         BehaviorEntry {
///             name: "nginx is installed".into(),
///             resource: Some("nginx-pkg".into()),
///             behavior_type: None,
///             assert_state: Some("present".into()),
///             verify: Some(VerifyCommand {
///                 command: "dpkg -l nginx | grep -q '^ii'".into(),
///                 exit_code: Some(0),
///                 ..Default::default()
///             }),
///             convergence: None,
///         },
///     ],
/// };
/// assert_eq!(spec.behaviors.len(), 1);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorSpec {
    /// Spec name (human-readable).
    pub name: String,
    /// Path to the forjar config file under test.
    pub config: String,
    /// Target machine (optional — defaults to all machines).
    #[serde(default)]
    pub machine: Option<String>,
    /// List of behavior assertions.
    pub behaviors: Vec<BehaviorEntry>,
}

impl BehaviorSpec {
    /// Count the number of behavior entries.
    pub fn behavior_count(&self) -> usize {
        self.behaviors.len()
    }

    /// Get all resource IDs referenced by this spec.
    pub fn referenced_resources(&self) -> Vec<&str> {
        self.behaviors
            .iter()
            .filter_map(|b| b.resource.as_deref())
            .collect()
    }
}

/// A single behavior assertion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorEntry {
    /// Behavior name (human-readable).
    pub name: String,
    /// Resource ID this behavior applies to.
    #[serde(default)]
    pub resource: Option<String>,
    /// Behavior type (omit for standard resource assertion, "convergence" for idempotency).
    #[serde(default, rename = "type")]
    pub behavior_type: Option<String>,
    /// Expected resource state (present, running, file, etc.).
    #[serde(default, rename = "state")]
    pub assert_state: Option<String>,
    /// Verify command assertion.
    #[serde(default)]
    pub verify: Option<VerifyCommand>,
    /// Convergence assertion (for type: convergence).
    #[serde(default)]
    pub convergence: Option<ConvergenceAssert>,
}

impl BehaviorEntry {
    /// Whether this is a convergence (idempotency) check.
    pub fn is_convergence(&self) -> bool {
        self.behavior_type.as_deref() == Some("convergence")
    }

    /// Whether this entry has a verify command.
    pub fn has_verify(&self) -> bool {
        self.verify.is_some()
    }
}

/// Command-based verification assertion.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VerifyCommand {
    /// Shell command to execute for verification.
    pub command: String,
    /// Expected exit code (default: 0).
    #[serde(default)]
    pub exit_code: Option<i32>,
    /// Expected stdout content (exact match).
    #[serde(default)]
    pub stdout: Option<String>,
    /// Expected substring in stderr.
    #[serde(default)]
    pub stderr_contains: Option<String>,
    /// Expected file existence check.
    #[serde(default)]
    pub file_exists: Option<String>,
    /// Expected file content (exact or BLAKE3 hash).
    #[serde(default)]
    pub file_content: Option<String>,
    /// Expected port to be open.
    #[serde(default)]
    pub port_open: Option<u16>,
    /// Retry count before declaring failure.
    #[serde(default)]
    pub retries: Option<u32>,
    /// Retry delay in seconds.
    #[serde(default)]
    pub retry_delay_secs: Option<u32>,
}

/// Convergence-specific assertion (second apply is no-op).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConvergenceAssert {
    /// Expected second apply result (typically "noop").
    #[serde(default)]
    pub second_apply: Option<String>,
    /// Whether state should be unchanged after second apply.
    #[serde(default)]
    pub state_unchanged: Option<bool>,
}

/// FJ-2602: Result of running a single behavior assertion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorResult {
    /// Behavior name.
    pub name: String,
    /// Whether the assertion passed.
    pub passed: bool,
    /// Failure message (if any).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure: Option<String>,
    /// Actual exit code (if verify command was run).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual_exit_code: Option<i32>,
    /// Actual stdout (if captured).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual_stdout: Option<String>,
    /// Duration in milliseconds.
    #[serde(default)]
    pub duration_ms: u64,
}

/// FJ-2602: Aggregate result of running a behavior spec.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorReport {
    /// Spec name.
    pub spec_name: String,
    /// Per-behavior results (soft assertions — all collected before reporting).
    pub results: Vec<BehaviorResult>,
    /// Total behaviors.
    pub total: usize,
    /// Passed count.
    pub passed: usize,
    /// Failed count.
    pub failed: usize,
}

impl BehaviorReport {
    /// Build a report from behavior results.
    pub fn from_results(spec_name: String, results: Vec<BehaviorResult>) -> Self {
        let total = results.len();
        let passed = results.iter().filter(|r| r.passed).count();
        let failed = total - passed;
        Self {
            spec_name,
            results,
            total,
            passed,
            failed,
        }
    }

    /// Whether all behaviors passed.
    pub fn all_passed(&self) -> bool {
        self.failed == 0
    }

    /// Format a human-readable summary.
    pub fn format_summary(&self) -> String {
        let mut out = format!(
            "Behavior Spec: {}\n{}\n",
            self.spec_name,
            "=".repeat(40 + self.spec_name.len())
        );

        for result in &self.results {
            let status = if result.passed { "PASS" } else { "FAIL" };
            out.push_str(&format!("  [{status}] {}", result.name));
            if let Some(ref failure) = result.failure {
                out.push_str(&format!(" — {failure}"));
            }
            out.push('\n');
        }

        out.push_str(&format!("\n{}/{} passed", self.passed, self.total));
        if self.failed > 0 {
            out.push_str(&format!(", {} FAILED", self.failed));
        }
        out.push('\n');
        out
    }
}

impl fmt::Display for BehaviorReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format_summary())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn behavior_spec_serde_roundtrip() {
        let yaml = r#"
name: nginx test
config: examples/nginx.yaml
machine: web-1
behaviors:
  - name: nginx installed
    resource: nginx-pkg
    state: present
    verify:
      command: "dpkg -l nginx"
      exit_code: 0
  - name: idempotency
    type: convergence
    convergence:
      second_apply: noop
      state_unchanged: true
"#;
        let spec: BehaviorSpec = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(spec.name, "nginx test");
        assert_eq!(spec.machine.as_deref(), Some("web-1"));
        assert_eq!(spec.behaviors.len(), 2);
        assert_eq!(spec.behaviors[0].assert_state.as_deref(), Some("present"));
        assert!(spec.behaviors[1].is_convergence());
    }

    #[test]
    fn behavior_spec_behavior_count() {
        let spec = BehaviorSpec {
            name: "test".into(),
            config: "c.yaml".into(),
            machine: None,
            behaviors: vec![
                BehaviorEntry {
                    name: "a".into(),
                    resource: Some("r1".into()),
                    behavior_type: None,
                    assert_state: None,
                    verify: None,
                    convergence: None,
                },
                BehaviorEntry {
                    name: "b".into(),
                    resource: Some("r2".into()),
                    behavior_type: None,
                    assert_state: None,
                    verify: None,
                    convergence: None,
                },
            ],
        };
        assert_eq!(spec.behavior_count(), 2);
    }

    #[test]
    fn behavior_spec_referenced_resources() {
        let spec = BehaviorSpec {
            name: "test".into(),
            config: "c.yaml".into(),
            machine: None,
            behaviors: vec![
                BehaviorEntry {
                    name: "a".into(),
                    resource: Some("pkg".into()),
                    behavior_type: None,
                    assert_state: None,
                    verify: None,
                    convergence: None,
                },
                BehaviorEntry {
                    name: "b".into(),
                    resource: None,
                    behavior_type: Some("convergence".into()),
                    assert_state: None,
                    verify: None,
                    convergence: None,
                },
                BehaviorEntry {
                    name: "c".into(),
                    resource: Some("svc".into()),
                    behavior_type: None,
                    assert_state: None,
                    verify: None,
                    convergence: None,
                },
            ],
        };
        let refs = spec.referenced_resources();
        assert_eq!(refs, vec!["pkg", "svc"]);
    }

    #[test]
    fn behavior_entry_is_convergence() {
        let entry = BehaviorEntry {
            name: "test".into(),
            resource: None,
            behavior_type: Some("convergence".into()),
            assert_state: None,
            verify: None,
            convergence: Some(ConvergenceAssert {
                second_apply: Some("noop".into()),
                state_unchanged: Some(true),
            }),
        };
        assert!(entry.is_convergence());
        assert!(!entry.has_verify());
    }

    #[test]
    fn behavior_entry_has_verify() {
        let entry = BehaviorEntry {
            name: "check".into(),
            resource: Some("pkg".into()),
            behavior_type: None,
            assert_state: Some("present".into()),
            verify: Some(VerifyCommand {
                command: "dpkg -l pkg".into(),
                exit_code: Some(0),
                ..Default::default()
            }),
            convergence: None,
        };
        assert!(entry.has_verify());
        assert!(!entry.is_convergence());
    }

    #[test]
    fn verify_command_defaults() {
        let vc = VerifyCommand::default();
        assert!(vc.command.is_empty());
        assert!(vc.exit_code.is_none());
        assert!(vc.stdout.is_none());
        assert!(vc.stderr_contains.is_none());
        assert!(vc.port_open.is_none());
    }

    #[test]
    fn verify_command_all_fields() {
        let yaml = r#"
command: "curl -sf http://localhost:8080"
exit_code: 0
stdout: "ok"
stderr_contains: "warn"
file_exists: "/tmp/marker"
file_content: "blake3:abc"
port_open: 8080
retries: 3
retry_delay_secs: 5
"#;
        let vc: VerifyCommand = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(vc.exit_code, Some(0));
        assert_eq!(vc.stdout.as_deref(), Some("ok"));
        assert_eq!(vc.port_open, Some(8080));
        assert_eq!(vc.retries, Some(3));
    }

    #[test]
    fn behavior_report_all_passed() {
        let results = vec![
            BehaviorResult {
                name: "a".into(),
                passed: true,
                failure: None,
                actual_exit_code: Some(0),
                actual_stdout: None,
                duration_ms: 50,
            },
            BehaviorResult {
                name: "b".into(),
                passed: true,
                failure: None,
                actual_exit_code: Some(0),
                actual_stdout: None,
                duration_ms: 30,
            },
        ];
        let report = BehaviorReport::from_results("nginx".into(), results);
        assert!(report.all_passed());
        assert_eq!(report.total, 2);
        assert_eq!(report.passed, 2);
        assert_eq!(report.failed, 0);
    }

    #[test]
    fn behavior_report_with_failures() {
        let results = vec![
            BehaviorResult {
                name: "installed".into(),
                passed: true,
                failure: None,
                actual_exit_code: Some(0),
                actual_stdout: None,
                duration_ms: 50,
            },
            BehaviorResult {
                name: "running".into(),
                passed: false,
                failure: Some("exit code 1, expected 0".into()),
                actual_exit_code: Some(1),
                actual_stdout: Some("inactive".into()),
                duration_ms: 100,
            },
        ];
        let report = BehaviorReport::from_results("nginx".into(), results);
        assert!(!report.all_passed());
        assert_eq!(report.passed, 1);
        assert_eq!(report.failed, 1);
    }

    #[test]
    fn behavior_report_format_summary() {
        let results = vec![
            BehaviorResult {
                name: "pkg installed".into(),
                passed: true,
                failure: None,
                actual_exit_code: None,
                actual_stdout: None,
                duration_ms: 10,
            },
            BehaviorResult {
                name: "svc running".into(),
                passed: false,
                failure: Some("not active".into()),
                actual_exit_code: None,
                actual_stdout: None,
                duration_ms: 20,
            },
        ];
        let report = BehaviorReport::from_results("test".into(), results);
        let summary = report.format_summary();
        assert!(summary.contains("[PASS] pkg installed"));
        assert!(summary.contains("[FAIL] svc running"));
        assert!(summary.contains("not active"));
        assert!(summary.contains("1/2 passed"));
        assert!(summary.contains("1 FAILED"));
    }

    #[test]
    fn behavior_report_display() {
        let report = BehaviorReport::from_results("s".into(), vec![]);
        let display = format!("{report}");
        assert!(display.contains("Behavior Spec: s"));
        assert!(display.contains("0/0 passed"));
    }

    #[test]
    fn convergence_assert_defaults() {
        let ca = ConvergenceAssert::default();
        assert!(ca.second_apply.is_none());
        assert!(ca.state_unchanged.is_none());
    }

    #[test]
    fn behavior_report_empty() {
        let report = BehaviorReport::from_results("empty".into(), vec![]);
        assert!(report.all_passed());
        assert_eq!(report.total, 0);
    }
}
