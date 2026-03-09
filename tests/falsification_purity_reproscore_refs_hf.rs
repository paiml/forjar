//! FJ-1304/1305/1329/1350: Store purity, repro score, references, HF config.
//! Usage: cargo test --test falsification_purity_reproscore_refs_hf

use forjar::core::store::hf_config::{parse_hf_config_str, required_kernels};
use forjar::core::store::purity::{
    classify, level_label, recipe_purity, PurityLevel, PuritySignals,
};
use forjar::core::store::reference::{is_valid_blake3_hash, scan_file_refs};
use forjar::core::store::repro_score::{compute_score, grade, ReproInput};
use std::collections::BTreeSet;

// ── FJ-1305: PurityLevel ordering ──

#[test]
fn purity_level_ordering() {
    assert!(PurityLevel::Pure < PurityLevel::Pinned);
    assert!(PurityLevel::Pinned < PurityLevel::Constrained);
    assert!(PurityLevel::Constrained < PurityLevel::Impure);
}

// ── FJ-1305: classify ──

#[test]
fn classify_pure() {
    let s = PuritySignals {
        has_version: true,
        has_store: true,
        has_sandbox: true,
        ..Default::default()
    };
    let r = classify("pkg", &s);
    assert_eq!(r.level, PurityLevel::Pure);
    assert!(r.reasons[0].contains("sandbox"));
}

#[test]
fn classify_pinned_no_sandbox() {
    let s = PuritySignals {
        has_version: true,
        has_store: true,
        has_sandbox: false,
        ..Default::default()
    };
    assert_eq!(classify("pkg", &s).level, PurityLevel::Pinned);
}

#[test]
fn classify_pinned_no_store() {
    let s = PuritySignals {
        has_version: true,
        has_store: false,
        has_sandbox: true,
        ..Default::default()
    };
    assert_eq!(classify("pkg", &s).level, PurityLevel::Pinned);
}

#[test]
fn classify_constrained() {
    let s = PuritySignals {
        has_version: false,
        ..Default::default()
    };
    assert_eq!(classify("pkg", &s).level, PurityLevel::Constrained);
}

#[test]
fn classify_impure_curl_pipe() {
    let s = PuritySignals {
        has_curl_pipe: true,
        has_version: true,
        has_store: true,
        has_sandbox: true,
        ..Default::default()
    };
    assert_eq!(classify("pkg", &s).level, PurityLevel::Impure);
}

#[test]
fn classify_monotonicity() {
    let s = PuritySignals {
        has_version: true,
        has_store: true,
        has_sandbox: true,
        dep_levels: vec![PurityLevel::Impure],
        ..Default::default()
    };
    let r = classify("pkg", &s);
    assert_eq!(r.level, PurityLevel::Impure);
    assert!(r.reasons.iter().any(|r| r.contains("dependency")));
}

// ── FJ-1305: recipe_purity & level_label ──

#[test]
fn recipe_purity_max() {
    assert_eq!(
        recipe_purity(&[PurityLevel::Pure, PurityLevel::Pinned]),
        PurityLevel::Pinned
    );
    assert_eq!(recipe_purity(&[]), PurityLevel::Pure);
}

#[test]
fn level_labels() {
    assert_eq!(level_label(PurityLevel::Pure), "Pure (0)");
    assert_eq!(level_label(PurityLevel::Impure), "Impure (3)");
}

// ── FJ-1305: PurityLevel serde ──

#[test]
fn purity_level_serde() {
    for level in [
        PurityLevel::Pure,
        PurityLevel::Pinned,
        PurityLevel::Constrained,
        PurityLevel::Impure,
    ] {
        let json = serde_json::to_string(&level).unwrap();
        let parsed: PurityLevel = serde_json::from_str(&json).unwrap();
        assert_eq!(level, parsed);
    }
}

// ── FJ-1329: compute_score ──

fn ri(name: &str, purity: PurityLevel, store: bool, lock: bool) -> ReproInput {
    ReproInput {
        name: name.into(),
        purity,
        has_store: store,
        has_lock_pin: lock,
    }
}

#[test]
fn score_empty_is_perfect() {
    let s = compute_score(&[]);
    assert!((s.composite - 100.0).abs() < 0.01);
}

#[test]
fn score_all_pure() {
    let s = compute_score(&[
        ri("a", PurityLevel::Pure, true, true),
        ri("b", PurityLevel::Pure, true, true),
    ]);
    assert!((s.composite - 100.0).abs() < 0.01);
    assert!((s.purity_score - 100.0).abs() < 0.01);
    assert!((s.store_score - 100.0).abs() < 0.01);
    assert!((s.lock_score - 100.0).abs() < 0.01);
}

#[test]
fn score_impure_no_store_no_lock() {
    let s = compute_score(&[ri("x", PurityLevel::Impure, false, false)]);
    assert!((s.composite).abs() < 0.01); // 0*0.5 + 0*0.3 + 0*0.2 = 0
}

#[test]
fn score_mixed() {
    let s = compute_score(&[
        ri("a", PurityLevel::Pure, true, true),
        ri("b", PurityLevel::Impure, false, false),
    ]);
    // purity: (100+0)/2=50, store: 50%, lock: 50%
    let expected = 50.0 * 0.5 + 50.0 * 0.3 + 50.0 * 0.2;
    assert!((s.composite - expected).abs() < 0.01);
}

#[test]
fn score_per_resource() {
    let s = compute_score(&[ri("a", PurityLevel::Pure, true, true)]);
    assert_eq!(s.resources.len(), 1);
    assert_eq!(s.resources[0].name, "a");
    assert!(s.resources[0].has_store);
}

// ── FJ-1329: grade ──

#[test]
fn grade_thresholds() {
    assert_eq!(grade(100.0), "A");
    assert_eq!(grade(90.0), "A");
    assert_eq!(grade(89.9), "B");
    assert_eq!(grade(75.0), "B");
    assert_eq!(grade(74.9), "C");
    assert_eq!(grade(50.0), "C");
    assert_eq!(grade(49.9), "D");
    assert_eq!(grade(25.0), "D");
    assert_eq!(grade(24.9), "F");
    assert_eq!(grade(0.0), "F");
}

// ── FJ-1329: ReproScore serde ──

#[test]
fn repro_score_serde() {
    let s = compute_score(&[ri("x", PurityLevel::Pinned, true, false)]);
    let json = serde_json::to_string(&s).unwrap();
    let parsed: forjar::core::store::repro_score::ReproScore = serde_json::from_str(&json).unwrap();
    assert_eq!(s, parsed);
}

// ── FJ-1304: is_valid_blake3_hash ──

#[test]
fn valid_blake3_hashes() {
    let valid = format!("blake3:{}", "a".repeat(64));
    assert!(is_valid_blake3_hash(&valid));
    let upper = format!("blake3:{}", "A".repeat(64));
    assert!(is_valid_blake3_hash(&upper));
}

#[test]
fn invalid_blake3_hashes() {
    assert!(!is_valid_blake3_hash("blake3:tooshort"));
    assert!(!is_valid_blake3_hash("sha256:aaaa"));
    assert!(!is_valid_blake3_hash(""));
    let non_hex = format!("blake3:{}g", "a".repeat(63));
    assert!(!is_valid_blake3_hash(&non_hex));
}

// ── FJ-1304: scan_file_refs ──

#[test]
fn scan_finds_known_refs() {
    let hash = format!("blake3:{}", "ab12".repeat(16));
    let content = format!("config: {hash}\nother text");
    let known: BTreeSet<String> = [hash.clone()].into();
    let refs = scan_file_refs(content.as_bytes(), &known);
    assert_eq!(refs.len(), 1);
    assert!(refs.contains(&hash));
}

#[test]
fn scan_ignores_unknown_refs() {
    let hash = format!("blake3:{}", "ff".repeat(32));
    let content = format!("ref: {hash}");
    let known = BTreeSet::new(); // not in known set
    assert!(scan_file_refs(content.as_bytes(), &known).is_empty());
}

#[test]
fn scan_empty_content() {
    assert!(scan_file_refs(b"", &BTreeSet::new()).is_empty());
}

// ── FJ-1350: parse_hf_config_str ──

#[test]
fn parse_llama_config() {
    let json = r#"{"model_type": "llama", "architectures": ["LlamaForCausalLM"],
        "hidden_size": 4096, "num_attention_heads": 32, "num_key_value_heads": 8,
        "num_hidden_layers": 32, "intermediate_size": 14336, "vocab_size": 128256}"#;
    let config = parse_hf_config_str(json).unwrap();
    assert_eq!(config.model_type, "llama");
    assert_eq!(config.num_attention_heads, Some(32));
    assert_eq!(config.num_key_value_heads, Some(8));
}

#[test]
fn parse_gpt2_config() {
    let json = r#"{"model_type": "gpt2", "hidden_size": 768, "num_attention_heads": 12,
        "num_hidden_layers": 12, "vocab_size": 50257}"#;
    let config = parse_hf_config_str(json).unwrap();
    assert_eq!(config.model_type, "gpt2");
    assert!(config.num_key_value_heads.is_none()); // MHA, no GQA
}

#[test]
fn parse_invalid_json() {
    assert!(parse_hf_config_str("not json").is_err());
}

// ── FJ-1350: required_kernels ──

#[test]
fn kernels_llama_gqa() {
    let json = r#"{"model_type": "llama", "num_attention_heads": 32, "num_key_value_heads": 8}"#;
    let config = parse_hf_config_str(json).unwrap();
    let kernels = required_kernels(&config);
    let ops: Vec<&str> = kernels.iter().map(|k| k.op.as_str()).collect();
    assert!(ops.contains(&"rmsnorm"));
    assert!(ops.contains(&"silu"));
    assert!(ops.contains(&"rope"));
    assert!(ops.contains(&"swiglu"));
    assert!(ops.contains(&"gqa")); // GQA because kv_heads < heads
    assert!(ops.contains(&"softmax"));
    assert!(ops.contains(&"matmul"));
    assert!(!ops.contains(&"bias_add")); // llama has no bias
}

#[test]
fn kernels_gpt2_mha() {
    let json = r#"{"model_type": "gpt2", "num_attention_heads": 12}"#;
    let config = parse_hf_config_str(json).unwrap();
    let kernels = required_kernels(&config);
    let ops: Vec<&str> = kernels.iter().map(|k| k.op.as_str()).collect();
    assert!(ops.contains(&"layernorm")); // gpt2 uses LayerNorm
    assert!(ops.contains(&"gelu"));
    assert!(ops.contains(&"absolute_position"));
    assert!(ops.contains(&"gelu_mlp"));
    assert!(ops.contains(&"bias_add")); // gpt2 has bias
    assert!(ops.contains(&"tied_embeddings")); // gpt2 uses tied embeddings
    assert!(ops.contains(&"attention")); // MHA, not GQA
}

#[test]
fn kernels_qwen2_with_bias() {
    let json = r#"{"model_type": "qwen2", "num_attention_heads": 28, "num_key_value_heads": 4}"#;
    let config = parse_hf_config_str(json).unwrap();
    let kernels = required_kernels(&config);
    let ops: Vec<&str> = kernels.iter().map(|k| k.op.as_str()).collect();
    assert!(ops.contains(&"bias_add")); // qwen2 has bias
    assert!(ops.contains(&"gqa"));
}

#[test]
fn kernels_deepseek_qk_norm() {
    let json =
        r#"{"model_type": "deepseek_v2", "num_attention_heads": 16, "num_key_value_heads": 2}"#;
    let config = parse_hf_config_str(json).unwrap();
    let kernels = required_kernels(&config);
    let ops: Vec<&str> = kernels.iter().map(|k| k.op.as_str()).collect();
    assert!(ops.contains(&"qk_norm")); // deepseek has QK norm
}
