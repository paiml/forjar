//! FJ-2604: Infrastructure mutation testing types — operators, results, scoring.

use serde::{Deserialize, Serialize};
use std::fmt;

/// FJ-2604: Infrastructure mutation operator.
///
/// # Examples
///
/// ```
/// use forjar::core::types::MutationOperator;
///
/// let op = MutationOperator::DeleteFile;
/// assert_eq!(op.to_string(), "delete_file");
/// assert_eq!(op.description(), "Remove a managed file");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutationOperator {
    /// Remove a managed file from the filesystem.
    DeleteFile,
    /// Change content of a managed file.
    ModifyContent,
    /// Alter file mode/owner/group.
    ChangePermissions,
    /// Stop a managed service via systemctl.
    StopService,
    /// Remove a managed package via apt/yum.
    RemovePackage,
    /// Kill a managed process.
    KillProcess,
    /// Unmount a managed filesystem.
    UnmountFilesystem,
    /// Modify a managed config file partially.
    CorruptConfig,
}

impl MutationOperator {
    /// Human-readable description of what this mutation does.
    pub fn description(self) -> &'static str {
        match self {
            Self::DeleteFile => "Remove a managed file",
            Self::ModifyContent => "Change file content",
            Self::ChangePermissions => "Alter file mode/owner",
            Self::StopService => "Stop a managed service",
            Self::RemovePackage => "Remove a managed package",
            Self::KillProcess => "Kill a managed process",
            Self::UnmountFilesystem => "Unmount a managed filesystem",
            Self::CorruptConfig => "Modify a managed config file",
        }
    }

    /// Resource types this operator applies to.
    pub fn applicable_types(self) -> &'static [&'static str] {
        match self {
            Self::DeleteFile | Self::ModifyContent | Self::ChangePermissions => &["file"],
            Self::StopService | Self::KillProcess => &["service"],
            Self::RemovePackage => &["package"],
            Self::UnmountFilesystem => &["mount"],
            Self::CorruptConfig => &["file"],
        }
    }
}

impl fmt::Display for MutationOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DeleteFile => write!(f, "delete_file"),
            Self::ModifyContent => write!(f, "modify_content"),
            Self::ChangePermissions => write!(f, "change_permissions"),
            Self::StopService => write!(f, "stop_service"),
            Self::RemovePackage => write!(f, "remove_package"),
            Self::KillProcess => write!(f, "kill_process"),
            Self::UnmountFilesystem => write!(f, "unmount_filesystem"),
            Self::CorruptConfig => write!(f, "corrupt_config"),
        }
    }
}

/// FJ-2604: Result of a single mutation test.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationResult {
    /// Resource ID that was mutated.
    pub resource_id: String,
    /// Resource type (file, package, service, etc.).
    pub resource_type: String,
    /// Which mutation was applied.
    pub operator: MutationOperator,
    /// Whether drift detection caught the mutation.
    pub detected: bool,
    /// Whether re-convergence succeeded after mutation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reconverged: Option<bool>,
    /// Duration of the mutation test in milliseconds.
    #[serde(default)]
    pub duration_ms: u64,
    /// Error message if the mutation test itself failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl MutationResult {
    /// Whether this mutation was caught (detected and reconverged).
    pub fn is_killed(&self) -> bool {
        self.detected
    }

    /// Whether this mutation survived (not detected).
    pub fn is_survived(&self) -> bool {
        !self.detected
    }
}

impl fmt::Display for MutationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status = if self.detected { "KILLED" } else { "SURVIVED" };
        write!(
            f,
            "[{status}] {}/{}: {} ({}ms)",
            self.resource_id, self.resource_type, self.operator, self.duration_ms
        )
    }
}

/// FJ-2604: Mutation testing score and grade.
///
/// # Examples
///
/// ```
/// use forjar::core::types::MutationScore;
///
/// let score = MutationScore {
///     total: 20,
///     detected: 18,
///     survived: 2,
///     errored: 0,
/// };
/// assert!((score.score_pct() - 90.0).abs() < 0.01);
/// assert_eq!(score.grade(), 'A');
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MutationScore {
    /// Total mutations attempted.
    pub total: usize,
    /// Mutations detected by drift detection.
    pub detected: usize,
    /// Mutations that survived (not detected).
    pub survived: usize,
    /// Mutations where the test itself errored.
    pub errored: usize,
}

impl MutationScore {
    /// Mutation score as percentage (detected / total * 100).
    pub fn score_pct(&self) -> f64 {
        if self.total == 0 {
            return 100.0;
        }
        (self.detected as f64 / self.total as f64) * 100.0
    }

    /// Letter grade based on mutation score.
    pub fn grade(&self) -> char {
        let pct = self.score_pct();
        if pct >= 90.0 {
            'A'
        } else if pct >= 80.0 {
            'B'
        } else if pct >= 60.0 {
            'C'
        } else {
            'F'
        }
    }
}

impl fmt::Display for MutationScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Mutation Score: {:.0}% (Grade {})\n  {}/{} detected, {} survived, {} errored",
            self.score_pct(),
            self.grade(),
            self.detected,
            self.total,
            self.survived,
            self.errored,
        )
    }
}

/// FJ-2604: Per-resource-type mutation summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeMutationSummary {
    /// Resource type (file, package, service, etc.).
    pub resource_type: String,
    /// Total mutations for this type.
    pub total: usize,
    /// Detected mutations for this type.
    pub detected: usize,
}

impl TypeMutationSummary {
    /// Detection rate as percentage.
    pub fn detection_pct(&self) -> f64 {
        if self.total == 0 {
            return 100.0;
        }
        (self.detected as f64 / self.total as f64) * 100.0
    }
}

impl fmt::Display for TypeMutationSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {}/{} detected ({:.0}%)",
            self.resource_type,
            self.detected,
            self.total,
            self.detection_pct(),
        )
    }
}

/// FJ-2604: Complete mutation testing report.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MutationReport {
    /// Overall score.
    pub score: MutationScore,
    /// Per-resource-type summaries.
    pub by_type: Vec<TypeMutationSummary>,
    /// Individual mutation results.
    pub results: Vec<MutationResult>,
    /// Undetected mutations (for targeted improvement).
    pub undetected: Vec<MutationResult>,
}

impl MutationReport {
    /// Build a report from individual results.
    pub fn from_results(results: Vec<MutationResult>) -> Self {
        let total = results.len();
        let detected = results.iter().filter(|r| r.detected).count();
        let survived = results
            .iter()
            .filter(|r| !r.detected && r.error.is_none())
            .count();
        let errored = results.iter().filter(|r| r.error.is_some()).count();

        let undetected: Vec<MutationResult> = results
            .iter()
            .filter(|r| !r.detected && r.error.is_none())
            .cloned()
            .collect();

        let by_type = Self::summarize_by_type(&results);

        Self {
            score: MutationScore {
                total,
                detected,
                survived,
                errored,
            },
            by_type,
            results,
            undetected,
        }
    }

    fn summarize_by_type(results: &[MutationResult]) -> Vec<TypeMutationSummary> {
        let mut types: std::collections::HashMap<&str, (usize, usize)> =
            std::collections::HashMap::new();
        for r in results {
            let entry = types.entry(r.resource_type.as_str()).or_default();
            entry.0 += 1;
            if r.detected {
                entry.1 += 1;
            }
        }
        let mut summaries: Vec<TypeMutationSummary> = types
            .into_iter()
            .map(|(rt, (total, detected))| TypeMutationSummary {
                resource_type: rt.to_string(),
                total,
                detected,
            })
            .collect();
        summaries.sort_by(|a, b| a.resource_type.cmp(&b.resource_type));
        summaries
    }

    /// Format human-readable mutation report.
    pub fn format_summary(&self) -> String {
        let mut out = format!("{}\n", self.score);
        out.push_str("=================================\n");
        for summary in &self.by_type {
            out.push_str(&format!("  {summary}\n"));
        }
        if !self.undetected.is_empty() {
            out.push_str("\nUndetected mutations:\n");
            for u in &self.undetected {
                out.push_str(&format!("  - {u}\n"));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutation_operator_display() {
        assert_eq!(MutationOperator::DeleteFile.to_string(), "delete_file");
        assert_eq!(MutationOperator::StopService.to_string(), "stop_service");
        assert_eq!(
            MutationOperator::RemovePackage.to_string(),
            "remove_package"
        );
    }

    #[test]
    fn mutation_operator_description() {
        assert_eq!(
            MutationOperator::DeleteFile.description(),
            "Remove a managed file"
        );
    }

    #[test]
    fn mutation_operator_applicable_types() {
        assert_eq!(MutationOperator::DeleteFile.applicable_types(), &["file"]);
        assert_eq!(
            MutationOperator::StopService.applicable_types(),
            &["service"]
        );
        assert_eq!(
            MutationOperator::RemovePackage.applicable_types(),
            &["package"]
        );
    }

    #[test]
    fn mutation_operator_serde_roundtrip() {
        for op in [
            MutationOperator::DeleteFile,
            MutationOperator::ModifyContent,
            MutationOperator::StopService,
            MutationOperator::RemovePackage,
            MutationOperator::KillProcess,
        ] {
            let json = serde_json::to_string(&op).unwrap();
            let parsed: MutationOperator = serde_json::from_str(&json).unwrap();
            assert_eq!(op, parsed);
        }
    }

    #[test]
    fn mutation_result_killed() {
        let r = MutationResult {
            resource_id: "nginx-config".into(),
            resource_type: "file".into(),
            operator: MutationOperator::DeleteFile,
            detected: true,
            reconverged: Some(true),
            duration_ms: 150,
            error: None,
        };
        assert!(r.is_killed());
        assert!(!r.is_survived());
        let display = r.to_string();
        assert!(display.contains("KILLED"));
        assert!(display.contains("nginx-config"));
    }

    #[test]
    fn mutation_result_survived() {
        let r = MutationResult {
            resource_id: "curl-pkg".into(),
            resource_type: "package".into(),
            operator: MutationOperator::RemovePackage,
            detected: false,
            reconverged: None,
            duration_ms: 200,
            error: None,
        };
        assert!(r.is_survived());
        assert!(!r.is_killed());
        assert!(r.to_string().contains("SURVIVED"));
    }

    #[test]
    fn mutation_score_perfect() {
        let score = MutationScore {
            total: 10,
            detected: 10,
            survived: 0,
            errored: 0,
        };
        assert!((score.score_pct() - 100.0).abs() < 0.01);
        assert_eq!(score.grade(), 'A');
    }

    #[test]
    fn mutation_score_grade_boundaries() {
        let grade_a = MutationScore {
            total: 10,
            detected: 9,
            survived: 1,
            errored: 0,
        };
        assert_eq!(grade_a.grade(), 'A');

        let grade_b = MutationScore {
            total: 10,
            detected: 8,
            survived: 2,
            errored: 0,
        };
        assert_eq!(grade_b.grade(), 'B');

        let grade_c = MutationScore {
            total: 10,
            detected: 6,
            survived: 4,
            errored: 0,
        };
        assert_eq!(grade_c.grade(), 'C');

        let grade_f = MutationScore {
            total: 10,
            detected: 5,
            survived: 5,
            errored: 0,
        };
        assert_eq!(grade_f.grade(), 'F');
    }

    #[test]
    fn mutation_score_empty() {
        let score = MutationScore::default();
        assert!((score.score_pct() - 100.0).abs() < 0.01);
        assert_eq!(score.grade(), 'A');
    }

    #[test]
    fn mutation_score_display() {
        let score = MutationScore {
            total: 20,
            detected: 17,
            survived: 3,
            errored: 0,
        };
        let s = score.to_string();
        assert!(s.contains("85%"));
        assert!(s.contains("Grade B"));
        assert!(s.contains("17/20"));
    }

    #[test]
    fn type_mutation_summary_display() {
        let s = TypeMutationSummary {
            resource_type: "file".into(),
            total: 12,
            detected: 12,
        };
        assert!((s.detection_pct() - 100.0).abs() < 0.01);
        let display = s.to_string();
        assert!(display.contains("file"));
        assert!(display.contains("12/12"));
    }

    #[test]
    fn mutation_report_from_results() {
        let results = vec![
            MutationResult {
                resource_id: "config".into(),
                resource_type: "file".into(),
                operator: MutationOperator::DeleteFile,
                detected: true,
                reconverged: Some(true),
                duration_ms: 100,
                error: None,
            },
            MutationResult {
                resource_id: "config".into(),
                resource_type: "file".into(),
                operator: MutationOperator::ModifyContent,
                detected: true,
                reconverged: Some(true),
                duration_ms: 120,
                error: None,
            },
            MutationResult {
                resource_id: "nginx".into(),
                resource_type: "package".into(),
                operator: MutationOperator::RemovePackage,
                detected: false,
                reconverged: None,
                duration_ms: 200,
                error: None,
            },
        ];
        let report = MutationReport::from_results(results);
        assert_eq!(report.score.total, 3);
        assert_eq!(report.score.detected, 2);
        assert_eq!(report.score.survived, 1);
        assert_eq!(report.undetected.len(), 1);
        assert_eq!(report.undetected[0].resource_id, "nginx");
        assert_eq!(report.by_type.len(), 2);
    }

    #[test]
    fn mutation_report_format_summary() {
        let results = vec![MutationResult {
            resource_id: "svc".into(),
            resource_type: "service".into(),
            operator: MutationOperator::StopService,
            detected: false,
            reconverged: None,
            duration_ms: 50,
            error: None,
        }];
        let report = MutationReport::from_results(results);
        let text = report.format_summary();
        assert!(text.contains("Grade F"));
        assert!(text.contains("Undetected"));
        assert!(text.contains("svc"));
    }
}
