//! Coverage tests for status_fleet_detail, status_insights,
//! and status_intelligence_ext (drift frequency through drift age).

#![allow(unused_imports)]
use super::status_convergence::*;
use super::status_fleet_detail::*;
use super::status_insights::*;
use super::status_intelligence_ext::*;
use super::status_operational::*;
use super::status_predictive::*;

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
        "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: converged\n    hash: \"blake3:abc\"\n    applied_at: \"2026-02-28T00:00:00Z\"\n    duration_seconds: 1.5\n  g:\n    type: service\n    status: drifted\n    hash: \"blake3:def\"\n    duration_seconds: 2.0\n  h:\n    type: package\n    status: failed\n    hash: \"blake3:ghi\"\n    duration_seconds: 0.5\n"
    }

    /// Create temp dir with both flat and subdirectory lock files.
    fn make_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for m in &["web", "db"] {
            // Subdirectory pattern: {m}/state.lock.yaml
            write_yaml(dir.path(), &format!("{m}/state.lock.yaml"), state_lock());
            // Flat pattern: {m}.lock.yaml
            write_yaml(dir.path(), &format!("{m}/state.lock.yaml"), state_lock());
        }
        dir
    }

    fn make_empty_dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    fn make_forjar_config(dir: &std::path::Path) -> std::path::PathBuf {
        let content = concat!(
            "version: \"1.0\"\n",
            "name: test\n",
            "machines:\n",
            "  web:\n",
            "    hostname: web.example.com\n",
            "    addr: 10.0.0.1\n",
            "  db:\n",
            "    hostname: db.example.com\n",
            "    addr: 10.0.0.2\n",
            "resources:\n",
            "  my_file:\n",
            "    type: file\n",
            "    path: /tmp/f\n",
            "    machine: web\n",
            "    content: \"hello\"\n",
            "  my_pkg:\n",
            "    type: package\n",
            "    machine:\n",
            "      - web\n",
            "      - db\n",
            "    packages:\n",
            "      - curl\n",
        );
        let p = dir.join("forjar.yaml");
        std::fs::write(&p, content).unwrap();
        p
    }

    // ═══════════════════════════════════════════════════════════════════
    // status_fleet_detail
    // ═══════════════════════════════════════════════════════════════════

    // cmd_status_last_drift_time
    #[test]
    fn last_drift_time_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_last_drift_time(d.path(), None, false).is_ok());
    }
    #[test]
    fn last_drift_time_data() {
        let d = make_dir();
        assert!(cmd_status_last_drift_time(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn last_drift_time_json() {
        let d = make_dir();
        assert!(cmd_status_last_drift_time(d.path(), None, true).is_ok());
    }

    // cmd_status_machine_resource_count (takes file: &Path)
    #[test]
    fn machine_resource_count_data() {
        let d = make_dir();
        let cfg = make_forjar_config(d.path());
        assert!(cmd_status_machine_resource_count(&cfg, false).is_ok());
    }
    #[test]
    fn machine_resource_count_json() {
        let d = make_dir();
        let cfg = make_forjar_config(d.path());
        assert!(cmd_status_machine_resource_count(&cfg, true).is_ok());
    }
    #[test]
    fn machine_resource_count_missing() {
        let d = make_empty_dir();
        let missing = d.path().join("nonexistent.yaml");
        assert!(cmd_status_machine_resource_count(&missing, false).is_err());
    }

    // cmd_status_convergence_score
    #[test]
    fn convergence_score_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_convergence_score(d.path(), false).is_ok());
    }
    #[test]
    fn convergence_score_data() {
        let d = make_dir();
        assert!(cmd_status_convergence_score(d.path(), false).is_ok());
    }
    #[test]
    fn convergence_score_json() {
        let d = make_dir();
        assert!(cmd_status_convergence_score(d.path(), true).is_ok());
    }

    // cmd_status_apply_success_rate
    #[test]
    fn apply_success_rate_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_apply_success_rate(d.path(), None, false).is_ok());
    }
    #[test]
    fn apply_success_rate_data() {
        let d = make_dir();
        assert!(cmd_status_apply_success_rate(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn apply_success_rate_json() {
        let d = make_dir();
        assert!(cmd_status_apply_success_rate(d.path(), None, true).is_ok());
    }

    // cmd_status_error_rate
    #[test]
    fn error_rate_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_error_rate(d.path(), None, false).is_ok());
    }
    #[test]
    fn error_rate_data() {
        let d = make_dir();
        assert!(cmd_status_error_rate(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn error_rate_json() {
        let d = make_dir();
        assert!(cmd_status_error_rate(d.path(), None, true).is_ok());
    }

    // cmd_status_fleet_health_summary
    #[test]
    fn fleet_health_summary_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_fleet_health_summary(d.path(), false).is_ok());
    }
    #[test]
    fn fleet_health_summary_data() {
        let d = make_dir();
        assert!(cmd_status_fleet_health_summary(d.path(), false).is_ok());
    }
    #[test]
    fn fleet_health_summary_json() {
        let d = make_dir();
        assert!(cmd_status_fleet_health_summary(d.path(), true).is_ok());
    }

    // cmd_status_machine_convergence_history
    #[test]
    fn machine_conv_history_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_machine_convergence_history(d.path(), None, false).is_ok());
    }
    #[test]
    fn machine_conv_history_data() {
        let d = make_dir();
        assert!(cmd_status_machine_convergence_history(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn machine_conv_history_json() {
        let d = make_dir();
        assert!(cmd_status_machine_convergence_history(d.path(), None, true).is_ok());
    }

    // cmd_status_drift_history
    #[test]
    fn drift_history_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_drift_history(d.path(), None, false).is_ok());
    }
    #[test]
    fn drift_history_data() {
        let d = make_dir();
        assert!(cmd_status_drift_history(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn drift_history_json() {
        let d = make_dir();
        assert!(cmd_status_drift_history(d.path(), None, true).is_ok());
    }

    // cmd_status_resource_failure_rate
    #[test]
    fn resource_failure_rate_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_resource_failure_rate(d.path(), None, false).is_ok());
    }
    #[test]
    fn resource_failure_rate_data() {
        let d = make_dir();
        assert!(cmd_status_resource_failure_rate(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn resource_failure_rate_json() {
        let d = make_dir();
        assert!(cmd_status_resource_failure_rate(d.path(), None, true).is_ok());
    }

    // ═══════════════════════════════════════════════════════════════════
    // status_insights
    // ═══════════════════════════════════════════════════════════════════

    // cmd_status_machine_uptime_estimate
    #[test]
    fn uptime_estimate_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_machine_uptime_estimate(d.path(), None, false).is_ok());
    }
    #[test]
    fn uptime_estimate_data() {
        let d = make_dir();
        assert!(cmd_status_machine_uptime_estimate(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn uptime_estimate_json() {
        let d = make_dir();
        assert!(cmd_status_machine_uptime_estimate(d.path(), None, true).is_ok());
    }

    // cmd_status_fleet_resource_type_breakdown
    #[test]
    fn type_breakdown_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_fleet_resource_type_breakdown(d.path(), None, false).is_ok());
    }
    #[test]
    fn type_breakdown_data() {
        let d = make_dir();
        assert!(cmd_status_fleet_resource_type_breakdown(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn type_breakdown_json() {
        let d = make_dir();
        assert!(cmd_status_fleet_resource_type_breakdown(d.path(), None, true).is_ok());
    }

    // cmd_status_resource_convergence_time
    #[test]
    fn resource_conv_time_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_resource_convergence_time(d.path(), None, false).is_ok());
    }
    #[test]
    fn resource_conv_time_data() {
        let d = make_dir();
        assert!(cmd_status_resource_convergence_time(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn resource_conv_time_json() {
        let d = make_dir();
        assert!(cmd_status_resource_convergence_time(d.path(), None, true).is_ok());
    }

    // cmd_status_machine_drift_age
    #[test]
    fn machine_drift_age_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_machine_drift_age(d.path(), None, false).is_ok());
    }
    #[test]
    fn machine_drift_age_data() {
        let d = make_dir();
        assert!(cmd_status_machine_drift_age(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn machine_drift_age_json() {
        let d = make_dir();
        assert!(cmd_status_machine_drift_age(d.path(), None, true).is_ok());
    }

    // cmd_status_fleet_failed_resources
    #[test]
    fn fleet_failed_resources_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_fleet_failed_resources(d.path(), None, false).is_ok());
    }
    #[test]
    fn fleet_failed_resources_data() {
        let d = make_dir();
        assert!(cmd_status_fleet_failed_resources(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn fleet_failed_resources_json() {
        let d = make_dir();
        assert!(cmd_status_fleet_failed_resources(d.path(), None, true).is_ok());
    }

    // cmd_status_resource_dependency_health
    #[test]
    fn dependency_health_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_resource_dependency_health(d.path(), None, false).is_ok());
    }
    #[test]
    fn dependency_health_data() {
        let d = make_dir();
        assert!(cmd_status_resource_dependency_health(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn dependency_health_json() {
        let d = make_dir();
        assert!(cmd_status_resource_dependency_health(d.path(), None, true).is_ok());
    }

    // ═══════════════════════════════════════════════════════════════════
    // status_intelligence_ext (drift frequency through drift age)
    // ═══════════════════════════════════════════════════════════════════

    // cmd_status_machine_resource_drift_frequency
    #[test]
    fn drift_frequency_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_machine_resource_drift_frequency(d.path(), None, false).is_ok());
    }
    #[test]
    fn drift_frequency_data() {
        let d = make_dir();
        assert!(cmd_status_machine_resource_drift_frequency(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn drift_frequency_json() {
        let d = make_dir();
        assert!(cmd_status_machine_resource_drift_frequency(d.path(), None, true).is_ok());
    }

    // cmd_status_fleet_resource_drift_frequency
    #[test]
    fn fleet_drift_frequency_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_fleet_resource_drift_frequency(d.path(), None, false).is_ok());
    }
    #[test]
    fn fleet_drift_frequency_data() {
        let d = make_dir();
        assert!(cmd_status_fleet_resource_drift_frequency(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn fleet_drift_frequency_json() {
        let d = make_dir();
        assert!(cmd_status_fleet_resource_drift_frequency(d.path(), None, true).is_ok());
    }

    // cmd_status_machine_resource_apply_duration_trend
    #[test]
    fn apply_duration_trend_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_machine_resource_apply_duration_trend(d.path(), None, false).is_ok());
    }
    #[test]
    fn apply_duration_trend_data() {
        let d = make_dir();
        assert!(
            cmd_status_machine_resource_apply_duration_trend(d.path(), Some("web"), false).is_ok()
        );
    }
    #[test]
    fn apply_duration_trend_json() {
        let d = make_dir();
        assert!(cmd_status_machine_resource_apply_duration_trend(d.path(), None, true).is_ok());
    }

    // cmd_status_machine_resource_convergence_streak
    #[test]
    fn convergence_streak_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_machine_resource_convergence_streak(d.path(), None, false).is_ok());
    }
    #[test]
    fn convergence_streak_data() {
        let d = make_dir();
        assert!(
            cmd_status_machine_resource_convergence_streak(d.path(), Some("web"), false).is_ok()
        );
    }
    #[test]
    fn convergence_streak_json() {
        let d = make_dir();
        assert!(cmd_status_machine_resource_convergence_streak(d.path(), None, true).is_ok());
    }

    // cmd_status_fleet_resource_convergence_streak
    #[test]
    fn fleet_conv_streak_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_fleet_resource_convergence_streak(d.path(), None, false).is_ok());
    }
    #[test]
    fn fleet_conv_streak_data() {
        let d = make_dir();
        assert!(cmd_status_fleet_resource_convergence_streak(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn fleet_conv_streak_json() {
        let d = make_dir();
        assert!(cmd_status_fleet_resource_convergence_streak(d.path(), None, true).is_ok());
    }

    // cmd_status_machine_resource_error_distribution
    #[test]
    fn error_distribution_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_machine_resource_error_distribution(d.path(), None, false).is_ok());
    }
    #[test]
    fn error_distribution_data() {
        let d = make_dir();
        assert!(
            cmd_status_machine_resource_error_distribution(d.path(), Some("web"), false).is_ok()
        );
    }
    #[test]
    fn error_distribution_json() {
        let d = make_dir();
        assert!(cmd_status_machine_resource_error_distribution(d.path(), None, true).is_ok());
    }

    // cmd_status_machine_resource_drift_age
    #[test]
    fn intel_ext_drift_age_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_machine_resource_drift_age(d.path(), None, false).is_ok());
    }
    #[test]
    fn intel_ext_drift_age_data() {
        let d = make_dir();
        assert!(cmd_status_machine_resource_drift_age(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn intel_ext_drift_age_json() {
        let d = make_dir();
        assert!(cmd_status_machine_resource_drift_age(d.path(), None, true).is_ok());
    }

    // cmd_status_fleet_resource_drift_age
    #[test]
    fn fleet_drift_age_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_fleet_resource_drift_age(d.path(), None, false).is_ok());
    }
    #[test]
    fn fleet_drift_age_data() {
        let d = make_dir();
        assert!(cmd_status_fleet_resource_drift_age(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn fleet_drift_age_json() {
        let d = make_dir();
        assert!(cmd_status_fleet_resource_drift_age(d.path(), None, true).is_ok());
    }
}
