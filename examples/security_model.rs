//! FJ-2300: Security model — secret providers, path policy, authorization.
//!
//! ```bash
//! cargo run --example security_model
//! ```

use forjar::core::types::{
    AuthzResult, PathPolicy, SecretConfig, SecretProvider, SecretRef, SecretScanFinding,
    SecretScanResult,
};

fn main() {
    // Secret providers
    println!("=== Secret Providers ===");
    for provider in [
        SecretProvider::Env,
        SecretProvider::File,
        SecretProvider::Sops,
        SecretProvider::Op,
    ] {
        println!("  {provider}");
    }
    println!();

    // Secret config
    let config = SecretConfig {
        provider: SecretProvider::Sops,
        path: None,
        file: Some("secrets.enc.yaml".into()),
    };
    println!("=== Secret Config ===");
    println!("  Provider: {}", config.provider);
    println!("  File: {}", config.file.as_deref().unwrap_or("none"));
    println!();

    // Secret references found in config
    let refs = vec![
        SecretRef {
            name: "db_password".into(),
            template: "{{ secrets.db_password }}".into(),
            resource_id: "db-config".into(),
            field: "content".into(),
        },
        SecretRef {
            name: "api_key".into(),
            template: "{{ secrets.api_key }}".into(),
            resource_id: "app-config".into(),
            field: "content".into(),
        },
    ];
    println!("=== Secret References ===");
    for r in &refs {
        println!(
            "  {} -> {} in {}.{}",
            r.template, r.name, r.resource_id, r.field
        );
    }
    println!();

    // Path policy
    let policy = PathPolicy {
        deny_paths: vec![
            "/etc/shadow".into(),
            "/etc/sudoers".into(),
            "/etc/sudoers.d/*".into(),
            "/root/.ssh/authorized_keys".into(),
        ],
    };
    println!("=== Path Policy ===");
    let test_paths = [
        "/etc/shadow",
        "/etc/sudoers.d/custom",
        "/etc/nginx/nginx.conf",
        "/root/.ssh/authorized_keys",
        "/home/user/.bashrc",
    ];
    for path in test_paths {
        let status = if policy.is_denied(path) {
            "DENIED"
        } else {
            "allowed"
        };
        println!("  {path}: {status}");
    }
    println!();

    // Authorization
    println!("=== Authorization ===");
    let results = vec![
        AuthzResult::Allowed,
        AuthzResult::Denied {
            operator: "eve".into(),
            machine: "production-db".into(),
        },
    ];
    for r in &results {
        println!("  {r}");
    }
    println!();

    // Secret scan
    let scan = SecretScanResult::from_findings(
        vec![SecretScanFinding {
            resource_id: "db-config".into(),
            field: "content".into(),
            pattern: "password:".into(),
            preview: "password: s3cr***".into(),
        }],
        15,
    );
    println!("=== Secret Scan ===");
    println!(
        "  Scanned: {} fields, Clean: {}, Findings: {}",
        scan.scanned_fields,
        scan.clean,
        scan.findings.len()
    );
    for f in &scan.findings {
        println!("  {f}");
    }
}
