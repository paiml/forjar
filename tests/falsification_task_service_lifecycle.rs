//! FJ-2700: Service mode lifecycle — health checks, restarts, state transitions.
//!
//! Popperian rejection criteria for:
//! - FJ-2700: plan_service_action (Start, CheckHealth, Restart, Stop, Wait)
//! - FJ-2700: process_health_check (healthy/unhealthy, consecutive failures)
//! - FJ-2700: apply_restart (PID swap, counter increment, failure reset)
//! - FJ-2700: apply_start (initial start, PID assignment)
//! - FJ-2700: apply_stop (PID cleared, reason recorded)
//! - FJ-2700: restart_backoff (exponential with cap)
//! - FJ-2700: format_service_summary (healthy, unhealthy, stopped)
//!
//! Usage: cargo test --test falsification_task_service_lifecycle

use forjar::core::task::service::{
    apply_restart, apply_start, apply_stop, format_service_summary, plan_service_action,
    process_health_check, restart_backoff, ServiceAction,
};
use forjar::core::types::{
    HealthCheck, HealthCheckResult, RestartPolicy, ServiceEvent, ServiceState,
};

// ============================================================================
// FJ-2700: plan_service_action — state machine decisions
// ============================================================================

#[test]
fn plan_start_when_no_pid() {
    let state = ServiceState::default();
    let policy = RestartPolicy::default();
    let hc = HealthCheck {
        command: "true".into(),
        ..Default::default()
    };
    assert_eq!(
        plan_service_action(&state, &policy, &hc),
        ServiceAction::Start
    );
}

#[test]
fn plan_check_health_when_running_no_checks_yet() {
    let state = ServiceState {
        pid: Some(1234),
        healthy: false,
        consecutive_failures: 0,
        last_check: None,
        restart_count: 0,
    };
    let policy = RestartPolicy::default();
    let hc = HealthCheck {
        command: "curl localhost".into(),
        ..Default::default()
    };
    assert_eq!(
        plan_service_action(&state, &policy, &hc),
        ServiceAction::CheckHealth
    );
}

#[test]
fn plan_wait_after_successful_check() {
    let state = ServiceState {
        pid: Some(1234),
        healthy: true,
        consecutive_failures: 0,
        last_check: Some("2026-03-09T00:00:00Z".into()),
        restart_count: 0,
    };
    let policy = RestartPolicy::default();
    let hc = HealthCheck {
        command: "true".into(),
        interval: Some("10s".into()),
        ..Default::default()
    };
    match plan_service_action(&state, &policy, &hc) {
        ServiceAction::Wait { delay_secs } => {
            assert!((delay_secs - 10.0).abs() < 0.01);
        }
        other => panic!("expected Wait, got {:?}", other),
    }
}

#[test]
fn plan_restart_on_consecutive_failures_exceeding_retries() {
    let state = ServiceState {
        pid: Some(1234),
        healthy: false,
        consecutive_failures: 3,
        last_check: Some("t".into()),
        restart_count: 0,
    };
    let policy = RestartPolicy::default();
    let hc = HealthCheck {
        command: "true".into(),
        retries: Some(3),
        ..Default::default()
    };
    assert_eq!(
        plan_service_action(&state, &policy, &hc),
        ServiceAction::Restart
    );
}

#[test]
fn plan_stop_when_max_restarts_exceeded() {
    let state = ServiceState {
        pid: Some(1234),
        healthy: false,
        consecutive_failures: 0,
        last_check: None,
        restart_count: 5,
    };
    let policy = RestartPolicy {
        max_restarts: 5,
        ..Default::default()
    };
    let hc = HealthCheck {
        command: "true".into(),
        ..Default::default()
    };
    match plan_service_action(&state, &policy, &hc) {
        ServiceAction::Stop(reason) => {
            assert!(reason.contains("max restarts"));
        }
        other => panic!("expected Stop, got {:?}", other),
    }
}

#[test]
fn plan_default_retries_is_three() {
    // With no retries configured, default is 3
    let state = ServiceState {
        pid: Some(1234),
        healthy: false,
        consecutive_failures: 3,
        last_check: Some("t".into()),
        restart_count: 0,
    };
    let policy = RestartPolicy::default();
    let hc = HealthCheck {
        command: "true".into(),
        retries: None, // default → 3
        ..Default::default()
    };
    assert_eq!(
        plan_service_action(&state, &policy, &hc),
        ServiceAction::Restart
    );
}

// ============================================================================
// FJ-2700: process_health_check — state updates
// ============================================================================

#[test]
fn health_check_success_resets_failures() {
    let state = ServiceState {
        pid: Some(1234),
        healthy: false,
        consecutive_failures: 2,
        last_check: None,
        restart_count: 0,
    };
    let result = HealthCheckResult {
        healthy: true,
        exit_code: 0,
        duration_secs: 0.01,
        checked_at: "2026-03-09T00:00:00Z".into(),
        stdout: String::new(),
    };
    let (new_state, events) = process_health_check(&state, &result);
    assert!(new_state.healthy);
    assert_eq!(new_state.consecutive_failures, 0);
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0], ServiceEvent::HealthOk { .. }));
}

#[test]
fn health_check_failure_increments_counter() {
    let state = ServiceState {
        pid: Some(1234),
        healthy: true,
        consecutive_failures: 0,
        last_check: None,
        restart_count: 0,
    };
    let result = HealthCheckResult {
        healthy: false,
        exit_code: 1,
        duration_secs: 5.0,
        checked_at: "2026-03-09T00:00:00Z".into(),
        stdout: "timeout".into(),
    };
    let (new_state, events) = process_health_check(&state, &result);
    assert!(!new_state.healthy);
    assert_eq!(new_state.consecutive_failures, 1);
    assert_eq!(events.len(), 1);
    match &events[0] {
        ServiceEvent::HealthFail {
            exit_code,
            consecutive,
            ..
        } => {
            assert_eq!(*exit_code, 1);
            assert_eq!(*consecutive, 1);
        }
        other => panic!("expected HealthFail, got {:?}", other),
    }
}

#[test]
fn health_check_updates_last_check_timestamp() {
    let state = ServiceState {
        pid: Some(1234),
        last_check: None,
        ..Default::default()
    };
    let result = HealthCheckResult {
        healthy: true,
        exit_code: 0,
        duration_secs: 0.01,
        checked_at: "2026-03-09T12:00:00Z".into(),
        stdout: String::new(),
    };
    let (new_state, _) = process_health_check(&state, &result);
    assert_eq!(
        new_state.last_check.as_deref(),
        Some("2026-03-09T12:00:00Z")
    );
}

// ============================================================================
// FJ-2700: apply_restart — PID swap and counter
// ============================================================================

#[test]
fn apply_restart_swaps_pid() {
    let state = ServiceState {
        pid: Some(1000),
        healthy: false,
        consecutive_failures: 3,
        restart_count: 1,
        last_check: Some("t".into()),
    };
    let (new_state, event) = apply_restart(&state, 2000, "2026-03-09T00:00:00Z");
    assert_eq!(new_state.pid, Some(2000));
    assert_eq!(new_state.restart_count, 2);
    assert_eq!(new_state.consecutive_failures, 0);
    assert!(new_state.last_check.is_none());
    match event {
        ServiceEvent::Restarted {
            old_pid,
            new_pid,
            restart_count,
            ..
        } => {
            assert_eq!(old_pid, 1000);
            assert_eq!(new_pid, 2000);
            assert_eq!(restart_count, 2);
        }
        other => panic!("expected Restarted, got {:?}", other),
    }
}

// ============================================================================
// FJ-2700: apply_start
// ============================================================================

#[test]
fn apply_start_sets_pid() {
    let (state, event) = apply_start(4242, "2026-03-09T00:00:00Z");
    assert_eq!(state.pid, Some(4242));
    assert_eq!(state.restart_count, 0);
    assert!(!state.healthy);
    match event {
        ServiceEvent::Started { pid, .. } => assert_eq!(pid, 4242),
        other => panic!("expected Started, got {:?}", other),
    }
}

// ============================================================================
// FJ-2700: apply_stop
// ============================================================================

#[test]
fn apply_stop_clears_pid() {
    let state = ServiceState {
        pid: Some(1234),
        healthy: true,
        consecutive_failures: 0,
        restart_count: 2,
        last_check: Some("t".into()),
    };
    let (new_state, event) = apply_stop(&state, "max restarts exceeded", "2026-03-09T01:00:00Z");
    assert!(new_state.pid.is_none());
    assert!(!new_state.healthy);
    assert_eq!(new_state.restart_count, 2); // preserved
    match event {
        ServiceEvent::Stopped { reason, .. } => {
            assert!(reason.contains("max restarts"));
        }
        other => panic!("expected Stopped, got {:?}", other),
    }
}

// ============================================================================
// FJ-2700: restart_backoff — exponential with cap
// ============================================================================

#[test]
fn backoff_exponential_growth() {
    let policy = RestartPolicy::default(); // base=1.0, max=60.0
    let state0 = ServiceState {
        restart_count: 0,
        ..Default::default()
    };
    let state1 = ServiceState {
        restart_count: 1,
        ..Default::default()
    };
    let state2 = ServiceState {
        restart_count: 2,
        ..Default::default()
    };

    let b0 = restart_backoff(&policy, &state0);
    let b1 = restart_backoff(&policy, &state1);
    let b2 = restart_backoff(&policy, &state2);

    assert!((b0 - 1.0).abs() < 0.01);
    assert!((b1 - 2.0).abs() < 0.01);
    assert!((b2 - 4.0).abs() < 0.01);
}

#[test]
fn backoff_caps_at_max() {
    let policy = RestartPolicy {
        backoff_max_secs: 30.0,
        ..Default::default()
    };
    let state = ServiceState {
        restart_count: 10,
        ..Default::default()
    };
    let delay = restart_backoff(&policy, &state);
    assert!((delay - 30.0).abs() < 0.01);
}

// ============================================================================
// FJ-2700: format_service_summary
// ============================================================================

#[test]
fn summary_healthy_service() {
    let state = ServiceState {
        pid: Some(42),
        healthy: true,
        consecutive_failures: 0,
        last_check: Some("t".into()),
        restart_count: 0,
    };
    let summary = format_service_summary("my-app", &state);
    assert!(summary.contains("my-app"));
    assert!(summary.contains("healthy"));
    assert!(summary.contains("pid=42"));
}

#[test]
fn summary_unhealthy_service() {
    let state = ServiceState {
        pid: Some(99),
        healthy: false,
        consecutive_failures: 2,
        last_check: Some("t".into()),
        restart_count: 0,
    };
    let summary = format_service_summary("db", &state);
    assert!(summary.contains("unhealthy"));
    assert!(summary.contains("failures=2"));
}

#[test]
fn summary_stopped_service() {
    let state = ServiceState {
        pid: None,
        ..Default::default()
    };
    let summary = format_service_summary("worker", &state);
    assert!(summary.contains("stopped"));
}

#[test]
fn summary_with_restarts() {
    let state = ServiceState {
        pid: Some(500),
        healthy: true,
        consecutive_failures: 0,
        last_check: Some("t".into()),
        restart_count: 3,
    };
    let summary = format_service_summary("api", &state);
    assert!(summary.contains("restarts=3"));
}

// ============================================================================
// FJ-2700: Full lifecycle scenario — start → check → fail → restart → stop
// ============================================================================

fn hc_result(healthy: bool, at: &str) -> HealthCheckResult {
    HealthCheckResult {
        healthy,
        exit_code: if healthy { 0 } else { 1 },
        duration_secs: 0.01,
        checked_at: at.into(),
        stdout: String::new(),
    }
}

#[test]
fn full_lifecycle_start_check_fail_restart_stop() {
    let policy = RestartPolicy {
        max_restarts: 2,
        ..Default::default()
    };
    let hc = HealthCheck {
        command: "curl localhost/health".into(),
        retries: Some(2),
        ..Default::default()
    };

    // Start → first health check → healthy
    assert_eq!(
        plan_service_action(&ServiceState::default(), &policy, &hc),
        ServiceAction::Start
    );
    let (state, _) = apply_start(100, "t0");
    let (state, _) = process_health_check(&state, &hc_result(true, "t1"));
    assert!(state.healthy);

    // Two failures → restart
    let (state, _) = process_health_check(&state, &hc_result(false, "t2"));
    let (state, _) = process_health_check(&state, &hc_result(false, "t3"));
    assert_eq!(
        plan_service_action(&state, &policy, &hc),
        ServiceAction::Restart
    );
    let (state, _) = apply_restart(&state, 200, "t4");

    // Two more failures → second restart → max exceeded → stop
    let (state, _) = process_health_check(&state, &hc_result(false, "t5"));
    let (state, _) = process_health_check(&state, &hc_result(false, "t6"));
    let (state, _) = apply_restart(&state, 300, "t7");
    match plan_service_action(&state, &policy, &hc) {
        ServiceAction::Stop(reason) => assert!(reason.contains("max restarts")),
        other => panic!("expected Stop, got {:?}", other),
    }
}
