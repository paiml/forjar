//! FJ-1333–FJ-1336: Universal provider import interface.
//!
//! Any external tool can seed the forjar store. Each provider shells out to its
//! native CLI, captures outputs, BLAKE3-hashes them, and stores the result.
//! After import, all store entries are identical — provider-agnostic.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Supported import providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImportProvider {
    /// Debian/Ubuntu apt package manager.
    Apt,
    /// Rust cargo package manager.
    Cargo,
    /// Python uv package installer.
    Uv,
    /// Nix flake-based builds.
    Nix,
    /// Docker container images.
    Docker,
    /// OpenTofu infrastructure outputs.
    Tofu,
    /// Terraform infrastructure outputs.
    Terraform,
    /// Apr model registry.
    Apr,
}

/// Configuration for a provider import operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportConfig {
    /// Which provider to use
    pub provider: ImportProvider,

    /// Package/image/reference to import
    pub reference: String,

    /// Optional version pin
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// Target architecture (e.g., "x86_64")
    #[serde(default = "default_arch")]
    pub arch: String,

    /// Extra provider-specific options
    #[serde(default)]
    pub options: BTreeMap<String, String>,
}

fn default_arch() -> String {
    "x86_64".to_string()
}

/// Result of a provider import operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportResult {
    /// Store hash of the imported artifact
    pub store_hash: String,

    /// Store path where the artifact was placed
    pub store_path: String,

    /// Number of files captured
    pub file_count: u64,

    /// Total size in bytes
    pub total_size: u64,

    /// Provider that produced this import
    pub provider: ImportProvider,

    /// Upstream reference for provenance
    pub origin_ref: String,

    /// The CLI command that was invoked
    pub cli_command: String,
}

/// Generate the CLI command for a provider import.
pub fn import_command(config: &ImportConfig) -> String {
    match config.provider {
        ImportProvider::Apt => {
            let ver = config
                .version
                .as_deref()
                .map_or(String::new(), |v| format!("={v}"));
            format!(
                "apt-get install -y --download-only {}{ver}",
                config.reference
            )
        }
        ImportProvider::Cargo => {
            let ver = config
                .version
                .as_deref()
                .map_or(String::new(), |v| format!(" --version {v}"));
            format!("cargo install{ver} --root $STAGING {}", config.reference)
        }
        ImportProvider::Uv => {
            let ver = config
                .version
                .as_deref()
                .map_or(config.reference.clone(), |v| {
                    format!("{}=={v}", config.reference)
                });
            format!("uv pip install --target $STAGING {ver}")
        }
        ImportProvider::Nix => {
            format!("nix build --print-out-paths {}", config.reference)
        }
        ImportProvider::Docker => {
            let tag = config
                .version
                .as_deref()
                .map_or(String::new(), |v| format!(":{v}"));
            format!("docker create {}{tag} && docker export", config.reference)
        }
        ImportProvider::Tofu => {
            format!("tofu -chdir={} output -json", config.reference)
        }
        ImportProvider::Terraform => {
            format!("terraform -chdir={} output -json", config.reference)
        }
        ImportProvider::Apr => {
            format!("apr pull {}", config.reference)
        }
    }
}

/// Generate the upstream reference string for provenance metadata.
pub fn origin_ref_string(config: &ImportConfig) -> String {
    let ver = config
        .version
        .as_deref()
        .map_or(String::new(), |v| format!("@{v}"));
    match config.provider {
        ImportProvider::Apt => format!("apt:{}{ver}", config.reference),
        ImportProvider::Cargo => format!("cargo:{}{ver}", config.reference),
        ImportProvider::Uv => format!("uv:{}{ver}", config.reference),
        ImportProvider::Nix => config.reference.clone(),
        ImportProvider::Docker => format!("docker:{}{ver}", config.reference),
        ImportProvider::Tofu => format!("tofu:{}", config.reference),
        ImportProvider::Terraform => format!("terraform:{}", config.reference),
        ImportProvider::Apr => format!("apr:{}{ver}", config.reference),
    }
}

/// Validate an import configuration.
pub fn validate_import(config: &ImportConfig) -> Vec<String> {
    let mut errors = Vec::new();

    if config.reference.is_empty() {
        errors.push("import reference cannot be empty".to_string());
    }
    if config.arch.is_empty() {
        errors.push("arch cannot be empty".to_string());
    }

    // Provider-specific validation
    match config.provider {
        ImportProvider::Nix => {
            if !config.reference.contains('#') && !config.reference.starts_with("nixpkgs") {
                errors.push(
                    "nix reference should be in flake format (e.g., nixpkgs#ripgrep)".to_string(),
                );
            }
        }
        ImportProvider::Docker => {
            if config.reference.contains(' ') {
                errors.push("docker image name cannot contain spaces".to_string());
            }
        }
        _ => {}
    }

    errors
}

/// Parse import config from YAML.
pub fn parse_import_config(yaml: &str) -> Result<ImportConfig, String> {
    serde_yaml_ng::from_str(yaml).map_err(|e| format!("invalid import config: {e}"))
}

/// Map provider to its output capture method description.
pub fn capture_method(provider: ImportProvider) -> &'static str {
    match provider {
        ImportProvider::Apt => "package files via dpkg manifest",
        ImportProvider::Cargo => "binary output in $CARGO_HOME/bin/",
        ImportProvider::Uv => "virtualenv contents",
        ImportProvider::Nix => "output tree in /nix/store/",
        ImportProvider::Docker => "filesystem snapshot from container export",
        ImportProvider::Tofu => "state outputs as YAML",
        ImportProvider::Terraform => "state outputs as YAML",
        ImportProvider::Apr => "model artifacts (gguf, safetensors)",
    }
}

/// List all supported providers.
pub fn all_providers() -> Vec<ImportProvider> {
    vec![
        ImportProvider::Apt,
        ImportProvider::Cargo,
        ImportProvider::Uv,
        ImportProvider::Nix,
        ImportProvider::Docker,
        ImportProvider::Tofu,
        ImportProvider::Terraform,
        ImportProvider::Apr,
    ]
}
