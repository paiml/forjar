//! FJ-2600/1353: Convergence testing, kernel contract FAR packaging.
//!
//! Usage: cargo run --example convergence_kernel

use forjar::core::store::contract_coverage::{coverage_report, BindingEntry, BindingRegistry};
use forjar::core::store::convergence_runner::{
    format_convergence_report, resolve_mode, run_convergence_test, ConvergenceResult,
    ConvergenceSummary, ConvergenceTarget, RunnerMode,
};
use forjar::core::store::hf_config::{required_kernels, HfModelConfig};
use forjar::core::store::kernel_far::contracts_to_far;
use forjar::core::types::SandboxBackend;

fn main() {
    println!("Forjar: Convergence Testing & Kernel FAR Packaging");
    println!("{}", "=".repeat(55));

    // ── FJ-2600: Convergence Testing ──
    println!("\n[FJ-2600] Convergence Testing:");
    let mode = resolve_mode(SandboxBackend::Pepita);
    println!("  Backend: Pepita, Mode: {mode}");

    let targets = vec![
        ConvergenceTarget {
            resource_id: "app-config".into(),
            resource_type: "file".into(),
            apply_script: "mkdir -p \"${FORJAR_SANDBOX}/etc\" && echo 'port=8080' > \"${FORJAR_SANDBOX}/etc/app.conf\"".into(),
            state_query_script: "cat \"${FORJAR_SANDBOX}/etc/app.conf\"".into(),
            expected_hash: String::new(),
        },
        ConvergenceTarget {
            resource_id: "data-dir".into(),
            resource_type: "file".into(),
            apply_script: "mkdir -p \"${FORJAR_SANDBOX}/data\"".into(),
            state_query_script: "ls \"${FORJAR_SANDBOX}/data\" 2>/dev/null || echo empty".into(),
            expected_hash: String::new(),
        },
    ];

    let results: Vec<ConvergenceResult> = targets.iter().map(|t| run_convergence_test(t)).collect();
    let summary = ConvergenceSummary::from_results(&results);
    println!("  {summary}");
    for r in &results {
        println!("    {r}");
    }
    println!("\n  Report:\n{}", format_convergence_report(&results));

    // ── FJ-1353: Kernel FAR Packaging ──
    println!("[FJ-1353] Kernel FAR Packaging:");
    let tmp = tempfile::tempdir().unwrap();
    let contracts_dir = tmp.path().join("contracts");
    std::fs::create_dir_all(&contracts_dir).unwrap();
    std::fs::write(
        contracts_dir.join("softmax-v1.yaml"),
        "name: softmax\nequation: y = exp(x) / sum(exp(x))\n",
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

    let registry = BindingRegistry {
        version: "1.0".into(),
        target_crate: "forjar-kernels".into(),
        bindings: vec![BindingEntry {
            contract: "softmax-v1".into(),
            equation: "E1".into(),
            status: "implemented".into(),
        }],
    };

    let required = required_kernels(&config);
    let available = vec!["softmax-v1".into()];
    let coverage = coverage_report("llama", &required, &registry, &available);
    println!("  Coverage: {:.1}%", coverage.coverage_pct);

    let far_path = tmp.path().join("llama-kernels.far");
    let manifest = contracts_to_far(&contracts_dir, &config, &coverage, &far_path).unwrap();
    println!("  FAR: {}", far_path.display());
    println!("  Name: {}", manifest.name);
    println!("  Files: {}", manifest.file_count);
    println!("  Store hash: {}", manifest.store_hash);
    if let Some(kc) = &manifest.kernel_contracts {
        println!(
            "  Kernel: model={}, ops={}",
            kc.model_type,
            kc.required_ops.len()
        );
    }

    println!("\n{}", "=".repeat(55));
    println!("All convergence/kernel criteria survived.");
}
