//! Coverage tests for status_convergence, status_health, status_observability.

use super::status_convergence::*;
use super::status_health::*;
use super::status_observability::*;
use std::io::Write;

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
        "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: converged\n    hash: \"blake3:abc\"\n    applied_at: \"2026-02-28T00:00:00Z\"\n    duration_seconds: 0.5\n  g:\n    type: service\n    status: drifted\n    hash: \"blake3:def\"\n  h:\n    type: package\n    status: failed\n    hash: \"blake3:ghi\"\n"
    }

    fn state_lock_db() -> &'static str {
        "schema: \"1.0\"\nmachine: db\nhostname: db\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: converged\n    hash: \"blake3:abc\"\n    applied_at: \"2026-02-28T00:00:00Z\"\n    duration_seconds: 0.3\n  g:\n    type: service\n    status: drifted\n    hash: \"blake3:def\"\n  h:\n    type: package\n    status: failed\n    hash: \"blake3:ghi\"\n"
    }

    fn make_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web/state.lock.yaml", state_lock());
        write_yaml(dir.path(), "db/state.lock.yaml", state_lock_db());
        dir
    }

    // ── status_convergence: cmd_status_timeline ──

    #[test]
    fn test_cov_cmd_status_timeline() {
        let dir = make_dir();
        let _ = cmd_status_timeline(dir.path(), None, false);
    }

    #[test]
    fn test_cov_cmd_status_timeline_json() {
        let dir = make_dir();
        let _ = cmd_status_timeline(dir.path(), None, true);
    }

    // ── status_convergence: cmd_status_changes_since ──

    #[test]
    fn test_cov_cmd_status_changes_since() {
        let dir = make_dir();
        let _ = cmd_status_changes_since(dir.path(), "HEAD", false);
    }

    #[test]
    fn test_cov_cmd_status_changes_since_json() {
        let dir = make_dir();
        let _ = cmd_status_changes_since(dir.path(), "HEAD", true);
    }

    // ── status_convergence: cmd_status_since ──

    #[test]
    fn test_cov_cmd_status_since() {
        let dir = make_dir();
        let _ = cmd_status_since(dir.path(), None, "1h", false);
    }

    #[test]
    fn test_cov_cmd_status_since_json() {
        let dir = make_dir();
        let _ = cmd_status_since(dir.path(), None, "1h", true);
    }

    // ── status_convergence: cmd_status_summary_by ──

    #[test]
    fn test_cov_cmd_status_summary_by_machine() {
        let dir = make_dir();
        let _ = cmd_status_summary_by(dir.path(), None, "machine", false);
    }

    #[test]
    fn test_cov_cmd_status_summary_by_machine_json() {
        let dir = make_dir();
        let _ = cmd_status_summary_by(dir.path(), None, "machine", true);
    }

    // ── status_convergence: cmd_status_convergence_rate ──

    #[test]
    fn test_cov_cmd_status_convergence_rate() {
        let dir = make_dir();
        let _ = cmd_status_convergence_rate(dir.path(), None, false);
    }

    #[test]
    fn test_cov_cmd_status_convergence_rate_json() {
        let dir = make_dir();
        let _ = cmd_status_convergence_rate(dir.path(), None, true);
    }

    // ── status_convergence: cmd_status_convergence_time ──

    #[test]
    fn test_cov_cmd_status_convergence_time() {
        let dir = make_dir();
        let _ = cmd_status_convergence_time(dir.path(), None, false);
    }

    #[test]
    fn test_cov_cmd_status_convergence_time_json() {
        let dir = make_dir();
        let _ = cmd_status_convergence_time(dir.path(), None, true);
    }

    // ── status_convergence: cmd_status_convergence_history ──

    #[test]
    fn test_cov_cmd_status_convergence_history() {
        let dir = make_dir();
        let _ = cmd_status_convergence_history(dir.path(), None, false);
    }

    #[test]
    fn test_cov_cmd_status_convergence_history_json() {
        let dir = make_dir();
        let _ = cmd_status_convergence_history(dir.path(), None, true);
    }

    // ── status_health: cmd_status_health ──

    #[test]
    fn test_cov_cmd_status_health() {
        let dir = make_dir();
        // status_health reads lock.yaml (not state.lock.yaml), write that too
        write_yaml(dir.path(), "web/lock.yaml", state_lock());
        let _ = cmd_status_health(dir.path(), None, false);
    }

    #[test]
    fn test_cov_cmd_status_health_json() {
        let dir = make_dir();
        write_yaml(dir.path(), "web/lock.yaml", state_lock());
        let _ = cmd_status_health(dir.path(), None, true);
    }

    // ── status_health: cmd_status_stale ──

    #[test]
    fn test_cov_cmd_status_stale() {
        let dir = make_dir();
        write_yaml(dir.path(), "web/lock.yaml", state_lock());
        let _ = cmd_status_stale(dir.path(), None, 0, false);
    }

    #[test]
    fn test_cov_cmd_status_stale_json() {
        let dir = make_dir();
        write_yaml(dir.path(), "web/lock.yaml", state_lock());
        let _ = cmd_status_stale(dir.path(), None, 0, true);
    }

    // ── status_health: cmd_status_expired ──

    #[test]
    fn test_cov_cmd_status_expired() {
        let dir = make_dir();
        let _ = cmd_status_expired(dir.path(), None, "1h", false);
    }

    #[test]
    fn test_cov_cmd_status_expired_json() {
        let dir = make_dir();
        let _ = cmd_status_expired(dir.path(), None, "1h", true);
    }

    // ── status_health: cmd_status_stale_resources ──

    #[test]
    fn test_cov_cmd_status_stale_resources() {
        let dir = make_dir();
        let _ = cmd_status_stale_resources(dir.path(), None, false);
    }

    #[test]
    fn test_cov_cmd_status_stale_resources_json() {
        let dir = make_dir();
        let _ = cmd_status_stale_resources(dir.path(), None, true);
    }

    // ── status_health: cmd_status_health_threshold ──

    #[test]
    fn test_cov_cmd_status_health_threshold() {
        let dir = make_dir();
        let _ = cmd_status_health_threshold(dir.path(), None, 50, false);
    }

    #[test]
    fn test_cov_cmd_status_health_threshold_json() {
        let dir = make_dir();
        let _ = cmd_status_health_threshold(dir.path(), None, 50, true);
    }

    // ── status_health: cmd_status_health_score ──

    #[test]
    fn test_cov_cmd_status_health_score() {
        let dir = make_dir();
        let _ = cmd_status_health_score(dir.path(), None, false);
    }

    #[test]
    fn test_cov_cmd_status_health_score_json() {
        let dir = make_dir();
        let _ = cmd_status_health_score(dir.path(), None, true);
    }

    // ── status_observability: cmd_status_prometheus ──

    #[test]
    fn test_cov_cmd_status_prometheus() {
        let dir = make_dir();
        let _ = cmd_status_prometheus(dir.path(), None);
    }

    #[test]
    fn test_cov_cmd_status_prometheus_filtered() {
        let dir = make_dir();
        let _ = cmd_status_prometheus(dir.path(), Some("web"));
    }

    // ── status_observability: cmd_status_export ──

    #[test]
    fn test_cov_cmd_status_export() {
        let dir = make_dir();
        let out = dir.path().join("export.json");
        let _ = cmd_status_export(dir.path(), None, &out, false);
    }

    #[test]
    fn test_cov_cmd_status_export_json() {
        let dir = make_dir();
        let out = dir.path().join("export.json");
        let _ = cmd_status_export(dir.path(), None, &out, true);
    }

    // ── status_observability: cmd_status_anomalies ──

    #[test]
    fn test_cov_cmd_status_anomalies() {
        let dir = make_dir();
        let _ = cmd_status_anomalies(dir.path(), None, false);
    }

    #[test]
    fn test_cov_cmd_status_anomalies_json() {
        let dir = make_dir();
        let _ = cmd_status_anomalies(dir.path(), None, true);
    }

    // ── status_observability: cmd_status_diff_from ──

    #[test]
    fn test_cov_cmd_status_diff_from() {
        let dir = make_dir();
        // Create a snapshot directory with data to diff against
        write_yaml(
            dir.path(),
            ".snapshots/snap1/web/state.lock.yaml",
            state_lock(),
        );
        write_yaml(
            dir.path(),
            ".snapshots/snap1/db/state.lock.yaml",
            state_lock_db(),
        );
        let _ = cmd_status_diff_from(dir.path(), "snap1", false);
    }

    #[test]
    fn test_cov_cmd_status_diff_from_json() {
        let dir = make_dir();
        write_yaml(
            dir.path(),
            ".snapshots/snap1/web/state.lock.yaml",
            state_lock(),
        );
        let _ = cmd_status_diff_from(dir.path(), "snap1", true);
    }

    // ── status_observability: cmd_status_error_summary ──

    #[test]
    fn test_cov_cmd_status_error_summary() {
        let dir = make_dir();
        let _ = cmd_status_error_summary(dir.path(), None, false);
    }

    #[test]
    fn test_cov_cmd_status_error_summary_json() {
        let dir = make_dir();
        let _ = cmd_status_error_summary(dir.path(), None, true);
    }

    // ── Extra coverage: machine filter paths ──

    #[test]
    fn test_cov_timeline_with_filter() {
        let dir = make_dir();
        let _ = cmd_status_timeline(dir.path(), Some("web"), false);
    }

    #[test]
    fn test_cov_summary_by_type() {
        let dir = make_dir();
        let _ = cmd_status_summary_by(dir.path(), None, "type", false);
    }

    #[test]
    fn test_cov_summary_by_status() {
        let dir = make_dir();
        let _ = cmd_status_summary_by(dir.path(), None, "status", false);
    }

    #[test]
    fn test_cov_convergence_rate_filtered() {
        let dir = make_dir();
        let _ = cmd_status_convergence_rate(dir.path(), Some("web"), false);
    }

    #[test]
    fn test_cov_anomalies_filtered() {
        let dir = make_dir();
        let _ = cmd_status_anomalies(dir.path(), Some("web"), false);
    }

    #[test]
    fn test_cov_health_threshold_fail() {
        let dir = make_dir();
        // threshold 100 should fail since not all converged
        let _ = cmd_status_health_threshold(dir.path(), None, 100, false);
    }

    #[test]
    fn test_cov_diff_from_missing_snapshot() {
        let dir = make_dir();
        let result = cmd_status_diff_from(dir.path(), "nonexistent", false);
        assert!(result.is_err());
    }

    #[test]
    fn test_cov_since_with_filter() {
        let dir = make_dir();
        let _ = cmd_status_since(dir.path(), Some("web"), "30m", false);
    }

    #[test]
    fn test_cov_stale_resources_filtered() {
        let dir = make_dir();
        let _ = cmd_status_stale_resources(dir.path(), Some("web"), false);
    }

    #[test]
    fn test_cov_empty_dir_timeline() {
        let dir = tempfile::tempdir().unwrap();
        let _ = cmd_status_timeline(dir.path(), None, false);
    }

    #[test]
    fn test_cov_empty_dir_health() {
        let dir = tempfile::tempdir().unwrap();
        let _ = cmd_status_health(dir.path(), None, false);
    }

    #[test]
    fn test_cov_empty_dir_prometheus() {
        let dir = tempfile::tempdir().unwrap();
        let _ = cmd_status_prometheus(dir.path(), None);
    }

    #[test]
    fn test_cov_empty_dir_anomalies() {
        let dir = tempfile::tempdir().unwrap();
        let _ = cmd_status_anomalies(dir.path(), None, false);
    }

    #[test]
    fn test_cov_empty_dir_convergence_rate() {
        let dir = tempfile::tempdir().unwrap();
        let _ = cmd_status_convergence_rate(dir.path(), None, false);
    }

    #[test]
    fn test_cov_empty_dir_error_summary() {
        let dir = tempfile::tempdir().unwrap();
        let _ = cmd_status_error_summary(dir.path(), None, false);
    }

    #[test]
    fn test_cov_empty_dir_stale_resources() {
        let dir = tempfile::tempdir().unwrap();
        let _ = cmd_status_stale_resources(dir.path(), None, false);
    }

    #[test]
    fn test_cov_empty_dir_health_threshold() {
        let dir = tempfile::tempdir().unwrap();
        let _ = cmd_status_health_threshold(dir.path(), None, 80, false);
    }

    #[test]
    fn test_cov_expired_with_filter() {
        let dir = make_dir();
        let _ = cmd_status_expired(dir.path(), Some("web"), "24h", false);
    }

    #[test]
    fn test_cov_export_filtered() {
        let dir = make_dir();
        let out = dir.path().join("out.json");
        let _ = cmd_status_export(dir.path(), Some("web"), &out, false);
    }

    #[test]
    fn test_cov_convergence_time_filtered() {
        let dir = make_dir();
        let _ = cmd_status_convergence_time(dir.path(), Some("web"), false);
    }

    #[test]
    fn test_cov_convergence_history_filtered() {
        let dir = make_dir();
        let _ = cmd_status_convergence_history(dir.path(), Some("web"), false);
    }

    #[test]
    fn test_cov_health_score_filtered() {
        let dir = make_dir();
        let _ = cmd_status_health_score(dir.path(), Some("web"), false);
    }

    #[test]
    fn test_cov_stale_high_days() {
        let dir = make_dir();
        write_yaml(dir.path(), "web/lock.yaml", state_lock());
        let _ = cmd_status_stale(dir.path(), None, 365, false);
    }

    #[test]
    fn test_cov_summary_by_unknown_dimension() {
        let dir = make_dir();
        let result = cmd_status_summary_by(dir.path(), None, "bogus", false);
        assert!(result.is_err());
    }

    #[test]
    fn test_cov_error_summary_filtered() {
        let dir = make_dir();
        let _ = cmd_status_error_summary(dir.path(), Some("web"), false);
    }
}
