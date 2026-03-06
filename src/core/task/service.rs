//! FJ-2700: Service mode runtime — health check loop, restart decisions, state transitions.
//!
//! Implements the service lifecycle: start → health check → restart/stop.
//! The executor drives the loop; this module provides pure decision functions.

use crate::core::types::{
    HealthCheck, HealthCheckResult, RestartPolicy, ServiceEvent, ServiceState,
};

/// Action the executor should take for a service-mode task.
#[derive(Debug, Clone, PartialEq)]
pub enum ServiceAction {
    /// Start the service process (initial or after restart decision).
    Start,
    /// Run a health check.
    CheckHealth,
    /// Restart the service (health failures exceeded threshold).
    Restart,
    /// Stop the service (max restarts exceeded or manual stop).
    Stop(String),
    /// Wait for the next health check interval.
    Wait { delay_secs: f64 },
}

/// Determine the next action for a service given its current state.
///
/// # Examples
///
/// ```
/// use forjar::core::task::service::{plan_service_action, ServiceAction};
/// use forjar::core::types::{HealthCheck, RestartPolicy, ServiceState};
///
/// let state = ServiceState::default(); // no PID → not started
/// let policy = RestartPolicy::default();
/// let hc = HealthCheck { command: "true".into(), ..Default::default() };
/// assert_eq!(plan_service_action(&state, &policy, &hc), ServiceAction::Start);
/// ```
pub fn plan_service_action(
    state: &ServiceState,
    policy: &RestartPolicy,
    health_check: &HealthCheck,
) -> ServiceAction {
    // Not started yet
    if state.pid.is_none() {
        return ServiceAction::Start;
    }

    // Check if we've exceeded max restarts
    if state.restart_count > 0 && !policy.should_restart(state.restart_count) {
        return ServiceAction::Stop(format!(
            "max restarts ({}) exceeded",
            policy.max_restarts
        ));
    }

    // Consecutive failures exceed health check retries → restart
    let retries = health_check.retries.unwrap_or(3);
    if state.consecutive_failures >= retries {
        return ServiceAction::Restart;
    }

    // Otherwise, check health or wait
    if state.last_check.is_none() {
        ServiceAction::CheckHealth
    } else {
        let delay = parse_interval(health_check.interval.as_deref());
        ServiceAction::Wait { delay_secs: delay }
    }
}

/// Update service state after a health check result.
///
/// Returns the updated state and any service events generated.
///
/// # Examples
///
/// ```
/// use forjar::core::task::service::process_health_check;
/// use forjar::core::types::{HealthCheckResult, ServiceState};
///
/// let state = ServiceState { pid: Some(1234), healthy: true, ..Default::default() };
/// let result = HealthCheckResult {
///     healthy: true, exit_code: 0, duration_secs: 0.01,
///     checked_at: "2026-03-06T00:00:00Z".into(), stdout: String::new(),
/// };
/// let (new_state, events) = process_health_check(&state, &result);
/// assert!(new_state.healthy);
/// assert_eq!(new_state.consecutive_failures, 0);
/// assert_eq!(events.len(), 1); // HealthOk event
/// ```
pub fn process_health_check(
    state: &ServiceState,
    result: &HealthCheckResult,
) -> (ServiceState, Vec<ServiceEvent>) {
    let mut events = Vec::new();
    let mut new_state = state.clone();
    new_state.last_check = Some(result.checked_at.clone());

    if result.healthy {
        new_state.healthy = true;
        new_state.consecutive_failures = 0;
        events.push(ServiceEvent::HealthOk {
            at: result.checked_at.clone(),
        });
    } else {
        new_state.consecutive_failures += 1;
        new_state.healthy = false;
        events.push(ServiceEvent::HealthFail {
            exit_code: result.exit_code,
            consecutive: new_state.consecutive_failures,
            at: result.checked_at.clone(),
        });
    }

    (new_state, events)
}

/// Apply a restart to the service state, returning updated state and event.
///
/// # Examples
///
/// ```
/// use forjar::core::task::service::apply_restart;
/// use forjar::core::types::ServiceState;
///
/// let state = ServiceState {
///     pid: Some(1000), healthy: false, consecutive_failures: 3,
///     restart_count: 0, last_check: None,
/// };
/// let (new_state, event) = apply_restart(&state, 2000, "2026-03-06T00:00:00Z");
/// assert_eq!(new_state.pid, Some(2000));
/// assert_eq!(new_state.restart_count, 1);
/// assert_eq!(new_state.consecutive_failures, 0);
/// ```
pub fn apply_restart(
    state: &ServiceState,
    new_pid: u32,
    timestamp: &str,
) -> (ServiceState, ServiceEvent) {
    let old_pid = state.pid.unwrap_or(0);
    let restart_count = state.restart_count + 1;

    let new_state = ServiceState {
        pid: Some(new_pid),
        healthy: false,
        consecutive_failures: 0,
        last_check: None,
        restart_count,
    };

    let event = ServiceEvent::Restarted {
        old_pid,
        new_pid,
        restart_count,
        at: timestamp.to_string(),
    };

    (new_state, event)
}

/// Apply initial start to the service state.
pub fn apply_start(pid: u32, timestamp: &str) -> (ServiceState, ServiceEvent) {
    let state = ServiceState {
        pid: Some(pid),
        healthy: false,
        consecutive_failures: 0,
        last_check: None,
        restart_count: 0,
    };
    let event = ServiceEvent::Started {
        pid,
        at: timestamp.to_string(),
    };
    (state, event)
}

/// Apply stop to the service state.
pub fn apply_stop(state: &ServiceState, reason: &str, timestamp: &str) -> (ServiceState, ServiceEvent) {
    let new_state = ServiceState {
        pid: None,
        healthy: false,
        consecutive_failures: state.consecutive_failures,
        last_check: state.last_check.clone(),
        restart_count: state.restart_count,
    };
    let event = ServiceEvent::Stopped {
        reason: reason.to_string(),
        at: timestamp.to_string(),
    };
    (new_state, event)
}

/// Calculate backoff delay for a restart using the policy.
pub fn restart_backoff(policy: &RestartPolicy, state: &ServiceState) -> f64 {
    policy.backoff_secs(state.restart_count)
}

/// Format a human-readable service summary.
///
/// # Examples
///
/// ```
/// use forjar::core::task::service::format_service_summary;
/// use forjar::core::types::ServiceState;
///
/// let state = ServiceState {
///     pid: Some(42), healthy: true, consecutive_failures: 0,
///     last_check: Some("2026-03-06T00:00:00Z".into()), restart_count: 0,
/// };
/// let summary = format_service_summary("my-svc", &state);
/// assert!(summary.contains("pid=42"));
/// assert!(summary.contains("healthy"));
/// ```
pub fn format_service_summary(name: &str, state: &ServiceState) -> String {
    let status = match state.pid {
        None => "stopped".to_string(),
        Some(pid) if state.healthy => format!("healthy (pid={pid})"),
        Some(pid) => format!(
            "unhealthy (pid={pid}, failures={})",
            state.consecutive_failures
        ),
    };

    let restarts = if state.restart_count > 0 {
        format!(" restarts={}", state.restart_count)
    } else {
        String::new()
    };

    format!("service/{name}: {status}{restarts}")
}

/// Parse a duration string like "30s", "5m", "1h" to seconds.
pub(crate) fn parse_interval(interval: Option<&str>) -> f64 {
    let s = match interval {
        Some(s) => s,
        None => return 30.0, // default 30s
    };

    if let Some(num) = s.strip_suffix('s') {
        num.parse::<f64>().unwrap_or(30.0)
    } else if let Some(num) = s.strip_suffix('m') {
        num.parse::<f64>().unwrap_or(0.5) * 60.0
    } else if let Some(num) = s.strip_suffix('h') {
        num.parse::<f64>().unwrap_or(1.0) * 3600.0
    } else {
        s.parse::<f64>().unwrap_or(30.0)
    }
}

