//! Example: Namespace-isolated secret execution (FJ-3306)
//!
//! Demonstrates ephemeral secret injection into isolated child
//! processes with full audit trail.
//!
//! ```bash
//! cargo run --example secret_namespace
//! ```

use forjar::core::ephemeral::ResolvedEphemeral;
use forjar::core::secret_audit::{audit_summary, format_audit_summary, read_audit};
use forjar::core::secret_namespace::{
    build_isolated_env, execute_isolated, format_result, verify_no_leak, NamespaceConfig,
};
use tempfile::TempDir;

fn make_secret(key: &str, value: &str) -> ResolvedEphemeral {
    ResolvedEphemeral {
        key: key.into(),
        value: value.into(),
        hash: blake3::hash(value.as_bytes()).to_hex().to_string(),
    }
}

fn main() {
    println!("=== Namespace-Isolated Secret Execution (FJ-3306) ===\n");

    let dir = TempDir::new().unwrap();

    // 1. Configure namespace
    let config = NamespaceConfig {
        namespace_id: "ns-forjar-demo-001".into(),
        audit_enabled: true,
        state_dir: Some(dir.path().to_path_buf()),
        inherit_env: vec!["PATH".into()],
    };

    let secrets = vec![
        make_secret("DB_PASSWORD", "super-secret-db-pass"),
        make_secret("API_KEY", "prod-api-key-1234"),
    ];

    // 2. Show isolated environment
    println!("1. Building isolated environment:");
    let env = build_isolated_env(&config, &secrets);
    for (k, v) in &env {
        let display = if k == "DB_PASSWORD" || k == "API_KEY" {
            format!("{}...", &v[..8])
        } else if k == "PATH" {
            format!("{}...", &v[..20])
        } else {
            v.clone()
        };
        println!("   {k}={display}");
    }

    // 3. Execute command with secrets
    println!("\n2. Executing script with injected secrets:");
    let script = r#"
echo "DB connection: postgresql://app:${DB_PASSWORD}@db:5432/prod"
echo "API configured: key=$(echo $API_KEY | cut -c1-4)****"
echo "Namespace: $FORJAR_NAMESPACE"
echo "HOME is: ${HOME:-<not set>}"
    "#;

    let result = execute_isolated(&config, &secrets, "sh", &["-c", script]).unwrap();
    println!("{}", format_result(&result));
    println!("   stdout:");
    for line in result.stdout.lines() {
        println!("     {line}");
    }

    // 4. Verify no leak
    println!("\n3. Leak verification:");
    let no_leak_db = verify_no_leak("DB_PASSWORD");
    let no_leak_api = verify_no_leak("API_KEY");
    println!("   DB_PASSWORD not in parent env: {no_leak_db}");
    println!("   API_KEY not in parent env: {no_leak_api}");

    // 5. Show audit trail
    println!("\n4. Audit trail:");
    let events = read_audit(dir.path()).unwrap();
    for event in &events {
        println!(
            "   [{:>8}] key={} hash={}...",
            event.event_type,
            event.key,
            &event.value_hash[..16]
        );
    }

    let summary = audit_summary(&events);
    println!("\n{}", format_audit_summary(&summary));

    println!("\nDone.");
}
