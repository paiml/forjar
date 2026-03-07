//! Coverage tests for dispatch_status.rs — try_status_phase75 and try_status_phase79/82.

use super::dispatch_status::*;

fn setup_state() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("web1")).unwrap();
    std::fs::write(
        dir.path().join("web1/state.lock.yaml"),
        "resources:\n  nginx:\n    resource_type: Package\n    status: Converged\n    hash: abc123\n    applied_at: '2025-01-01T00:00:00Z'\n    duration_seconds: 2.5\n",
    ).unwrap();
    dir
}

// ── try_status_phase75 (12 boolean flags) ──

#[test]
fn p75_machine_resource_churn_rate() {
    let d = setup_state();
    assert!(try_status_phase75(d.path(), None, false, true, false, false, false, false, false, false, false, false, false, false, false).is_some());
}

#[test]
fn p75_fleet_resource_staleness() {
    let d = setup_state();
    assert!(try_status_phase75(d.path(), None, false, false, true, false, false, false, false, false, false, false, false, false, false).is_some());
}

#[test]
fn p75_machine_convergence_trend() {
    let d = setup_state();
    assert!(try_status_phase75(d.path(), None, false, false, false, true, false, false, false, false, false, false, false, false, false).is_some());
}

#[test]
fn p75_machine_capacity_utilization() {
    let d = setup_state();
    assert!(try_status_phase75(d.path(), None, false, false, false, false, true, false, false, false, false, false, false, false, false).is_some());
}

#[test]
fn p75_fleet_configuration_entropy() {
    let d = setup_state();
    assert!(try_status_phase75(d.path(), None, false, false, false, false, false, true, false, false, false, false, false, false, false).is_some());
}

#[test]
fn p75_machine_resource_freshness() {
    let d = setup_state();
    assert!(try_status_phase75(d.path(), None, false, false, false, false, false, false, true, false, false, false, false, false, false).is_some());
}

#[test]
fn p75_machine_error_budget() {
    let d = setup_state();
    assert!(try_status_phase75(d.path(), None, false, false, false, false, false, false, false, true, false, false, false, false, false).is_some());
}

#[test]
fn p75_fleet_compliance_score() {
    let d = setup_state();
    assert!(try_status_phase75(d.path(), None, false, false, false, false, false, false, false, false, true, false, false, false, false).is_some());
}

#[test]
fn p75_machine_mean_time_to_recovery() {
    let d = setup_state();
    assert!(try_status_phase75(d.path(), None, false, false, false, false, false, false, false, false, false, true, false, false, false).is_some());
}

#[test]
fn p75_machine_resource_dependency_health() {
    let d = setup_state();
    assert!(try_status_phase75(d.path(), None, false, false, false, false, false, false, false, false, false, false, true, false, false).is_some());
}

#[test]
fn p75_fleet_resource_type_health() {
    let d = setup_state();
    assert!(try_status_phase75(d.path(), None, false, false, false, false, false, false, false, false, false, false, false, true, false).is_some());
}

#[test]
fn p75_machine_resource_convergence_rate() {
    let d = setup_state();
    assert!(try_status_phase75(d.path(), None, false, false, false, false, false, false, false, false, false, false, false, false, true).is_some());
}

#[test]
fn p75_none() {
    let d = setup_state();
    assert!(try_status_phase75(d.path(), None, false, false, false, false, false, false, false, false, false, false, false, false, false).is_none());
}

// Helper: 24-bool call to try_status_phase79 with nth bool set to true.
// Params after sd/machine/json: 24 booleans.
fn call_p79(d: &std::path::Path, idx: usize) -> Option<Result<(), String>> {
    let mut flags = [false; 24];
    if idx < 24 {
        flags[idx] = true;
    }
    try_status_phase79(
        d, None, false,
        flags[0], flags[1], flags[2], flags[3], flags[4], flags[5],
        flags[6], flags[7], flags[8], flags[9], flags[10], flags[11],
        flags[12], flags[13], flags[14], flags[15], flags[16], flags[17],
        flags[18], flags[19], flags[20], flags[21], flags[22], flags[23],
    )
}

#[test] fn p79_0()  { let d = setup_state(); assert!(call_p79(d.path(), 0).is_some()); }
#[test] fn p79_1()  { let d = setup_state(); assert!(call_p79(d.path(), 1).is_some()); }
#[test] fn p79_2()  { let d = setup_state(); assert!(call_p79(d.path(), 2).is_some()); }
#[test] fn p79_3()  { let d = setup_state(); assert!(call_p79(d.path(), 3).is_some()); }
#[test] fn p79_4()  { let d = setup_state(); assert!(call_p79(d.path(), 4).is_some()); }
#[test] fn p79_5()  { let d = setup_state(); assert!(call_p79(d.path(), 5).is_some()); }
#[test] fn p79_6()  { let d = setup_state(); assert!(call_p79(d.path(), 6).is_some()); }
#[test] fn p79_7()  { let d = setup_state(); assert!(call_p79(d.path(), 7).is_some()); }
#[test] fn p79_8()  { let d = setup_state(); assert!(call_p79(d.path(), 8).is_some()); }
#[test] fn p79_9()  { let d = setup_state(); assert!(call_p79(d.path(), 9).is_some()); }
#[test] fn p79_10() { let d = setup_state(); assert!(call_p79(d.path(), 10).is_some()); }
#[test] fn p79_11() { let d = setup_state(); assert!(call_p79(d.path(), 11).is_some()); }
#[test] fn p79_12() { let d = setup_state(); assert!(call_p79(d.path(), 12).is_some()); }
#[test] fn p79_13() { let d = setup_state(); assert!(call_p79(d.path(), 13).is_some()); }
#[test] fn p79_14() { let d = setup_state(); assert!(call_p79(d.path(), 14).is_some()); }
#[test] fn p79_15() { let d = setup_state(); assert!(call_p79(d.path(), 15).is_some()); }
#[test] fn p79_16() { let d = setup_state(); assert!(call_p79(d.path(), 16).is_some()); }
#[test] fn p79_17() { let d = setup_state(); assert!(call_p79(d.path(), 17).is_some()); }
#[test] fn p79_18() { let d = setup_state(); assert!(call_p79(d.path(), 18).is_some()); }
#[test] fn p79_19() { let d = setup_state(); assert!(call_p79(d.path(), 19).is_some()); }
#[test] fn p79_20() { let d = setup_state(); assert!(call_p79(d.path(), 20).is_some()); }
#[test] fn p79_21() { let d = setup_state(); assert!(call_p79(d.path(), 21).is_some()); }
#[test] fn p79_22() { let d = setup_state(); assert!(call_p79(d.path(), 22).is_some()); }
#[test] fn p79_23() { let d = setup_state(); assert!(call_p79(d.path(), 23).is_some()); }
#[test] fn p79_none() { let d = setup_state(); assert!(call_p79(d.path(), 99).is_none()); }
