//! Coverage tests for dispatch_status.rs — exercises all try_status_phase* dispatchers.

use super::dispatch_status::*;

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

// ── try_status_phase59a ──

#[test]
fn p59a_resource_health() {
    let d = setup_state();
    assert!(try_status_phase59a(d.path(), None, false, true, false, false, false, false, false, false, false).is_some());
}

#[test]
fn p59a_machine_health_summary() {
    let d = setup_state();
    assert!(try_status_phase59a(d.path(), None, false, false, true, false, false, false, false, false, false).is_some());
}

#[test]
fn p59a_last_apply_status() {
    let d = setup_state();
    assert!(try_status_phase59a(d.path(), None, false, false, false, true, false, false, false, false, false).is_some());
}

#[test]
fn p59a_resource_staleness() {
    let d = setup_state();
    assert!(try_status_phase59a(d.path(), None, false, false, false, false, true, false, false, false, false).is_some());
}

#[test]
fn p59a_convergence_percentage() {
    let d = setup_state();
    assert!(try_status_phase59a(d.path(), None, false, false, false, false, false, true, false, false, false).is_some());
}

#[test]
fn p59a_failed_count() {
    let d = setup_state();
    assert!(try_status_phase59a(d.path(), None, false, false, false, false, false, false, true, false, false).is_some());
}

#[test]
fn p59a_drift_count() {
    let d = setup_state();
    assert!(try_status_phase59a(d.path(), None, false, false, false, false, false, false, false, true, false).is_some());
}

#[test]
fn p59a_resource_duration() {
    let d = setup_state();
    assert!(try_status_phase59a(d.path(), None, false, false, false, false, false, false, false, false, true).is_some());
}

#[test]
fn p59a_none() {
    let d = setup_state();
    assert!(try_status_phase59a(d.path(), None, false, false, false, false, false, false, false, false, false).is_none());
}

// ── try_status_phase62 ──

#[test]
fn p62_fleet_convergence() {
    let d = setup_state();
    assert!(try_status_phase62(d.path(), None, false, None, false, true, false, false, false, false, false).is_some());
}

#[test]
fn p62_resource_hash() {
    let d = setup_state();
    assert!(try_status_phase62(d.path(), None, false, None, false, false, true, false, false, false, false).is_some());
}

#[test]
fn p62_machine_drift_summary() {
    let d = setup_state();
    assert!(try_status_phase62(d.path(), None, false, None, false, false, false, true, false, false, false).is_some());
}

#[test]
fn p62_apply_history_count() {
    let d = setup_state();
    assert!(try_status_phase62(d.path(), None, false, None, false, false, false, false, true, false, false).is_some());
}

#[test]
fn p62_lock_file_count() {
    let d = setup_state();
    assert!(try_status_phase62(d.path(), None, false, None, false, false, false, false, false, true, false).is_some());
}

#[test]
fn p62_none() {
    let d = setup_state();
    assert!(try_status_phase62(d.path(), None, false, None, false, false, false, false, false, false, false).is_none());
}

// ── try_status_phase65 ──

#[test]
fn p65_resource_apply_age() {
    let d = setup_state();
    assert!(try_status_phase65(d.path(), None, false, None, true, false, false, false, false, false, false, false, false).is_some());
}

#[test]
fn p65_machine_uptime() {
    let d = setup_state();
    assert!(try_status_phase65(d.path(), None, false, None, false, true, false, false, false, false, false, false, false).is_some());
}

#[test]
fn p65_resource_churn() {
    let d = setup_state();
    assert!(try_status_phase65(d.path(), None, false, None, false, false, true, false, false, false, false, false, false).is_some());
}

#[test]
fn p65_last_drift_time() {
    let d = setup_state();
    assert!(try_status_phase65(d.path(), None, false, None, false, false, false, true, false, false, false, false, false).is_some());
}

#[test]
fn p65_convergence_score() {
    let d = setup_state();
    assert!(try_status_phase65(d.path(), None, false, None, false, false, false, false, false, true, false, false, false).is_some());
}

#[test]
fn p65_apply_success_rate() {
    let d = setup_state();
    assert!(try_status_phase65(d.path(), None, false, None, false, false, false, false, false, false, true, false, false).is_some());
}

#[test]
fn p65_error_rate() {
    let d = setup_state();
    assert!(try_status_phase65(d.path(), None, false, None, false, false, false, false, false, false, false, true, false).is_some());
}

#[test]
fn p65_fleet_health_summary() {
    let d = setup_state();
    assert!(try_status_phase65(d.path(), None, false, None, false, false, false, false, false, false, false, false, true).is_some());
}

#[test]
fn p65_none() {
    let d = setup_state();
    assert!(try_status_phase65(d.path(), None, false, None, false, false, false, false, false, false, false, false, false).is_none());
}

// ── try_status_phase68 ──

#[test]
fn p68_machine_convergence_history() {
    let d = setup_state();
    assert!(try_status_phase68(d.path(), None, false, true, false, false, false, false, false, false, false, false).is_some());
}

#[test]
fn p68_drift_history() {
    let d = setup_state();
    assert!(try_status_phase68(d.path(), None, false, false, true, false, false, false, false, false, false, false).is_some());
}

#[test]
fn p68_resource_failure_rate() {
    let d = setup_state();
    assert!(try_status_phase68(d.path(), None, false, false, false, true, false, false, false, false, false, false).is_some());
}

#[test]
fn p68_machine_last_apply() {
    let d = setup_state();
    assert!(try_status_phase68(d.path(), None, false, false, false, false, true, false, false, false, false, false).is_some());
}

#[test]
fn p68_fleet_drift_summary() {
    let d = setup_state();
    assert!(try_status_phase68(d.path(), None, false, false, false, false, false, true, false, false, false, false).is_some());
}

#[test]
fn p68_resource_apply_duration() {
    let d = setup_state();
    assert!(try_status_phase68(d.path(), None, false, false, false, false, false, false, true, false, false, false).is_some());
}

#[test]
fn p68_machine_resource_health() {
    let d = setup_state();
    assert!(try_status_phase68(d.path(), None, false, false, false, false, false, false, false, true, false, false).is_some());
}

#[test]
fn p68_fleet_convergence_trend() {
    let d = setup_state();
    assert!(try_status_phase68(d.path(), None, false, false, false, false, false, false, false, false, true, false).is_some());
}

#[test]
fn p68_resource_state_distribution() {
    let d = setup_state();
    assert!(try_status_phase68(d.path(), None, false, false, false, false, false, false, false, false, false, true).is_some());
}

#[test]
fn p68_none() {
    let d = setup_state();
    assert!(try_status_phase68(d.path(), None, false, false, false, false, false, false, false, false, false, false).is_none());
}

// ── try_status_phase73 ──

#[test]
fn p73_machine_drift_age() {
    let d = setup_state();
    assert!(try_status_phase73(d.path(), None, false, true, false, false, false, false, false).is_some());
}

#[test]
fn p73_fleet_failed_resources() {
    let d = setup_state();
    assert!(try_status_phase73(d.path(), None, false, false, true, false, false, false, false).is_some());
}

#[test]
fn p73_resource_dependency_health() {
    let d = setup_state();
    assert!(try_status_phase73(d.path(), None, false, false, false, true, false, false, false).is_some());
}

#[test]
fn p73_machine_resource_age_distribution() {
    let d = setup_state();
    assert!(try_status_phase73(d.path(), None, false, false, false, false, true, false, false).is_some());
}

#[test]
fn p73_fleet_convergence_velocity() {
    let d = setup_state();
    assert!(try_status_phase73(d.path(), None, false, false, false, false, false, true, false).is_some());
}

#[test]
fn p73_resource_failure_correlation() {
    let d = setup_state();
    assert!(try_status_phase73(d.path(), None, false, false, false, false, false, false, true).is_some());
}

#[test]
fn p73_none() {
    let d = setup_state();
    assert!(try_status_phase73(d.path(), None, false, false, false, false, false, false, false).is_none());
}

// ── try_status_phase85 ──

#[test]
fn p85_machine_resource_drift_frequency() {
    let d = setup_state();
    assert!(try_status_phase85(d.path(), None, false, true, false, false, false, false, false).is_some());
}

#[test]
fn p85_fleet_resource_drift_frequency() {
    let d = setup_state();
    assert!(try_status_phase85(d.path(), None, false, false, true, false, false, false, false).is_some());
}

#[test]
fn p85_machine_resource_apply_duration_trend() {
    let d = setup_state();
    assert!(try_status_phase85(d.path(), None, false, false, false, true, false, false, false).is_some());
}

#[test]
fn p85_machine_resource_convergence_streak() {
    let d = setup_state();
    assert!(try_status_phase85(d.path(), None, false, false, false, false, true, false, false).is_some());
}

#[test]
fn p85_fleet_resource_convergence_streak() {
    let d = setup_state();
    assert!(try_status_phase85(d.path(), None, false, false, false, false, false, true, false).is_some());
}

#[test]
fn p85_machine_resource_error_distribution() {
    let d = setup_state();
    assert!(try_status_phase85(d.path(), None, false, false, false, false, false, false, true).is_some());
}

#[test]
fn p85_none() {
    let d = setup_state();
    assert!(try_status_phase85(d.path(), None, false, false, false, false, false, false, false).is_none());
}

// ── json mode variants ──

#[test]
fn p59a_resource_health_json() {
    let d = setup_state();
    assert!(try_status_phase59a(d.path(), None, true, true, false, false, false, false, false, false, false).is_some());
}

#[test]
fn p62_fleet_convergence_json() {
    let d = setup_state();
    assert!(try_status_phase62(d.path(), None, true, None, false, true, false, false, false, false, false).is_some());
}

#[test]
fn p65_convergence_score_json() {
    let d = setup_state();
    assert!(try_status_phase65(d.path(), None, true, None, false, false, false, false, false, true, false, false, false).is_some());
}

#[test]
fn p68_drift_history_json() {
    let d = setup_state();
    assert!(try_status_phase68(d.path(), None, true, false, true, false, false, false, false, false, false, false).is_some());
}
