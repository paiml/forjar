//! Example: Script secret leakage detection (FJ-3307)
//!
//! Demonstrates scanning shell scripts for secret leakage patterns
//! before they're deployed to machines.
//!
//! ```bash
//! cargo run --example script_secret_lint
//! ```

use forjar::core::script_secret_lint;

fn main() {
    println!("=== Script Secret Leakage Detection (FJ-3307) ===\n");

    // Clean script
    println!("1. Clean script:");
    let clean = r#"#!/bin/bash
set -euo pipefail
apt-get update -y
apt-get install -y nginx
systemctl enable nginx
systemctl start nginx
echo "nginx installed successfully"
"#;
    let result = script_secret_lint::scan_script(clean);
    println!("  Lines scanned: {}", result.lines_scanned);
    println!("  Clean: {}", result.clean());

    // Script with various leaks
    println!("\n2. Script with secret leaks:");
    let leaky = r#"#!/bin/bash
# This script has intentional security issues for demonstration

echo $PASSWORD
printf '%s' "${SECRET}" > config.yml
curl -u admin:hunter2 https://api.example.com
sshpass -p mypassword ssh user@host
export TOKEN=ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghij
DATABASE_URL=postgres://app:s3cret@db.internal:5432/prod
"#;
    let result = script_secret_lint::scan_script(leaky);
    println!("  Lines scanned: {}", result.lines_scanned);
    println!("  Findings: {}", result.findings.len());
    for f in &result.findings {
        println!(
            "    Line {}: [{}] {}",
            f.line, f.pattern_name, f.matched_text
        );
    }

    // Validate function
    println!("\n3. Validate (pass/fail):");
    match script_secret_lint::validate_no_leaks(clean) {
        Ok(()) => println!("  Clean script: PASS"),
        Err(e) => println!("  Clean script: FAIL — {e}"),
    }
    match script_secret_lint::validate_no_leaks(leaky) {
        Ok(()) => println!("  Leaky script: PASS (unexpected)"),
        Err(e) => {
            let lines: Vec<&str> = e.lines().collect();
            println!("  Leaky script: FAIL ({} findings)", lines.len() - 1);
        }
    }

    // Comments are skipped
    println!("\n4. Comments skipped:");
    let commented = "# echo $PASSWORD\n# sshpass -p secret ssh host\n";
    let result = script_secret_lint::scan_script(commented);
    println!("  Commented-out secrets: Clean={}", result.clean());

    println!("\nDone.");
}
