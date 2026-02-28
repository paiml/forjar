//! Coverage tests for status_drift, status_queries, status_trends, status_fleet.

use super::status_drift::*;
use super::status_queries::*;
use super::status_trends::*;
use super::status_fleet::*;

#[cfg(test)]
mod tests {
    use super::*;

    fn write_file(dir: &std::path::Path, name: &str, content: &str) {
        let p = dir.join(name);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&p, content).unwrap();
    }

    fn state_lock_yaml() -> &'static str {
        "schema: \"1.0\"\n\
         machine: web\n\
         hostname: web\n\
         generated_at: \"2026-02-28T00:00:00Z\"\n\
         generator: forjar\n\
         blake3_version: \"1.8\"\n\
         resources:\n\
         \x20 f:\n\
         \x20   type: file\n\
         \x20   status: converged\n\
         \x20   hash: \"blake3:abc123def456\"\n\
         \x20   applied_at: \"2026-02-27T12:00:00Z\"\n\
         \x20   duration_seconds: 0.5\n\
         \x20 g:\n\
         \x20   type: service\n\
         \x20   status: drifted\n\
         \x20   hash: \"blake3:def456abc789\"\n\
         \x20   applied_at: \"2026-02-26T12:00:00Z\"\n\
         \x20   duration_seconds: 1.2\n\
         \x20 h:\n\
         \x20   type: package\n\
         \x20   status: failed\n\
         \x20   hash: \"blake3:ghi789jkl012\"\n\
         \x20   applied_at: \"2026-02-25T12:00:00Z\"\n\
         \x20   duration_seconds: 3.0\n"
    }

    fn events_jsonl() -> &'static str {
        "{\"timestamp\":\"2026-02-25T10:00:00Z\",\"resource\":\"g\",\"status\":\"Failed\"}\n\
         {\"timestamp\":\"2026-02-25T12:00:00Z\",\"resource\":\"g\",\"status\":\"Drifted\"}\n\
         {\"timestamp\":\"2026-02-26T10:00:00Z\",\"resource\":\"g\",\"status\":\"Converged\"}\n\
         {\"timestamp\":\"2026-02-26T14:00:00Z\",\"resource\":\"h\",\"status\":\"Failed\"}\n\
         {\"timestamp\":\"2026-02-27T10:00:00Z\",\"resource\":\"h\",\"status\":\"Failed\"}\n\
         {\"timestamp\":\"2026-02-27T14:00:00Z\",\"resource\":\"h\",\"status\":\"Converged\"}\n\
         {\"timestamp\":\"2026-02-28T10:00:00Z\",\"resource\":\"f\",\"status\":\"Converged\"}\n"
    }

    /// Set up a state dir with two machines (web, db) each having:
    ///   - {machine}/state.lock.yaml  (for discover_machines + load_lock)
    ///   - {machine}.lock.yaml        (for functions reading directly)
    ///   - {machine}.events.jsonl     (for event-reading functions)
    fn make_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let sd = dir.path();
        for m in &["web", "db"] {
            write_file(sd, &format!("{}/state.lock.yaml", m), state_lock_yaml());
            write_file(sd, &format!("{}.lock.yaml", m), state_lock_yaml());
            write_file(sd, &format!("{}.events.jsonl", m), events_jsonl());
        }
        dir
    }

    // ──────────────────────────────────────────────
    // status_drift.rs
    // ──────────────────────────────────────────────

    #[test]
    fn drift_details_plain() {
        let d = make_dir();
        let _ = cmd_status_drift_details(d.path(), None, false);
    }

    #[test]
    fn drift_details_json() {
        let d = make_dir();
        let _ = cmd_status_drift_details(d.path(), Some("web"), true);
    }

    #[test]
    fn drift_summary_plain() {
        let d = make_dir();
        let _ = cmd_status_drift_summary(d.path(), None, false);
    }

    #[test]
    fn drift_summary_json() {
        let d = make_dir();
        let _ = cmd_status_drift_summary(d.path(), Some("web"), true);
    }

    #[test]
    fn drift_velocity_plain() {
        let d = make_dir();
        let _ = cmd_status_drift_velocity(d.path(), None, false);
    }

    #[test]
    fn drift_velocity_json() {
        let d = make_dir();
        let _ = cmd_status_drift_velocity(d.path(), Some("web"), true);
    }

    #[test]
    fn drift_forecast_plain() {
        let d = make_dir();
        let _ = cmd_status_drift_forecast(d.path(), None, false);
    }

    #[test]
    fn drift_forecast_json() {
        let d = make_dir();
        let _ = cmd_status_drift_forecast(d.path(), Some("web"), true);
    }

    #[test]
    fn drift_details_all_plain() {
        let d = make_dir();
        let _ = cmd_status_drift_details_all(d.path(), false);
    }

    #[test]
    fn drift_details_all_json() {
        let d = make_dir();
        let _ = cmd_status_drift_details_all(d.path(), true);
    }

    #[test]
    fn drift_trend_plain() {
        let d = make_dir();
        let _ = cmd_status_drift_trend(d.path(), None, false);
    }

    #[test]
    fn drift_trend_json() {
        let d = make_dir();
        let _ = cmd_status_drift_trend(d.path(), Some("web"), true);
    }

    #[test]
    fn config_drift_plain() {
        let d = make_dir();
        let _ = cmd_status_config_drift(d.path(), None, false);
    }

    #[test]
    fn config_drift_json() {
        let d = make_dir();
        let _ = cmd_status_config_drift(d.path(), Some("db"), true);
    }

    // ──────────────────────────────────────────────
    // status_queries.rs
    // ──────────────────────────────────────────────

    #[test]
    fn count_plain() {
        let d = make_dir();
        let _ = cmd_status_count(d.path(), None, false);
    }

    #[test]
    fn count_json() {
        let d = make_dir();
        let _ = cmd_status_count(d.path(), Some("web"), true);
    }

    #[test]
    fn format_json() {
        let d = make_dir();
        let _ = cmd_status_format(d.path(), None, "json");
    }

    #[test]
    fn format_csv() {
        let d = make_dir();
        let _ = cmd_status_format(d.path(), Some("web"), "csv");
    }

    #[test]
    fn format_table() {
        let d = make_dir();
        let _ = cmd_status_format(d.path(), None, "table");
    }

    #[test]
    fn format_unknown() {
        let d = make_dir();
        let r = cmd_status_format(d.path(), None, "xml");
        assert!(r.is_err());
    }

    #[test]
    fn compact_plain() {
        let d = make_dir();
        let _ = cmd_status_compact(d.path(), None, false);
    }

    #[test]
    fn compact_json() {
        let d = make_dir();
        let _ = cmd_status_compact(d.path(), Some("web"), true);
    }

    #[test]
    fn json_lines_no_filter() {
        let d = make_dir();
        let _ = cmd_status_json_lines(d.path(), None);
    }

    #[test]
    fn json_lines_with_filter() {
        let d = make_dir();
        let _ = cmd_status_json_lines(d.path(), Some("db"));
    }

    #[test]
    fn machines_only_plain() {
        let d = make_dir();
        let _ = cmd_status_machines_only(d.path(), None, false);
    }

    #[test]
    fn machines_only_json() {
        let d = make_dir();
        let _ = cmd_status_machines_only(d.path(), Some("web"), true);
    }

    #[test]
    fn resources_by_type_plain() {
        let d = make_dir();
        let _ = cmd_status_resources_by_type(d.path(), None, false);
    }

    #[test]
    fn resources_by_type_json() {
        let d = make_dir();
        let _ = cmd_status_resources_by_type(d.path(), Some("db"), true);
    }

    // ──────────────────────────────────────────────
    // status_trends.rs
    // ──────────────────────────────────────────────

    #[test]
    fn change_frequency_plain() {
        let d = make_dir();
        let _ = cmd_status_change_frequency(d.path(), None, false);
    }

    #[test]
    fn change_frequency_json() {
        let d = make_dir();
        let _ = cmd_status_change_frequency(d.path(), Some("web"), true);
    }

    #[test]
    fn last_apply_duration_plain() {
        let d = make_dir();
        let _ = cmd_status_last_apply_duration(d.path(), None, false);
    }

    #[test]
    fn last_apply_duration_json() {
        let d = make_dir();
        let _ = cmd_status_last_apply_duration(d.path(), Some("db"), true);
    }

    #[test]
    fn trend_plain() {
        let d = make_dir();
        let _ = cmd_status_trend(d.path(), None, 5, false);
    }

    #[test]
    fn trend_json() {
        let d = make_dir();
        let _ = cmd_status_trend(d.path(), Some("web"), 3, true);
    }

    #[test]
    fn prediction_plain() {
        let d = make_dir();
        let _ = cmd_status_prediction(d.path(), None, false);
    }

    #[test]
    fn prediction_json() {
        let d = make_dir();
        let _ = cmd_status_prediction(d.path(), Some("web"), true);
    }

    #[test]
    fn histogram_plain() {
        let d = make_dir();
        let _ = cmd_status_histogram(d.path(), None, false);
    }

    #[test]
    fn histogram_json() {
        let d = make_dir();
        let _ = cmd_status_histogram(d.path(), Some("db"), true);
    }

    #[test]
    fn mttr_plain() {
        let d = make_dir();
        let _ = cmd_status_mttr(d.path(), None, false);
    }

    #[test]
    fn mttr_json() {
        let d = make_dir();
        let _ = cmd_status_mttr(d.path(), Some("web"), true);
    }

    // ──────────────────────────────────────────────
    // status_fleet.rs
    // ──────────────────────────────────────────────

    #[test]
    fn fleet_overview_plain() {
        let d = make_dir();
        let _ = cmd_status_fleet_overview(d.path(), false);
    }

    #[test]
    fn fleet_overview_json() {
        let d = make_dir();
        let _ = cmd_status_fleet_overview(d.path(), true);
    }

    #[test]
    fn machine_health_plain() {
        let d = make_dir();
        let _ = cmd_status_machine_health(d.path(), None, false);
    }

    #[test]
    fn machine_health_json() {
        let d = make_dir();
        let _ = cmd_status_machine_health(d.path(), Some("web"), true);
    }

    #[test]
    fn machine_summary_plain() {
        let d = make_dir();
        let _ = cmd_status_machine_summary(d.path(), None, false);
    }

    #[test]
    fn machine_summary_json() {
        let d = make_dir();
        let _ = cmd_status_machine_summary(d.path(), Some("db"), true);
    }

    #[test]
    fn executive_summary_plain() {
        let d = make_dir();
        let _ = cmd_status_executive_summary(d.path(), false);
    }

    #[test]
    fn executive_summary_json() {
        let d = make_dir();
        let _ = cmd_status_executive_summary(d.path(), true);
    }

    #[test]
    fn pipeline_status_plain() {
        let d = make_dir();
        let _ = cmd_status_pipeline_status(d.path(), None, false);
    }

    #[test]
    fn pipeline_status_json() {
        let d = make_dir();
        let _ = cmd_status_pipeline_status(d.path(), Some("web"), true);
    }

    // ──────────────────────────────────────────────
    // Edge cases: empty state dir
    // ──────────────────────────────────────────────

    #[test]
    fn empty_dir_drift_details() {
        let d = tempfile::tempdir().unwrap();
        let _ = cmd_status_drift_details(d.path(), None, false);
    }

    #[test]
    fn empty_dir_count() {
        let d = tempfile::tempdir().unwrap();
        let _ = cmd_status_count(d.path(), None, true);
    }

    #[test]
    fn empty_dir_compact() {
        let d = tempfile::tempdir().unwrap();
        let _ = cmd_status_compact(d.path(), None, false);
    }

    #[test]
    fn empty_dir_json_lines() {
        let d = tempfile::tempdir().unwrap();
        let _ = cmd_status_json_lines(d.path(), None);
    }

    #[test]
    fn empty_dir_machines_only() {
        let d = tempfile::tempdir().unwrap();
        let _ = cmd_status_machines_only(d.path(), None, false);
    }

    #[test]
    fn empty_dir_fleet_overview() {
        let d = tempfile::tempdir().unwrap();
        let _ = cmd_status_fleet_overview(d.path(), false);
    }

    #[test]
    fn empty_dir_executive_summary() {
        let d = tempfile::tempdir().unwrap();
        let _ = cmd_status_executive_summary(d.path(), true);
    }

    #[test]
    fn empty_dir_histogram() {
        let d = tempfile::tempdir().unwrap();
        let _ = cmd_status_histogram(d.path(), None, false);
    }

    #[test]
    fn empty_dir_prediction() {
        let d = tempfile::tempdir().unwrap();
        let _ = cmd_status_prediction(d.path(), None, false);
    }

    #[test]
    fn empty_dir_change_frequency() {
        let d = tempfile::tempdir().unwrap();
        let _ = cmd_status_change_frequency(d.path(), None, true);
    }
}
