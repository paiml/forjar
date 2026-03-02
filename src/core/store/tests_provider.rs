//! Tests for FJ-1333–FJ-1336: Universal provider import.

use super::provider::{
    all_providers, capture_method, import_command, origin_ref_string, parse_import_config,
    validate_import, ImportConfig, ImportProvider,
};
use std::collections::BTreeMap;

fn apt_config() -> ImportConfig {
    ImportConfig {
        provider: ImportProvider::Apt,
        reference: "nginx".to_string(),
        version: Some("1.24.0".to_string()),
        arch: "x86_64".to_string(),
        options: BTreeMap::new(),
    }
}

fn cargo_config() -> ImportConfig {
    ImportConfig {
        provider: ImportProvider::Cargo,
        reference: "ripgrep".to_string(),
        version: Some("14.1.0".to_string()),
        arch: "x86_64".to_string(),
        options: BTreeMap::new(),
    }
}

#[test]
fn test_fj1333_import_command_apt() {
    let cmd = import_command(&apt_config());
    assert!(cmd.contains("apt-get install"));
    assert!(cmd.contains("nginx=1.24.0"));
}

#[test]
fn test_fj1333_import_command_apt_no_version() {
    let mut cfg = apt_config();
    cfg.version = None;
    let cmd = import_command(&cfg);
    assert!(cmd.contains("nginx"));
    assert!(!cmd.contains('='));
}

#[test]
fn test_fj1333_import_command_cargo() {
    let cmd = import_command(&cargo_config());
    assert!(cmd.contains("cargo install"));
    assert!(cmd.contains("--version 14.1.0"));
    assert!(cmd.contains("ripgrep"));
}

#[test]
fn test_fj1334_import_command_nix() {
    let cfg = ImportConfig {
        provider: ImportProvider::Nix,
        reference: "nixpkgs#ripgrep".to_string(),
        version: None,
        arch: "x86_64".to_string(),
        options: BTreeMap::new(),
    };
    let cmd = import_command(&cfg);
    assert!(cmd.contains("nix build"));
    assert!(cmd.contains("nixpkgs#ripgrep"));
}

#[test]
fn test_fj1335_import_command_docker() {
    let cfg = ImportConfig {
        provider: ImportProvider::Docker,
        reference: "ubuntu".to_string(),
        version: Some("24.04".to_string()),
        arch: "x86_64".to_string(),
        options: BTreeMap::new(),
    };
    let cmd = import_command(&cfg);
    assert!(cmd.contains("docker create ubuntu:24.04"));
    assert!(cmd.contains("docker export"));
}

#[test]
fn test_fj1336_import_command_tofu() {
    let cfg = ImportConfig {
        provider: ImportProvider::Tofu,
        reference: "./infra/".to_string(),
        version: None,
        arch: "x86_64".to_string(),
        options: BTreeMap::new(),
    };
    let cmd = import_command(&cfg);
    assert!(cmd.contains("tofu"));
    assert!(cmd.contains("./infra/"));
}

#[test]
fn test_fj1336_import_command_terraform() {
    let cfg = ImportConfig {
        provider: ImportProvider::Terraform,
        reference: "./infra/".to_string(),
        version: None,
        arch: "x86_64".to_string(),
        options: BTreeMap::new(),
    };
    let cmd = import_command(&cfg);
    assert!(cmd.contains("terraform"));
}

#[test]
fn test_fj1333_import_command_uv() {
    let cfg = ImportConfig {
        provider: ImportProvider::Uv,
        reference: "flask".to_string(),
        version: Some("3.0.0".to_string()),
        arch: "x86_64".to_string(),
        options: BTreeMap::new(),
    };
    let cmd = import_command(&cfg);
    assert!(cmd.contains("uv pip install"));
    assert!(cmd.contains("flask==3.0.0"));
}

#[test]
fn test_fj1333_import_command_apr() {
    let cfg = ImportConfig {
        provider: ImportProvider::Apr,
        reference: "meta-llama/Llama-3".to_string(),
        version: None,
        arch: "x86_64".to_string(),
        options: BTreeMap::new(),
    };
    let cmd = import_command(&cfg);
    assert!(cmd.contains("apr pull"));
    assert!(cmd.contains("meta-llama/Llama-3"));
}

#[test]
fn test_fj1333_origin_ref_apt() {
    let origin = origin_ref_string(&apt_config());
    assert_eq!(origin, "apt:nginx@1.24.0");
}

#[test]
fn test_fj1333_origin_ref_cargo() {
    let origin = origin_ref_string(&cargo_config());
    assert_eq!(origin, "cargo:ripgrep@14.1.0");
}

#[test]
fn test_fj1334_origin_ref_nix() {
    let cfg = ImportConfig {
        provider: ImportProvider::Nix,
        reference: "nixpkgs#ripgrep".to_string(),
        version: None,
        arch: "x86_64".to_string(),
        options: BTreeMap::new(),
    };
    assert_eq!(origin_ref_string(&cfg), "nixpkgs#ripgrep");
}

#[test]
fn test_fj1335_origin_ref_docker() {
    let cfg = ImportConfig {
        provider: ImportProvider::Docker,
        reference: "ubuntu".to_string(),
        version: Some("24.04".to_string()),
        arch: "x86_64".to_string(),
        options: BTreeMap::new(),
    };
    assert_eq!(origin_ref_string(&cfg), "docker:ubuntu@24.04");
}

#[test]
fn test_fj1333_validate_valid() {
    let errors = validate_import(&apt_config());
    assert!(errors.is_empty());
}

#[test]
fn test_fj1333_validate_empty_reference() {
    let mut cfg = apt_config();
    cfg.reference = String::new();
    let errors = validate_import(&cfg);
    assert!(errors.iter().any(|e| e.contains("reference")));
}

#[test]
fn test_fj1334_validate_nix_format() {
    let cfg = ImportConfig {
        provider: ImportProvider::Nix,
        reference: "ripgrep".to_string(),
        version: None,
        arch: "x86_64".to_string(),
        options: BTreeMap::new(),
    };
    let errors = validate_import(&cfg);
    assert!(errors.iter().any(|e| e.contains("flake format")));
}

#[test]
fn test_fj1335_validate_docker_no_spaces() {
    let cfg = ImportConfig {
        provider: ImportProvider::Docker,
        reference: "my image".to_string(),
        version: None,
        arch: "x86_64".to_string(),
        options: BTreeMap::new(),
    };
    let errors = validate_import(&cfg);
    assert!(errors.iter().any(|e| e.contains("spaces")));
}

#[test]
fn test_fj1333_parse_yaml() {
    let yaml = r#"
provider: apt
reference: nginx
version: "1.24.0"
arch: x86_64
"#;
    let cfg = parse_import_config(yaml).unwrap();
    assert_eq!(cfg.provider, ImportProvider::Apt);
    assert_eq!(cfg.reference, "nginx");
    assert_eq!(cfg.version.as_deref(), Some("1.24.0"));
}

#[test]
fn test_fj1333_parse_yaml_invalid() {
    assert!(parse_import_config("invalid: [yaml").is_err());
}

#[test]
fn test_fj1333_capture_method_all() {
    for provider in all_providers() {
        let method = capture_method(provider);
        assert!(
            !method.is_empty(),
            "empty capture method for {:?}",
            provider
        );
    }
}

#[test]
fn test_fj1333_all_providers_count() {
    assert_eq!(all_providers().len(), 8);
}

#[test]
fn test_fj1333_serde_roundtrip() {
    let cfg = apt_config();
    let yaml = serde_yaml_ng::to_string(&cfg).unwrap();
    let parsed: ImportConfig = serde_yaml_ng::from_str(&yaml).unwrap();
    assert_eq!(cfg, parsed);
}

#[test]
fn test_fj1333_provider_json_serde() {
    for provider in all_providers() {
        let json = serde_json::to_string(&provider).unwrap();
        let parsed: ImportProvider = serde_json::from_str(&json).unwrap();
        assert_eq!(provider, parsed);
    }
}
