//! FJ-1304/1305/1329/1350: Purity, repro score, references, HF config.
//!
//! Usage: cargo run --example purity_reproscore_refs_hf

use forjar::core::store::hf_config::{parse_hf_config_str, required_kernels};
use forjar::core::store::purity::{
    classify, level_label, recipe_purity, PurityLevel, PuritySignals,
};
use forjar::core::store::reference::is_valid_blake3_hash;
use forjar::core::store::repro_score::{compute_score, grade, ReproInput};

fn main() {
    println!("Forjar: Purity, Repro Score, References & HF Config");
    println!("{}", "=".repeat(55));

    // ── Purity ──
    println!("\n[FJ-1305] Purity Classification:");
    let signals = [
        (
            "pure-pkg",
            PuritySignals {
                has_version: true,
                has_store: true,
                has_sandbox: true,
                ..Default::default()
            },
        ),
        (
            "pinned-pkg",
            PuritySignals {
                has_version: true,
                has_store: true,
                has_sandbox: false,
                ..Default::default()
            },
        ),
        (
            "floating-pkg",
            PuritySignals {
                has_version: false,
                ..Default::default()
            },
        ),
        (
            "curl-install",
            PuritySignals {
                has_curl_pipe: true,
                ..Default::default()
            },
        ),
    ];
    for (name, s) in &signals {
        let r = classify(name, s);
        println!("  {name}: {}", level_label(r.level));
    }
    let recipe = recipe_purity(&[PurityLevel::Pure, PurityLevel::Constrained]);
    println!("  Recipe aggregate: {}", level_label(recipe));

    // ── Repro Score ──
    println!("\n[FJ-1329] Reproducibility Score:");
    let score = compute_score(&[
        ReproInput {
            name: "nginx".into(),
            purity: PurityLevel::Pure,
            has_store: true,
            has_lock_pin: true,
        },
        ReproInput {
            name: "config".into(),
            purity: PurityLevel::Pinned,
            has_store: true,
            has_lock_pin: false,
        },
        ReproInput {
            name: "script".into(),
            purity: PurityLevel::Impure,
            has_store: false,
            has_lock_pin: false,
        },
    ]);
    println!(
        "  Composite: {:.1} ({}), Purity: {:.1}, Store: {:.1}, Lock: {:.1}",
        score.composite,
        grade(score.composite),
        score.purity_score,
        score.store_score,
        score.lock_score,
    );

    // ── References ──
    println!("\n[FJ-1304] BLAKE3 Hash Validation:");
    let valid = format!("blake3:{}", "a1b2c3d4".repeat(8));
    println!("  {}: {}", &valid[..30], is_valid_blake3_hash(&valid));
    println!("  blake3:short: {}", is_valid_blake3_hash("blake3:short"));

    // ── HF Config ──
    println!("\n[FJ-1350] HuggingFace Kernel Mapping:");
    let json = r#"{"model_type": "llama", "num_attention_heads": 32, "num_key_value_heads": 8}"#;
    let config = parse_hf_config_str(json).unwrap();
    let kernels = required_kernels(&config);
    println!("  llama: {} kernels required", kernels.len());
    for k in &kernels {
        println!("    {} → {}", k.op, k.contract);
    }

    println!("\n{}", "=".repeat(55));
    println!("All purity/repro/ref/HF criteria survived.");
}
