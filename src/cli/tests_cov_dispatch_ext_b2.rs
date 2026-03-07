//! Coverage tests for dispatch_status_ext_b.rs — analytics, fleet, reports, queries, display phases.

use super::dispatch_status_ext_b::*;

fn setup_state() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("web1")).unwrap();
    std::fs::write(
        dir.path().join("web1/state.lock.yaml"),
        "resources:\n  nginx:\n    resource_type: Package\n    status: Converged\n    hash: abc123\n    applied_at: '2025-01-01T00:00:00Z'\n    duration_seconds: 2.5\n",
    ).unwrap();
    std::fs::write(
        dir.path().join("web1/events.jsonl"),
        "{\"ts\":\"2026-01-01T00:00:00Z\",\"event\":\"resource_converged\",\"resource\":\"nginx\",\"machine\":\"web1\"}\n",
    ).unwrap();
    dir
}

// ── try_status_analytics (10 flags) ──

#[test] fn analytics_0()  { let d = setup_state(); assert!(try_status_analytics(d.path(), None, false, true, false, false, false, false, false, false, false, false, false).is_some()); }
#[test] fn analytics_1()  { let d = setup_state(); assert!(try_status_analytics(d.path(), None, false, false, true, false, false, false, false, false, false, false, false).is_some()); }
#[test] fn analytics_2()  { let d = setup_state(); assert!(try_status_analytics(d.path(), None, false, false, false, true, false, false, false, false, false, false, false).is_some()); }
#[test] fn analytics_3()  { let d = setup_state(); assert!(try_status_analytics(d.path(), None, false, false, false, false, true, false, false, false, false, false, false).is_some()); }
#[test] fn analytics_4()  { let d = setup_state(); assert!(try_status_analytics(d.path(), None, false, false, false, false, false, true, false, false, false, false, false).is_some()); }
#[test] fn analytics_5()  { let d = setup_state(); assert!(try_status_analytics(d.path(), None, false, false, false, false, false, false, true, false, false, false, false).is_some()); }
#[test] fn analytics_6()  { let d = setup_state(); assert!(try_status_analytics(d.path(), None, false, false, false, false, false, false, false, true, false, false, false).is_some()); }
#[test] fn analytics_7()  { let d = setup_state(); assert!(try_status_analytics(d.path(), None, false, false, false, false, false, false, false, false, true, false, false).is_some()); }
#[test] fn analytics_8()  { let d = setup_state(); assert!(try_status_analytics(d.path(), None, false, false, false, false, false, false, false, false, false, true, false).is_some()); }
#[test] fn analytics_9()  { let d = setup_state(); assert!(try_status_analytics(d.path(), None, false, false, false, false, false, false, false, false, false, false, true).is_some()); }
#[test] fn analytics_none() { let d = setup_state(); assert!(try_status_analytics(d.path(), None, false, false, false, false, false, false, false, false, false, false, false).is_none()); }

// ── try_status_fleet (10 flags) ──

#[test] fn fleet_0()  { let d = setup_state(); assert!(try_status_fleet(d.path(), None, false, true, false, false, false, false, false, false, false, false, false).is_some()); }
#[test] fn fleet_1()  { let d = setup_state(); assert!(try_status_fleet(d.path(), None, false, false, true, false, false, false, false, false, false, false, false).is_some()); }
#[test] fn fleet_2()  { let d = setup_state(); assert!(try_status_fleet(d.path(), None, false, false, false, true, false, false, false, false, false, false, false).is_some()); }
#[test] fn fleet_3()  { let d = setup_state(); assert!(try_status_fleet(d.path(), None, false, false, false, false, true, false, false, false, false, false, false).is_some()); }
#[test] fn fleet_4()  { let d = setup_state(); assert!(try_status_fleet(d.path(), None, false, false, false, false, false, true, false, false, false, false, false).is_some()); }
#[test] fn fleet_5()  { let d = setup_state(); assert!(try_status_fleet(d.path(), None, false, false, false, false, false, false, true, false, false, false, false).is_some()); }
#[test] fn fleet_6()  { let d = setup_state(); assert!(try_status_fleet(d.path(), None, false, false, false, false, false, false, false, true, false, false, false).is_some()); }
#[test] fn fleet_7()  { let d = setup_state(); assert!(try_status_fleet(d.path(), None, false, false, false, false, false, false, false, false, true, false, false).is_some()); }
#[test] fn fleet_8()  { let d = setup_state(); assert!(try_status_fleet(d.path(), None, false, false, false, false, false, false, false, false, false, true, false).is_some()); }
#[test] fn fleet_9()  { let d = setup_state(); assert!(try_status_fleet(d.path(), None, false, false, false, false, false, false, false, false, false, false, true).is_some()); }
#[test] fn fleet_none() { let d = setup_state(); assert!(try_status_fleet(d.path(), None, false, false, false, false, false, false, false, false, false, false, false).is_none()); }

// ── try_status_reports (11 flags, some Option) ──

#[test] fn reports_health_score() { let d = setup_state(); assert!(try_status_reports(d.path(), None, false, true, &None, false, false, false, None, false, &None, false, false, false).is_some()); }
#[test] fn reports_staleness() { let d = setup_state(); assert!(try_status_reports(d.path(), None, false, false, &Some("7d".into()), false, false, false, None, false, &None, false, false, false).is_some()); }
#[test] fn reports_cost_estimate() { let d = setup_state(); assert!(try_status_reports(d.path(), None, false, false, &None, true, false, false, None, false, &None, false, false, false).is_some()); }
#[test] fn reports_capacity() { let d = setup_state(); assert!(try_status_reports(d.path(), None, false, false, &None, false, true, false, None, false, &None, false, false, false).is_some()); }
#[test] fn reports_prediction() { let d = setup_state(); assert!(try_status_reports(d.path(), None, false, false, &None, false, false, true, None, false, &None, false, false, false).is_some()); }
#[test] fn reports_trend() { let d = setup_state(); assert!(try_status_reports(d.path(), None, false, false, &None, false, false, false, Some(10), false, &None, false, false, false).is_some()); }
#[test] fn reports_mttr() { let d = setup_state(); assert!(try_status_reports(d.path(), None, false, false, &None, false, false, false, None, true, &None, false, false, false).is_some()); }
#[test] fn reports_compliance() { let d = setup_state(); assert!(try_status_reports(d.path(), None, false, false, &None, false, false, false, None, false, &Some("pci".into()), false, false, false).is_some()); }
#[test] fn reports_sla() { let d = setup_state(); assert!(try_status_reports(d.path(), None, false, false, &None, false, false, false, None, false, &None, true, false, false).is_some()); }
#[test] fn reports_resource_age() { let d = setup_state(); assert!(try_status_reports(d.path(), None, false, false, &None, false, false, false, None, false, &None, false, true, false).is_some()); }
#[test] fn reports_drift_summary() { let d = setup_state(); assert!(try_status_reports(d.path(), None, false, false, &None, false, false, false, None, false, &None, false, false, true).is_some()); }
#[test] fn reports_none() { let d = setup_state(); assert!(try_status_reports(d.path(), None, false, false, &None, false, false, false, None, false, &None, false, false, false).is_none()); }

// ── try_status_queries_b (8 flags, some Option) ──

#[test] fn qb_since() { let d = setup_state(); assert!(try_status_queries_b(d.path(), None, false, &Some("1h".into()), false, None, false, false, false, &None, false).is_some()); }
#[test] fn qb_stale() { let d = setup_state(); assert!(try_status_queries_b(d.path(), None, false, &None, true, None, false, false, false, &None, false).is_some()); }
#[test] fn qb_threshold() { let d = setup_state(); assert!(try_status_queries_b(d.path(), None, false, &None, false, Some(80), false, false, false, &None, false).is_some()); }
#[test] fn qb_machines_only() { let d = setup_state(); assert!(try_status_queries_b(d.path(), None, false, &None, false, None, true, false, false, &None, false).is_some()); }
#[test] fn qb_resources_by_type() { let d = setup_state(); assert!(try_status_queries_b(d.path(), None, false, &None, false, None, false, true, false, &None, false).is_some()); }
#[test] fn qb_anomalies() { let d = setup_state(); assert!(try_status_queries_b(d.path(), None, false, &None, false, None, false, false, true, &None, false).is_some()); }
#[test] fn qb_diff_from() { let d = setup_state(); assert!(try_status_queries_b(d.path(), None, false, &None, false, None, false, false, false, &Some("gen-1".into()), false).is_some()); }
#[test] fn qb_count() { let d = setup_state(); assert!(try_status_queries_b(d.path(), None, false, &None, false, None, false, false, false, &None, true).is_some()); }
#[test] fn qb_none() { let d = setup_state(); assert!(try_status_queries_b(d.path(), None, false, &None, false, None, false, false, false, &None, false).is_none()); }

// ── try_status_display (10 flags, some Option) ──

#[test] fn display_prometheus() { let d = setup_state(); assert!(try_status_display(d.path(), None, false, &None, true, &None, &None, &None, false, false, false, None, &None).is_some()); }
#[test] fn display_timeline() { let d = setup_state(); assert!(try_status_display(d.path(), None, false, &None, false, &None, &None, &None, true, false, false, None, &None).is_some()); }
#[test] fn display_drift_details() { let d = setup_state(); assert!(try_status_display(d.path(), None, false, &None, false, &None, &None, &None, false, true, false, None, &None).is_some()); }
#[test] fn display_health() { let d = setup_state(); assert!(try_status_display(d.path(), None, false, &None, false, &None, &None, &None, false, false, true, None, &None).is_some()); }
#[test] fn display_stale() { let d = setup_state(); assert!(try_status_display(d.path(), None, false, &None, false, &None, &None, &None, false, false, false, Some(7), &None).is_some()); }
#[test] fn display_format() { let d = setup_state(); assert!(try_status_display(d.path(), None, false, &Some("yaml".into()), false, &None, &None, &None, false, false, false, None, &None).is_some()); }
#[test] fn display_expired() { let d = setup_state(); assert!(try_status_display(d.path(), None, false, &None, false, &Some("7d".into()), &None, &None, false, false, false, None, &None).is_some()); }
#[test] fn display_changes_since() { let d = setup_state(); assert!(try_status_display(d.path(), None, false, &None, false, &None, &Some("2025-01-01".into()), &None, false, false, false, None, &None).is_some()); }
#[test] fn display_summary_by() { let d = setup_state(); assert!(try_status_display(d.path(), None, false, &None, false, &None, &None, &Some("type".into()), false, false, false, None, &None).is_some()); }
#[test] fn display_failed_since() { let d = setup_state(); assert!(try_status_display(d.path(), None, false, &None, false, &None, &None, &None, false, false, false, None, &Some("1h".into())).is_some()); }
#[test] fn display_none() { let d = setup_state(); assert!(try_status_display(d.path(), None, false, &None, false, &None, &None, &None, false, false, false, None, &None).is_none()); }

// ── try_status_queries_a (10 flags, some Option) ──

#[test] fn qa_convergence_rate() { let d = setup_state(); assert!(try_status_queries_a(d.path(), None, false, true, false, false, false, &None, &None, false, false, &None, false).is_some()); }
#[test] fn qa_top_failures() { let d = setup_state(); assert!(try_status_queries_a(d.path(), None, false, false, true, false, false, &None, &None, false, false, &None, false).is_some()); }
#[test] fn qa_dependency_health() { let d = setup_state(); assert!(try_status_queries_a(d.path(), None, false, false, false, true, false, &None, &None, false, false, &None, false).is_some()); }
#[test] fn qa_histogram() { let d = setup_state(); assert!(try_status_queries_a(d.path(), None, false, false, false, false, true, &None, &None, false, false, &None, false).is_some()); }
#[test] fn qa_alerts() { let d = setup_state(); assert!(try_status_queries_a(d.path(), None, false, false, false, false, false, &None, &None, true, false, &None, false).is_some()); }
#[test] fn qa_compact() { let d = setup_state(); assert!(try_status_queries_a(d.path(), None, false, false, false, false, false, &None, &None, false, true, &None, false).is_some()); }
#[test] fn qa_json_lines() { let d = setup_state(); assert!(try_status_queries_a(d.path(), None, false, false, false, false, false, &None, &None, false, false, &None, true).is_some()); }
#[test] fn qa_none() { let d = setup_state(); assert!(try_status_queries_a(d.path(), None, false, false, false, false, false, &None, &None, false, false, &None, false).is_none()); }
