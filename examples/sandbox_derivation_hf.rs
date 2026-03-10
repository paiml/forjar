//! FJ-1316/1342/1350: Sandbox planning, derivation lifecycle, HF kernel mapping.
//!
//! Usage: cargo run --example sandbox_derivation_hf

use forjar::core::store::derivation::{Derivation, DerivationInput};
use forjar::core::store::derivation_exec::{
    is_store_hit, plan_derivation, simulate_derivation, skipped_steps,
};
use forjar::core::store::hf_config::{parse_hf_config_str, required_kernels};
use forjar::core::store::sandbox::{SandboxConfig, SandboxLevel};
use forjar::core::store::sandbox_exec::{
    export_overlay_upper, oci_layout_plan, plan_sandbox_build, seccomp_rules_for_level,
    sha256_digest, simulate_sandbox_build, validate_plan, OverlayConfig,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn main() {
    println!("Forjar: Sandbox, Derivation & HF Kernel Pipeline");
    println!("{}", "=".repeat(50));

    // ── Sandbox Planning ──
    println!("\n[Sandbox Planning]");

    let config = SandboxConfig {
        level: SandboxLevel::Full,
        memory_mb: 4096,
        cpus: 8.0,
        timeout: 300,
        bind_mounts: vec![],
        env: vec![],
    };

    let inputs = BTreeMap::from([("src".into(), PathBuf::from("/store/abc/content"))]);
    let plan = plan_sandbox_build(
        &config,
        "hash1234567890ab",
        &inputs,
        "make install",
        Path::new("/var/lib/forjar/store"),
    );

    println!("  Level: Full");
    println!("  Steps: {}", plan.steps.len());
    for step in &plan.steps {
        println!("    {}. {}", step.step, step.description);
    }

    let errors = validate_plan(&plan);
    println!("  Validation errors: {}", errors.len());

    // Seccomp rules
    let rules = seccomp_rules_for_level(SandboxLevel::Full);
    println!("  Seccomp rules (Full): {}", rules.len());
    for r in &rules {
        println!("    - {} ({})", r.syscall, r.action);
    }

    // Simulate build
    let sim = simulate_sandbox_build(
        &config,
        "hash1234567890ab",
        &inputs,
        "make install",
        Path::new("/store"),
    );
    println!("  Simulated build hash: {}", sim.output_hash);

    // ── OCI Export ──
    println!("\n[OCI Export]");

    let overlay = OverlayConfig {
        lower_dirs: vec![PathBuf::from("/store/abc/content")],
        upper_dir: PathBuf::from("/tmp/upper"),
        work_dir: PathBuf::from("/tmp/work"),
        merged_dir: PathBuf::from("/tmp/merged"),
    };

    let export_steps = export_overlay_upper(&overlay, Path::new("/tmp/layer.tar.gz"));
    println!("  Export steps: {}", export_steps.len());
    for step in &export_steps {
        println!("    {}. {}", step.step, step.description);
    }

    let oci_steps = oci_layout_plan(Path::new("/tmp/oci-image"), "myapp:latest");
    println!("  OCI layout steps: {}", oci_steps.len());
    for step in &oci_steps {
        println!("    {}. {}", step.step, step.description);
    }

    // SHA256
    let digest = sha256_digest(b"hello forjar");
    println!("  SHA256 digest: {}...", &digest[..16]);

    // ── Derivation Lifecycle ──
    println!("\n[Derivation Lifecycle]");

    let drv = Derivation {
        inputs: BTreeMap::from([
            (
                "src".into(),
                DerivationInput::Store {
                    store: "blake3:aaaa".into(),
                },
            ),
            (
                "deps".into(),
                DerivationInput::Store {
                    store: "blake3:bbbb".into(),
                },
            ),
        ]),
        script: "gcc -O2 main.c -o main".into(),
        sandbox: None,
        arch: "x86_64".into(),
        out_var: "$out".into(),
    };

    // Plan derivation (cache miss)
    let resolved = BTreeMap::from([
        ("src".into(), "blake3:aaaa".into()),
        ("deps".into(), "blake3:bbbb".into()),
    ]);
    let store_entries: Vec<String> = vec![];
    match plan_derivation(&drv, &resolved, &store_entries, Path::new("/store")) {
        Ok(plan) => {
            println!("  Inputs: {}", plan.input_paths.len());
            println!("  Steps: {}", plan.steps.len());
            println!("  Store hit: {}", is_store_hit(&plan));
            println!("  Skipped: {}", skipped_steps(&plan));
        }
        Err(e) => println!("  Plan error: {e}"),
    }

    // Simulate derivation
    let sim = simulate_derivation(&drv, &resolved, &store_entries, Path::new("/store"));
    match sim {
        Ok(result) => {
            println!("  Simulated store hash: {}", result.store_hash);
            println!("  Closure hash: {}", result.closure_hash);
            println!("  Depth: {}", result.derivation_depth);
        }
        Err(e) => println!("  Simulate error: {e}"),
    }

    // ── HF Kernel Mapping ──
    println!("\n[HF Kernel Mapping]");

    let configs = [
        (
            "LLaMA",
            r#"{"model_type":"llama","architectures":["LlamaForCausalLM"],"hidden_size":4096,"num_attention_heads":32,"num_key_value_heads":8,"intermediate_size":11008,"vocab_size":32000}"#,
        ),
        (
            "GPT-2",
            r#"{"model_type":"gpt2","architectures":["GPT2LMHeadModel"],"hidden_size":768,"num_attention_heads":12,"vocab_size":50257}"#,
        ),
        (
            "Qwen2",
            r#"{"model_type":"qwen2","architectures":["Qwen2ForCausalLM"],"hidden_size":2048,"num_attention_heads":16,"num_key_value_heads":2,"intermediate_size":5504,"vocab_size":151936}"#,
        ),
        (
            "DeepSeek",
            r#"{"model_type":"deepseek","architectures":["DeepseekForCausalLM"],"hidden_size":4096,"num_attention_heads":32,"vocab_size":102400}"#,
        ),
    ];

    for (name, json) in &configs {
        match parse_hf_config_str(json) {
            Ok(config) => {
                let kernels = required_kernels(&config);
                println!("  {name}: {} kernel requirements", kernels.len());
                for k in &kernels {
                    println!("    - {} (contract: {})", k.op, k.contract);
                }
            }
            Err(e) => println!("  {name}: parse error: {e}"),
        }
    }

    println!("\n{}", "=".repeat(50));
    println!("All sandbox/derivation/HF criteria survived.");
}
