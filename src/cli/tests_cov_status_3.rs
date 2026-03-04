//! Coverage tests for status_compliance, status_alerts, status_cost,
//! status_failures, status_resources, status_resource_detail.

use super::status_alerts::*;
use super::status_compliance::*;
use super::status_cost::*;
use super::status_failures::*;
use super::status_resource_detail::*;
use super::status_resources::*;

#[cfg(test)]
mod tests {
    use super::*;

    fn write_yaml(dir: &std::path::Path, name: &str, content: &str) {
        let p = dir.join(name);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&p, content).unwrap();
    }

    fn state_lock() -> &'static str {
        "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: converged\n    hash: \"blake3:abc\"\n  g:\n    type: service\n    status: drifted\n    hash: \"blake3:def\"\n  h:\n    type: package\n    status: failed\n    hash: \"blake3:ghi\"\n"
    }

    fn make_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for m in &["web", "db"] {
            write_yaml(dir.path(), &format!("{m}/state.lock.yaml"), state_lock());
            write_yaml(dir.path(), &format!("{m}/state.lock.yaml"), state_lock());
        }
        dir
    }

    fn make_dir_with_events() -> tempfile::TempDir {
        let dir = make_dir();
        let ev = r#"{"resource":"f","status":"converged","timestamp":"2026-02-28T01:00:00Z"}"#;
        for m in &["web", "db"] {
            write_yaml(
                dir.path(),
                &format!("{m}.events.jsonl"),
                &format!("{ev}\n{ev}\n"),
            );
            write_yaml(
                dir.path(),
                &format!("{m}/events.jsonl"),
                "2026-02-28T01:00:00Z apply success\n",
            );
        }
        dir
    }

    // ── status_compliance ──
    #[test]
    fn compliance_plain() {
        let d = make_dir();
        let _ = cmd_status_compliance(d.path(), None, "baseline", false);
    }
    #[test]
    fn compliance_json() {
        let d = make_dir();
        let _ = cmd_status_compliance(d.path(), Some("web"), "baseline", true);
    }
    #[test]
    fn compliance_report_plain() {
        let d = make_dir();
        let _ = cmd_status_compliance_report(d.path(), None, "baseline", false);
    }
    #[test]
    fn compliance_report_json() {
        let d = make_dir();
        let _ = cmd_status_compliance_report(d.path(), Some("web"), "baseline", true);
    }
    #[test]
    fn security_posture_plain() {
        let d = make_dir();
        let _ = cmd_status_security_posture(d.path(), None, false);
    }
    #[test]
    fn security_posture_json() {
        let d = make_dir();
        let _ = cmd_status_security_posture(d.path(), Some("web"), true);
    }
    #[test]
    fn audit_trail_plain() {
        let d = make_dir_with_events();
        let _ = cmd_status_audit_trail(d.path(), None, false);
    }
    #[test]
    fn audit_trail_json() {
        let d = make_dir_with_events();
        let _ = cmd_status_audit_trail(d.path(), Some("web"), true);
    }
    #[test]
    fn sla_report_plain() {
        let d = make_dir();
        let _ = cmd_status_sla_report(d.path(), None, false);
    }
    #[test]
    fn sla_report_json() {
        let d = make_dir();
        let _ = cmd_status_sla_report(d.path(), Some("web"), true);
    }
    #[test]
    fn dependency_health_plain() {
        let d = make_dir();
        let _ = cmd_status_dependency_health(d.path(), None, false);
    }
    #[test]
    fn dependency_health_json() {
        let d = make_dir();
        let _ = cmd_status_dependency_health(d.path(), Some("web"), true);
    }

    // ── status_alerts ──
    #[test]
    fn alerts_plain() {
        let d = make_dir();
        let _ = cmd_status_alerts(d.path(), None, false);
    }
    #[test]
    fn alerts_json() {
        let d = make_dir();
        let _ = cmd_status_alerts(d.path(), Some("web"), true);
    }
    #[test]
    fn uptime_plain() {
        let d = make_dir();
        let _ = cmd_status_uptime(d.path(), None, false);
    }
    #[test]
    fn uptime_json() {
        let d = make_dir();
        let _ = cmd_status_uptime(d.path(), Some("web"), true);
    }
    #[test]
    fn diagnostic_plain() {
        let d = make_dir();
        let _ = cmd_status_diagnostic(d.path(), None, false);
    }
    #[test]
    fn diagnostic_json() {
        let d = make_dir();
        let _ = cmd_status_diagnostic(d.path(), Some("web"), true);
    }

    // ── status_cost ──
    #[test]
    fn staleness_report_plain() {
        let d = make_dir();
        let _ = cmd_status_staleness_report(d.path(), None, "7d", false);
    }
    #[test]
    fn staleness_report_json() {
        let d = make_dir();
        let _ = cmd_status_staleness_report(d.path(), Some("web"), "1d", true);
    }
    #[test]
    fn cost_estimate_plain() {
        let d = make_dir();
        let _ = cmd_status_cost_estimate(d.path(), None, false);
    }
    #[test]
    fn cost_estimate_json() {
        let d = make_dir();
        let _ = cmd_status_cost_estimate(d.path(), Some("web"), true);
    }
    #[test]
    fn capacity_plain() {
        let d = make_dir();
        let _ = cmd_status_capacity(d.path(), None, false);
    }
    #[test]
    fn capacity_json() {
        let d = make_dir();
        let _ = cmd_status_capacity(d.path(), Some("web"), true);
    }

    // ── status_failures ──
    #[test]
    fn top_failures_plain() {
        let d = make_dir();
        let _ = cmd_status_top_failures(d.path(), None, false);
    }
    #[test]
    fn top_failures_json() {
        let d = make_dir();
        let _ = cmd_status_top_failures(d.path(), Some("web"), true);
    }
    #[test]
    fn failed_since_plain() {
        let d = make_dir();
        let _ = cmd_status_failed_since(d.path(), None, "2020-01-01", false);
    }
    #[test]
    fn failed_since_json() {
        let d = make_dir();
        let _ = cmd_status_failed_since(d.path(), Some("web"), "2020-01-01", true);
    }
    #[test]
    fn failed_resources_plain() {
        let d = make_dir();
        let _ = cmd_status_failed_resources(d.path(), None, false);
    }
    #[test]
    fn failed_resources_json() {
        let d = make_dir();
        let _ = cmd_status_failed_resources(d.path(), Some("web"), true);
    }
    #[test]
    fn hash_verify_plain() {
        let d = make_dir();
        let _ = cmd_status_hash_verify(d.path(), None, false);
    }
    #[test]
    fn hash_verify_json() {
        let d = make_dir();
        let _ = cmd_status_hash_verify(d.path(), Some("web"), true);
    }
    #[test]
    fn lock_age_plain() {
        let d = make_dir();
        let _ = cmd_status_lock_age(d.path(), None, false);
    }
    #[test]
    fn lock_age_json() {
        let d = make_dir();
        let _ = cmd_status_lock_age(d.path(), Some("web"), true);
    }
    #[test]
    fn config_hash_plain() {
        let d = make_dir();
        let _ = cmd_status_config_hash(d.path(), None, false);
    }
    #[test]
    fn config_hash_json() {
        let d = make_dir();
        let _ = cmd_status_config_hash(d.path(), Some("web"), true);
    }
    #[test]
    fn recommendations_plain() {
        let d = make_dir();
        let _ = cmd_status_recommendations(d.path(), None, false);
    }
    #[test]
    fn recommendations_json() {
        let d = make_dir();
        let _ = cmd_status_recommendations(d.path(), Some("web"), true);
    }

    // ── status_resources ──
    #[test]
    fn resource_age_plain() {
        let d = make_dir();
        let _ = cmd_status_resource_age(d.path(), None, false);
    }
    #[test]
    fn resource_age_json() {
        let d = make_dir();
        let _ = cmd_status_resource_age(d.path(), Some("web"), true);
    }
    #[test]
    fn resource_cost_plain() {
        let d = make_dir();
        let _ = cmd_status_resource_cost(d.path(), None, false);
    }
    #[test]
    fn resource_cost_json() {
        let d = make_dir();
        let _ = cmd_status_resource_cost(d.path(), Some("web"), true);
    }
    #[test]
    fn resource_size_plain() {
        let d = make_dir();
        let _ = cmd_status_resource_size(d.path(), None, false);
    }
    #[test]
    fn resource_size_json() {
        let d = make_dir();
        let _ = cmd_status_resource_size(d.path(), Some("web"), true);
    }
    #[test]
    fn resource_graph_plain() {
        let d = make_dir();
        let _ = cmd_status_resource_graph(d.path(), None, false);
    }
    #[test]
    fn resource_graph_json() {
        let d = make_dir();
        let _ = cmd_status_resource_graph(d.path(), Some("web"), true);
    }

    // ── status_resource_detail ──
    #[test]
    fn resource_timeline_plain() {
        let d = make_dir_with_events();
        let _ = cmd_status_resource_timeline(d.path(), None, false);
    }
    #[test]
    fn resource_timeline_json() {
        let d = make_dir_with_events();
        let _ = cmd_status_resource_timeline(d.path(), Some("web"), true);
    }
    #[test]
    fn resource_dependencies_plain() {
        let d = make_dir();
        let _ = cmd_status_resource_dependencies(d.path(), None, false);
    }
    #[test]
    fn resource_dependencies_json() {
        let d = make_dir();
        let _ = cmd_status_resource_dependencies(d.path(), Some("web"), true);
    }
    #[test]
    fn resource_inputs_plain() {
        let d = make_dir();
        let _ = cmd_status_resource_inputs(d.path(), None, false);
    }
    #[test]
    fn resource_inputs_json() {
        let d = make_dir();
        let _ = cmd_status_resource_inputs(d.path(), Some("web"), true);
    }
    #[test]
    fn resource_types_summary_plain() {
        let d = make_dir();
        let _ = cmd_status_resource_types_summary(d.path(), None, false);
    }
    #[test]
    fn resource_types_summary_json() {
        let d = make_dir();
        let _ = cmd_status_resource_types_summary(d.path(), Some("web"), true);
    }
    #[test]
    fn resource_health_plain() {
        let d = make_dir();
        let _ = cmd_status_resource_health(d.path(), None, false);
    }
    #[test]
    fn resource_health_json() {
        let d = make_dir();
        let _ = cmd_status_resource_health(d.path(), Some("web"), true);
    }

    #[test]
    fn tally_machine_health_with_data() {
        let d = make_dir();
        let (_t, _c, _f, _dr) = tally_machine_health(d.path(), "web");
    }

    #[test]
    fn tally_machine_health_missing() {
        let d = tempfile::tempdir().unwrap();
        let (t, _c, _f, _dr) = tally_machine_health(d.path(), "nonexistent");
        assert_eq!(t, 0);
    }

    #[test]
    fn machine_health_summary_plain() {
        let d = make_dir();
        let _ = cmd_status_machine_health_summary(d.path(), None, false);
    }
    #[test]
    fn machine_health_summary_json() {
        let d = make_dir();
        let _ = cmd_status_machine_health_summary(d.path(), Some("web"), true);
    }
    #[test]
    fn last_apply_status_plain() {
        let d = make_dir_with_events();
        let _ = cmd_status_last_apply_status(d.path(), None, false);
    }
    #[test]
    fn last_apply_status_json() {
        let d = make_dir_with_events();
        let _ = cmd_status_last_apply_status(d.path(), Some("web"), true);
    }
    #[test]
    fn resource_staleness_plain() {
        let d = make_dir();
        let _ = cmd_status_resource_staleness(d.path(), None, false);
    }
    #[test]
    fn resource_staleness_json() {
        let d = make_dir();
        let _ = cmd_status_resource_staleness(d.path(), Some("web"), true);
    }

    // ── empty-state edge cases ──
    #[test]
    fn compliance_empty() {
        let d = tempfile::tempdir().unwrap();
        let _ = cmd_status_compliance(d.path(), None, "none", false);
    }
    #[test]
    fn alerts_empty() {
        let d = tempfile::tempdir().unwrap();
        let _ = cmd_status_alerts(d.path(), None, false);
    }
    #[test]
    fn recommendations_empty() {
        let d = tempfile::tempdir().unwrap();
        let _ = cmd_status_recommendations(d.path(), None, true);
    }
    #[test]
    fn timeline_empty() {
        let d = tempfile::tempdir().unwrap();
        let _ = cmd_status_resource_timeline(d.path(), None, false);
    }
}
