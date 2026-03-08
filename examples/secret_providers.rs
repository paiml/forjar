//! FJ-2300: Secret provider example.
//!
//! Demonstrates env and file-based secret providers with redaction.
//!
//! ```bash
//! FORJAR_SECRET_DB_PASSWORD=s3cret cargo run --example secret_providers
//! ```

use forjar::core::resolver::{redact_secrets, resolve_secret_with_provider};

fn main() {
    demo_env_provider();
    demo_file_provider();
    demo_redaction();
}

#[allow(clippy::disallowed_methods)]
fn demo_env_provider() {
    println!("=== FJ-2300: Env Secret Provider ===\n");

    std::env::set_var("FORJAR_SECRET_DB_PASSWORD", "s3cret_p4ssw0rd");

    match resolve_secret_with_provider("db_password", Some("env"), None) {
        Ok(val) => println!("  {{{{secrets.db_password}}}} → {val}"),
        Err(e) => println!("  Error: {e}"),
    }

    std::env::remove_var("FORJAR_SECRET_DB_PASSWORD");
    match resolve_secret_with_provider("db_password", Some("env"), None) {
        Ok(val) => println!("  After remove: {val}"),
        Err(e) => println!("  After remove: {e}\n"),
    }
}

fn demo_file_provider() {
    println!("=== FJ-2300: File Secret Provider ===\n");

    let dir = tempfile::tempdir().unwrap();
    let secret_file = dir.path().join("api_key");
    std::fs::write(&secret_file, "sk-live-abc123\n").unwrap();

    match resolve_secret_with_provider("api_key", Some("file"), dir.path().to_str()) {
        Ok(val) => println!("  {{{{secrets.api_key}}}} → {val}"),
        Err(e) => println!("  Error: {e}"),
    }

    match resolve_secret_with_provider("missing", Some("file"), dir.path().to_str()) {
        Ok(val) => println!("  missing: {val}"),
        Err(e) => println!("  missing: {e}\n"),
    }
}

fn demo_redaction() {
    println!("=== FJ-2300: Secret Redaction ===\n");

    let log = "Connecting to db with password s3cret_p4ssw0rd on host 10.0.0.1";
    let secrets = vec!["s3cret_p4ssw0rd".to_string()];
    let redacted = redact_secrets(log, &secrets);
    println!("  Original: {log}");
    println!("  Redacted: {redacted}");
}
