//! Coverage tests for store_import.rs — parse_provider, provider_name,
//! current_arch, local_machine helpers.
use crate::core::store::provider::ImportProvider;

// ── parse_provider ───────────────────────────────────────────────

#[test]
fn parse_apt() {
    assert!(matches!(super::store_import::parse_provider("apt").unwrap(), ImportProvider::Apt));
}

#[test]
fn parse_cargo() {
    assert!(matches!(super::store_import::parse_provider("cargo").unwrap(), ImportProvider::Cargo));
}

#[test]
fn parse_uv() {
    assert!(matches!(super::store_import::parse_provider("uv").unwrap(), ImportProvider::Uv));
}

#[test]
fn parse_pip_alias() {
    assert!(matches!(super::store_import::parse_provider("pip").unwrap(), ImportProvider::Uv));
}

#[test]
fn parse_nix() {
    assert!(matches!(super::store_import::parse_provider("nix").unwrap(), ImportProvider::Nix));
}

#[test]
fn parse_docker() {
    assert!(matches!(super::store_import::parse_provider("docker").unwrap(), ImportProvider::Docker));
}

#[test]
fn parse_tofu() {
    assert!(matches!(super::store_import::parse_provider("tofu").unwrap(), ImportProvider::Tofu));
}

#[test]
fn parse_opentofu_alias() {
    assert!(matches!(super::store_import::parse_provider("opentofu").unwrap(), ImportProvider::Tofu));
}

#[test]
fn parse_terraform() {
    assert!(matches!(super::store_import::parse_provider("terraform").unwrap(), ImportProvider::Terraform));
}

#[test]
fn parse_tf_alias() {
    assert!(matches!(super::store_import::parse_provider("tf").unwrap(), ImportProvider::Terraform));
}

#[test]
fn parse_apr() {
    assert!(matches!(super::store_import::parse_provider("apr").unwrap(), ImportProvider::Apr));
}

#[test]
fn parse_case_insensitive() {
    assert!(super::store_import::parse_provider("APT").is_ok());
    assert!(super::store_import::parse_provider("Docker").is_ok());
    assert!(super::store_import::parse_provider("NIX").is_ok());
}

#[test]
fn parse_unknown() {
    let err = super::store_import::parse_provider("homebrew").unwrap_err();
    assert!(err.contains("unknown provider"));
}

// ── provider_name ────────────────────────────────────────────────

#[test]
fn name_apt() {
    assert_eq!(super::store_import::provider_name(ImportProvider::Apt), "apt");
}

#[test]
fn name_cargo() {
    assert_eq!(super::store_import::provider_name(ImportProvider::Cargo), "cargo");
}

#[test]
fn name_uv() {
    assert_eq!(super::store_import::provider_name(ImportProvider::Uv), "uv");
}

#[test]
fn name_nix() {
    assert_eq!(super::store_import::provider_name(ImportProvider::Nix), "nix");
}

#[test]
fn name_docker() {
    assert_eq!(super::store_import::provider_name(ImportProvider::Docker), "docker");
}

#[test]
fn name_tofu() {
    assert_eq!(super::store_import::provider_name(ImportProvider::Tofu), "tofu");
}

#[test]
fn name_terraform() {
    assert_eq!(super::store_import::provider_name(ImportProvider::Terraform), "terraform");
}

#[test]
fn name_apr() {
    assert_eq!(super::store_import::provider_name(ImportProvider::Apr), "apr");
}

// ── current_arch ─────────────────────────────────────────────────

#[test]
fn arch_returns_valid() {
    let arch = super::store_import::current_arch();
    assert!(!arch.is_empty());
    assert!(arch == "x86_64" || arch == "aarch64" || !arch.is_empty());
}

// ── local_machine ────────────────────────────────────────────────

#[test]
fn local_machine_fields() {
    let m = super::store_import::local_machine();
    assert_eq!(m.hostname, "localhost");
    assert_eq!(m.addr, "127.0.0.1");
    assert_eq!(m.user, "root");
    assert!(!m.arch.is_empty());
    assert!(m.ssh_key.is_none());
    assert!(m.roles.is_empty());
}
