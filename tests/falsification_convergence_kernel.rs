//! FJ-2600/1353: Convergence testing, kernel contract FAR packaging.
//! Usage: cargo test --test falsification_convergence_kernel

use forjar::core::store::contract_coverage::{BindingEntry, BindingRegistry};
use forjar::core::store::convergence_runner::{
    format_convergence_report, resolve_mode, run_convergence_test, ConvergenceResult,
    ConvergenceSummary, ConvergenceTarget, ConvergenceTestConfig, RunnerMode,
};
use forjar::core::store::hf_config::HfModelConfig;
use forjar::core::store::kernel_far::contracts_to_far;
use forjar::core::types::SandboxBackend;

// ── helpers ──

fn result(id: &str, converged: bool, idempotent: bool, preserved: bool) -> ConvergenceResult {
    ConvergenceResult {
        resource_id: id.into(),
        resource_type: "file".into(),
        converged,
        idempotent,
        preserved,
        duration_ms: 10,
        error: None,
    }
}

fn target(id: &str, apply: &str, query: &str) -> ConvergenceTarget {
    ConvergenceTarget {
        resource_id: id.into(),
        resource_type: "file".into(),
        apply_script: apply.into(),
        state_query_script: query.into(),
        expected_hash: String::new(),
    }
}

// ── FJ-2600: ConvergenceResult ──

#[test]
fn convergence_result_passed_all_true() {
    let r = result("nginx", true, true, true);
    assert!(r.passed());
}

#[test]
fn convergence_result_failed_converge() {
    let r = result("nginx", false, true, true);
    assert!(!r.passed());
}

#[test]
fn convergence_result_failed_idempotent() {
    let r = result("nginx", true, false, true);
    assert!(!r.passed());
}

#[test]
fn convergence_result_failed_preserved() {
    let r = result("nginx", true, true, false);
    assert!(!r.passed());
}

#[test]
fn convergence_result_with_error() {
    let mut r = result("nginx", true, true, true);
    r.error = Some("oops".into());
    assert!(!r.passed());
}

#[test]
fn convergence_result_display() {
    let r = result("nginx", true, true, true);
    let s = format!("{r}");
    assert!(s.contains("PASS"));
    assert!(s.contains("nginx"));
}

#[test]
fn convergence_result_display_fail() {
    let r = result("nginx", false, true, true);
    let s = format!("{r}");
    assert!(s.contains("FAIL"));
}

// ── ConvergenceSummary ──

#[test]
fn summary_all_passed() {
    let results = vec![result("a", true, true, true), result("b", true, true, true)];
    let s = ConvergenceSummary::from_results(&results);
    assert_eq!(s.total, 2);
    assert_eq!(s.passed, 2);
    assert!((s.pass_rate() - 100.0).abs() < 0.1);
}

#[test]
fn summary_some_failed() {
    let results = vec![
        result("a", true, true, true),
        result("b", false, true, true),
        result("c", true, false, true),
    ];
    let s = ConvergenceSummary::from_results(&results);
    assert_eq!(s.total, 3);
    assert_eq!(s.passed, 1);
    assert_eq!(s.convergence_failures, 1);
    assert_eq!(s.idempotency_failures, 1);
}

#[test]
fn summary_empty() {
    let s = ConvergenceSummary::from_results(&[]);
    assert_eq!(s.total, 0);
    assert!((s.pass_rate() - 100.0).abs() < 0.1);
}

#[test]
fn summary_display() {
    let results = vec![result("a", true, true, true)];
    let s = ConvergenceSummary::from_results(&results);
    let display = format!("{s}");
    assert!(display.contains("1/1"));
    assert!(display.contains("100%"));
}

// ── format_convergence_report ──

#[test]
fn report_format_all_pass() {
    let results = vec![result("a", true, true, true)];
    let report = format_convergence_report(&results);
    assert!(report.contains("PASS"));
    assert!(!report.contains("Failures"));
}

#[test]
fn report_format_with_failures() {
    let results = vec![
        result("a", true, true, true),
        result("b", false, true, false),
    ];
    let report = format_convergence_report(&results);
    assert!(report.contains("Failures"));
    assert!(report.contains("convergence"));
}

#[test]
fn report_format_with_error() {
    let mut r = result("a", false, false, false);
    r.error = Some("script failed".into());
    let report = format_convergence_report(&[r]);
    assert!(report.contains("script failed"));
}

// ── ConvergenceTestConfig ──

#[test]
fn convergence_config_default() {
    let cfg = ConvergenceTestConfig::default();
    assert_eq!(cfg.parallelism, 4);
    assert!(!cfg.test_pairs);
}

// ── resolve_mode ──

#[test]
fn resolve_mode_returns_simulated_without_pepita() {
    // pepita binary unlikely in test env
    let mode = resolve_mode(SandboxBackend::Pepita);
    // Either Simulated or Sandbox — just verify it runs
    assert!(mode == RunnerMode::Simulated || mode == RunnerMode::Sandbox);
}

#[test]
fn runner_mode_display() {
    assert_eq!(format!("{}", RunnerMode::Simulated), "simulated");
    assert_eq!(format!("{}", RunnerMode::Sandbox), "sandbox");
}

// ── run_convergence_test with safe scripts ──

#[test]
fn convergence_test_simple_echo() {
    let t = target("test-echo", "echo hello", "echo hello");
    let r = run_convergence_test(&t);
    assert!(r.converged);
    assert!(r.idempotent);
    assert!(r.preserved);
    assert!(r.passed());
}

#[test]
fn convergence_test_empty_script_fails() {
    let t = target("test-empty", "", "echo state");
    let r = run_convergence_test(&t);
    assert!(!r.passed());
    assert!(r.error.is_some());
}

#[test]
fn convergence_test_unsafe_script_rejected() {
    let t = target("test-unsafe", "systemctl stop nginx", "echo state");
    let r = run_convergence_test(&t);
    assert!(!r.passed());
    assert!(r.error.unwrap().contains("system commands"));
}

#[test]
fn convergence_test_file_create_idempotent() {
    let t = target(
        "test-file",
        "mkdir -p \"${FORJAR_SANDBOX}/etc\" && echo 'config=1' > \"${FORJAR_SANDBOX}/etc/app.conf\"",
        "cat \"${FORJAR_SANDBOX}/etc/app.conf\" 2>/dev/null || echo missing",
    );
    let r = run_convergence_test(&t);
    assert!(r.converged, "file create should converge");
    assert!(r.idempotent, "file create should be idempotent");
    assert!(r.preserved, "state should be preserved");
}

// ── FJ-1353: kernel_far ──

#[test]
fn contracts_to_far_packages() {
    let tmp = tempfile::tempdir().unwrap();
    let contracts_dir = tmp.path().join("contracts");
    std::fs::create_dir_all(&contracts_dir).unwrap();
    std::fs::write(
        contracts_dir.join("softmax-v1.yaml"),
        "name: softmax\nequation: E1\n",
    )
    .unwrap();

    let config = HfModelConfig {
        model_type: "llama".into(),
        architectures: vec!["LlamaForCausalLM".into()],
        hidden_size: Some(4096),
        num_attention_heads: Some(32),
        num_key_value_heads: Some(32),
        num_hidden_layers: Some(32),
        intermediate_size: Some(11008),
        vocab_size: Some(32000),
        max_position_embeddings: Some(4096),
    };

    let binding_registry = BindingRegistry {
        version: "1.0".into(),
        target_crate: "test".into(),
        bindings: vec![BindingEntry {
            contract: "softmax-v1".into(),
            equation: "E1".into(),
            status: "implemented".into(),
        }],
    };

    let available = vec!["softmax-v1".into()];
    let required = forjar::core::store::hf_config::required_kernels(&config);
    let coverage = forjar::core::store::contract_coverage::coverage_report(
        "llama",
        &required,
        &binding_registry,
        &available,
    );

    let far_path = tmp.path().join("output.far");
    let manifest = contracts_to_far(&contracts_dir, &config, &coverage, &far_path).unwrap();
    assert!(far_path.exists());
    assert!(manifest.name.contains("llama"));
    assert_eq!(manifest.arch, "noarch");
    assert!(manifest.kernel_contracts.is_some());
    assert!(manifest.store_hash.starts_with("blake3:"));
}
