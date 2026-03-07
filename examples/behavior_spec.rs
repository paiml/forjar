//! FJ-2602: Behavior-driven infrastructure spec example.
//!
//! Demonstrates the behavior spec types for declarative infrastructure testing.
//!
//! ```bash
//! cargo run --example behavior_spec
//! ```

use forjar::core::types::{
    BehaviorEntry, BehaviorReport, BehaviorResult, BehaviorSpec, ConvergenceAssert, VerifyCommand,
};

fn main() {
    demo_spec_parsing();
    demo_report();
}

fn demo_spec_parsing() {
    println!("=== FJ-2602: Behavior Spec ===\n");

    let spec = BehaviorSpec {
        name: "nginx web server".into(),
        config: "examples/nginx.yaml".into(),
        machine: Some("web-1".into()),
        behaviors: vec![
            BehaviorEntry {
                name: "nginx package is installed".into(),
                resource: Some("nginx-pkg".into()),
                behavior_type: None,
                assert_state: Some("present".into()),
                verify: Some(VerifyCommand {
                    command: "dpkg -l nginx | grep -q '^ii'".into(),
                    exit_code: Some(0),
                    ..Default::default()
                }),
                convergence: None,
            },
            BehaviorEntry {
                name: "nginx config syntax valid".into(),
                resource: Some("nginx-config".into()),
                behavior_type: None,
                assert_state: Some("file".into()),
                verify: Some(VerifyCommand {
                    command: "nginx -t".into(),
                    exit_code: Some(0),
                    stderr_contains: Some("syntax is ok".into()),
                    ..Default::default()
                }),
                convergence: None,
            },
            BehaviorEntry {
                name: "nginx service is running".into(),
                resource: Some("nginx-service".into()),
                behavior_type: None,
                assert_state: Some("running".into()),
                verify: Some(VerifyCommand {
                    command: "systemctl is-active nginx".into(),
                    exit_code: Some(0),
                    stdout: Some("active".into()),
                    ..Default::default()
                }),
                convergence: None,
            },
            BehaviorEntry {
                name: "port 80 is open".into(),
                resource: Some("nginx-firewall".into()),
                behavior_type: None,
                assert_state: None,
                verify: Some(VerifyCommand {
                    command: "ss -tlnp | grep ':80'".into(),
                    exit_code: Some(0),
                    ..Default::default()
                }),
                convergence: None,
            },
            BehaviorEntry {
                name: "idempotency holds".into(),
                resource: None,
                behavior_type: Some("convergence".into()),
                assert_state: None,
                verify: None,
                convergence: Some(ConvergenceAssert {
                    second_apply: Some("noop".into()),
                    state_unchanged: Some(true),
                }),
            },
        ],
    };

    println!("  Spec: {}", spec.name);
    println!("  Config: {}", spec.config);
    println!("  Machine: {}", spec.machine.as_deref().unwrap_or("all"));
    println!("  Behaviors: {}", spec.behavior_count());
    println!("  Resources: {:?}", spec.referenced_resources());
    println!();

    for b in &spec.behaviors {
        let kind = if b.is_convergence() {
            "convergence"
        } else {
            "resource"
        };
        println!("  [{kind}] {}", b.name);
    }
    println!();
}

fn demo_report() {
    println!("=== FJ-2602: Behavior Report ===\n");

    let results = vec![
        BehaviorResult {
            name: "nginx package is installed".into(),
            passed: true,
            failure: None,
            actual_exit_code: Some(0),
            actual_stdout: None,
            duration_ms: 120,
        },
        BehaviorResult {
            name: "nginx config syntax valid".into(),
            passed: true,
            failure: None,
            actual_exit_code: Some(0),
            actual_stdout: None,
            duration_ms: 85,
        },
        BehaviorResult {
            name: "nginx service is running".into(),
            passed: false,
            failure: Some("expected stdout 'active', got 'inactive'".into()),
            actual_exit_code: Some(0),
            actual_stdout: Some("inactive".into()),
            duration_ms: 50,
        },
        BehaviorResult {
            name: "port 80 is open".into(),
            passed: false,
            failure: Some("exit code 1, expected 0".into()),
            actual_exit_code: Some(1),
            actual_stdout: None,
            duration_ms: 30,
        },
        BehaviorResult {
            name: "idempotency holds".into(),
            passed: true,
            failure: None,
            actual_exit_code: None,
            actual_stdout: None,
            duration_ms: 2500,
        },
    ];

    let report = BehaviorReport::from_results("nginx web server".into(), results);
    print!("{report}");
}
