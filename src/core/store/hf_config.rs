//! FJ-1350: HuggingFace config.json parser and architecture-to-kernel mapping.
//!
//! Reads a HuggingFace `config.json`, extracts the `model_type` and architecture
//! parameters, then maps deterministically to the kernel contracts required for
//! that model family (from arch-constraints-v1.yaml).

use serde::Deserialize;
use std::path::Path;

/// Parsed fields from a HuggingFace `config.json`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct HfModelConfig {
    /// Model type identifier (e.g., "llama", "qwen2").
    pub model_type: String,
    /// Architecture class names.
    #[serde(default)]
    pub architectures: Vec<String>,
    /// Hidden layer dimension.
    pub hidden_size: Option<u64>,
    /// Number of attention heads.
    pub num_attention_heads: Option<u64>,
    /// Number of key-value heads (for GQA).
    pub num_key_value_heads: Option<u64>,
    /// Number of hidden layers.
    pub num_hidden_layers: Option<u64>,
    /// Intermediate MLP dimension.
    pub intermediate_size: Option<u64>,
    /// Vocabulary size.
    pub vocab_size: Option<u64>,
    /// Maximum sequence length.
    pub max_position_embeddings: Option<u64>,
}

/// A kernel operation required by a model architecture.
#[derive(Debug, Clone, PartialEq)]
pub struct KernelRequirement {
    /// Operation name (e.g., "softmax", "matmul").
    pub op: String,
    /// Contract name (e.g., "softmax-kernel-v1").
    pub contract: String,
}

/// Parse a HuggingFace `config.json` from a file path.
pub fn parse_hf_config(path: &Path) -> Result<HfModelConfig, String> {
    let data =
        std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    parse_hf_config_str(&data)
}

/// Parse a HuggingFace `config.json` from a JSON string.
pub fn parse_hf_config_str(json: &str) -> Result<HfModelConfig, String> {
    serde_json::from_str(json).map_err(|e| format!("parse config.json: {e}"))
}

/// Architecture constraint fields derived from `model_type`.
struct ArchConstraints {
    norm_type: NormType,
    activation: Activation,
    positional_encoding: PosEncoding,
    mlp_type: MlpType,
    has_bias: bool,
    tied_embeddings: bool,
    has_qk_norm: bool,
}

#[derive(Clone, Copy)]
enum NormType {
    RmsNorm,
    LayerNorm,
}

#[derive(Clone, Copy)]
enum Activation {
    Silu,
    Gelu,
}

#[derive(Clone, Copy)]
enum PosEncoding {
    Rope,
    Absolute,
}

#[derive(Clone, Copy)]
enum MlpType {
    SwiGlu,
    GeluMlp,
}

/// Map `model_type` to architecture constraints (from arch-constraints-v1.yaml).
fn arch_constraints(model_type: &str) -> ArchConstraints {
    match model_type {
        "qwen2" | "qwen2_moe" => ArchConstraints {
            norm_type: NormType::RmsNorm,
            activation: Activation::Silu,
            positional_encoding: PosEncoding::Rope,
            mlp_type: MlpType::SwiGlu,
            has_bias: true,
            tied_embeddings: false,
            has_qk_norm: false,
        },
        "llama" | "codellama" => ArchConstraints {
            norm_type: NormType::RmsNorm,
            activation: Activation::Silu,
            positional_encoding: PosEncoding::Rope,
            mlp_type: MlpType::SwiGlu,
            has_bias: false,
            tied_embeddings: false,
            has_qk_norm: false,
        },
        "mistral" | "mixtral" => ArchConstraints {
            norm_type: NormType::RmsNorm,
            activation: Activation::Silu,
            positional_encoding: PosEncoding::Rope,
            mlp_type: MlpType::SwiGlu,
            has_bias: false,
            tied_embeddings: false,
            has_qk_norm: false,
        },
        "gemma" | "gemma2" => ArchConstraints {
            norm_type: NormType::RmsNorm,
            activation: Activation::Gelu,
            positional_encoding: PosEncoding::Rope,
            mlp_type: MlpType::GeluMlp,
            has_bias: false,
            tied_embeddings: true,
            has_qk_norm: false,
        },
        "phi" | "phi3" => ArchConstraints {
            norm_type: NormType::RmsNorm,
            activation: Activation::Silu,
            positional_encoding: PosEncoding::Rope,
            mlp_type: MlpType::SwiGlu,
            has_bias: true,
            tied_embeddings: false,
            has_qk_norm: false,
        },
        "starcoder2" => ArchConstraints {
            norm_type: NormType::LayerNorm,
            activation: Activation::Gelu,
            positional_encoding: PosEncoding::Rope,
            mlp_type: MlpType::GeluMlp,
            has_bias: true,
            tied_embeddings: false,
            has_qk_norm: false,
        },
        "gpt2" | "gpt_neo" | "gpt_neox" => ArchConstraints {
            norm_type: NormType::LayerNorm,
            activation: Activation::Gelu,
            positional_encoding: PosEncoding::Absolute,
            mlp_type: MlpType::GeluMlp,
            has_bias: true,
            tied_embeddings: true,
            has_qk_norm: false,
        },
        "falcon" => ArchConstraints {
            norm_type: NormType::LayerNorm,
            activation: Activation::Gelu,
            positional_encoding: PosEncoding::Rope,
            mlp_type: MlpType::GeluMlp,
            has_bias: false,
            tied_embeddings: false,
            has_qk_norm: false,
        },
        "internlm2" => ArchConstraints {
            norm_type: NormType::RmsNorm,
            activation: Activation::Silu,
            positional_encoding: PosEncoding::Rope,
            mlp_type: MlpType::SwiGlu,
            has_bias: false,
            tied_embeddings: false,
            has_qk_norm: false,
        },
        "deepseek_v2" => ArchConstraints {
            norm_type: NormType::RmsNorm,
            activation: Activation::Silu,
            positional_encoding: PosEncoding::Rope,
            mlp_type: MlpType::SwiGlu,
            has_bias: false,
            tied_embeddings: false,
            has_qk_norm: true,
        },
        // Default to llama-like constraints for unknown architectures
        _ => ArchConstraints {
            norm_type: NormType::RmsNorm,
            activation: Activation::Silu,
            positional_encoding: PosEncoding::Rope,
            mlp_type: MlpType::SwiGlu,
            has_bias: false,
            tied_embeddings: false,
            has_qk_norm: false,
        },
    }
}

/// Determine kernel contracts required by a model configuration.
///
/// Uses the `model_type` field to look up architecture constraints, then
/// derives attention type (GQA vs MHA) from head counts.
pub fn required_kernels(config: &HfModelConfig) -> Vec<KernelRequirement> {
    let ac = arch_constraints(&config.model_type);
    let mut kernels = Vec::new();

    // Normalization
    match ac.norm_type {
        NormType::RmsNorm => kernels.push(KernelRequirement {
            op: "rmsnorm".to_string(),
            contract: "rmsnorm-kernel-v1".to_string(),
        }),
        NormType::LayerNorm => kernels.push(KernelRequirement {
            op: "layernorm".to_string(),
            contract: "layernorm-kernel-v1".to_string(),
        }),
    }

    // Activation
    match ac.activation {
        Activation::Silu => kernels.push(KernelRequirement {
            op: "silu".to_string(),
            contract: "silu-kernel-v1".to_string(),
        }),
        Activation::Gelu => kernels.push(KernelRequirement {
            op: "gelu".to_string(),
            contract: "gelu-kernel-v1".to_string(),
        }),
    }

    // Positional encoding
    match ac.positional_encoding {
        PosEncoding::Rope => kernels.push(KernelRequirement {
            op: "rope".to_string(),
            contract: "rope-kernel-v1".to_string(),
        }),
        PosEncoding::Absolute => kernels.push(KernelRequirement {
            op: "absolute_position".to_string(),
            contract: "absolute-position-v1".to_string(),
        }),
    }

    // MLP type
    match ac.mlp_type {
        MlpType::SwiGlu => kernels.push(KernelRequirement {
            op: "swiglu".to_string(),
            contract: "swiglu-kernel-v1".to_string(),
        }),
        MlpType::GeluMlp => kernels.push(KernelRequirement {
            op: "gelu_mlp".to_string(),
            contract: "gelu-kernel-v1".to_string(),
        }),
    }

    // Conditional flags
    if ac.has_bias {
        kernels.push(KernelRequirement {
            op: "bias_add".to_string(),
            contract: "bias-add-v1".to_string(),
        });
    }
    if ac.tied_embeddings {
        kernels.push(KernelRequirement {
            op: "tied_embeddings".to_string(),
            contract: "tied-embeddings-v1".to_string(),
        });
    }
    if ac.has_qk_norm {
        kernels.push(KernelRequirement {
            op: "qk_norm".to_string(),
            contract: "qk-norm-v1".to_string(),
        });
    }

    // Attention type: GQA when num_key_value_heads < num_attention_heads
    let is_gqa = match (config.num_attention_heads, config.num_key_value_heads) {
        (Some(heads), Some(kv_heads)) => kv_heads < heads,
        _ => false,
    };
    if is_gqa {
        kernels.push(KernelRequirement {
            op: "gqa".to_string(),
            contract: "gqa-kernel-v1".to_string(),
        });
    } else {
        kernels.push(KernelRequirement {
            op: "attention".to_string(),
            contract: "attention-kernel-v1".to_string(),
        });
    }

    // Universal kernels (always required)
    kernels.push(KernelRequirement {
        op: "softmax".to_string(),
        contract: "softmax-kernel-v1".to_string(),
    });
    kernels.push(KernelRequirement {
        op: "matmul".to_string(),
        contract: "matmul-kernel-v1".to_string(),
    });
    kernels.push(KernelRequirement {
        op: "embedding_lookup".to_string(),
        contract: "embedding-lookup-v1".to_string(),
    });

    kernels
}
