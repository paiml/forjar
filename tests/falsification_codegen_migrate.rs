//! FJ-005/044/1350: Script codegen, Docker→pepita migration, HF config parsing.
//!
//! Popperian rejection criteria for:
//! - FJ-005: check_script, apply_script, state_query_script dispatch for all
//!   resource types, sudo wrapping, recipe rejection
//! - FJ-044: docker_to_pepita (state mapping, ports→netns, image warning,
//!   volumes warning, env warning, restart warning), migrate_config
//! - FJ-1350: parse_hf_config_str, required_kernels (llama, qwen2, gpt2,
//!   gemma, deepseek_v2, GQA detection)
//!
//! Usage: cargo test --test falsification_codegen_migrate

use forjar::core::codegen::{apply_script, check_script, state_query_script};
use forjar::core::migrate::docker_to_pepita;
use forjar::core::store::hf_config::{parse_hf_config_str, required_kernels};
use forjar::core::types::*;

// ============================================================================
// FJ-005: Script codegen dispatch
// ============================================================================

fn make_resource(rtype: ResourceType) -> Resource {
    let mut r = Resource {
        resource_type: rtype.clone(),
        ..Default::default()
    };
    // Set minimum fields so scripts generate valid output
    match rtype {
        ResourceType::Package => {
            r.packages = vec!["curl".into()];
        }
        ResourceType::File => {
            r.path = Some("/etc/test.conf".into());
            r.content = Some("key=value".into());
        }
        ResourceType::Service => {
            r.name = Some("nginx".into());
        }
        ResourceType::Mount => {
            r.path = Some("/mnt/data".into());
            r.source = Some("/dev/sda1".into());
        }
        ResourceType::User => {
            r.name = Some("deploy".into());
        }
        ResourceType::Docker => {
            r.name = Some("app".into());
            r.image = Some("nginx:latest".into());
        }
        ResourceType::Cron => {
            r.name = Some("backup".into());
            r.schedule = Some("0 * * * *".into());
            r.command = Some("backup.sh".into());
        }
        ResourceType::Pepita => {
            r.name = Some("sandbox".into());
        }
        ResourceType::Network => {
            r.port = Some("443".into());
            r.protocol = Some("tcp".into());
            r.action = Some("allow".into());
        }
        ResourceType::Model => {
            r.name = Some("llama".into());
        }
        ResourceType::Gpu => {
            r.name = Some("gpu0".into());
        }
        ResourceType::Task => {
            r.command = Some("echo task".into());
        }
        ResourceType::WasmBundle => {
            r.path = Some("/opt/plugin.wasm".into());
            r.source = Some("/src/plugin.wasm".into());
        }
        ResourceType::Build => {
            r.command = Some("cargo build".into());
        }
        _ => {}
    }
    r
}

#[test]
fn codegen_check_all_types() {
    let types = [
        ResourceType::Package,
        ResourceType::File,
        ResourceType::Service,
        ResourceType::Mount,
        ResourceType::User,
        ResourceType::Docker,
        ResourceType::Cron,
        ResourceType::Network,
        ResourceType::Pepita,
        ResourceType::Model,
        ResourceType::Gpu,
        ResourceType::Task,
        ResourceType::WasmBundle,
        ResourceType::Image,
        ResourceType::Build,
    ];
    for rtype in types {
        let r = make_resource(rtype.clone());
        let result = check_script(&r);
        assert!(
            result.is_ok(),
            "check_script failed for {rtype:?}: {result:?}"
        );
        assert!(!result.unwrap().is_empty());
    }
}

#[test]
fn codegen_apply_all_types() {
    let types = [
        ResourceType::Package,
        ResourceType::File,
        ResourceType::Service,
        ResourceType::Mount,
        ResourceType::User,
        ResourceType::Docker,
        ResourceType::Cron,
        ResourceType::Network,
        ResourceType::Pepita,
        ResourceType::Model,
        ResourceType::Gpu,
        ResourceType::Task,
        ResourceType::WasmBundle,
        ResourceType::Image,
        ResourceType::Build,
    ];
    for rtype in types {
        let r = make_resource(rtype.clone());
        let result = apply_script(&r);
        assert!(
            result.is_ok(),
            "apply_script failed for {rtype:?}: {result:?}"
        );
        assert!(!result.unwrap().is_empty());
    }
}

#[test]
fn codegen_state_query_all_types() {
    let types = [
        ResourceType::Package,
        ResourceType::File,
        ResourceType::Service,
        ResourceType::Mount,
        ResourceType::User,
        ResourceType::Docker,
        ResourceType::Cron,
        ResourceType::Network,
        ResourceType::Pepita,
        ResourceType::Model,
        ResourceType::Gpu,
        ResourceType::Task,
        ResourceType::WasmBundle,
        ResourceType::Image,
        ResourceType::Build,
    ];
    for rtype in types {
        let r = make_resource(rtype.clone());
        let result = state_query_script(&r);
        assert!(
            result.is_ok(),
            "state_query_script failed for {rtype:?}: {result:?}"
        );
    }
}

#[test]
fn codegen_recipe_rejects() {
    let r = make_resource(ResourceType::Recipe);
    assert!(check_script(&r).is_err());
    assert!(apply_script(&r).is_err());
    assert!(state_query_script(&r).is_err());
}

#[test]
fn codegen_apply_sudo_wraps() {
    let mut r = make_resource(ResourceType::Package);
    r.sudo = true;
    let script = apply_script(&r).unwrap();
    assert!(script.contains("sudo bash"));
    assert!(script.contains("FORJAR_SUDO"));
}

#[test]
fn codegen_apply_no_sudo() {
    let r = make_resource(ResourceType::Package);
    let script = apply_script(&r).unwrap();
    assert!(!script.contains("sudo bash"));
}

// ============================================================================
// FJ-044: Docker → pepita migration
// ============================================================================

#[test]
fn migrate_running_docker() {
    let mut docker = make_resource(ResourceType::Docker);
    docker.state = Some("running".into());
    let result = docker_to_pepita("app", &docker);
    assert_eq!(result.resource.resource_type, ResourceType::Pepita);
    assert_eq!(result.resource.state.as_deref(), Some("present"));
}

#[test]
fn migrate_stopped_docker() {
    let mut docker = make_resource(ResourceType::Docker);
    docker.state = Some("stopped".into());
    let result = docker_to_pepita("app", &docker);
    assert_eq!(result.resource.state.as_deref(), Some("absent"));
    assert!(result.warnings.iter().any(|w| w.contains("stopped")));
}

#[test]
fn migrate_absent_docker() {
    let mut docker = make_resource(ResourceType::Docker);
    docker.state = Some("absent".into());
    let result = docker_to_pepita("app", &docker);
    assert_eq!(result.resource.state.as_deref(), Some("absent"));
}

#[test]
fn migrate_unknown_state() {
    let mut docker = make_resource(ResourceType::Docker);
    docker.state = Some("paused".into());
    let result = docker_to_pepita("app", &docker);
    assert_eq!(result.resource.state.as_deref(), Some("present"));
    assert!(result.warnings.iter().any(|w| w.contains("paused")));
}

#[test]
fn migrate_ports_enable_netns() {
    let mut docker = make_resource(ResourceType::Docker);
    docker.ports = vec!["8080:80".into()];
    let result = docker_to_pepita("app", &docker);
    assert!(result.resource.netns);
    assert!(result.resource.ports.is_empty());
}

#[test]
fn migrate_image_warning() {
    let mut docker = make_resource(ResourceType::Docker);
    docker.image = Some("nginx:latest".into());
    let result = docker_to_pepita("app", &docker);
    assert!(result.warnings.iter().any(|w| w.contains("nginx:latest")));
    assert!(result.resource.image.is_none());
}

#[test]
fn migrate_volumes_warning() {
    let mut docker = make_resource(ResourceType::Docker);
    docker.volumes = vec!["/host:/container".into()];
    let result = docker_to_pepita("app", &docker);
    assert!(result.warnings.iter().any(|w| w.contains("volumes")));
    assert!(result.resource.volumes.is_empty());
}

#[test]
fn migrate_env_warning() {
    let mut docker = make_resource(ResourceType::Docker);
    docker.environment = vec!["PORT=8080".into()];
    let result = docker_to_pepita("app", &docker);
    assert!(result.warnings.iter().any(|w| w.contains("environment")));
    assert!(result.resource.environment.is_empty());
}

#[test]
fn migrate_restart_warning() {
    let mut docker = make_resource(ResourceType::Docker);
    docker.restart = Some("always".into());
    let result = docker_to_pepita("app", &docker);
    assert!(result.warnings.iter().any(|w| w.contains("restart")));
    assert!(result.resource.restart.is_none());
}

#[test]
fn migrate_default_state() {
    let docker = make_resource(ResourceType::Docker);
    let result = docker_to_pepita("app", &docker);
    assert_eq!(result.resource.state.as_deref(), Some("present"));
}

// ============================================================================
// FJ-1350: HF config parsing
// ============================================================================

fn llama_json() -> &'static str {
    r#"{
        "model_type": "llama",
        "architectures": ["LlamaForCausalLM"],
        "hidden_size": 4096,
        "num_attention_heads": 32,
        "num_key_value_heads": 8,
        "num_hidden_layers": 32,
        "intermediate_size": 11008,
        "vocab_size": 32000,
        "max_position_embeddings": 4096
    }"#
}

#[test]
fn hf_parse_llama() {
    let config = parse_hf_config_str(llama_json()).unwrap();
    assert_eq!(config.model_type, "llama");
    assert_eq!(config.hidden_size, Some(4096));
    assert_eq!(config.num_attention_heads, Some(32));
    assert_eq!(config.num_key_value_heads, Some(8));
}

#[test]
fn hf_parse_invalid() {
    assert!(parse_hf_config_str("not json").is_err());
}

#[test]
fn hf_parse_minimal() {
    let config = parse_hf_config_str(r#"{"model_type": "gpt2"}"#).unwrap();
    assert_eq!(config.model_type, "gpt2");
    assert!(config.hidden_size.is_none());
}

#[test]
fn hf_kernels_llama_gqa() {
    let config = parse_hf_config_str(llama_json()).unwrap();
    let kernels = required_kernels(&config);
    let ops: Vec<&str> = kernels.iter().map(|k| k.op.as_str()).collect();
    assert!(ops.contains(&"rmsnorm"));
    assert!(ops.contains(&"silu"));
    assert!(ops.contains(&"rope"));
    assert!(ops.contains(&"swiglu"));
    assert!(ops.contains(&"gqa")); // kv_heads(8) < heads(32)
    assert!(ops.contains(&"softmax"));
    assert!(ops.contains(&"matmul"));
    assert!(ops.contains(&"embedding_lookup"));
    assert!(!ops.contains(&"bias_add")); // llama has no bias
}

#[test]
fn hf_kernels_gpt2_absolute() {
    let config = parse_hf_config_str(r#"{"model_type": "gpt2"}"#).unwrap();
    let kernels = required_kernels(&config);
    let ops: Vec<&str> = kernels.iter().map(|k| k.op.as_str()).collect();
    assert!(ops.contains(&"layernorm"));
    assert!(ops.contains(&"gelu"));
    assert!(ops.contains(&"absolute_position"));
    assert!(ops.contains(&"gelu_mlp"));
    assert!(ops.contains(&"bias_add"));
    assert!(ops.contains(&"tied_embeddings"));
    assert!(ops.contains(&"attention")); // no GQA for gpt2
}

#[test]
fn hf_kernels_qwen2_bias() {
    let config = parse_hf_config_str(r#"{"model_type": "qwen2"}"#).unwrap();
    let kernels = required_kernels(&config);
    let ops: Vec<&str> = kernels.iter().map(|k| k.op.as_str()).collect();
    assert!(ops.contains(&"bias_add")); // qwen2 has bias
    assert!(ops.contains(&"rmsnorm"));
}

#[test]
fn hf_kernels_gemma_tied() {
    let config = parse_hf_config_str(r#"{"model_type": "gemma"}"#).unwrap();
    let kernels = required_kernels(&config);
    let ops: Vec<&str> = kernels.iter().map(|k| k.op.as_str()).collect();
    assert!(ops.contains(&"tied_embeddings"));
    assert!(ops.contains(&"gelu"));
}

#[test]
fn hf_kernels_deepseek_qk_norm() {
    let config = parse_hf_config_str(r#"{"model_type": "deepseek_v2"}"#).unwrap();
    let kernels = required_kernels(&config);
    let ops: Vec<&str> = kernels.iter().map(|k| k.op.as_str()).collect();
    assert!(ops.contains(&"qk_norm"));
}

#[test]
fn hf_kernels_mha_when_equal_heads() {
    let json = r#"{
        "model_type": "llama",
        "num_attention_heads": 32,
        "num_key_value_heads": 32
    }"#;
    let config = parse_hf_config_str(json).unwrap();
    let kernels = required_kernels(&config);
    let ops: Vec<&str> = kernels.iter().map(|k| k.op.as_str()).collect();
    assert!(ops.contains(&"attention")); // MHA, not GQA
    assert!(!ops.contains(&"gqa"));
}

#[test]
fn hf_kernels_unknown_arch_defaults() {
    let config = parse_hf_config_str(r#"{"model_type": "custom_model_2026"}"#).unwrap();
    let kernels = required_kernels(&config);
    let ops: Vec<&str> = kernels.iter().map(|k| k.op.as_str()).collect();
    // Defaults to llama-like
    assert!(ops.contains(&"rmsnorm"));
    assert!(ops.contains(&"silu"));
    assert!(ops.contains(&"rope"));
}
