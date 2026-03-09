//! FJ-1350/2604/2105/1352: HF config, mutation operators, registry push, contract scaffold.
//!
//! Demonstrates:
//! - HuggingFace config.json parsing and kernel contract derivation
//! - Mutation operator applicability and script generation
//! - OCI registry push config validation and command generation
//! - Contract YAML stub scaffolding for missing kernels
//!
//! Usage: cargo run --example hf_mutation_registry

use forjar::core::store::contract_scaffold::scaffold_contracts;
use forjar::core::store::hf_config::{parse_hf_config_str, required_kernels, KernelRequirement};
use forjar::core::store::mutation_runner::{applicable_operators, mutation_script};
use forjar::core::store::registry_push::{
    head_check_command, validate_push_config, RegistryPushConfig,
};
use forjar::core::types::MutationOperator;

fn main() {
    println!("Forjar: HF Config, Mutation, Registry & Contract Scaffold");
    println!("{}", "=".repeat(55));

    // ── FJ-1350: HuggingFace config parsing ──
    println!("\n[FJ-1350] HuggingFace Config Parsing:");
    let json = r#"{
        "model_type": "llama",
        "architectures": ["LlamaForCausalLM"],
        "hidden_size": 4096,
        "num_attention_heads": 32,
        "num_key_value_heads": 8,
        "num_hidden_layers": 32,
        "intermediate_size": 11008,
        "vocab_size": 32000,
        "max_position_embeddings": 4096
    }"#;
    let config = parse_hf_config_str(json).unwrap();
    println!("  Model type: {}", config.model_type);
    println!(
        "  Heads: {}/{} (GQA)",
        config.num_attention_heads.unwrap(),
        config.num_key_value_heads.unwrap()
    );

    let kernels = required_kernels(&config);
    println!("  Required kernels ({}):", kernels.len());
    for k in &kernels {
        println!("    {} → {}", k.op, k.contract);
    }
    assert!(kernels.iter().any(|k| k.op == "rmsnorm"));
    assert!(kernels.iter().any(|k| k.op == "gqa"));

    // ── FJ-2604: Mutation Operators ──
    println!("\n[FJ-2604] Mutation Operators:");
    for rtype in &["file", "service", "package", "mount"] {
        let ops = applicable_operators(rtype);
        let names: Vec<String> = ops.iter().map(|o| o.to_string()).collect();
        println!("  {rtype}: {}", names.join(", "));
    }

    let script = mutation_script(MutationOperator::DeleteFile, "nginx.conf");
    println!("  DeleteFile script: {}", script.trim());
    assert!(script.contains("rm -f"));

    // ── FJ-2105: Registry Push ──
    println!("\n[FJ-2105] Registry Push Config:");
    let push_config = RegistryPushConfig {
        registry: "ghcr.io".into(),
        name: "myorg/myapp".into(),
        tag: "v1.0".into(),
        check_existing: true,
    };
    let errors = validate_push_config(&push_config);
    println!("  Config valid: {}", errors.is_empty());
    assert!(errors.is_empty());

    let cmd = head_check_command("ghcr.io", "myorg/myapp", "sha256:abc123");
    println!("  HEAD check: {}", &cmd[..60]);

    // ── FJ-1352: Contract Scaffold ──
    println!("\n[FJ-1352] Contract Scaffold:");
    let missing = vec![
        KernelRequirement {
            op: "softmax".into(),
            contract: "softmax-kernel-v1".into(),
        },
        KernelRequirement {
            op: "matmul".into(),
            contract: "matmul-kernel-v1".into(),
        },
    ];
    let stubs = scaffold_contracts(&missing, "forjar-ci");
    for stub in &stubs {
        println!(
            "  Generated: {} ({} bytes)",
            stub.filename,
            stub.yaml_content.len()
        );
    }
    assert_eq!(stubs.len(), 2);
    assert!(stubs[0].yaml_content.contains("EQ-SOFTMAX-01"));

    println!("\n{}", "=".repeat(55));
    println!("All HF/mutation/registry/scaffold criteria survived.");
}
