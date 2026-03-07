//! FJ-2605: Resource coverage model — five levels of testing maturity.

use serde::{Deserialize, Serialize};
use std::fmt;

/// FJ-2605: Resource testing coverage level (L0–L5).
///
/// Each level subsumes the previous — L3 implies L2, L1, and L0.
///
/// # Examples
///
/// ```
/// use forjar::core::types::CoverageLevel;
///
/// let level = CoverageLevel::L3;
/// assert_eq!(level.label(), "convergence tested");
/// assert!(level >= CoverageLevel::L1);
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CoverageLevel {
    /// No tests — resource is untested.
    #[default]
    L0,
    /// Unit tested — codegen script and planner action verified.
    L1,
    /// Behavior spec — YAML `.spec.yaml` with verify commands.
    L2,
    /// Convergence tested — apply-verify-reapply-verify in sandbox.
    L3,
    /// Mutation tested — all applicable mutations detected.
    L4,
    /// Preservation tested — pairwise preservation with co-located resources.
    L5,
}

impl CoverageLevel {
    /// Human-readable label for the coverage level.
    pub fn label(self) -> &'static str {
        match self {
            Self::L0 => "no tests",
            Self::L1 => "unit tested",
            Self::L2 => "behavior spec",
            Self::L3 => "convergence tested",
            Self::L4 => "mutation tested",
            Self::L5 => "preservation tested",
        }
    }

    /// Numeric value (0–5) for threshold comparison.
    pub fn value(self) -> u8 {
        match self {
            Self::L0 => 0,
            Self::L1 => 1,
            Self::L2 => 2,
            Self::L3 => 3,
            Self::L4 => 4,
            Self::L5 => 5,
        }
    }
}

impl fmt::Display for CoverageLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "L{} ({})", self.value(), self.label())
    }
}

/// Per-resource coverage assessment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceCoverage {
    /// Resource identifier.
    pub resource_id: String,
    /// Assessed coverage level.
    pub level: CoverageLevel,
    /// Resource type (for grouping).
    pub resource_type: String,
}

/// Coverage report summary.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CoverageReport {
    /// Per-resource coverage entries.
    pub resources: Vec<ResourceCoverage>,
    /// Minimum coverage level across all resources.
    pub min_level: CoverageLevel,
    /// Average coverage level (as float for precision).
    pub avg_level: f64,
    /// Count of resources at each level.
    pub histogram: [u32; 6],
}

impl CoverageReport {
    /// Build a report from resource coverage entries.
    pub fn from_entries(resources: Vec<ResourceCoverage>) -> Self {
        let mut histogram = [0u32; 6];
        let mut min_level = CoverageLevel::L5;
        let mut total: u32 = 0;

        for entry in &resources {
            let idx = entry.level.value() as usize;
            histogram[idx] += 1;
            if entry.level < min_level {
                min_level = entry.level;
            }
            total += entry.level.value() as u32;
        }

        let avg_level = if resources.is_empty() {
            0.0
        } else {
            total as f64 / resources.len() as f64
        };

        if resources.is_empty() {
            min_level = CoverageLevel::L0;
        }

        Self {
            resources,
            min_level,
            avg_level,
            histogram,
        }
    }

    /// Check if all resources meet a minimum coverage threshold.
    pub fn meets_threshold(&self, threshold: CoverageLevel) -> bool {
        self.min_level >= threshold
    }

    /// Format as a human-readable report.
    pub fn format_report(&self) -> String {
        let mut out = String::from("Resource Coverage Report\n========================\n");
        for entry in &self.resources {
            let padding = 20usize.saturating_sub(entry.resource_id.len());
            out.push_str(&format!(
                "{}:{}{}\n",
                entry.resource_id,
                " ".repeat(padding),
                entry.level
            ));
        }
        out.push_str(&format!(
            "\nMin: {}  Avg: {:.1}  Total: {}\n",
            self.min_level,
            self.avg_level,
            self.resources.len()
        ));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coverage_level_ordering() {
        assert!(CoverageLevel::L0 < CoverageLevel::L1);
        assert!(CoverageLevel::L1 < CoverageLevel::L2);
        assert!(CoverageLevel::L4 < CoverageLevel::L5);
    }

    #[test]
    fn coverage_level_display() {
        assert_eq!(CoverageLevel::L0.to_string(), "L0 (no tests)");
        assert_eq!(CoverageLevel::L3.to_string(), "L3 (convergence tested)");
        assert_eq!(CoverageLevel::L5.to_string(), "L5 (preservation tested)");
    }

    #[test]
    fn coverage_level_value() {
        assert_eq!(CoverageLevel::L0.value(), 0);
        assert_eq!(CoverageLevel::L5.value(), 5);
    }

    #[test]
    fn coverage_level_default_is_l0() {
        assert_eq!(CoverageLevel::default(), CoverageLevel::L0);
    }

    #[test]
    fn coverage_report_empty() {
        let report = CoverageReport::from_entries(vec![]);
        assert_eq!(report.min_level, CoverageLevel::L0);
        assert_eq!(report.avg_level, 0.0);
        assert!(report.meets_threshold(CoverageLevel::L0));
    }

    #[test]
    fn coverage_report_basic() {
        let entries = vec![
            ResourceCoverage {
                resource_id: "nginx-pkg".into(),
                level: CoverageLevel::L4,
                resource_type: "package".into(),
            },
            ResourceCoverage {
                resource_id: "nginx-config".into(),
                level: CoverageLevel::L3,
                resource_type: "file".into(),
            },
            ResourceCoverage {
                resource_id: "app-deploy".into(),
                level: CoverageLevel::L1,
                resource_type: "task".into(),
            },
        ];
        let report = CoverageReport::from_entries(entries);
        assert_eq!(report.min_level, CoverageLevel::L1);
        assert!((report.avg_level - 2.67).abs() < 0.1);
        assert_eq!(report.histogram[1], 1); // L1
        assert_eq!(report.histogram[3], 1); // L3
        assert_eq!(report.histogram[4], 1); // L4
        assert!(report.meets_threshold(CoverageLevel::L1));
        assert!(!report.meets_threshold(CoverageLevel::L2));
    }

    #[test]
    fn coverage_report_format() {
        let entries = vec![ResourceCoverage {
            resource_id: "pkg".into(),
            level: CoverageLevel::L2,
            resource_type: "package".into(),
        }];
        let report = CoverageReport::from_entries(entries);
        let text = report.format_report();
        assert!(text.contains("pkg:"));
        assert!(text.contains("L2 (behavior spec)"));
        assert!(text.contains("Min: L2"));
    }

    #[test]
    fn coverage_level_serde_roundtrip() {
        for level in [
            CoverageLevel::L0,
            CoverageLevel::L1,
            CoverageLevel::L2,
            CoverageLevel::L3,
            CoverageLevel::L4,
            CoverageLevel::L5,
        ] {
            let json = serde_json::to_string(&level).unwrap();
            let parsed: CoverageLevel = serde_json::from_str(&json).unwrap();
            assert_eq!(level, parsed);
        }
    }

    #[test]
    fn coverage_report_all_l5() {
        let entries = vec![
            ResourceCoverage {
                resource_id: "a".into(),
                level: CoverageLevel::L5,
                resource_type: "file".into(),
            },
            ResourceCoverage {
                resource_id: "b".into(),
                level: CoverageLevel::L5,
                resource_type: "file".into(),
            },
        ];
        let report = CoverageReport::from_entries(entries);
        assert!(report.meets_threshold(CoverageLevel::L5));
        assert_eq!(report.avg_level, 5.0);
    }
}
