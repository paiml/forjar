//! FJ-2700/2702/2703/2704: Task framework runtime demonstration.
//!
//! Demonstrates:
//! - Dispatch mode: param substitution, validation, script generation, invocation recording
//! - Quality gates: exit code, JSON threshold, regex matching
//! - GPU scheduling: round-robin, multi-GPU, utilization
//! - Barrier synchronization: multi-machine coordination
//! - Service lifecycle: start → health check → restart → stop
//!
//! Usage: cargo run --example task_runtime

use forjar::core::task::dispatch::{
    dispatch_script, format_dispatch_summary, prepare_dispatch, record_invocation, success_rate,
    validate_dispatch,
};
use forjar::core::task::service::{
    apply_restart, apply_start, format_service_summary, plan_service_action, process_health_check,
    restart_backoff, ServiceAction,
};
use forjar::core::task::{evaluate_gate, gpu_env_vars, GateResult};
use forjar::core::types::{
    BarrierTask, DispatchConfig, DispatchInvocation, DispatchState, GpuSchedule, HealthCheck,
    HealthCheckResult, QualityGate, RestartPolicy, ServiceState,
};

fn main() {
    println!("Forjar: Task Framework Runtime");
    println!("{}", "=".repeat(50));

    // ── FJ-2700: Dispatch Mode ──
    println!("\n[FJ-2700] Dispatch Mode:");
    let config = DispatchConfig {
        name: "deploy".into(),
        command: "deploy --env {{ env }} --region {{ region }}".into(),
        params: vec![
            ("env".into(), "production".into()),
            ("region".into(), "us-east-1".into()),
        ],
        timeout_secs: Some(300),
    };
    assert!(validate_dispatch(&config).is_ok());
    let prepared = prepare_dispatch(&config, &[]);
    println!("  Command: {}", prepared.command);
    assert_eq!(
        prepared.command,
        "deploy --env production --region us-east-1"
    );

    let script = dispatch_script(&prepared);
    println!("  Script starts with: {}", &script[..19]);
    assert!(script.starts_with("set -euo pipefail"));

    let mut state = DispatchState::default();
    for i in 0..5 {
        record_invocation(
            &mut state,
            DispatchInvocation {
                timestamp: format!("2026-03-09T0{i}:00:00Z"),
                exit_code: if i == 2 { 1 } else { 0 },
                duration_ms: 500 + i * 100,
                caller: Some("ci".into()),
            },
            10,
        );
    }
    println!("  Success rate: {:.0}%", success_rate(&state));
    assert!((success_rate(&state) - 80.0).abs() < 0.01);
    let summary = format_dispatch_summary("deploy", &state);
    println!("  {}", summary.lines().next().unwrap());

    // ── FJ-2702: Quality Gates ──
    println!("\n[FJ-2702] Quality Gates:");

    // Exit code gate
    assert_eq!(
        evaluate_gate(&QualityGate::default(), 0, ""),
        GateResult::Pass
    );
    println!("  Exit code 0: Pass");

    // JSON threshold gate
    let gate = QualityGate {
        parse: Some("json".into()),
        field: Some("grade".into()),
        threshold: vec!["A".into(), "B".into()],
        ..Default::default()
    };
    let result = evaluate_gate(&gate, 0, r#"{"grade":"A","score":95}"#);
    println!("  JSON grade=A threshold [A,B]: {:?}", result);
    assert_eq!(result, GateResult::Pass);

    // Regex gate
    let gate = QualityGate {
        regex: Some(r"ALL \d+ TESTS PASSED".into()),
        ..Default::default()
    };
    assert_eq!(
        evaluate_gate(&gate, 0, "ALL 42 TESTS PASSED"),
        GateResult::Pass
    );
    println!("  Regex 'ALL \\d+ TESTS PASSED': Pass");

    // ── FJ-2703: GPU Scheduling ──
    println!("\n[FJ-2703] GPU Scheduling:");
    let vars = gpu_env_vars(Some(0));
    println!("  GPU device 0: {:?}", vars);
    assert_eq!(vars[0].0, "CUDA_VISIBLE_DEVICES");

    let tasks = &["train", "eval", "infer", "export"];
    let schedule = GpuSchedule::round_robin(tasks, 2);
    for task in tasks {
        println!(
            "  {task} → GPU {}",
            schedule.cuda_visible_devices(task).unwrap()
        );
    }
    assert!(schedule.fully_utilized());
    println!("  Fully utilized: {}", schedule.fully_utilized());

    // ── FJ-2704: Barrier ──
    println!("\n[FJ-2704] Barrier Synchronization:");
    let mut barrier =
        BarrierTask::new("deploy-sync", vec!["web".into(), "api".into(), "db".into()]);
    println!("  {barrier}");
    barrier.mark_complete("web");
    barrier.mark_complete("api");
    println!("  After web+api: {barrier}");
    barrier.mark_complete("db");
    println!("  After db: {barrier}");
    assert!(barrier.is_satisfied());

    // ── FJ-2700: Service Lifecycle ──
    println!("\n[FJ-2700] Service Lifecycle:");
    let policy = RestartPolicy {
        max_restarts: 2,
        ..Default::default()
    };
    let hc = HealthCheck {
        command: "curl localhost:8080/health".into(),
        retries: Some(2),
        ..Default::default()
    };

    // Start
    let (state, _) = apply_start(1000, "t0");
    println!("  {}", format_service_summary("web", &state));

    // Healthy check
    let (state, _) = process_health_check(
        &state,
        &HealthCheckResult {
            healthy: true,
            exit_code: 0,
            duration_secs: 0.01,
            checked_at: "t1".into(),
            stdout: String::new(),
        },
    );
    println!("  {}", format_service_summary("web", &state));
    assert!(state.healthy);

    // Failures → restart
    let (state, _) = process_health_check(
        &state,
        &HealthCheckResult {
            healthy: false,
            exit_code: 1,
            duration_secs: 5.0,
            checked_at: "t2".into(),
            stdout: String::new(),
        },
    );
    let (state, _) = process_health_check(
        &state,
        &HealthCheckResult {
            healthy: false,
            exit_code: 1,
            duration_secs: 5.0,
            checked_at: "t3".into(),
            stdout: String::new(),
        },
    );
    assert_eq!(
        plan_service_action(&state, &policy, &hc),
        ServiceAction::Restart
    );
    let backoff = restart_backoff(&policy, &state);
    println!("  Backoff: {backoff:.1}s");
    let (state, _) = apply_restart(&state, 2000, "t4");
    println!("  {}", format_service_summary("web", &state));
    assert_eq!(state.restart_count, 1);

    println!("\n{}", "=".repeat(50));
    println!("All task runtime criteria survived.");
}
