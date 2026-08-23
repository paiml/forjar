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
#[derive(Clone, Copy)]
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

/// `qwen2` / `qwen2_moe` constraints.
const ARCH_QWEN2: ArchConstraints = ArchConstraints {
    norm_type: NormType::RmsNorm,
    activation: Activation::Silu,
    positional_encoding: PosEncoding::Rope,
    mlp_type: MlpType::SwiGlu,
    has_bias: true,
    tied_embeddings: false,
    has_qk_norm: false,
};

/// `llama` / `codellama` constraints.
const ARCH_LLAMA: ArchConstraints = ArchConstraints {
    norm_type: NormType::RmsNorm,
    activation: Activation::Silu,
    positional_encoding: PosEncoding::Rope,
    mlp_type: MlpType::SwiGlu,
    has_bias: false,
    tied_embeddings: false,
    has_qk_norm: false,
};

/// `mistral` / `mixtral` constraints.
const ARCH_MISTRAL: ArchConstraints = ArchConstraints {
    norm_type: NormType::RmsNorm,
    activation: Activation::Silu,
    positional_encoding: PosEncoding::Rope,
    mlp_type: MlpType::SwiGlu,
    has_bias: false,
    tied_embeddings: false,
    has_qk_norm: false,
};

/// `gemma` / `gemma2` constraints.
const ARCH_GEMMA: ArchConstraints = ArchConstraints {
    norm_type: NormType::RmsNorm,
    activation: Activation::Gelu,
    positional_encoding: PosEncoding::Rope,
    mlp_type: MlpType::GeluMlp,
    has_bias: false,
    tied_embeddings: true,
    has_qk_norm: false,
};

/// `phi` / `phi3` constraints.
const ARCH_PHI: ArchConstraints = ArchConstraints {
    norm_type: NormType::RmsNorm,
    activation: Activation::Silu,
    positional_encoding: PosEncoding::Rope,
    mlp_type: MlpType::SwiGlu,
    has_bias: true,
    tied_embeddings: false,
    has_qk_norm: false,
};

/// `starcoder2` constraints.
const ARCH_STARCODER2: ArchConstraints = ArchConstraints {
    norm_type: NormType::LayerNorm,
    activation: Activation::Gelu,
    positional_encoding: PosEncoding::Rope,
    mlp_type: MlpType::GeluMlp,
    has_bias: true,
    tied_embeddings: false,
    has_qk_norm: false,
};

/// `gpt2` / `gpt_neo` / `gpt_neox` constraints.
const ARCH_GPT2: ArchConstraints = ArchConstraints {
    norm_type: NormType::LayerNorm,
    activation: Activation::Gelu,
    positional_encoding: PosEncoding::Absolute,
    mlp_type: MlpType::GeluMlp,
    has_bias: true,
    tied_embeddings: true,
    has_qk_norm: false,
};

/// `falcon` constraints.
const ARCH_FALCON: ArchConstraints = ArchConstraints {
    norm_type: NormType::LayerNorm,
    activation: Activation::Gelu,
    positional_encoding: PosEncoding::Rope,
    mlp_type: MlpType::GeluMlp,
    has_bias: false,
    tied_embeddings: false,
    has_qk_norm: false,
};

/// `internlm2` constraints.
const ARCH_INTERNLM2: ArchConstraints = ArchConstraints {
    norm_type: NormType::RmsNorm,
    activation: Activation::Silu,
    positional_encoding: PosEncoding::Rope,
    mlp_type: MlpType::SwiGlu,
    has_bias: false,
    tied_embeddings: false,
    has_qk_norm: false,
};

/// `deepseek_v2` constraints.
const ARCH_DEEPSEEK_V2: ArchConstraints = ArchConstraints {
    norm_type: NormType::RmsNorm,
    activation: Activation::Silu,
    positional_encoding: PosEncoding::Rope,
    mlp_type: MlpType::SwiGlu,
    has_bias: false,
    tied_embeddings: false,
    has_qk_norm: true,
};

/// Constraints for unknown architectures: llama-like.
const ARCH_DEFAULT: ArchConstraints = ArchConstraints {
    norm_type: NormType::RmsNorm,
    activation: Activation::Silu,
    positional_encoding: PosEncoding::Rope,
    mlp_type: MlpType::SwiGlu,
    has_bias: false,
    tied_embeddings: false,
    has_qk_norm: false,
};

/// `model_type` -> constraints, one row per literal the mapping recognises.
///
/// The rows are in the same order as the arms they replace, and every literal
/// is distinct, so a first-match scan of this table selects exactly what the
/// equivalent `match` selected.
const ARCH_TABLE: &[(&str, ArchConstraints)] = &[
    ("qwen2", ARCH_QWEN2),
    ("qwen2_moe", ARCH_QWEN2),
    ("llama", ARCH_LLAMA),
    ("codellama", ARCH_LLAMA),
    ("mistral", ARCH_MISTRAL),
    ("mixtral", ARCH_MISTRAL),
    ("gemma", ARCH_GEMMA),
    ("gemma2", ARCH_GEMMA),
    ("phi", ARCH_PHI),
    ("phi3", ARCH_PHI),
    ("starcoder2", ARCH_STARCODER2),
    ("gpt2", ARCH_GPT2),
    ("gpt_neo", ARCH_GPT2),
    ("gpt_neox", ARCH_GPT2),
    ("falcon", ARCH_FALCON),
    ("internlm2", ARCH_INTERNLM2),
    ("deepseek_v2", ARCH_DEEPSEEK_V2),
];

/// Map `model_type` to architecture constraints (from arch-constraints-v1.yaml).
fn arch_constraints(model_type: &str) -> ArchConstraints {
    ARCH_TABLE
        .iter()
        .find(|(name, _)| *name == model_type)
        .map_or(ARCH_DEFAULT, |(_, constraints)| *constraints)
}

/// Build one requirement from a borrowed op/contract pair.
fn kernel(op: &str, contract: &str) -> KernelRequirement {
    KernelRequirement {
        op: op.to_string(),
        contract: contract.to_string(),
    }
}

/// Normalization kernel.
fn norm_kernel(norm_type: NormType) -> KernelRequirement {
    match norm_type {
        NormType::RmsNorm => kernel("rmsnorm", "rmsnorm-kernel-v1"),
        NormType::LayerNorm => kernel("layernorm", "layernorm-kernel-v1"),
    }
}

/// Activation kernel.
fn activation_kernel(activation: Activation) -> KernelRequirement {
    match activation {
        Activation::Silu => kernel("silu", "silu-kernel-v1"),
        Activation::Gelu => kernel("gelu", "gelu-kernel-v1"),
    }
}

/// Positional-encoding kernel.
fn positional_kernel(positional_encoding: PosEncoding) -> KernelRequirement {
    match positional_encoding {
        PosEncoding::Rope => kernel("rope", "rope-kernel-v1"),
        PosEncoding::Absolute => kernel("absolute_position", "absolute-position-v1"),
    }
}

/// MLP kernel.
fn mlp_kernel(mlp_type: MlpType) -> KernelRequirement {
    match mlp_type {
        MlpType::SwiGlu => kernel("swiglu", "swiglu-kernel-v1"),
        MlpType::GeluMlp => kernel("gelu_mlp", "gelu-kernel-v1"),
    }
}

/// Kernels demanded by the boolean architecture flags, in flag order.
fn flag_kernels(ac: &ArchConstraints) -> Vec<KernelRequirement> {
    let flags = [
        (ac.has_bias, "bias_add", "bias-add-v1"),
        (ac.tied_embeddings, "tied_embeddings", "tied-embeddings-v1"),
        (ac.has_qk_norm, "qk_norm", "qk-norm-v1"),
    ];
    flags
        .iter()
        .filter(|(set, _, _)| *set)
        .map(|(_, op, contract)| kernel(op, contract))
        .collect()
}

/// Attention kernel: GQA when `num_key_value_heads` < `num_attention_heads`.
fn attention_kernel(config: &HfModelConfig) -> KernelRequirement {
    let is_gqa = match (config.num_attention_heads, config.num_key_value_heads) {
        (Some(heads), Some(kv_heads)) => kv_heads < heads,
        _ => false,
    };
    if is_gqa {
        kernel("gqa", "gqa-kernel-v1")
    } else {
        kernel("attention", "attention-kernel-v1")
    }
}

/// Kernels every model requires, in emission order.
const UNIVERSAL_KERNELS: &[(&str, &str)] = &[
    ("softmax", "softmax-kernel-v1"),
    ("matmul", "matmul-kernel-v1"),
    ("embedding_lookup", "embedding-lookup-v1"),
];

/// Determine kernel contracts required by a model configuration.
///
/// Uses the `model_type` field to look up architecture constraints, then
/// derives attention type (GQA vs MHA) from head counts.
pub fn required_kernels(config: &HfModelConfig) -> Vec<KernelRequirement> {
    let ac = arch_constraints(&config.model_type);
    let mut kernels = vec![
        norm_kernel(ac.norm_type),
        activation_kernel(ac.activation),
        positional_kernel(ac.positional_encoding),
        mlp_kernel(ac.mlp_type),
    ];
    kernels.extend(flag_kernels(&ac));
    kernels.push(attention_kernel(config));
    kernels.extend(
        UNIVERSAL_KERNELS
            .iter()
            .map(|(op, contract)| kernel(op, contract)),
    );
    kernels
}
