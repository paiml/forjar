//! FJ-2301: Doctor diagnostics types — system health checks.
//!
//! Types for `forjar doctor` command output: system prerequisites,
//! machine connectivity, tool versions, and issue reporting.

use serde::{Deserialize, Serialize};

/// FJ-2301: Complete doctor report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorReport {
    /// System-level information.
    pub system: SystemInfo,
    /// Per-machine health checks.
    pub machines: Vec<MachineHealth>,
    /// Tool availability checks.
    pub tools: Vec<ToolCheck>,
    /// Detected issues and warnings.
    pub issues: Vec<DoctorIssue>,
}

impl DoctorReport {
    /// Whether the system is healthy (no errors).
    pub fn is_healthy(&self) -> bool {
        self.issues
            .iter()
            .all(|i| !matches!(i.severity, IssueSeverity::Error))
    }

    /// Count of issues by severity.
    pub fn issue_counts(&self) -> (usize, usize, usize) {
        let errors = self
            .issues
            .iter()
            .filter(|i| matches!(i.severity, IssueSeverity::Error))
            .count();
        let warnings = self
            .issues
            .iter()
            .filter(|i| matches!(i.severity, IssueSeverity::Warning))
            .count();
        let info = self
            .issues
            .iter()
            .filter(|i| matches!(i.severity, IssueSeverity::Info))
            .count();
        (errors, warnings, info)
    }

    /// Format a human-readable summary.
    pub fn format_summary(&self) -> String {
        let mut out = String::new();
        self.format_system_section(&mut out);
        self.format_machines_section(&mut out);
        self.format_tools_section(&mut out);
        self.format_issues_section(&mut out);
        out
    }

    fn format_system_section(&self, out: &mut String) {
        out.push_str("System:\n");
        out.push_str(&format!(
            "  forjar version: {}\n",
            self.system.forjar_version
        ));
        let status = self.system.dir_status();
        out.push_str(&format!(
            "  state directory: {} ({status})\n",
            self.system.state_dir
        ));
        if let Some(db_size) = self.system.db_size_bytes {
            let mb = db_size as f64 / (1024.0 * 1024.0);
            out.push_str(&format!("  state.db: {mb:.1}MB"));
            if let Some(v) = self.system.db_schema_version {
                out.push_str(&format!(", schema v{v}"));
            }
            out.push('\n');
        }
        self.system.format_log_line(out);
        out.push('\n');
    }

    fn format_machines_section(&self, out: &mut String) {
        if self.machines.is_empty() {
            return;
        }
        out.push_str("Machines:\n");
        for m in &self.machines {
            m.format_line(out);
        }
        out.push('\n');
    }

    fn format_tools_section(&self, out: &mut String) {
        if self.tools.is_empty() {
            return;
        }
        out.push_str("Tools:\n");
        for t in &self.tools {
            t.format_line(out);
        }
        out.push('\n');
    }

    fn format_issues_section(&self, out: &mut String) {
        if self.issues.is_empty() {
            return;
        }
        out.push_str("Issues:\n");
        for issue in &self.issues {
            let prefix = match issue.severity {
                IssueSeverity::Error => "ERROR",
                IssueSeverity::Warning => "WARNING",
                IssueSeverity::Info => "INFO",
            };
            out.push_str(&format!("  {prefix}: {}\n", issue.message));
        }
    }
}

/// System-level information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    /// Forjar version.
    pub forjar_version: String,
    /// State directory path.
    pub state_dir: String,
    /// Whether state directory exists.
    pub state_dir_exists: bool,
    /// Whether state directory is writable.
    pub state_dir_writable: bool,
    /// Size of state.db in bytes.
    #[serde(default)]
    pub db_size_bytes: Option<u64>,
    /// Schema version of state.db.
    #[serde(default)]
    pub db_schema_version: Option<u32>,
    /// Total run log storage in bytes.
    #[serde(default)]
    pub run_log_size_bytes: Option<u64>,
    /// Number of machines with run logs.
    #[serde(default)]
    pub run_log_machine_count: Option<u32>,
    /// Log budget in bytes.
    #[serde(default)]
    pub log_budget_bytes: Option<u64>,
}

impl SystemInfo {
    fn dir_status(&self) -> &'static str {
        if self.state_dir_exists && self.state_dir_writable {
            "exists, writable"
        } else if self.state_dir_exists {
            "exists, NOT writable"
        } else {
            "MISSING"
        }
    }

    fn format_log_line(&self, out: &mut String) {
        if let Some(log_size) = self.run_log_size_bytes {
            let mb = log_size as f64 / (1024.0 * 1024.0);
            out.push_str(&format!("  run logs: {mb:.0}MB"));
            if let Some(count) = self.run_log_machine_count {
                out.push_str(&format!(" across {count} machines"));
            }
            if let Some(budget) = self.log_budget_bytes {
                let budget_mb = budget as f64 / (1024.0 * 1024.0);
                out.push_str(&format!(" (budget: {budget_mb:.0}MB)"));
            }
            out.push('\n');
        }
    }
}

/// Per-machine health check result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineHealth {
    /// Machine name.
    pub name: String,
    /// SSH connectivity status.
    pub ssh_status: SshStatus,
    /// Number of managed resources.
    #[serde(default)]
    pub resource_count: Option<u32>,
    /// Current generation number.
    #[serde(default)]
    pub generation: Option<u32>,
    /// Number of stored runs.
    #[serde(default)]
    pub stored_runs: Option<u32>,
}

impl MachineHealth {
    fn format_line(&self, out: &mut String) {
        out.push_str(&format!("  {}: ", self.name));
        match &self.ssh_status {
            SshStatus::Ok { latency_ms } => out.push_str(&format!("SSH OK ({latency_ms:.0}ms)")),
            SshStatus::Failed { error } => out.push_str(&format!("SSH FAILED — {error}")),
            SshStatus::Local => out.push_str("local"),
            SshStatus::Container => out.push_str("container"),
        }
        if let Some(count) = self.resource_count {
            out.push_str(&format!(", {count} resources"));
        }
        if let Some(gen) = self.generation {
            out.push_str(&format!(", gen {gen}"));
        }
        out.push('\n');
    }
}

/// SSH connectivity status.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SshStatus {
    /// SSH connection succeeded.
    Ok {
        /// Round-trip latency in milliseconds.
        latency_ms: f64,
    },
    /// SSH connection failed.
    Failed {
        /// Error description.
        error: String,
    },
    /// Local machine (no SSH needed).
    Local,
    /// Container transport (no SSH needed).
    Container,
}

/// Tool availability check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCheck {
    /// Tool name (e.g., "bashrs", "docker", "pepita").
    pub name: String,
    /// Whether the tool is available.
    pub available: bool,
    /// Version string if available.
    #[serde(default)]
    pub version: Option<String>,
    /// Install hint if not available.
    #[serde(default)]
    pub install_hint: Option<String>,
}

impl ToolCheck {
    fn format_line(&self, out: &mut String) {
        out.push_str(&format!("  {}: ", self.name));
        if self.available {
            match &self.version {
                Some(v) => out.push_str(&format!("v{v} (OK)")),
                None => out.push_str("(OK)"),
            }
        } else {
            out.push_str("NOT FOUND");
            if let Some(ref hint) = self.install_hint {
                out.push_str(&format!(" — {hint}"));
            }
        }
        out.push('\n');
    }
}

/// A detected issue or warning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorIssue {
    /// Issue severity.
    pub severity: IssueSeverity,
    /// Human-readable message.
    pub message: String,
    /// Suggested fix command.
    #[serde(default)]
    pub fix_hint: Option<String>,
}

/// Issue severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueSeverity {
    /// Critical problem — apply will fail.
    Error,
    /// Non-critical — apply may have issues.
    Warning,
    /// Informational.
    Info,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_report(issues: Vec<DoctorIssue>) -> DoctorReport {
        DoctorReport {
            system: SystemInfo {
                forjar_version: "1.1.1".into(),
                state_dir: "./state/".into(),
                state_dir_exists: true,
                state_dir_writable: true,
                db_size_bytes: Some(2_400_000),
                db_schema_version: Some(3),
                run_log_size_bytes: Some(49_000_000),
                run_log_machine_count: Some(3),
                log_budget_bytes: Some(500_000_000),
            },
            machines: vec![
                MachineHealth {
                    name: "intel".into(),
                    ssh_status: SshStatus::Ok { latency_ms: 12.5 },
                    resource_count: Some(17),
                    generation: Some(13),
                    stored_runs: Some(10),
                },
                MachineHealth {
                    name: "lambda".into(),
                    ssh_status: SshStatus::Failed {
                        error: "Connection refused".into(),
                    },
                    resource_count: Some(7),
                    generation: Some(3),
                    stored_runs: Some(2),
                },
            ],
            tools: vec![
                ToolCheck {
                    name: "bashrs".into(),
                    available: true,
                    version: Some("6.64.0".into()),
                    install_hint: None,
                },
                ToolCheck {
                    name: "docker".into(),
                    available: false,
                    version: None,
                    install_hint: Some("apt install docker.io".into()),
                },
            ],
            issues,
        }
    }

    #[test]
    fn doctor_report_healthy() {
        let report = sample_report(vec![]);
        assert!(report.is_healthy());
        assert_eq!(report.issue_counts(), (0, 0, 0));
    }

    #[test]
    fn doctor_report_with_warnings_is_healthy() {
        let report = sample_report(vec![DoctorIssue {
            severity: IssueSeverity::Warning,
            message: "lambda is unreachable".into(),
            fix_hint: None,
        }]);
        assert!(report.is_healthy());
        assert_eq!(report.issue_counts(), (0, 1, 0));
    }

    #[test]
    fn doctor_report_with_error_not_healthy() {
        let report = sample_report(vec![DoctorIssue {
            severity: IssueSeverity::Error,
            message: "state directory not writable".into(),
            fix_hint: Some("chmod 700 state/".into()),
        }]);
        assert!(!report.is_healthy());
        assert_eq!(report.issue_counts(), (1, 0, 0));
    }

    #[test]
    fn doctor_report_mixed_issues() {
        let report = sample_report(vec![
            DoctorIssue {
                severity: IssueSeverity::Error,
                message: "error".into(),
                fix_hint: None,
            },
            DoctorIssue {
                severity: IssueSeverity::Warning,
                message: "warn".into(),
                fix_hint: None,
            },
            DoctorIssue {
                severity: IssueSeverity::Info,
                message: "info".into(),
                fix_hint: None,
            },
        ]);
        assert!(!report.is_healthy());
        assert_eq!(report.issue_counts(), (1, 1, 1));
    }

    #[test]
    fn doctor_report_format_summary() {
        let report = sample_report(vec![DoctorIssue {
            severity: IssueSeverity::Warning,
            message: "lambda is unreachable".into(),
            fix_hint: None,
        }]);
        let summary = report.format_summary();
        assert!(summary.contains("forjar version: 1.1.1"));
        assert!(summary.contains("exists, writable"));
        assert!(summary.contains("intel: SSH OK"));
        assert!(summary.contains("lambda: SSH FAILED"));
        assert!(summary.contains("bashrs"));
        assert!(summary.contains("v6.64.0"));
        assert!(summary.contains("docker"));
        assert!(summary.contains("NOT FOUND"));
        assert!(summary.contains("WARNING: lambda is unreachable"));
    }

    #[test]
    fn doctor_report_format_missing_state() {
        let mut report = sample_report(vec![]);
        report.system.state_dir_exists = false;
        let summary = report.format_summary();
        assert!(summary.contains("MISSING"));
    }

    #[test]
    fn doctor_report_format_not_writable() {
        let mut report = sample_report(vec![]);
        report.system.state_dir_writable = false;
        let summary = report.format_summary();
        assert!(summary.contains("NOT writable"));
    }

    #[test]
    fn ssh_status_serde_roundtrip() {
        let ok = SshStatus::Ok { latency_ms: 42.5 };
        let json = serde_json::to_string(&ok).unwrap();
        let parsed: SshStatus = serde_json::from_str(&json).unwrap();
        if let SshStatus::Ok { latency_ms } = parsed {
            assert!((latency_ms - 42.5).abs() < 0.01);
        } else {
            panic!("expected Ok variant");
        }
    }

    #[test]
    fn tool_check_serde_roundtrip() {
        let tool = ToolCheck {
            name: "bashrs".into(),
            available: true,
            version: Some("6.64.0".into()),
            install_hint: None,
        };
        let json = serde_json::to_string(&tool).unwrap();
        let parsed: ToolCheck = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "bashrs");
        assert!(parsed.available);
    }

    #[test]
    fn doctor_issue_serde_roundtrip() {
        let issue = DoctorIssue {
            severity: IssueSeverity::Error,
            message: "test".into(),
            fix_hint: Some("fix".into()),
        };
        let json = serde_json::to_string(&issue).unwrap();
        let parsed: DoctorIssue = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.severity, IssueSeverity::Error);
        assert_eq!(parsed.fix_hint.as_deref(), Some("fix"));
    }

    #[test]
    fn machine_health_local() {
        let m = MachineHealth {
            name: "localhost".into(),
            ssh_status: SshStatus::Local,
            resource_count: Some(5),
            generation: Some(1),
            stored_runs: None,
        };
        let json = serde_json::to_string(&m).unwrap();
        assert!(json.contains("local"));
    }

    #[test]
    fn machine_health_container() {
        let m = MachineHealth {
            name: "test-box".into(),
            ssh_status: SshStatus::Container,
            resource_count: None,
            generation: None,
            stored_runs: None,
        };
        let json = serde_json::to_string(&m).unwrap();
        assert!(json.contains("container"));
    }
}
