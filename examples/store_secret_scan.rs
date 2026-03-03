//! Secret scanning framework — detect plaintext secrets in configs.
//!
//! Demonstrates the 15-pattern secret scanner that enforces age
//! encryption for all sensitive values in forjar configurations.
//!
//! Run: `cargo run --example store_secret_scan`

use forjar::core::store::secret_scan::{is_encrypted, scan_text, scan_yaml_str};

fn main() {
    println!("=== Secret Scanning Demo ===\n");
    demo_clean_text();
    demo_aws_keys();
    demo_pem_keys();
    demo_github_tokens();
    demo_generic_secrets();
    demo_age_encrypted_bypass();
    demo_yaml_scanning();
    demo_multiple_findings();
    println!("\n=== All secret scan demos passed ===");
}

/// 1. Clean text produces no findings.
fn demo_clean_text() {
    println!("--- 1. Clean Text ---");
    let findings = scan_text("This is perfectly clean config text.");
    assert!(findings.is_empty());
    println!("  Clean text: 0 findings");

    let findings = scan_text("version: 1.24.0");
    assert!(findings.is_empty());
    println!("  Version string: 0 findings");

    let findings = scan_text("echo hello && echo world");
    assert!(findings.is_empty());
    println!("  Shell commands: 0 findings");
    println!("  Clean text verified\n");
}

/// 2. AWS access key detection.
fn demo_aws_keys() {
    println!("--- 2. AWS Key Detection ---");

    // AWS access key (AKIA prefix + 16 alphanumeric)
    let findings = scan_text("key=AKIAIOSFODNN7EXAMPLE");
    assert!(!findings.is_empty());
    let (name, redacted) = &findings[0];
    assert_eq!(name, "aws_access_key");
    println!("  AWS access key: detected ({redacted})");

    // AWS secret key
    let findings = scan_text("aws_secret_access_key = wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY");
    assert!(!findings.is_empty());
    println!("  AWS secret key: detected ({})", findings[0].1);
    println!("  AWS key detection verified\n");
}

/// 3. PEM private key header detection.
fn demo_pem_keys() {
    println!("--- 3. PEM Private Key Detection ---");

    let patterns = [
        ("RSA", "-----BEGIN RSA PRIVATE KEY-----"),
        ("EC", "-----BEGIN EC PRIVATE KEY-----"),
        ("DSA", "-----BEGIN DSA PRIVATE KEY-----"),
        ("OpenSSH", "-----BEGIN OPENSSH PRIVATE KEY-----"),
    ];

    for (label, header) in &patterns {
        let findings = scan_text(header);
        assert!(!findings.is_empty(), "{label} key must be detected");
        assert_eq!(findings[0].0, "private_key_pem");
        println!("  {label} private key: detected");
    }

    // Public key should NOT be flagged
    let findings = scan_text("-----BEGIN PUBLIC KEY-----");
    let pem_findings: Vec<_> = findings
        .iter()
        .filter(|(name, _)| name == "private_key_pem")
        .collect();
    assert!(pem_findings.is_empty());
    println!("  Public key: not flagged (correct)");
    println!("  PEM detection verified\n");
}

/// 4. GitHub token detection.
fn demo_github_tokens() {
    println!("--- 4. GitHub Token Detection ---");

    let pat = format!("ghp_{}", "A".repeat(40));
    let findings = scan_text(&pat);
    assert!(!findings.is_empty());
    assert_eq!(findings[0].0, "github_token");
    println!("  GitHub PAT (ghp_): detected");

    let secret = format!("ghs_{}", "B".repeat(40));
    let findings = scan_text(&secret);
    assert!(!findings.is_empty());
    println!("  GitHub secret (ghs_): detected");
    println!("  GitHub token detection verified\n");
}

/// 5. Generic secret patterns.
fn demo_generic_secrets() {
    println!("--- 5. Generic Secret Patterns ---");

    // JWT token
    let jwt = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0";
    let findings = scan_text(jwt);
    let jwt_hit = findings.iter().any(|(n, _)| n == "jwt_token");
    assert!(jwt_hit, "JWT must be detected");
    println!("  JWT token: detected");

    // Stripe key (constructed at runtime to avoid GitHub push protection)
    let stripe = format!("sk_live_{}", "a".repeat(24));
    let findings = scan_text(&stripe);
    let stripe_hit = findings.iter().any(|(n, _)| n == "stripe_key");
    assert!(stripe_hit, "Stripe key must be detected");
    println!("  Stripe key: detected");

    // Database URL with password
    let db_url = "postgres://admin:secretpass123@db.example.com/mydb";
    let findings = scan_text(db_url);
    let db_hit = findings.iter().any(|(n, _)| n == "database_url_pass");
    assert!(db_hit, "DB URL with password must be detected");
    println!("  Database URL (password): detected");

    // SSH password
    let findings = scan_text("sshpass -p mysecretpass ssh user@host");
    let ssh_hit = findings.iter().any(|(n, _)| n == "ssh_password");
    assert!(ssh_hit, "sshpass must be detected");
    println!("  sshpass: detected");
    println!("  Generic pattern detection verified\n");
}

/// 6. Age-encrypted values bypass scanning.
fn demo_age_encrypted_bypass() {
    println!("--- 6. Age Encryption Bypass ---");

    assert!(!is_encrypted("plain text secret"));
    println!("  Plain text: not encrypted");

    assert!(is_encrypted("ENC[age,abc123...]"));
    println!("  ENC[age,...]: encrypted");

    // Encrypted value with embedded secret pattern should be skipped
    let encrypted = "ENC[age,AKIAIOSFODNN7EXAMPLE]";
    assert!(is_encrypted(encrypted));
    let findings = scan_text(encrypted);
    assert!(findings.is_empty(), "encrypted values must be skipped");
    println!("  Encrypted AWS key: 0 findings (bypassed)");
    println!("  Age encryption bypass verified\n");
}

/// 7. Full YAML config scanning.
fn demo_yaml_scanning() {
    println!("--- 7. YAML Config Scanning ---");

    // Clean config
    let clean_yaml = r#"
name: my-app
version: "1.0.0"
resources:
  nginx:
    type: package
    packages: [nginx]
    version: "1.24.0"
"#;
    let result = scan_yaml_str(clean_yaml);
    assert!(result.clean);
    assert!(result.findings.is_empty());
    println!(
        "  Clean YAML: {} fields scanned, 0 findings",
        result.scanned_fields
    );

    // Config with secret
    let dirty_yaml = r#"
name: my-app
params:
  db_password: "password = supersecretvalue123"
  api_key: "api_key = abcdefghijklmnopqrstuvwxyz123456"
"#;
    let result = scan_yaml_str(dirty_yaml);
    assert!(!result.clean);
    assert!(!result.findings.is_empty());
    for f in &result.findings {
        println!(
            "  Finding: [{}] {} at {}",
            f.pattern_name, f.matched_text, f.location
        );
    }
    println!(
        "  Dirty YAML: {} fields, {} findings",
        result.scanned_fields,
        result.findings.len()
    );

    // Config with encrypted value
    let safe_yaml = r#"
params:
  db_password: "ENC[age,encrypted-secret-here]"
"#;
    let result = scan_yaml_str(safe_yaml);
    assert!(result.clean);
    println!("  Encrypted YAML: 0 findings (safe)");
    println!("  YAML scanning verified\n");
}

/// 8. Multiple secrets in one text.
fn demo_multiple_findings() {
    println!("--- 8. Multiple Findings ---");

    let text = format!("key1=AKIAIOSFODNN7EXAMPLE key2=ghp_{}", "A".repeat(40));
    let findings = scan_text(&text);
    assert!(
        findings.len() >= 2,
        "must detect at least 2 secrets, got {}",
        findings.len()
    );
    for (name, redacted) in &findings {
        println!("  [{name}] {redacted}");
    }
    println!("  {} findings in single text", findings.len());
    println!("  Multiple findings verified\n");
}
