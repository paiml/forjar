//! FJ-2001/2301: Query engine types and run log types.
//!
//! Popperian rejection criteria for:
//! - FJ-2001: HealthSummary (totals, pct, empty, perfect, format_table)
//! - FJ-2001: TimingStats (from_sorted, empty, single, format_compact)
//! - FJ-2001: ChurnMetric (pct, zero total, full churn)
//! - FJ-2001: QueryParams (defaults), QueryOutputFormat, QueryResult serde
//! - FJ-2301: RunMeta (new, record_resource variants, multi-resource accounting)
//! - FJ-2301: RunLogEntry (format_log sections, format_json, Display, serde)
//! - FJ-2301: LogRetention (defaults), RunSummary accounting
//!
//! Usage: cargo test --test falsification_query_runlog

use forjar::core::types::{
    ChurnMetric, HealthSummary, LogRetention, MachineHealthRow, QueryOutputFormat, QueryParams,
    QueryResult, ResourceRunStatus, RunLogEntry, RunMeta, TimingStats,
};

// ============================================================================
// FJ-2001: HealthSummary
// ============================================================================

fn sample_health() -> HealthSummary {
    HealthSummary {
        machines: vec![
            MachineHealthRow {
                name: "intel".into(),
                total: 17,
                converged: 15,
                drifted: 1,
                failed: 1,
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
    assert_eq!(h.total_converged(), 22);
    assert_eq!(h.total_drifted(), 2);
    assert_eq!(h.total_failed(), 1);
}

#[test]
fn health_summary_pct() {
    let h = sample_health();
    let pct = h.stack_health_pct();
    assert!((pct - 88.0).abs() < 0.1);
}

#[test]
fn health_summary_empty() {
    let h = HealthSummary { machines: vec![] };
    assert_eq!(h.stack_health_pct(), 100.0);
    assert_eq!(h.total_resources(), 0);
}

#[test]
fn health_summary_perfect() {
    let h = HealthSummary {
        machines: vec![MachineHealthRow {
            name: "prod".into(),
            total: 50,
            converged: 50,
            drifted: 0,
            failed: 0,
            last_apply: "2026-03-09T00:00:00Z".into(),
            generation: 1,
        }],
    };
    assert_eq!(h.stack_health_pct(), 100.0);
}

#[test]
fn health_summary_format_table() {
    let h = sample_health();
    let table = h.format_table();
    assert!(table.contains("intel"));
    assert!(table.contains("jetson"));
    assert!(table.contains("TOTAL"));
    assert!(table.contains("MACHINE"));
    assert!(table.contains("RESOURCES"));
    assert!(table.contains("88%"));
}

// ============================================================================
// FJ-2001: TimingStats
// ============================================================================

#[test]
fn timing_stats_from_sorted() {
    let durations = vec![0.1, 0.2, 0.3, 0.5, 0.8, 1.0, 1.5, 2.0, 3.0, 5.0];
    let stats = TimingStats::from_sorted(&durations).unwrap();
    assert_eq!(stats.count, 10);
    assert!(stats.avg > 1.0);
    assert_eq!(stats.p50, 1.0);
    assert_eq!(stats.max, 5.0);
}

#[test]
fn timing_stats_empty_returns_none() {
    assert!(TimingStats::from_sorted(&[]).is_none());
}

#[test]
fn timing_stats_single_element() {
    let stats = TimingStats::from_sorted(&[2.5]).unwrap();
    assert_eq!(stats.count, 1);
    assert!((stats.avg - 2.5).abs() < 0.001);
    assert_eq!(stats.p50, 2.5);
    assert_eq!(stats.max, 2.5);
}

#[test]
fn timing_stats_format_compact() {
    let stats = TimingStats {
        count: 100,
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

// ============================================================================
// FJ-2001: ChurnMetric
// ============================================================================

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
fn churn_metric_full_churn() {
    let c = ChurnMetric {
        resource_id: "volatile".into(),
        changed_gens: 10,
        total_gens: 10,
    };
    assert!((c.churn_pct() - 100.0).abs() < 0.01);
}

// ============================================================================
// FJ-2001: QueryParams & QueryOutputFormat
// ============================================================================

#[test]
fn query_params_defaults() {
    let p = QueryParams::default();
    assert_eq!(p.limit, 50);
    assert_eq!(p.format, QueryOutputFormat::Table);
    assert!(!p.history);
    assert!(!p.drift);
    assert!(!p.timing);
    assert!(!p.churn);
    assert!(!p.failures);
    assert!(!p.health);
    assert!(p.keywords.is_none());
    assert!(p.machine.is_none());
}

#[test]
fn query_output_format_default_is_table() {
    assert_eq!(QueryOutputFormat::default(), QueryOutputFormat::Table);
}

#[test]
fn query_result_serde_roundtrip() {
    let r = QueryResult {
        resource_id: "svc-nginx".into(),
        machine: "intel".into(),
        resource_type: "service".into(),
        status: "converged".into(),
        generation: 7,
        applied_at: "2026-03-09T12:00:00Z".into(),
        duration_secs: 0.3,
        state_hash: Some("blake3:deadbeef".into()),
        path: None,
        fts_rank: Some(-2.5),
    };
    let json = serde_json::to_string(&r).unwrap();
    let parsed: QueryResult = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.resource_id, "svc-nginx");
    assert_eq!(parsed.generation, 7);
    assert_eq!(parsed.state_hash, Some("blake3:deadbeef".into()));
}

// ============================================================================
// FJ-2301: RunMeta
// ============================================================================

#[test]
fn run_meta_new() {
    let meta = RunMeta::new("r-abc123".into(), "intel".into(), "apply".into());
    assert_eq!(meta.run_id, "r-abc123");
    assert_eq!(meta.machine, "intel");
    assert_eq!(meta.command, "apply");
    assert!(meta.resources.is_empty());
    assert_eq!(meta.summary.total, 0);
    assert!(meta.generation.is_none());
    assert!(meta.operator.is_none());
}

#[test]
fn run_meta_record_converged() {
    let mut meta = RunMeta::new("r-1".into(), "m".into(), "apply".into());
    meta.record_resource(
        "pkg-nginx",
        ResourceRunStatus::Converged {
            exit_code: Some(0),
            duration_secs: Some(1.5),
            failed: false,
        },
    );
    assert_eq!(meta.summary.total, 1);
    assert_eq!(meta.summary.converged, 1);
    assert_eq!(meta.summary.failed, 0);
    assert!(meta.resources.contains_key("pkg-nginx"));
}

#[test]
fn run_meta_record_failed() {
    let mut meta = RunMeta::new("r-2".into(), "m".into(), "apply".into());
    meta.record_resource(
        "svc-broken",
        ResourceRunStatus::Converged {
            exit_code: Some(1),
            duration_secs: Some(0.5),
            failed: true,
        },
    );
    assert_eq!(meta.summary.failed, 1);
    assert_eq!(meta.summary.converged, 0);
}

#[test]
fn run_meta_record_noop() {
    let mut meta = RunMeta::new("r-3".into(), "m".into(), "apply".into());
    meta.record_resource("already-ok", ResourceRunStatus::Noop);
    assert_eq!(meta.summary.noop, 1);
    assert_eq!(meta.summary.total, 1);
}

#[test]
fn run_meta_record_skipped() {
    let mut meta = RunMeta::new("r-4".into(), "m".into(), "apply".into());
    meta.record_resource(
        "dep-failed",
        ResourceRunStatus::Skipped {
            reason: Some("dependency pkg-broken failed".into()),
        },
    );
    assert_eq!(meta.summary.skipped, 1);
}

#[test]
fn run_meta_multi_resource_accounting() {
    let mut meta = RunMeta::new("r-5".into(), "prod".into(), "apply".into());
    meta.record_resource(
        "a",
        ResourceRunStatus::Converged {
            exit_code: Some(0),
            duration_secs: None,
            failed: false,
        },
    );
    meta.record_resource("b", ResourceRunStatus::Noop);
    meta.record_resource(
        "c",
        ResourceRunStatus::Converged {
            exit_code: Some(1),
            duration_secs: None,
            failed: true,
        },
    );
    meta.record_resource(
        "d",
        ResourceRunStatus::Skipped {
            reason: Some("c failed".into()),
        },
    );
    assert_eq!(meta.summary.total, 4);
    assert_eq!(meta.summary.converged, 1);
    assert_eq!(meta.summary.noop, 1);
    assert_eq!(meta.summary.failed, 1);
    assert_eq!(meta.summary.skipped, 1);
    assert_eq!(meta.resources.len(), 4);
}

// ============================================================================
// FJ-2301: RunLogEntry
// ============================================================================

fn sample_log_entry() -> RunLogEntry {
    RunLogEntry {
        resource_id: "pkg-nginx".into(),
        resource_type: "package".into(),
        action: "apply".into(),
        machine: "web-1".into(),
        transport: "ssh".into(),
        script: "apt-get install -y nginx".into(),
        script_hash: "blake3:abc123".into(),
        stdout: "Reading package lists...".into(),
        stderr: String::new(),
        exit_code: 0,
        duration_secs: 1.234,
        started_at: "2026-03-09T14:30:00Z".into(),
        finished_at: "2026-03-09T14:30:01Z".into(),
    }
}

#[test]
fn run_log_format_log_sections() {
    let entry = sample_log_entry();
    let log = entry.format_log();
    assert!(log.contains("=== FORJAR TRANSPORT LOG ==="));
    assert!(log.contains("resource: pkg-nginx"));
    assert!(log.contains("type: package"));
    assert!(log.contains("action: apply"));
    assert!(log.contains("machine: web-1"));
    assert!(log.contains("transport: ssh"));
    assert!(log.contains("=== SCRIPT ==="));
    assert!(log.contains("apt-get install -y nginx"));
    assert!(log.contains("=== STDOUT ==="));
    assert!(log.contains("Reading package lists..."));
    assert!(log.contains("=== STDERR ==="));
    assert!(log.contains("=== RESULT ==="));
    assert!(log.contains("exit_code: 0"));
    assert!(log.contains("duration_secs: 1.234"));
}

#[test]
fn run_log_format_json() {
    let entry = sample_log_entry();
    let json = entry.format_json();
    assert!(json.contains("\"resource_id\":\"pkg-nginx\""));
    assert!(json.contains("\"exit_code\":0"));
}

#[test]
fn run_log_format_json_pretty() {
    let entry = sample_log_entry();
    let pretty = entry.format_json_pretty();
    assert!(pretty.contains("\"resource_id\": \"pkg-nginx\""));
    assert!(pretty.contains('\n'));
}

#[test]
fn run_log_display_trait() {
    let entry = sample_log_entry();
    let display = format!("{entry}");
    assert!(display.contains("=== FORJAR TRANSPORT LOG ==="));
    assert!(display.contains("exit_code: 0"));
}

#[test]
fn run_log_serde_roundtrip() {
    let entry = sample_log_entry();
    let json = serde_json::to_string(&entry).unwrap();
    let parsed: RunLogEntry = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.resource_id, "pkg-nginx");
    assert_eq!(parsed.exit_code, 0);
    assert_eq!(parsed.transport, "ssh");
}

// ============================================================================
// FJ-2301: LogRetention defaults
// ============================================================================

#[test]
fn log_retention_defaults() {
    let r = LogRetention::default();
    assert_eq!(r.keep_runs, 10);
    assert_eq!(r.keep_failed, 50);
    assert_eq!(r.max_log_size, 10 * 1024 * 1024);
    assert_eq!(r.max_total_size, 500 * 1024 * 1024);
}

// ============================================================================
// FJ-2301: RunMeta serde
// ============================================================================

#[test]
fn run_meta_serde_roundtrip() {
    let mut meta = RunMeta::new("r-test".into(), "intel".into(), "destroy".into());
    meta.generation = Some(42);
    meta.operator = Some("noah@machine".into());
    meta.record_resource(
        "pkg",
        ResourceRunStatus::Converged {
            exit_code: Some(0),
            duration_secs: Some(0.5),
            failed: false,
        },
    );
    let json = serde_json::to_string(&meta).unwrap();
    let parsed: RunMeta = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.run_id, "r-test");
    assert_eq!(parsed.command, "destroy");
    assert_eq!(parsed.generation, Some(42));
    assert_eq!(parsed.summary.converged, 1);
}
