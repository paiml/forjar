//! FJ-2602/2603/2604: Test runner types — unified test CLI, sandbox config, artifacts.

use serde::{Deserialize, Serialize};
use std::fmt;

/// FJ-2602: Unified test command configuration (`forjar test <subcommand>`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCommand {
    /// Which test subcommand to run.
    pub subcommand: TestSubcommand,
    /// Config file path.
    pub config: String,
    /// Run tests in parallel.
    #[serde(default)]
    pub parallel: bool,
    /// JSON output mode.
    #[serde(default)]
    pub json: bool,
    /// Verbose output.
    #[serde(default)]
    pub verbose: bool,
}

/// Test subcommand variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TestSubcommand {
    /// Behavior-driven infrastructure specs.
    Behavior,
    /// Convergence property testing.
    Convergence,
    /// Infrastructure mutation testing.
    Mutation,
    /// Run all test types.
    All,
}

impl fmt::Display for TestSubcommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Behavior => write!(f, "behavior"),
            Self::Convergence => write!(f, "convergence"),
            Self::Mutation => write!(f, "mutation"),
            Self::All => write!(f, "all"),
        }
    }
}

/// FJ-2603: Sandbox configuration for isolated testing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Sandbox backend.
    #[serde(default)]
    pub backend: SandboxBackend,
    /// Whether to clean up sandbox after test.
    #[serde(default = "default_true_test")]
    pub cleanup: bool,
    /// Timeout for sandbox operations in seconds.
    #[serde(default = "default_sandbox_timeout")]
    pub timeout_secs: u32,
    /// Whether to capture overlay filesystem diff.
    #[serde(default)]
    pub capture_overlay: bool,
}

fn default_true_test() -> bool {
    true
}
fn default_sandbox_timeout() -> u32 {
    300
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            backend: SandboxBackend::Pepita,
            cleanup: true,
            timeout_secs: 300,
            capture_overlay: false,
        }
    }
}

/// Sandbox backend for isolated testing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SandboxBackend {
    /// Pepita (overlay filesystem) — preferred.
    #[default]
    Pepita,
    /// Container (Docker/Podman) — fallback.
    Container,
    /// Chroot — minimal isolation.
    Chroot,
}

impl fmt::Display for SandboxBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pepita => write!(f, "pepita"),
            Self::Container => write!(f, "container"),
            Self::Chroot => write!(f, "chroot"),
        }
    }
}

/// FJ-2603: Sandbox lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SandboxPhase {
    /// Sandbox created, not yet applied.
    Created,
    /// forjar apply completed in sandbox.
    Applied,
    /// Verification (behavior checks) completed.
    Verified,
    /// Sandbox destroyed.
    Destroyed,
}

/// FJ-2602: Test result for a single test run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    /// Test name.
    pub name: String,
    /// Test subcommand type.
    pub test_type: TestSubcommand,
    /// Whether the test passed.
    pub passed: bool,
    /// Duration in seconds.
    pub duration_secs: f64,
    /// Failure message (if any).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Artifacts produced.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<TestArtifact>,
}

impl fmt::Display for TestResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status = if self.passed { "PASS" } else { "FAIL" };
        write!(
            f,
            "[{status}] {} ({}, {:.2}s)",
            self.name, self.test_type, self.duration_secs,
        )?;
        if let Some(ref msg) = self.message {
            write!(f, " — {msg}")?;
        }
        Ok(())
    }
}

/// FJ-2602: Test artifact (log, overlay diff, report).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestArtifact {
    /// Artifact name.
    pub name: String,
    /// File path.
    pub path: String,
    /// MIME type (e.g., "text/plain", "application/json").
    #[serde(default)]
    pub content_type: Option<String>,
    /// Size in bytes.
    #[serde(default)]
    pub size_bytes: Option<u64>,
}

/// FJ-2602: Aggregated test suite report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSuiteReport {
    /// Total number of tests.
    pub total: u32,
    /// Number of passed tests.
    pub passed: u32,
    /// Number of failed tests.
    pub failed: u32,
    /// Number of skipped tests.
    pub skipped: u32,
    /// Total duration in seconds.
    pub duration_secs: f64,
    /// Per-test results.
    pub results: Vec<TestResult>,
}

impl TestSuiteReport {
    /// Pass rate as a percentage.
    pub fn pass_rate(&self) -> f64 {
        if self.total == 0 {
            return 100.0;
        }
        (self.passed as f64 / self.total as f64) * 100.0
    }

    /// Whether all tests passed.
    pub fn all_passed(&self) -> bool {
        self.failed == 0
    }

    /// Format a compact summary line.
    pub fn format_summary(&self) -> String {
        format!(
            "{} passed, {} failed, {} skipped ({:.1}s) — {:.0}%",
            self.passed,
            self.failed,
            self.skipped,
            self.duration_secs,
            self.pass_rate(),
        )
    }
}

/// FJ-2604: Coverage threshold for CI enforcement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageThreshold {
    /// Minimum line coverage percentage (e.g., 95.0).
    pub min_line_pct: f64,
    /// Minimum branch coverage percentage (e.g., 80.0).
    #[serde(default)]
    pub min_branch_pct: Option<f64>,
    /// Whether to fail CI on threshold violation.
    #[serde(default = "default_true_test")]
    pub enforce: bool,
}

impl CoverageThreshold {
    /// Check if coverage meets the threshold.
    pub fn check(&self, line_pct: f64, branch_pct: Option<f64>) -> bool {
        if line_pct < self.min_line_pct {
            return false;
        }
        if let (Some(min), Some(actual)) = (self.min_branch_pct, branch_pct) {
            if actual < min {
                return false;
            }
        }
        true
    }
}

/// FJ-2604: Coverage badge configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageBadge {
    /// Badge label (e.g., "coverage").
    #[serde(default = "default_badge_label")]
    pub label: String,
    /// Line coverage percentage.
    pub line_pct: f64,
    /// Badge color based on coverage.
    pub color: BadgeColor,
}

fn default_badge_label() -> String {
    "coverage".into()
}

impl CoverageBadge {
    /// Create a badge from coverage percentage.
    pub fn from_pct(pct: f64) -> Self {
        let color = BadgeColor::from_pct(pct);
        Self {
            label: "coverage".into(),
            line_pct: pct,
            color,
        }
    }
}

/// Badge color based on coverage percentage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BadgeColor {
    BrightGreen,
    Green,
    YellowGreen,
    Yellow,
    Orange,
    Red,
}

impl BadgeColor {
    /// Determine color from coverage percentage.
    pub fn from_pct(pct: f64) -> Self {
        if pct >= 95.0 {
            Self::BrightGreen
        } else if pct >= 90.0 {
            Self::Green
        } else if pct >= 80.0 {
            Self::YellowGreen
        } else if pct >= 70.0 {
            Self::Yellow
        } else if pct >= 60.0 {
            Self::Orange
        } else {
            Self::Red
        }
    }
}

impl fmt::Display for BadgeColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BrightGreen => write!(f, "brightgreen"),
            Self::Green => write!(f, "green"),
            Self::YellowGreen => write!(f, "yellowgreen"),
            Self::Yellow => write!(f, "yellow"),
            Self::Orange => write!(f, "orange"),
            Self::Red => write!(f, "red"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subcommand_display() {
        assert_eq!(TestSubcommand::Behavior.to_string(), "behavior");
        assert_eq!(TestSubcommand::Convergence.to_string(), "convergence");
        assert_eq!(TestSubcommand::Mutation.to_string(), "mutation");
        assert_eq!(TestSubcommand::All.to_string(), "all");
    }

    #[test]
    fn sandbox_config_default() {
        let c = SandboxConfig::default();
        assert_eq!(c.backend, SandboxBackend::Pepita);
        assert!(c.cleanup);
        assert_eq!(c.timeout_secs, 300);
    }

    #[test]
    fn sandbox_backend_display() {
        assert_eq!(SandboxBackend::Pepita.to_string(), "pepita");
        assert_eq!(SandboxBackend::Container.to_string(), "container");
        assert_eq!(SandboxBackend::Chroot.to_string(), "chroot");
    }

    #[test]
    fn test_result_display_pass() {
        let r = TestResult {
            name: "nginx installed".into(),
            test_type: TestSubcommand::Behavior,
            passed: true,
            duration_secs: 1.5,
            message: None,
            artifacts: vec![],
        };
        let s = r.to_string();
        assert!(s.contains("[PASS]"));
        assert!(s.contains("nginx installed"));
    }

    #[test]
    fn test_result_display_fail() {
        let r = TestResult {
            name: "convergence".into(),
            test_type: TestSubcommand::Convergence,
            passed: false,
            duration_secs: 5.0,
            message: Some("second apply not noop".into()),
            artifacts: vec![],
        };
        let s = r.to_string();
        assert!(s.contains("[FAIL]"));
        assert!(s.contains("second apply not noop"));
    }

    #[test]
    fn test_suite_report_pass_rate() {
        let r = TestSuiteReport {
            total: 10,
            passed: 8,
            failed: 1,
            skipped: 1,
            duration_secs: 30.0,
            results: vec![],
        };
        assert!((r.pass_rate() - 80.0).abs() < 0.01);
        assert!(!r.all_passed());
    }

    #[test]
    fn test_suite_report_all_passed() {
        let r = TestSuiteReport {
            total: 5,
            passed: 5,
            failed: 0,
            skipped: 0,
            duration_secs: 10.0,
            results: vec![],
        };
        assert!(r.all_passed());
        assert!((r.pass_rate() - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_suite_report_empty() {
        let r = TestSuiteReport {
            total: 0,
            passed: 0,
            failed: 0,
            skipped: 0,
            duration_secs: 0.0,
            results: vec![],
        };
        assert!((r.pass_rate() - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_suite_report_format() {
        let r = TestSuiteReport {
            total: 10,
            passed: 9,
            failed: 1,
            skipped: 0,
            duration_secs: 45.0,
            results: vec![],
        };
        let s = r.format_summary();
        assert!(s.contains("9 passed"));
        assert!(s.contains("1 failed"));
        assert!(s.contains("90%"));
    }

    #[test]
    fn coverage_threshold_check() {
        let t = CoverageThreshold {
            min_line_pct: 95.0,
            min_branch_pct: Some(80.0),
            enforce: true,
        };
        assert!(t.check(96.0, Some(85.0)));
        assert!(!t.check(94.0, Some(85.0)));
        assert!(!t.check(96.0, Some(75.0)));
        assert!(t.check(96.0, None)); // branch not reported
    }

    #[test]
    fn coverage_badge_from_pct() {
        let b = CoverageBadge::from_pct(97.0);
        assert_eq!(b.color, BadgeColor::BrightGreen);

        let b = CoverageBadge::from_pct(55.0);
        assert_eq!(b.color, BadgeColor::Red);
    }

    #[test]
    fn badge_color_from_pct_ranges() {
        assert_eq!(BadgeColor::from_pct(95.0), BadgeColor::BrightGreen);
        assert_eq!(BadgeColor::from_pct(92.0), BadgeColor::Green);
        assert_eq!(BadgeColor::from_pct(85.0), BadgeColor::YellowGreen);
        assert_eq!(BadgeColor::from_pct(75.0), BadgeColor::Yellow);
        assert_eq!(BadgeColor::from_pct(65.0), BadgeColor::Orange);
        assert_eq!(BadgeColor::from_pct(50.0), BadgeColor::Red);
    }

    #[test]
    fn badge_color_display() {
        assert_eq!(BadgeColor::BrightGreen.to_string(), "brightgreen");
        assert_eq!(BadgeColor::Red.to_string(), "red");
    }

    #[test]
    fn test_command_serde() {
        let cmd = TestCommand {
            subcommand: TestSubcommand::All,
            config: "forjar.yaml".into(),
            parallel: true,
            json: true,
            verbose: false,
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let parsed: TestCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.subcommand, TestSubcommand::All);
        assert!(parsed.parallel);
    }

    #[test]
    fn sandbox_config_serde() {
        let c = SandboxConfig {
            backend: SandboxBackend::Container,
            cleanup: false,
            timeout_secs: 600,
            capture_overlay: true,
        };
        let yaml = serde_yaml_ng::to_string(&c).unwrap();
        let parsed: SandboxConfig = serde_yaml_ng::from_str(&yaml).unwrap();
        assert_eq!(parsed.backend, SandboxBackend::Container);
        assert!(!parsed.cleanup);
    }
}
