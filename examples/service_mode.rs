//! Demonstrates FJ-2700 service mode types: restart policy, health checks, events.

use forjar::core::types::{
    DispatchConfig, GpuMemoryInfo, HealthCheckResult, IoHashes, RestartPolicy, ServiceEvent,
};

fn main() {
    // Restart policy with exponential backoff
    println!("=== Restart Policy ===");
    let policy = RestartPolicy::default();
    println!("  Max restarts: {}", policy.max_restarts);
    println!("  Reset after: {}s healthy", policy.reset_after_secs);
    println!();
    for i in 0..7 {
        let restart = policy.should_restart(i);
        let backoff = policy.backoff_secs(i);
        println!(
            "  Failure #{i}: restart={restart}, backoff={backoff:.1}s"
        );
    }

    // Health check results
    println!("\n=== Health Checks ===");
    let ok = HealthCheckResult {
        healthy: true,
        exit_code: 0,
        duration_secs: 0.05,
        checked_at: "2026-03-05T14:30:00Z".into(),
        stdout: "OK".into(),
    };
    println!("  {ok}");

    let fail = HealthCheckResult {
        healthy: false,
        exit_code: 1,
        duration_secs: 5.0,
        checked_at: "2026-03-05T14:30:30Z".into(),
        stdout: "connection refused".into(),
    };
    println!("  {fail}");

    // Service lifecycle events
    println!("\n=== Service Events ===");
    let events: Vec<ServiceEvent> = vec![
        ServiceEvent::Started { pid: 1234, at: "14:30:00".into() },
        ServiceEvent::HealthOk { at: "14:30:30".into() },
        ServiceEvent::HealthFail { exit_code: 1, consecutive: 1, at: "14:31:00".into() },
        ServiceEvent::HealthFail { exit_code: 1, consecutive: 2, at: "14:31:30".into() },
        ServiceEvent::HealthFail { exit_code: 1, consecutive: 3, at: "14:32:00".into() },
        ServiceEvent::Restarted {
            old_pid: 1234,
            new_pid: 5678,
            restart_count: 1,
            at: "14:32:01".into(),
        },
        ServiceEvent::HealthOk { at: "14:32:30".into() },
    ];
    for e in &events {
        println!("  {e}");
    }

    // Dispatch configuration
    println!("\n=== Dispatch Config ===");
    let dispatch = DispatchConfig {
        name: "deploy-staging".into(),
        command: "make deploy ENV=staging".into(),
        params: vec![
            ("env".into(), "staging".into()),
            ("version".into(), "1.2.3".into()),
        ],
        timeout_secs: Some(300),
    };
    println!("  Name: {}", dispatch.name);
    println!("  Command: {}", dispatch.command);
    println!("  Timeout: {:?}s", dispatch.timeout_secs);

    // IO hashes for state.lock.yaml
    println!("\n=== IO Hashes ===");
    let hashes = IoHashes {
        input_hash: Some("blake3:abc123def456".into()),
        output_hash: Some("blake3:789012fed345".into()),
        cached: true,
    };
    println!("  Complete: {}", hashes.is_complete());
    println!("  Cached: {}", hashes.is_cached());

    // GPU memory info
    println!("\n=== GPU Memory ===");
    let gpu = GpuMemoryInfo {
        total_bytes: Some(24 * 1024 * 1024 * 1024),
        used_bytes: Some(18 * 1024 * 1024 * 1024),
        device: Some("NVIDIA RTX 4090".into()),
    };
    println!("  Device: {:?}", gpu.device);
    println!("  Used: {:.0} MB", gpu.used_mb().unwrap());
    println!("  Utilization: {:.1}%", gpu.utilization_pct().unwrap());
}
