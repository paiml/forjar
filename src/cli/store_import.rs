//! FJ-1333–FJ-1340: `forjar import <provider> <ref>` — universal store import CLI.
//!
//! Import artifacts from any provider into the content-addressed store:
//! - `forjar import docker ubuntu:24.04`
//! - `forjar import nix nixpkgs#ripgrep`
//! - `forjar import apt nginx=1.24.0`
//! - `forjar import cargo ripgrep --version 14.1.0`
//! - `forjar import tofu ./infra/`
//! - `forjar import apr meta-llama/Llama-3`

use crate::core::store::provider::{
    all_providers, capture_method, import_command, origin_ref_string, validate_import,
    ImportConfig, ImportProvider,
};
use std::collections::BTreeMap;
use std::path::Path;

/// Import an artifact from a provider into the store.
pub(crate) fn cmd_store_import(
    provider: &str,
    reference: &str,
    version: Option<&str>,
    store_dir: &Path,
    json: bool,
) -> Result<(), String> {
    let import_provider = parse_provider(provider)?;
    let config = ImportConfig {
        provider: import_provider,
        reference: reference.to_string(),
        version: version.map(|v| v.to_string()),
        arch: current_arch(),
        options: BTreeMap::new(),
    };

    let errors = validate_import(&config);
    if !errors.is_empty() {
        return Err(format!("validation errors: {}", errors.join("; ")));
    }

    let cmd = import_command(&config);
    let origin = origin_ref_string(&config);
    let capture = capture_method(import_provider);

    if json {
        let j = serde_json::json!({
            "provider": provider,
            "reference": reference,
            "version": version,
            "command": cmd,
            "origin_ref": origin,
            "capture_method": capture,
            "store_dir": store_dir.display().to_string(),
            "status": "dry-run",
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&j).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        println!("Store import:");
        println!("  Provider: {provider}");
        println!("  Reference: {reference}");
        if let Some(v) = version {
            println!("  Version: {v}");
        }
        println!("  Command: {cmd}");
        println!("  Capture: {capture}");
        println!("  Origin: {origin}");
        println!("  Store: {}", store_dir.display());
        println!("  (import execution requires shell access — dry-run shown)");
    }
    Ok(())
}

/// List all supported providers.
pub(crate) fn cmd_import_providers(json: bool) -> Result<(), String> {
    let providers = all_providers();

    if json {
        let j: Vec<serde_json::Value> = providers
            .iter()
            .map(|p| {
                let name = provider_name(*p);
                serde_json::json!({
                    "name": name,
                    "capture_method": capture_method(*p),
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&j).unwrap_or_else(|_| "[]".to_string())
        );
    } else {
        println!("Supported import providers:");
        for p in &providers {
            println!("  {:12} — {}", provider_name(*p), capture_method(*p));
        }
    }
    Ok(())
}

fn parse_provider(name: &str) -> Result<ImportProvider, String> {
    match name.to_lowercase().as_str() {
        "apt" => Ok(ImportProvider::Apt),
        "cargo" => Ok(ImportProvider::Cargo),
        "uv" | "pip" => Ok(ImportProvider::Uv),
        "nix" => Ok(ImportProvider::Nix),
        "docker" => Ok(ImportProvider::Docker),
        "tofu" | "opentofu" => Ok(ImportProvider::Tofu),
        "terraform" | "tf" => Ok(ImportProvider::Terraform),
        "apr" => Ok(ImportProvider::Apr),
        _ => Err(format!(
            "unknown provider: {name}. Use: apt, cargo, uv, nix, docker, tofu, terraform, apr"
        )),
    }
}

fn provider_name(p: ImportProvider) -> &'static str {
    match p {
        ImportProvider::Apt => "apt",
        ImportProvider::Cargo => "cargo",
        ImportProvider::Uv => "uv",
        ImportProvider::Nix => "nix",
        ImportProvider::Docker => "docker",
        ImportProvider::Tofu => "tofu",
        ImportProvider::Terraform => "terraform",
        ImportProvider::Apr => "apr",
    }
}

fn current_arch() -> String {
    #[cfg(target_arch = "x86_64")]
    {
        "x86_64".to_string()
    }
    #[cfg(target_arch = "aarch64")]
    {
        "aarch64".to_string()
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        std::env::consts::ARCH.to_string()
    }
}
