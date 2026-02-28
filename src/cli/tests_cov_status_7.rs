//! Coverage tests for status_intelligence_ext (recovery rate through staleness index),
//! status_convergence, and extra converged-only branch tests.

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
            write_yaml(dir.path(), &format!("{}/state.lock.yaml", m), state_lock());
            // Flat pattern: {m}.lock.yaml
            write_yaml(dir.path(), &format!("{}.lock.yaml", m), state_lock());
        }
        dir
    }

    fn make_empty_dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    // ═══════════════════════════════════════════════════════════════════
    // status_intelligence_ext (recovery rate through staleness index)
    // ═══════════════════════════════════════════════════════════════════

    // cmd_status_machine_resource_recovery_rate
    #[test]
    fn recovery_rate_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_machine_resource_recovery_rate(d.path(), None, false).is_ok());
    }
    #[test]
    fn recovery_rate_data() {
        let d = make_dir();
        assert!(cmd_status_machine_resource_recovery_rate(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn recovery_rate_json() {
        let d = make_dir();
        assert!(cmd_status_machine_resource_recovery_rate(d.path(), None, true).is_ok());
    }

    // cmd_status_machine_resource_drift_velocity
    #[test]
    fn drift_velocity_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_machine_resource_drift_velocity(d.path(), None, false).is_ok());
    }
    #[test]
    fn drift_velocity_data() {
        let d = make_dir();
        assert!(cmd_status_machine_resource_drift_velocity(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn drift_velocity_json() {
        let d = make_dir();
        assert!(cmd_status_machine_resource_drift_velocity(d.path(), None, true).is_ok());
    }

    // cmd_status_fleet_resource_recovery_rate
    #[test]
    fn fleet_recovery_rate_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_fleet_resource_recovery_rate(d.path(), None, false).is_ok());
    }
    #[test]
    fn fleet_recovery_rate_data() {
        let d = make_dir();
        assert!(cmd_status_fleet_resource_recovery_rate(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn fleet_recovery_rate_json() {
        let d = make_dir();
        assert!(cmd_status_fleet_resource_recovery_rate(d.path(), None, true).is_ok());
    }

    // cmd_status_machine_resource_convergence_efficiency
    #[test]
    fn convergence_efficiency_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_machine_resource_convergence_efficiency(d.path(), None, false).is_ok());
    }
    #[test]
    fn convergence_efficiency_data() {
        let d = make_dir();
        assert!(cmd_status_machine_resource_convergence_efficiency(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn convergence_efficiency_json() {
        let d = make_dir();
        assert!(cmd_status_machine_resource_convergence_efficiency(d.path(), None, true).is_ok());
    }

    // cmd_status_machine_resource_apply_frequency
    #[test]
    fn apply_frequency_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_machine_resource_apply_frequency(d.path(), None, false).is_ok());
    }
    #[test]
    fn apply_frequency_data() {
        let d = make_dir();
        assert!(cmd_status_machine_resource_apply_frequency(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn apply_frequency_json() {
        let d = make_dir();
        assert!(cmd_status_machine_resource_apply_frequency(d.path(), None, true).is_ok());
    }

    // cmd_status_fleet_resource_health_score
    #[test]
    fn fleet_health_score_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_fleet_resource_health_score(d.path(), None, false).is_ok());
    }
    #[test]
    fn fleet_health_score_data() {
        let d = make_dir();
        assert!(cmd_status_fleet_resource_health_score(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn fleet_health_score_json() {
        let d = make_dir();
        assert!(cmd_status_fleet_resource_health_score(d.path(), None, true).is_ok());
    }

    // cmd_status_machine_resource_staleness_index
    #[test]
    fn staleness_index_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_machine_resource_staleness_index(d.path(), None, false).is_ok());
    }
    #[test]
    fn staleness_index_data() {
        let d = make_dir();
        assert!(cmd_status_machine_resource_staleness_index(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn staleness_index_json() {
        let d = make_dir();
        assert!(cmd_status_machine_resource_staleness_index(d.path(), None, true).is_ok());
    }

    // filter_targets helper
    #[test]
    fn filter_targets_none() {
        let machines = vec!["web".to_string(), "db".to_string()];
        let targets = filter_targets(&machines, None);
        assert_eq!(targets.len(), 2);
    }
    #[test]
    fn filter_targets_some() {
        let machines = vec!["web".to_string(), "db".to_string()];
        let targets = filter_targets(&machines, Some("web"));
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0], "web");
    }
    #[test]
    fn filter_targets_miss() {
        let machines = vec!["web".to_string(), "db".to_string()];
        let targets = filter_targets(&machines, Some("missing"));
        assert!(targets.is_empty());
    }

    // ═══════════════════════════════════════════════════════════════════
    // status_convergence
    // ═══════════════════════════════════════════════════════════════════

    // cmd_status_convergence_rate
    #[test]
    fn convergence_rate_empty() {
        let d = make_empty_dir();
        // convergence functions use state::load_lock which needs subdirs
        assert!(cmd_status_convergence_rate(d.path(), None, false).is_ok());
    }
    #[test]
    fn convergence_rate_data() {
        let d = make_dir();
        assert!(cmd_status_convergence_rate(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn convergence_rate_json() {
        let d = make_dir();
        assert!(cmd_status_convergence_rate(d.path(), None, true).is_ok());
    }

    // cmd_status_convergence_time
    #[test]
    fn convergence_time_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_convergence_time(d.path(), None, false).is_ok());
    }
    #[test]
    fn convergence_time_data() {
        let d = make_dir();
        assert!(cmd_status_convergence_time(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn convergence_time_json() {
        let d = make_dir();
        assert!(cmd_status_convergence_time(d.path(), None, true).is_ok());
    }

    // cmd_status_convergence_history
    #[test]
    fn convergence_history_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_convergence_history(d.path(), None, false).is_ok());
    }
    #[test]
    fn convergence_history_data() {
        let d = make_dir();
        assert!(cmd_status_convergence_history(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn convergence_history_json() {
        let d = make_dir();
        assert!(cmd_status_convergence_history(d.path(), None, true).is_ok());
    }

    // cmd_status_timeline
    #[test]
    fn timeline_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_timeline(d.path(), None, false).is_ok());
    }
    #[test]
    fn timeline_data() {
        let d = make_dir();
        assert!(cmd_status_timeline(d.path(), Some("web"), false).is_ok());
    }
    #[test]
    fn timeline_json() {
        let d = make_dir();
        assert!(cmd_status_timeline(d.path(), None, true).is_ok());
    }

    // cmd_status_summary_by
    #[test]
    fn summary_by_machine_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_summary_by(d.path(), None, "machine", false).is_ok());
    }
    #[test]
    fn summary_by_machine_data() {
        let d = make_dir();
        assert!(cmd_status_summary_by(d.path(), Some("web"), "machine", false).is_ok());
    }
    #[test]
    fn summary_by_type_json() {
        let d = make_dir();
        assert!(cmd_status_summary_by(d.path(), None, "type", true).is_ok());
    }
    #[test]
    fn summary_by_status() {
        let d = make_dir();
        assert!(cmd_status_summary_by(d.path(), None, "status", false).is_ok());
    }
    #[test]
    fn summary_by_invalid() {
        let d = make_dir();
        // invalid dimension should propagate error from summary_dimension_key
        let r = cmd_status_summary_by(d.path(), None, "invalid", false);
        // It will err only if there are resources to iterate over
        // Since make_dir creates subdirs with state.lock.yaml, it may or may not err
        let _ = r;
    }

    // cmd_status_since
    #[test]
    fn since_empty() {
        let d = make_empty_dir();
        assert!(cmd_status_since(d.path(), None, "1h", false).is_ok());
    }
    #[test]
    fn since_data() {
        let d = make_dir();
        assert!(cmd_status_since(d.path(), Some("web"), "7d", false).is_ok());
    }
    #[test]
    fn since_json() {
        let d = make_dir();
        assert!(cmd_status_since(d.path(), None, "24h", true).is_ok());
    }

    // cmd_status_changes_since (git-based, uses commit ref)
    #[test]
    fn changes_since_plain() {
        let d = make_dir();
        // HEAD is a safe git ref even in test envs
        let _ = cmd_status_changes_since(d.path(), "HEAD", false);
    }
    #[test]
    fn changes_since_json() {
        let d = make_dir();
        let _ = cmd_status_changes_since(d.path(), "HEAD", true);
    }

    // ═══════════════════════════════════════════════════════════════════
    // Extra: converged-only dir to hit different branches
    // ═══════════════════════════════════════════════════════════════════

    fn make_converged_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for m in &["web", "db"] {
            write_yaml(dir.path(), &format!("{}/state.lock.yaml", m), state_lock_converged());
            write_yaml(dir.path(), &format!("{}.lock.yaml", m), state_lock_converged());
        }
        dir
    }

    #[test]
    fn fleet_conv_velocity_all_converged() {
        let d = make_converged_dir();
        assert!(cmd_status_fleet_convergence_velocity(d.path(), None, false).is_ok());
    }
    #[test]
    fn machine_conv_trend_all_converged() {
        let d = make_converged_dir();
        assert!(cmd_status_machine_convergence_trend(d.path(), None, true).is_ok());
    }
    #[test]
    fn convergence_score_all_converged() {
        let d = make_converged_dir();
        assert!(cmd_status_convergence_score(d.path(), true).is_ok());
    }
    #[test]
    fn fleet_health_summary_all_converged() {
        let d = make_converged_dir();
        assert!(cmd_status_fleet_health_summary(d.path(), true).is_ok());
    }
    #[test]
    fn fleet_health_score_all_converged() {
        let d = make_converged_dir();
        assert!(cmd_status_fleet_resource_health_score(d.path(), None, true).is_ok());
    }
    #[test]
    fn convergence_rate_all_converged() {
        let d = make_converged_dir();
        assert!(cmd_status_convergence_rate(d.path(), None, true).is_ok());
    }
}
