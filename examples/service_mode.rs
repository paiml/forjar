//! Demonstrates FJ-2700 service + dispatch mode runtime:
//! health check loop, restart decisions, dispatch execution, state tracking.
//!
//! Run with: `cargo run --example service_mode`

use forjar::core::task::dispatch::{
    format_dispatch_summary, prepare_dispatch, record_invocation, success_rate,
};
use forjar::core::task::service::{
    apply_restart, apply_start, apply_stop, format_service_summary, plan_service_action,
    process_health_check, restart_backoff, ServiceAction,
};
use forjar::core::types::{
    DispatchConfig, DispatchInvocation, DispatchState, HealthCheck, HealthCheckResult,
    RestartPolicy, ServiceState,
};

fn main() {
    println!("FJ-2700: Service + Dispatch Mode Runtime\n");

    // ── Service Mode Lifecycle ──
    println!("=== Service Mode: Full Lifecycle ===\n");

    let policy = RestartPolicy {
        max_restarts: 3,
        backoff_base_secs: 1.0,
        backoff_max_secs: 30.0,
        reset_after_secs: 300,
    };
    let hc = HealthCheck {
        command: "curl -sf http://localhost:8080/health".into(),
        interval: Some("10s".into()),
        timeout: Some("5s".into()),
        retries: Some(3),
    };

    // Step 1: Start
    let state = ServiceState::default();
    let action = plan_service_action(&state, &policy, &hc);
    println!("  State: not started");
    println!("  Action: {action:?}");
    assert!(matches!(action, ServiceAction::Start));

    let (mut state, event) = apply_start(1234, "2026-03-06T10:00:00Z");
    println!("  Event: {event}");
    println!("  {}\n", format_service_summary("web-api", &state));

    // Step 2: Health OK
    let ok_result = HealthCheckResult {
        healthy: true,
        exit_code: 0,
        duration_secs: 0.05,
        checked_at: "2026-03-06T10:00:10Z".into(),
        stdout: "OK".into(),
    };
    let (s, events) = process_health_check(&state, &ok_result);
    state = s;
    for e in &events {
        println!("  Event: {e}");
    }
    println!("  {}", format_service_summary("web-api", &state));

    // Step 3: Three consecutive failures → restart
    println!("\n  --- Simulating 3 health failures ---");
    let fail_result = HealthCheckResult {
        healthy: false,
        exit_code: 1,
        duration_secs: 5.0,
        checked_at: "2026-03-06T10:01:00Z".into(),
        stdout: "connection refused".into(),
    };
    for i in 1..=3 {
        let (s, events) = process_health_check(&state, &fail_result);
        state = s;
        for e in &events {
            println!("  Event: {e}");
        }
        if i == 3 {
            let action = plan_service_action(&state, &policy, &hc);
            println!("  Action: {action:?}");
            let backoff = restart_backoff(&policy, &state);
            println!("  Backoff: {backoff:.1}s");
        }
    }

    // Step 4: Restart
    let (s, event) = apply_restart(&state, 5678, "2026-03-06T10:02:00Z");
    state = s;
    println!("  Event: {event}");
    println!("  {}\n", format_service_summary("web-api", &state));

    // Step 5: Another failure cycle → stop
    for _ in 0..3 {
        let (s, _) = process_health_check(&state, &fail_result);
        state = s;
    }
    let (s, event) = apply_restart(&state, 9012, "2026-03-06T10:03:00Z");
    state = s;
    println!("  Restart #2: {event}");
    for _ in 0..3 {
        let (s, _) = process_health_check(&state, &fail_result);
        state = s;
    }
    let (s, event) = apply_restart(&state, 3456, "2026-03-06T10:04:00Z");
    state = s;
    println!("  Restart #3: {event}");

    let action = plan_service_action(&state, &policy, &hc);
    println!("  Action: {action:?}");
    if let ServiceAction::Stop(reason) = &action {
        let (_, event) = apply_stop(&state, reason, "2026-03-06T10:05:00Z");
        println!("  Event: {event}");
    }

    // ── Dispatch Mode ──
    println!("\n=== Dispatch Mode: Parameterized Execution ===\n");

    let config = DispatchConfig {
        name: "deploy".into(),
        command: "deploy --env {{ env }} --tag {{ tag }}".into(),
        params: vec![("env".into(), "staging".into())],
        timeout_secs: Some(300),
    };

    let prepared = prepare_dispatch(&config, &[("tag".into(), "v1.2.3".into())]);
    println!("  Prepared: {}", prepared.command);
    println!("  Timeout: {:?}s", prepared.timeout_secs);

    // Simulate invocations
    let mut dispatch_state = DispatchState::default();
    let invocations = [
        ("2026-03-06T09:00:00Z", 0, 1500, "ci"),
        ("2026-03-06T10:00:00Z", 0, 1200, "admin"),
        ("2026-03-06T11:00:00Z", 1, 800, "ci"),
        ("2026-03-06T12:00:00Z", 0, 1100, "admin"),
    ];
    for (ts, code, ms, caller) in invocations {
        record_invocation(
            &mut dispatch_state,
            DispatchInvocation {
                timestamp: ts.into(),
                exit_code: code,
                duration_ms: ms,
                caller: Some(caller.into()),
            },
            10,
        );
    }

    println!("\n{}", format_dispatch_summary("deploy", &dispatch_state));
    println!("  Success rate: {:.0}%", success_rate(&dispatch_state));
}
