//! Coverage tests for dispatch_status.rs — all try_status_phase* routing.

#![allow(unused_imports)]
use super::dispatch_status::*;
use std::io::Write as IoWrite;

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

    fn setup_state() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Converged\n    hash: abc123\n    applied_at: '2025-01-01T00:00:00Z'\n    duration_seconds: 2.5\n  mysql:\n    resource_type: Package\n    status: Failed\n    hash: def456\n");
        write_yaml(dir.path(), "web1/events.jsonl", "{\"ts\":\"2026-01-01T00:00:00Z\",\"event\":\"resource_started\",\"resource\":\"nginx\",\"machine\":\"web1\"}\n{\"ts\":\"2026-01-01T00:01:00Z\",\"event\":\"resource_converged\",\"resource\":\"nginx\",\"machine\":\"web1\"}\n");
        dir
    }

    // try_status_phase59a: sd, machine, json, 8 bools
    #[test]
    fn test_p59a_resource_health() {
        let d = setup_state();
        assert!(try_status_phase59a(d.path(), None, false, true, false, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_p59a_machine_summary() {
        let d = setup_state();
        assert!(try_status_phase59a(d.path(), None, false, false, true, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_p59a_none() {
        let d = setup_state();
        assert!(try_status_phase59a(d.path(), None, false, false, false, false, false, false, false, false, false).is_none());
    }

    // try_status_phase62: sd, machine, json, file: Option<&Path>, 7 bools
    #[test]
    fn test_p62_machine_resource_map() {
        let d = setup_state();
        assert!(try_status_phase62(d.path(), None, false, None, true, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_p62_fleet_convergence() {
        let d = setup_state();
        assert!(try_status_phase62(d.path(), None, false, None, false, true, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_p62_none() {
        let d = setup_state();
        assert!(try_status_phase62(d.path(), None, false, None, false, false, false, false, false, false, false).is_none());
    }

    // try_status_phase65: sd, machine, json, file: Option<&Path>, 9 bools
    #[test]
    fn test_p65_resource_apply_age() {
        let d = setup_state();
        assert!(try_status_phase65(d.path(), None, false, None, true, false, false, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_p65_error_rate() {
        let d = setup_state();
        assert!(try_status_phase65(d.path(), None, false, None, false, false, false, false, false, false, false, true, false).is_some());
    }
    #[test]
    fn test_p65_none() {
        let d = setup_state();
        assert!(try_status_phase65(d.path(), None, false, None, false, false, false, false, false, false, false, false, false).is_none());
    }

    // try_status_phase68: sd, machine, json, 9 bools
    #[test]
    fn test_p68_convergence_history() {
        let d = setup_state();
        assert!(try_status_phase68(d.path(), None, false, true, false, false, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_p68_resource_state_dist() {
        let d = setup_state();
        assert!(try_status_phase68(d.path(), None, false, false, false, false, false, false, false, false, false, true).is_some());
    }
    #[test]
    fn test_p68_none() {
        let d = setup_state();
        assert!(try_status_phase68(d.path(), None, false, false, false, false, false, false, false, false, false, false).is_none());
    }

    // try_status_phase73: sd, machine, json, 6 bools
    #[test]
    fn test_p73_drift_age() {
        let d = setup_state();
        assert!(try_status_phase73(d.path(), None, false, true, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_p73_failure_correlation() {
        let d = setup_state();
        assert!(try_status_phase73(d.path(), None, false, false, false, false, false, false, true).is_some());
    }
    #[test]
    fn test_p73_none() {
        let d = setup_state();
        assert!(try_status_phase73(d.path(), None, false, false, false, false, false, false, false).is_none());
    }

    // try_status_phase75: sd, machine, json, 12 bools
    #[test]
    fn test_p75_churn_rate() {
        let d = setup_state();
        assert!(try_status_phase75(d.path(), None, false, true, false, false, false, false, false, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_p75_convergence_rate() {
        let d = setup_state();
        assert!(try_status_phase75(d.path(), None, false, false, false, false, false, false, false, false, false, false, false, true, false).is_some());
    }
    #[test]
    fn test_p75_none() {
        let d = setup_state();
        assert!(try_status_phase75(d.path(), None, false, false, false, false, false, false, false, false, false, false, false, false, false).is_none());
    }

    // try_status_phase79: sd, machine, json, 24 bools = 27 args
    #[test]
    fn test_p79_failure_correlation() {
        let d = setup_state();
        assert!(try_status_phase79(d.path(), None, false, true, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_p79_error_distribution() {
        let d = setup_state();
        assert!(try_status_phase79(d.path(), None, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, true).is_some());
    }
    #[test]
    fn test_p79_none() {
        let d = setup_state();
        assert!(try_status_phase79(d.path(), None, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false).is_none());
    }

    // try_status_phase82: sd, machine, json, 15 bools = 18 args
    #[test]
    fn test_p82_dep_lag() {
        let d = setup_state();
        assert!(try_status_phase82(d.path(), None, false, true, false, false, false, false, false, false, false, false, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_p82_none() {
        let d = setup_state();
        assert!(try_status_phase82(d.path(), None, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false).is_none());
    }

    // try_status_phase85: sd, machine, json, 6 bools
    #[test]
    fn test_p85_drift_freq() {
        let d = setup_state();
        assert!(try_status_phase85(d.path(), None, false, true, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_p85_none() {
        let d = setup_state();
        assert!(try_status_phase85(d.path(), None, false, false, false, false, false, false, false).is_none());
    }
}
