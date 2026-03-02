//! Demonstrates forjar's provider execution bridge:
//! generating import commands, staging, hashing, and meta.yaml writing.
//!
//! Run: `cargo run --example store_provider_import`

use forjar::core::store::provider::{
    all_providers, capture_method, import_command, origin_ref_string, validate_import,
    ImportConfig, ImportProvider,
};
use forjar::core::store::provider_exec::{build_staging_script, hash_staging_dir};
use std::collections::BTreeMap;
use std::path::PathBuf;

fn main() {
    println!("=== Forjar Store Provider Import Demo ===\n");
    demo_all_providers();
    demo_staging_script();
    demo_hash_staging();
    demo_validation();
    println!("\n=== All provider import demos passed ===");
}

fn demo_all_providers() {
    println!("--- 1. Provider CLI Commands ---");

    let configs = [
        ("apt", "curl", Some("7.88.1")),
        ("cargo", "ripgrep", Some("14.1.0")),
        ("uv", "requests", Some("2.31.0")),
        ("nix", "nixpkgs#ripgrep", None),
        ("docker", "alpine", Some("3.18")),
        ("tofu", "./infra", None),
        ("terraform", "./infra", None),
        ("apr", "mistral-7b", None),
    ];

    for (provider_str, reference, version) in configs {
        let provider = match provider_str {
            "apt" => ImportProvider::Apt,
            "cargo" => ImportProvider::Cargo,
            "uv" => ImportProvider::Uv,
            "nix" => ImportProvider::Nix,
            "docker" => ImportProvider::Docker,
            "tofu" => ImportProvider::Tofu,
            "terraform" => ImportProvider::Terraform,
            "apr" => ImportProvider::Apr,
            _ => unreachable!(),
        };

        let config = ImportConfig {
            provider,
            reference: reference.to_string(),
            version: version.map(|v| v.to_string()),
            arch: "x86_64".to_string(),
            options: BTreeMap::new(),
        };

        let cmd = import_command(&config);
        let origin = origin_ref_string(&config);
        let method = capture_method(provider);
        println!("  {provider_str:12} cmd: {cmd}");
        println!("  {provider_str:12} ref: {origin}");
        println!("  {provider_str:12} capture: {method}");
    }

    assert_eq!(all_providers().len(), 8);
    println!("  All 8 providers generate valid commands");
}

fn demo_staging_script() {
    println!("\n--- 2. Staging Script Generation ---");

    let staging = PathBuf::from("/tmp/forjar-staging/demo");
    let script = build_staging_script("apt-get install -y curl=7.88.1", &staging);
    println!("  Script:\n{}", indent(&script, "    "));
    assert!(script.contains("STAGING"));
    assert!(script.contains("mkdir -p"));
    println!("  Staging script includes $STAGING env and mkdir");
}

fn demo_hash_staging() {
    println!("\n--- 3. Staging Directory Hashing ---");

    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("binary"), b"ELF...fake binary").unwrap();
    std::fs::write(dir.path().join("config.yaml"), b"key: value").unwrap();
    let sub = dir.path().join("lib");
    std::fs::create_dir(&sub).unwrap();
    std::fs::write(sub.join("libfoo.so"), b"shared library").unwrap();

    let hash = hash_staging_dir(dir.path()).unwrap();
    println!("  Hash: {hash}");
    assert!(hash.starts_with("blake3:"));

    // Verify determinism
    let dir2 = tempfile::tempdir().unwrap();
    std::fs::write(dir2.path().join("binary"), b"ELF...fake binary").unwrap();
    std::fs::write(dir2.path().join("config.yaml"), b"key: value").unwrap();
    let sub2 = dir2.path().join("lib");
    std::fs::create_dir(&sub2).unwrap();
    std::fs::write(sub2.join("libfoo.so"), b"shared library").unwrap();

    let hash2 = hash_staging_dir(dir2.path()).unwrap();
    assert_eq!(hash, hash2);
    println!("  Determinism verified: identical content → identical hash");
}

fn demo_validation() {
    println!("\n--- 4. Import Validation ---");

    // Valid config
    let valid = ImportConfig {
        provider: ImportProvider::Apt,
        reference: "curl".to_string(),
        version: Some("7.88.1".to_string()),
        arch: "x86_64".to_string(),
        options: BTreeMap::new(),
    };
    assert!(validate_import(&valid).is_empty());
    println!("  Valid config: no errors");

    // Invalid: empty reference
    let invalid = ImportConfig {
        provider: ImportProvider::Apt,
        reference: String::new(),
        version: None,
        arch: "x86_64".to_string(),
        options: BTreeMap::new(),
    };
    let errors = validate_import(&invalid);
    assert!(!errors.is_empty());
    println!("  Empty reference: {} error(s)", errors.len());

    // Invalid: docker with spaces
    let bad_docker = ImportConfig {
        provider: ImportProvider::Docker,
        reference: "my image".to_string(),
        version: None,
        arch: "x86_64".to_string(),
        options: BTreeMap::new(),
    };
    let errors = validate_import(&bad_docker);
    assert!(errors.iter().any(|e| e.contains("spaces")));
    println!("  Docker spaces: caught");

    println!("  All validations verified");
}

fn indent(s: &str, prefix: &str) -> String {
    s.lines()
        .map(|line| format!("{prefix}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}
