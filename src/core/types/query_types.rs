//! FJ-2001: Query engine types — result structs for `forjar query`.
//!
//! Types for the SQLite-backed query engine: resource results, health
//! summaries, drift findings, timing stats, and output formatting.

use serde::{Deserialize, Serialize};

/// FJ-2001: A resource query result row.
///
/// # Examples
///
/// ```
/// use forjar::core::types::QueryResult;
///
/// let result = QueryResult {
///     resource_id: "bash-aliases".into(),
///     machine: "intel".into(),
///     resource_type: "file".into(),
///     status: "converged".into(),
///     generation: 12,
///     applied_at: "2026-02-16T16:32:55Z".into(),
///     duration_secs: 0.54,
///     state_hash: Some("blake3:a1b2c3".into()),
///     path: Some("/home/user/.bash_aliases".into()),
///     fts_rank: None,
/// };
/// assert_eq!(result.resource_id, "bash-aliases");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    /// Resource identifier.
    pub resource_id: String,
    /// Machine name.
    pub machine: String,
    /// Resource type (file, package, service, etc.).
    pub resource_type: String,
    /// Current status (converged, failed, drifted, destroyed).
    pub status: String,
    /// Generation number.
    pub generation: u32,
    /// ISO 8601 timestamp of last apply.
    pub applied_at: String,
    /// Duration of last apply in seconds.
    pub duration_secs: f64,
    /// State hash (BLAKE3).
    #[serde(default)]
    pub state_hash: Option<String>,
    /// File path (for file/mount resources).
    #[serde(default)]
    pub path: Option<String>,
    /// FTS5 rank score (lower = better match).
    #[serde(default)]
    pub fts_rank: Option<f64>,
}

/// FJ-2001: Stack-wide health summary.
///
/// # Examples
///
/// ```
/// use forjar::core::types::{HealthSummary, MachineHealthRow};
///
/// let summary = HealthSummary {
///     machines: vec![MachineHealthRow {
///         name: "intel".into(),
///         total: 17,
///         converged: 17,
///         drifted: 0,
///         failed: 0,
///         last_apply: "2026-02-16T16:44:39Z".into(),
///         generation: 12,
///     }],
/// };
/// assert_eq!(summary.stack_health_pct(), 100.0);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthSummary {
    /// Per-machine health rows.
    pub machines: Vec<MachineHealthRow>,
}

impl HealthSummary {
    /// Total resources across all machines.
    pub fn total_resources(&self) -> u32 {
        self.machines.iter().map(|m| m.total).sum()
    }

    /// Total converged resources.
    pub fn total_converged(&self) -> u32 {
        self.machines.iter().map(|m| m.converged).sum()
    }

    /// Total drifted resources.
    pub fn total_drifted(&self) -> u32 {
        self.machines.iter().map(|m| m.drifted).sum()
    }

    /// Total failed resources.
    pub fn total_failed(&self) -> u32 {
        self.machines.iter().map(|m| m.failed).sum()
    }

    /// Stack health percentage (converged / total * 100).
    pub fn stack_health_pct(&self) -> f64 {
        let total = self.total_resources();
        if total == 0 {
            return 100.0;
        }
        (self.total_converged() as f64 / total as f64) * 100.0
    }

    /// Format a human-readable health table.
    pub fn format_table(&self) -> String {
        let mut out = String::new();
        out.push_str(
            " MACHINE     RESOURCES  CONVERGED  DRIFTED  FAILED  LAST APPLY             GEN\n",
        );

        for m in &self.machines {
            out.push_str(&format!(
                " {:<11} {:<10} {:<10} {:<8} {:<7} {:<22} {}\n",
                m.name, m.total, m.converged, m.drifted, m.failed, m.last_apply, m.generation,
            ));
        }

        let total = self.total_resources();
        let conv = self.total_converged();
        let drift = self.total_drifted();
        let fail = self.total_failed();
        let pct = self.stack_health_pct();

        out.push_str(
            " ─────────────────────────────────────────────────────────────────────────\n",
        );
        out.push_str(&format!(
            " TOTAL       {total:<10} {conv:<10} {drift:<8} {fail:<7} Stack health: {pct:.0}%\n",
        ));
        out
    }
}

/// Per-machine health row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineHealthRow {
    /// Machine name.
    pub name: String,
    /// Total resource count.
    pub total: u32,
    /// Converged resource count.
    pub converged: u32,
    /// Drifted resource count.
    pub drifted: u32,
    /// Failed resource count.
    pub failed: u32,
    /// Last apply timestamp.
    pub last_apply: String,
    /// Current generation.
    pub generation: u32,
}

/// FJ-2004: Timing statistics for query enrichment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingStats {
    /// Number of data points.
    pub count: u32,
    /// Average duration in seconds.
    pub avg: f64,
    /// Median (p50) duration.
    pub p50: f64,
    /// 95th percentile duration.
    pub p95: f64,
    /// 99th percentile duration.
    pub p99: f64,
    /// Maximum duration.
    pub max: f64,
}

impl TimingStats {
    /// Calculate timing stats from a sorted slice of durations.
    pub fn from_sorted(durations: &[f64]) -> Option<Self> {
        if durations.is_empty() {
            return None;
        }
        let n = durations.len();
        let sum: f64 = durations.iter().sum();
        Some(Self {
            count: n as u32,
            avg: sum / n as f64,
            p50: durations[n / 2],
            p95: durations[(n as f64 * 0.95) as usize],
            p99: durations[((n as f64 * 0.99) as usize).min(n - 1)],
            max: durations[n - 1],
        })
    }

    /// Format as a compact summary string.
    pub fn format_compact(&self) -> String {
        format!(
            "avg={:.2}s p50={:.2}s p95={:.2}s",
            self.avg, self.p50, self.p95
        )
    }
}

/// FJ-2004: Resource churn metric.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChurnMetric {
    /// Resource identifier.
    pub resource_id: String,
    /// Number of generations where this resource changed.
    pub changed_gens: u32,
    /// Total number of generations.
    pub total_gens: u32,
}

impl ChurnMetric {
    /// Churn percentage (changed / total * 100).
    pub fn churn_pct(&self) -> f64 {
        if self.total_gens == 0 {
            return 0.0;
        }
        (self.changed_gens as f64 / self.total_gens as f64) * 100.0
    }
}

/// FJ-2001: Query output format.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QueryOutputFormat {
    /// Human-readable table (default).
    #[default]
    Table,
    /// JSON output.
    Json,
    /// CSV output.
    Csv,
    /// Raw SQL result.
    Sql,
}

/// FJ-2001: Parsed query parameters from CLI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryParams {
    /// FTS5 search keywords.
    #[serde(default)]
    pub keywords: Option<String>,
    /// Machine filter.
    #[serde(default)]
    pub machine: Option<String>,
    /// Resource type filter.
    #[serde(default)]
    pub resource_type: Option<String>,
    /// Status filter.
    #[serde(default)]
    pub status: Option<String>,
    /// Time range filter (e.g., "7d", "24h").
    #[serde(default)]
    pub since: Option<String>,
    /// Maximum results.
    #[serde(default = "default_limit")]
    pub limit: u32,
    /// Output format.
    #[serde(default)]
    pub format: QueryOutputFormat,
    /// Include generation history.
    #[serde(default)]
    pub history: bool,
    /// Include drift findings.
    #[serde(default)]
    pub drift: bool,
    /// Include timing stats.
    #[serde(default)]
    pub timing: bool,
    /// Include churn metrics.
    #[serde(default)]
    pub churn: bool,
    /// Include failure history.
    #[serde(default)]
    pub failures: bool,
    /// Health summary mode.
    #[serde(default)]
    pub health: bool,
}

fn default_limit() -> u32 {
    50
}

impl Default for QueryParams {
    fn default() -> Self {
        Self {
            keywords: None,
            machine: None,
            resource_type: None,
            status: None,
            since: None,
            limit: 50,
            format: QueryOutputFormat::Table,
            history: false,
            drift: false,
            timing: false,
            churn: false,
            failures: false,
            health: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_health() -> HealthSummary {
        HealthSummary {
            machines: vec![
                MachineHealthRow {
                    name: "intel".into(),
                    total: 17,
                    converged: 17,
                    drifted: 0,
                    failed: 0,
                    last_apply: "2026-02-16T16:44:39Z".into(),
                    generation: 12,
                },
                MachineHealthRow {
                    name: "jetson".into(),
                    total: 8,
                    converged: 7,
                    drifted: 1,
                    failed: 0,
                    last_apply: "2026-03-01T09:12:00Z".into(),
                    generation: 5,
                },
            ],
        }
    }

    #[test]
    fn health_summary_totals() {
        let h = sample_health();
        assert_eq!(h.total_resources(), 25);
        assert_eq!(h.total_converged(), 24);
        assert_eq!(h.total_drifted(), 1);
        assert_eq!(h.total_failed(), 0);
    }

    #[test]
    fn health_summary_pct() {
        let h = sample_health();
        let pct = h.stack_health_pct();
        assert!((pct - 96.0).abs() < 0.1);
    }

    #[test]
    fn health_summary_empty() {
        let h = HealthSummary { machines: vec![] };
        assert_eq!(h.stack_health_pct(), 100.0);
        assert_eq!(h.total_resources(), 0);
    }

    #[test]
    fn health_summary_format_table() {
        let h = sample_health();
        let table = h.format_table();
        assert!(table.contains("intel"));
        assert!(table.contains("jetson"));
        assert!(table.contains("TOTAL"));
        assert!(table.contains("96%"));
    }

    #[test]
    fn timing_stats_from_sorted() {
        let durations = vec![0.1, 0.2, 0.3, 0.5, 0.8, 1.0, 1.5, 2.0, 3.0, 5.0];
        let stats = TimingStats::from_sorted(&durations).unwrap();
        assert_eq!(stats.count, 10);
        assert!(stats.avg > 0.0);
        assert_eq!(stats.p50, 1.0); // index 5 of 10 elements
        assert_eq!(stats.max, 5.0);
    }

    #[test]
    fn timing_stats_empty() {
        assert!(TimingStats::from_sorted(&[]).is_none());
    }

    #[test]
    fn timing_stats_single() {
        let stats = TimingStats::from_sorted(&[1.5]).unwrap();
        assert_eq!(stats.count, 1);
        assert!((stats.avg - 1.5).abs() < 0.001);
        assert_eq!(stats.p50, 1.5);
    }

    #[test]
    fn timing_stats_format_compact() {
        let stats = TimingStats {
            count: 10,
            avg: 1.23,
            p50: 0.95,
            p95: 3.40,
            p99: 4.80,
            max: 5.00,
        };
        let s = stats.format_compact();
        assert!(s.contains("avg=1.23s"));
        assert!(s.contains("p50=0.95s"));
        assert!(s.contains("p95=3.40s"));
    }

    #[test]
    fn churn_metric_pct() {
        let c = ChurnMetric {
            resource_id: "bash-aliases".into(),
            changed_gens: 3,
            total_gens: 12,
        };
        assert!((c.churn_pct() - 25.0).abs() < 0.1);
    }

    #[test]
    fn churn_metric_zero_total() {
        let c = ChurnMetric {
            resource_id: "x".into(),
            changed_gens: 0,
            total_gens: 0,
        };
        assert_eq!(c.churn_pct(), 0.0);
    }

    #[test]
    fn query_params_default() {
        let p = QueryParams::default();
        assert_eq!(p.limit, 50);
        assert_eq!(p.format, QueryOutputFormat::Table);
        assert!(!p.history);
        assert!(!p.drift);
        assert!(!p.timing);
    }

    #[test]
    fn query_output_format_default() {
        assert_eq!(QueryOutputFormat::default(), QueryOutputFormat::Table);
    }

    #[test]
    fn query_result_serde_roundtrip() {
        let r = QueryResult {
            resource_id: "pkg".into(),
            machine: "intel".into(),
            resource_type: "package".into(),
            status: "converged".into(),
            generation: 5,
            applied_at: "2026-01-01".into(),
            duration_secs: 1.5,
            state_hash: Some("blake3:x".into()),
            path: None,
            fts_rank: Some(-1.5),
        };
        let json = serde_json::to_string(&r).unwrap();
        let parsed: QueryResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.resource_id, "pkg");
        assert_eq!(parsed.generation, 5);
    }

    #[test]
    fn health_summary_serde_roundtrip() {
        let h = sample_health();
        let json = serde_json::to_string(&h).unwrap();
        let parsed: HealthSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.machines.len(), 2);
        assert_eq!(parsed.total_resources(), 25);
    }
}
