//! Tests for service mode runtime (FJ-2700).

use super::service::*;
use crate::core::types::{
    HealthCheck, HealthCheckResult, RestartPolicy, ServiceEvent, ServiceState,
};

#[test]
fn plan_action_not_started() {
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
fn plan_action_healthy_wait() {
    let state = ServiceState {
        pid: Some(123),
        healthy: true,
        consecutive_failures: 0,
        last_check: Some("ts".into()),
        restart_count: 0,
    };
    let policy = RestartPolicy::default();
    let hc = HealthCheck {
        command: "true".into(),
        interval: Some("10s".into()),
        ..Default::default()
    };
    match plan_service_action(&state, &policy, &hc) {
        ServiceAction::Wait { delay_secs } => assert!((delay_secs - 10.0).abs() < 0.01),
        other => panic!("expected Wait, got {other:?}"),
    }
}

#[test]
fn plan_action_needs_restart() {
    let state = ServiceState {
        pid: Some(123),
        healthy: false,
        consecutive_failures: 3,
        last_check: Some("ts".into()),
        restart_count: 0,
    };
    let policy = RestartPolicy::default();
    let hc = HealthCheck {
        command: "check".into(),
        retries: Some(3),
        ..Default::default()
    };
    assert_eq!(
        plan_service_action(&state, &policy, &hc),
        ServiceAction::Restart
    );
}

#[test]
fn plan_action_max_restarts_exceeded() {
    let state = ServiceState {
        pid: Some(123),
        healthy: false,
        consecutive_failures: 3,
        last_check: Some("ts".into()),
        restart_count: 5,
    };
    let policy = RestartPolicy::default();
    let hc = HealthCheck {
        command: "check".into(),
        retries: Some(3),
        ..Default::default()
    };
    match plan_service_action(&state, &policy, &hc) {
        ServiceAction::Stop(reason) => assert!(reason.contains("max restarts")),
        other => panic!("expected Stop, got {other:?}"),
    }
}

#[test]
fn plan_action_first_health_check() {
    let state = ServiceState {
        pid: Some(123),
        healthy: false,
        consecutive_failures: 0,
        last_check: None,
        restart_count: 0,
    };
    let policy = RestartPolicy::default();
    let hc = HealthCheck {
        command: "check".into(),
        ..Default::default()
    };
    assert_eq!(
        plan_service_action(&state, &policy, &hc),
        ServiceAction::CheckHealth
    );
}

#[test]
fn process_health_check_ok() {
    let state = ServiceState {
        pid: Some(100),
        healthy: false,
        consecutive_failures: 2,
        last_check: None,
        restart_count: 0,
    };
    let result = HealthCheckResult {
        healthy: true,
        exit_code: 0,
        duration_secs: 0.05,
        checked_at: "t1".into(),
        stdout: String::new(),
    };
    let (new_state, events) = process_health_check(&state, &result);
    assert!(new_state.healthy);
    assert_eq!(new_state.consecutive_failures, 0);
    assert!(matches!(events[0], ServiceEvent::HealthOk { .. }));
}

#[test]
fn process_health_check_fail() {
    let state = ServiceState {
        pid: Some(100),
        healthy: true,
        consecutive_failures: 0,
        last_check: None,
        restart_count: 0,
    };
    let result = HealthCheckResult {
        healthy: false,
        exit_code: 1,
        duration_secs: 5.0,
        checked_at: "t2".into(),
        stdout: "timeout".into(),
    };
    let (new_state, events) = process_health_check(&state, &result);
    assert!(!new_state.healthy);
    assert_eq!(new_state.consecutive_failures, 1);
    assert!(matches!(events[0], ServiceEvent::HealthFail { .. }));
}

#[test]
fn apply_restart_updates_state() {
    let state = ServiceState {
        pid: Some(100),
        healthy: false,
        consecutive_failures: 3,
        last_check: Some("old".into()),
        restart_count: 1,
    };
    let (new_state, event) = apply_restart(&state, 200, "t3");
    assert_eq!(new_state.pid, Some(200));
    assert_eq!(new_state.restart_count, 2);
    assert_eq!(new_state.consecutive_failures, 0);
    assert!(new_state.last_check.is_none());
    assert!(matches!(
        event,
        ServiceEvent::Restarted {
            old_pid: 100,
            new_pid: 200,
            ..
        }
    ));
}

#[test]
fn apply_start_creates_state() {
    let (state, event) = apply_start(42, "t0");
    assert_eq!(state.pid, Some(42));
    assert_eq!(state.restart_count, 0);
    assert!(matches!(event, ServiceEvent::Started { pid: 42, .. }));
}

#[test]
fn apply_stop_clears_pid() {
    let state = ServiceState {
        pid: Some(100),
        healthy: true,
        consecutive_failures: 0,
        last_check: Some("ts".into()),
        restart_count: 2,
    };
    let (new_state, event) = apply_stop(&state, "manual", "t4");
    assert!(new_state.pid.is_none());
    assert!(!new_state.healthy);
    assert_eq!(new_state.restart_count, 2);
    assert!(matches!(event, ServiceEvent::Stopped { .. }));
}

#[test]
fn restart_backoff_exponential() {
    let policy = RestartPolicy {
        backoff_base_secs: 1.0,
        backoff_max_secs: 60.0,
        ..Default::default()
    };
    let state = ServiceState {
        restart_count: 0,
        ..Default::default()
    };
    assert!((restart_backoff(&policy, &state) - 1.0).abs() < 0.01);

    let state3 = ServiceState {
        restart_count: 3,
        ..Default::default()
    };
    assert!((restart_backoff(&policy, &state3) - 8.0).abs() < 0.01);
}

#[test]
fn format_summary_healthy() {
    let state = ServiceState {
        pid: Some(42),
        healthy: true,
        consecutive_failures: 0,
        last_check: Some("ts".into()),
        restart_count: 0,
    };
    let s = format_service_summary("web", &state);
    assert!(s.contains("healthy"));
    assert!(s.contains("pid=42"));
}

#[test]
fn format_summary_stopped() {
    let state = ServiceState::default();
    assert!(format_service_summary("api", &state).contains("stopped"));
}

#[test]
fn format_summary_with_restarts() {
    let state = ServiceState {
        pid: Some(99),
        healthy: true,
        consecutive_failures: 0,
        last_check: None,
        restart_count: 3,
    };
    assert!(format_service_summary("db", &state).contains("restarts=3"));
}

#[test]
fn parse_interval_seconds() {
    assert!((super::service::parse_interval(Some("10s")) - 10.0).abs() < 0.01);
}

#[test]
fn parse_interval_minutes() {
    assert!((super::service::parse_interval(Some("2m")) - 120.0).abs() < 0.01);
}

#[test]
fn parse_interval_hours() {
    assert!((super::service::parse_interval(Some("1h")) - 3600.0).abs() < 0.01);
}

#[test]
fn parse_interval_default() {
    assert!((super::service::parse_interval(None) - 30.0).abs() < 0.01);
}

#[test]
fn parse_interval_bare_number() {
    assert!((super::service::parse_interval(Some("15")) - 15.0).abs() < 0.01);
}

#[test]
fn full_service_lifecycle() {
    let policy = RestartPolicy {
        max_restarts: 2,
        ..Default::default()
    };
    let hc = HealthCheck {
        command: "check".into(),
        interval: Some("5s".into()),
        retries: Some(2),
        timeout: None,
    };

    // Start
    let (mut state, _) = apply_start(100, "t0");
    assert_eq!(
        plan_service_action(&state, &policy, &hc),
        ServiceAction::CheckHealth
    );

    // Health OK
    let ok = HealthCheckResult {
        healthy: true,
        exit_code: 0,
        duration_secs: 0.01,
        checked_at: "t1".into(),
        stdout: String::new(),
    };
    let (s, _) = process_health_check(&state, &ok);
    state = s;
    assert!(state.healthy);

    // Two consecutive failures → restart
    let fail = HealthCheckResult {
        healthy: false,
        exit_code: 1,
        duration_secs: 5.0,
        checked_at: "t2".into(),
        stdout: String::new(),
    };
    let (s, _) = process_health_check(&state, &fail);
    state = s;
    let (s, _) = process_health_check(&state, &fail);
    state = s;
    assert_eq!(state.consecutive_failures, 2);
    assert_eq!(
        plan_service_action(&state, &policy, &hc),
        ServiceAction::Restart
    );

    let (s, _) = apply_restart(&state, 200, "t3");
    state = s;
    assert_eq!(state.restart_count, 1);

    // Two more failures → restart again
    let (s, _) = process_health_check(&state, &fail);
    state = s;
    let (s, _) = process_health_check(&state, &fail);
    state = s;
    assert_eq!(
        plan_service_action(&state, &policy, &hc),
        ServiceAction::Restart
    );

    let (s, _) = apply_restart(&state, 300, "t4");
    state = s;
    assert_eq!(state.restart_count, 2);

    // Max restarts exceeded → stop
    match plan_service_action(&state, &policy, &hc) {
        ServiceAction::Stop(reason) => assert!(reason.contains("max restarts")),
        other => panic!("expected Stop, got {other:?}"),
    }
}
