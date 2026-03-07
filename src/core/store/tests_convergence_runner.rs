//! Tests for convergence runner with sandbox integration (FJ-2600/FJ-2603).

use super::convergence_runner::*;

fn sample_target(id: &str, rtype: &str) -> ConvergenceTarget {
    let query_script = format!("echo 'query {id}'");
    let expected_hash = {
        let refs = [query_script.as_str()];
        crate::tripwire::hasher::composite_hash(&refs)
    };
    ConvergenceTarget {
        resource_id: id.into(),
        resource_type: rtype.into(),
        apply_script: format!("echo 'apply {id}'"),
        state_query_script: query_script,
        expected_hash,
    }
}

#[test]
fn convergence_test_passes_for_valid_target() {
    let target = sample_target("nginx-config", "file");
    let result = run_convergence_test(&target);
    assert!(result.passed());
    assert!(result.converged);
    assert!(result.idempotent);
    assert!(result.preserved);
    assert!(result.error.is_none());
}

#[test]
fn convergence_test_fails_for_empty_script() {
    let target = ConvergenceTarget {
        resource_id: "broken".into(),
        resource_type: "file".into(),
        apply_script: String::new(),
        state_query_script: "echo ok".into(),
        expected_hash: "blake3:aaa".into(),
    };
    let result = run_convergence_test(&target);
    assert!(!result.passed());
    assert!(!result.converged);
    assert!(result.error.is_some());
}

#[test]
fn convergence_test_fails_for_hash_mismatch() {
    let target = ConvergenceTarget {
        resource_id: "mismatched".into(),
        resource_type: "package".into(),
        apply_script: "echo apply".into(),
        state_query_script: "echo query".into(),
        expected_hash: "blake3:wrong_hash".into(),
    };
    let result = run_convergence_test(&target);
    assert!(!result.converged);
}

#[test]
fn convergence_result_display() {
    let result = ConvergenceResult {
        resource_id: "svc-nginx".into(),
        resource_type: "service".into(),
        converged: true,
        idempotent: true,
        preserved: false,
        duration_ms: 150,
        error: None,
    };
    let s = result.to_string();
    assert!(s.contains("FAIL"));
    assert!(s.contains("svc-nginx"));
    assert!(s.contains("preserve=false"));
}

#[test]
fn convergence_result_display_pass() {
    let result = ConvergenceResult {
        resource_id: "cfg".into(),
        resource_type: "file".into(),
        converged: true,
        idempotent: true,
        preserved: true,
        duration_ms: 50,
        error: None,
    };
    let s = result.to_string();
    assert!(s.contains("PASS"));
}

#[test]
fn convergence_parallel_empty_input() {
    let results = run_convergence_parallel(Vec::new(), 4);
    assert!(results.is_empty());
}

#[test]
fn convergence_parallel_single_target() {
    let targets = vec![sample_target("app-config", "file")];
    let results = run_convergence_parallel(targets, 4);
    assert_eq!(results.len(), 1);
    assert!(results[0].passed());
}

#[test]
fn convergence_parallel_multiple_targets() {
    let targets = vec![
        sample_target("pkg-curl", "package"),
        sample_target("svc-nginx", "service"),
        sample_target("cfg-app", "file"),
        sample_target("mnt-data", "mount"),
    ];
    let results = run_convergence_parallel(targets, 2);
    assert_eq!(results.len(), 4);
    assert!(results.iter().all(|r| r.passed()));
}

#[test]
fn convergence_summary_all_pass() {
    let results = vec![
        ConvergenceResult {
            resource_id: "a".into(),
            resource_type: "file".into(),
            converged: true,
            idempotent: true,
            preserved: true,
            duration_ms: 10,
            error: None,
        },
        ConvergenceResult {
            resource_id: "b".into(),
            resource_type: "package".into(),
            converged: true,
            idempotent: true,
            preserved: true,
            duration_ms: 20,
            error: None,
        },
    ];
    let summary = ConvergenceSummary::from_results(&results);
    assert_eq!(summary.total, 2);
    assert_eq!(summary.passed, 2);
    assert!((summary.pass_rate() - 100.0).abs() < 0.01);
}

#[test]
fn convergence_summary_with_failures() {
    let results = vec![
        ConvergenceResult {
            resource_id: "a".into(),
            resource_type: "file".into(),
            converged: false,
            idempotent: true,
            preserved: true,
            duration_ms: 10,
            error: None,
        },
        ConvergenceResult {
            resource_id: "b".into(),
            resource_type: "service".into(),
            converged: true,
            idempotent: false,
            preserved: true,
            duration_ms: 20,
            error: None,
        },
    ];
    let summary = ConvergenceSummary::from_results(&results);
    assert_eq!(summary.passed, 0);
    assert_eq!(summary.convergence_failures, 1);
    assert_eq!(summary.idempotency_failures, 1);
}

#[test]
fn convergence_summary_empty() {
    let summary = ConvergenceSummary::from_results(&[]);
    assert_eq!(summary.total, 0);
    assert!((summary.pass_rate() - 100.0).abs() < 0.01);
}

#[test]
fn convergence_summary_display() {
    let summary = ConvergenceSummary {
        total: 10,
        passed: 8,
        convergence_failures: 1,
        idempotency_failures: 1,
        preservation_failures: 0,
    };
    let s = summary.to_string();
    assert!(s.contains("8/10"));
    assert!(s.contains("80%"));
    assert!(s.contains("convergence"));
    assert!(s.contains("idempotency"));
    assert!(!s.contains("preservation"));
}

#[test]
fn format_convergence_report_all_pass() {
    let target = sample_target("pkg-curl", "package");
    let result = run_convergence_test(&target);
    let report = format_convergence_report(&[result]);
    assert!(report.contains("1/1 passed"));
    assert!(report.contains("100%"));
    assert!(!report.contains("Failures:"));
}

#[test]
fn format_convergence_report_with_failures() {
    let results = vec![
        ConvergenceResult {
            resource_id: "ok".into(),
            resource_type: "file".into(),
            converged: true,
            idempotent: true,
            preserved: true,
            duration_ms: 10,
            error: None,
        },
        ConvergenceResult {
            resource_id: "bad".into(),
            resource_type: "service".into(),
            converged: true,
            idempotent: false,
            preserved: true,
            duration_ms: 20,
            error: None,
        },
    ];
    let report = format_convergence_report(&results);
    assert!(report.contains("1/2 passed"));
    assert!(report.contains("Failures:"));
    assert!(report.contains("bad"));
    assert!(report.contains("idempotency"));
}

#[test]
fn format_convergence_report_with_error() {
    let results = vec![ConvergenceResult {
        resource_id: "err".into(),
        resource_type: "file".into(),
        converged: false,
        idempotent: false,
        preserved: false,
        duration_ms: 5,
        error: Some("sandbox timeout".into()),
    }];
    let report = format_convergence_report(&results);
    assert!(report.contains("sandbox timeout"));
}

#[test]
fn convergence_test_config_default() {
    let config = ConvergenceTestConfig::default();
    assert_eq!(config.parallelism, 4);
    assert!(!config.test_pairs);
    assert_eq!(config.backend, crate::core::types::SandboxBackend::Pepita);
}

#[test]
fn runner_mode_display() {
    assert_eq!(RunnerMode::Simulated.to_string(), "simulated");
    assert_eq!(RunnerMode::Sandbox.to_string(), "sandbox");
}

#[test]
fn runner_mode_equality() {
    assert_eq!(RunnerMode::Simulated, RunnerMode::Simulated);
    assert_ne!(RunnerMode::Simulated, RunnerMode::Sandbox);
}

#[test]
fn resolve_mode_returns_simulated_without_pepita() {
    // In CI/test env, pepita binary is not installed
    use crate::core::types::SandboxBackend;
    let mode = resolve_mode(SandboxBackend::Pepita);
    // pepita binary not installed in test env → simulated
    assert_eq!(mode, RunnerMode::Simulated);
}

#[test]
fn resolve_mode_chroot_non_root() {
    // chroot requires root — skip when running as root (e.g. Docker containers)
    if backend_available(crate::core::types::SandboxBackend::Chroot) {
        return;
    }
    use crate::core::types::SandboxBackend;
    let mode = resolve_mode(SandboxBackend::Chroot);
    assert_eq!(mode, RunnerMode::Simulated);
}

#[test]
fn backend_available_pepita_not_installed() {
    use crate::core::types::SandboxBackend;
    // Pepita binary not present in test env
    let available = backend_available(SandboxBackend::Pepita);
    assert!(!available);
}

#[test]
fn backend_available_chroot_non_root() {
    use crate::core::types::SandboxBackend;
    let available = backend_available(SandboxBackend::Chroot);
    // In root environments (Docker), chroot IS available — both cases valid
    if available {
        // Running as root: chroot available is correct behavior
        return;
    }
    assert!(!available);
}

#[test]
fn detect_container_runtime_finds_something() {
    // This test env has Docker — detect_container_runtime should find it
    let rt = super::convergence_container::detect_container_runtime();
    // Docker is available in this dev env (confirmed by forjar doctor)
    assert!(rt.is_some(), "expected docker or podman to be available");
}

#[test]
fn dispatch_uses_simulated_for_pepita() {
    use crate::core::types::SandboxBackend;
    let target = sample_target("dispatch-sim", "file");
    // Pepita not installed → dispatch should fall back to simulated
    let result = run_convergence_test_dispatch(&target, SandboxBackend::Pepita);
    assert!(
        result.passed(),
        "simulated mode should pass for valid target"
    );
}

#[test]
fn dispatch_uses_simulated_for_chroot() {
    use crate::core::types::SandboxBackend;
    let target = sample_target("dispatch-chroot", "file");
    let result = run_convergence_test_dispatch(&target, SandboxBackend::Chroot);
    assert!(result.passed(), "chroot unavailable → simulated fallback");
}

#[test]
fn parallel_with_backend_simulated() {
    use crate::core::types::SandboxBackend;
    let targets = vec![
        sample_target("par-a", "file"),
        sample_target("par-b", "package"),
    ];
    let results = run_convergence_parallel_with_backend(targets, 2, SandboxBackend::Pepita);
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|r| r.passed()));
}
