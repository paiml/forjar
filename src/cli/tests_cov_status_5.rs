#![allow(unused)]
//! Coverage tests for status_predictive, status_operational.
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

    fn state_lock_converged() -> &'static str {
        "schema: \"1.0\"\nmachine: web\nhostname: web\ngenerated_at: \"2026-02-28T00:00:00Z\"\ngenerator: forjar\nblake3_version: \"1.8\"\nresources:\n  f:\n    type: file\n    status: converged\n    hash: \"blake3:abc\"\n  g:\n    type: service\n    status: converged\n    hash: \"blake3:def\"\n"
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

    // ═══════════════════════════════════════════════════════════════════
    // status_predictive
    // ═══════════════════════════════════════════════════════════════════

    // cmd_status_machine_resource_age_distribution
    #[test]
    fn age_distribution_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_machine_resource_age_distribution(d.path(), None, false).is_ok());
    }
    #[test]
    fn age_distribution_data() {
        let d = make_dir();
        assert!(cmd_status_machine_resource_age_distribution(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn age_distribution_json() {
        let d = make_dir();
        assert!(cmd_status_machine_resource_age_distribution(d.path(), None, true).is_ok());
    }

    // cmd_status_fleet_convergence_velocity
    #[test]
    fn convergence_velocity_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_fleet_convergence_velocity(d.path(), None, false).is_ok());
    }
    #[test]
    fn convergence_velocity_data() {
        let d = make_dir();
        assert!(cmd_status_fleet_convergence_velocity(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn convergence_velocity_json() {
        let d = make_dir();
        assert!(cmd_status_fleet_convergence_velocity(d.path(), None, true).is_ok());
    }

    // cmd_status_resource_failure_correlation
    #[test]
    fn failure_correlation_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_resource_failure_correlation(d.path(), None, false).is_ok());
    }
    #[test]
    fn failure_correlation_data() {
        let d = make_dir();
        assert!(cmd_status_resource_failure_correlation(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn failure_correlation_json() {
        let d = make_dir();
        assert!(cmd_status_resource_failure_correlation(d.path(), None, true).is_ok());
    }

    // cmd_status_machine_resource_churn_rate
    #[test]
    fn churn_rate_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_machine_resource_churn_rate(d.path(), None, false).is_ok());
    }
    #[test]
    fn churn_rate_data() {
        let d = make_dir();
        assert!(cmd_status_machine_resource_churn_rate(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn churn_rate_json() {
        let d = make_dir();
        assert!(cmd_status_machine_resource_churn_rate(d.path(), None, true).is_ok());
    }

    // cmd_status_fleet_resource_staleness
    #[test]
    fn fleet_staleness_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_fleet_resource_staleness(d.path(), None, false).is_ok());
    }
    #[test]
    fn fleet_staleness_data() {
        let d = make_dir();
        assert!(cmd_status_fleet_resource_staleness(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn fleet_staleness_json() {
        let d = make_dir();
        assert!(cmd_status_fleet_resource_staleness(d.path(), None, true).is_ok());
    }

    // cmd_status_machine_convergence_trend
    #[test]
    fn convergence_trend_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_machine_convergence_trend(d.path(), None, false).is_ok());
    }
    #[test]
    fn convergence_trend_data() {
        let d = make_dir();
        assert!(cmd_status_machine_convergence_trend(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn convergence_trend_json() {
        let d = make_dir();
        assert!(cmd_status_machine_convergence_trend(d.path(), None, true).is_ok());
    }

    // cmd_status_machine_capacity_utilization
    #[test]
    fn capacity_utilization_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_machine_capacity_utilization(d.path(), None, false).is_ok());
    }
    #[test]
    fn capacity_utilization_data() {
        let d = make_dir();
        assert!(cmd_status_machine_capacity_utilization(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn capacity_utilization_json() {
        let d = make_dir();
        assert!(cmd_status_machine_capacity_utilization(d.path(), None, true).is_ok());
    }

    // cmd_status_fleet_configuration_entropy
    #[test]
    fn config_entropy_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_fleet_configuration_entropy(d.path(), None, false).is_ok());
    }
    #[test]
    fn config_entropy_data() {
        let d = make_dir();
        assert!(cmd_status_fleet_configuration_entropy(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn config_entropy_json() {
        let d = make_dir();
        assert!(cmd_status_fleet_configuration_entropy(d.path(), None, true).is_ok());
    }

    // cmd_status_machine_resource_freshness
    #[test]
    fn resource_freshness_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_machine_resource_freshness(d.path(), None, false).is_ok());
    }
    #[test]
    fn resource_freshness_data() {
        let d = make_dir();
        assert!(cmd_status_machine_resource_freshness(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn resource_freshness_json() {
        let d = make_dir();
        assert!(cmd_status_machine_resource_freshness(d.path(), None, true).is_ok());
    }

    // ═══════════════════════════════════════════════════════════════════
    // status_operational
    // ═══════════════════════════════════════════════════════════════════

    // cmd_status_machine_last_apply
    #[test]
    fn last_apply_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_machine_last_apply(d.path(), None, false).is_ok());
    }
    #[test]
    fn last_apply_data() {
        let d = make_dir();
        assert!(cmd_status_machine_last_apply(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn last_apply_json() {
        let d = make_dir();
        assert!(cmd_status_machine_last_apply(d.path(), None, true).is_ok());
    }

    // cmd_status_fleet_drift_summary
    #[test]
    fn fleet_drift_summary_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_fleet_drift_summary(d.path(), None, false).is_ok());
    }
    #[test]
    fn fleet_drift_summary_data() {
        let d = make_dir();
        assert!(cmd_status_fleet_drift_summary(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn fleet_drift_summary_json() {
        let d = make_dir();
        assert!(cmd_status_fleet_drift_summary(d.path(), None, true).is_ok());
    }

    // cmd_status_resource_apply_duration
    #[test]
    fn apply_duration_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_resource_apply_duration(d.path(), None, false).is_ok());
    }
    #[test]
    fn apply_duration_data() {
        let d = make_dir();
        assert!(cmd_status_resource_apply_duration(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn apply_duration_json() {
        let d = make_dir();
        assert!(cmd_status_resource_apply_duration(d.path(), None, true).is_ok());
    }

    // cmd_status_machine_resource_health
    #[test]
    fn machine_health_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_machine_resource_health(d.path(), None, false).is_ok());
    }
    #[test]
    fn machine_health_data() {
        let d = make_dir();
        assert!(cmd_status_machine_resource_health(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn machine_health_json() {
        let d = make_dir();
        assert!(cmd_status_machine_resource_health(d.path(), None, true).is_ok());
    }

    // cmd_status_fleet_convergence_trend
    #[test]
    fn fleet_conv_trend_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_fleet_convergence_trend(d.path(), None, false).is_ok());
    }
    #[test]
    fn fleet_conv_trend_data() {
        let d = make_dir();
        assert!(cmd_status_fleet_convergence_trend(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn fleet_conv_trend_json() {
        let d = make_dir();
        assert!(cmd_status_fleet_convergence_trend(d.path(), None, true).is_ok());
    }

    // cmd_status_resource_state_distribution
    #[test]
    fn state_distribution_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_resource_state_distribution(d.path(), None, false).is_ok());
    }
    #[test]
    fn state_distribution_data() {
        let d = make_dir();
        assert!(cmd_status_resource_state_distribution(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn state_distribution_json() {
        let d = make_dir();
        assert!(cmd_status_resource_state_distribution(d.path(), None, true).is_ok());
    }

    // cmd_status_machine_apply_count
    #[test]
    fn apply_count_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_machine_apply_count(d.path(), None, false).is_ok());
    }
    #[test]
    fn apply_count_data() {
        let d = make_dir();
        assert!(cmd_status_machine_apply_count(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn apply_count_json() {
        let d = make_dir();
        assert!(cmd_status_machine_apply_count(d.path(), None, true).is_ok());
    }

    // cmd_status_fleet_apply_history
    #[test]
    fn fleet_apply_history_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_fleet_apply_history(d.path(), None, false).is_ok());
    }
    #[test]
    fn fleet_apply_history_data() {
        let d = make_dir();
        assert!(cmd_status_fleet_apply_history(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn fleet_apply_history_json() {
        let d = make_dir();
        assert!(cmd_status_fleet_apply_history(d.path(), None, true).is_ok());
    }

    // cmd_status_resource_hash_changes
    #[test]
    fn hash_changes_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_resource_hash_changes(d.path(), None, false).is_ok());
    }
    #[test]
    fn hash_changes_data() {
        let d = make_dir();
        assert!(cmd_status_resource_hash_changes(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn hash_changes_json() {
        let d = make_dir();
        assert!(cmd_status_resource_hash_changes(d.path(), None, true).is_ok());
    }
}
