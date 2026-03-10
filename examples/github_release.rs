//! GitHub Release resource — install binaries from GitHub Releases.
//!
//! Demonstrates the `github_release` resource type: validate config,
//! plan installation, check current state, and generate apply scripts.
//!
//! Usage: `cargo run --example github_release`

use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    eprintln!("--- GitHub Release Resource ---\n");

    let tmp = std::env::temp_dir().join("forjar-example-gh-release");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).ok();

    let config_path = tmp.join("forjar.yaml");
    std::fs::write(
        &config_path,
        r#"version: "1.0"
name: github-release-demo
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
    user: demo
resources:
  install-bat:
    type: github_release
    machine: local
    repo: sharkdp/bat
    tag: v0.24.0
    asset_pattern: "*x86_64-unknown-linux-gnu*"
    binary: bat
    install_dir: /tmp/fj034-demo/bin
  install-fd:
    type: github_release
    machine: local
    repo: sharkdp/fd
    tag: v10.2.0
    asset_pattern: "*x86_64-unknown-linux-gnu*"
    binary: fd
    install_dir: /tmp/fj034-demo/bin
"#,
    )
    .ok();

    let mut failures = 0u32;

    // Step 1: Validate
    eprintln!("Step 1: Validate config");
    let r = run_forjar(&["validate", "-f", &config_path.display().to_string()]);
    if r.success {
        eprintln!("  OK: {}", first_line(&r.output));
    } else {
        eprintln!("  FAIL: {}", first_line(&r.output));
        failures += 1;
    }

    // Step 2: Plan
    eprintln!("\nStep 2: Plan");
    let r = run_forjar(&["plan", "-f", &config_path.display().to_string()]);
    if r.success {
        eprintln!("  OK: plan generated");
        for line in r.output.lines().take(6) {
            if !line.trim().is_empty() {
                eprintln!("  {line}");
            }
        }
    } else {
        eprintln!("  FAIL: {}", first_line(&r.output));
        failures += 1;
    }

    // Step 3: Check (JSON)
    eprintln!("\nStep 3: Check status (JSON)");
    let r = run_forjar(&["check", "-f", &config_path.display().to_string(), "--json"]);
    if r.success {
        eprintln!("  OK: check completed");
        for line in r.output.lines().take(5) {
            eprintln!("  {line}");
        }
    } else {
        eprintln!("  FAIL: {}", first_line(&r.output));
        failures += 1;
    }

    // Step 4: Validate absent state
    eprintln!("\nStep 4: Validate absent state");
    let absent_path = tmp.join("absent.yaml");
    std::fs::write(
        &absent_path,
        r#"version: "1.0"
name: github-release-absent
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
    user: demo
resources:
  remove-bat:
    type: github_release
    machine: local
    repo: sharkdp/bat
    binary: bat
    state: absent
"#,
    )
    .ok();
    let r = run_forjar(&["validate", "-f", &absent_path.display().to_string()]);
    if r.success {
        eprintln!("  OK: absent config valid");
    } else {
        eprintln!("  FAIL: {}", first_line(&r.output));
        failures += 1;
    }

    let _ = std::fs::remove_dir_all(&tmp);
    eprintln!("\n--- Result: {failures} failure(s) ---");
    if failures > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

struct StepResult {
    success: bool,
    output: String,
}

fn run_forjar(args: &[&str]) -> StepResult {
    match Command::new("forjar").args(args).output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            StepResult {
                success: output.status.success(),
                output: format!("{stdout}{stderr}"),
            }
        }
        Err(e) => StepResult {
            success: false,
            output: format!("failed to execute forjar: {e}"),
        },
    }
}

fn first_line(s: &str) -> &str {
    s.lines().next().unwrap_or(s).trim()
}
