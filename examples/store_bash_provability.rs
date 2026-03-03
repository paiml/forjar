//! Bash provability (I8 invariant) — validate and purify shell scripts.
//!
//! Demonstrates the bashrs integration that ensures no raw shell reaches
//! the transport layer without validation or purification.
//!
//! Run: `cargo run --example store_bash_provability`

use forjar::core::purifier::{
    lint_error_count, lint_script, purify_script, validate_or_purify, validate_script,
};

fn main() {
    println!("=== Bash Provability (I8 Invariant) Demo ===\n");
    demo_validate_valid();
    demo_validate_pipeline();
    demo_lint_diagnostics();
    demo_purify_script();
    demo_validate_or_purify();
    demo_provider_scripts();
    demo_error_count();
    println!("\n=== All bash provability demos passed ===");
}

/// 1. validate_script() accepts valid shell.
fn demo_validate_valid() {
    println!("--- 1. Validate Valid Scripts ---");

    let scripts = [
        ("simple echo", "echo hello world"),
        ("variable", "NAME=forjar; echo $NAME"),
        ("conditional", "if [ -f /etc/hosts ]; then echo ok; fi"),
        ("pipeline", "cat file.txt | grep pattern | wc -l"),
        ("multi-command", "echo a && echo b || echo c"),
        ("subshell", "result=$(echo hello)"),
        ("for loop", "for f in *.txt; do echo $f; done"),
    ];

    for (label, script) in &scripts {
        let result = validate_script(script);
        assert!(result.is_ok(), "{label} must pass: {result:?}");
        println!("  {label}: PASS");
    }
    println!("  All valid scripts accepted\n");
}

/// 2. validate_script() in provider import pipeline.
fn demo_validate_pipeline() {
    println!("--- 2. Provider Import Pipeline ---");

    // Simulated provider commands (these are what provider_exec generates)
    let provider_cmds = [
        (
            "apt",
            "apt-get install -y --no-install-recommends curl=7.88.1",
        ),
        (
            "cargo",
            "cargo install ripgrep --version 14.1.0 --root $STAGING",
        ),
        (
            "nix",
            "nix build nixpkgs#ripgrep --out-link $STAGING/result",
        ),
        ("docker", "docker pull alpine:3.18"),
        ("uv", "uv pip install flask==3.0.2 --target $STAGING"),
    ];

    for (provider, cmd) in &provider_cmds {
        let result = validate_script(cmd);
        assert!(result.is_ok(), "{provider} command must validate");
        println!("  {provider}: validated OK");
    }
    println!("  All provider commands pass I8 gate\n");
}

/// 3. lint_script() returns structured diagnostics.
fn demo_lint_diagnostics() {
    println!("--- 3. Lint Diagnostics ---");

    let result = lint_script("echo hello");
    let error_count = result
        .diagnostics
        .iter()
        .filter(|d| d.severity == bashrs::linter::Severity::Error)
        .count();
    assert_eq!(error_count, 0);
    println!(
        "  'echo hello': {} diagnostics, {} errors",
        result.diagnostics.len(),
        error_count
    );

    // Script with warnings (not errors)
    let result = lint_script("#!/bin/bash\necho hello");
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.severity == bashrs::linter::Severity::Error)
        .collect();
    assert!(errors.is_empty(), "valid script must have 0 errors");
    println!(
        "  '#!/bin/bash\\necho hello': {} total, {} errors",
        result.diagnostics.len(),
        errors.len()
    );
    println!("  Lint diagnostics verified\n");
}

/// 4. purify_script() cleans and reformats shell.
fn demo_purify_script() {
    println!("--- 4. Purify Script ---");

    let scripts = [
        "echo hello world",
        "echo a; echo b",
        "mkdir -p /tmp/build && cd /tmp/build",
    ];

    for script in &scripts {
        let result = purify_script(script);
        assert!(result.is_ok(), "purify must succeed for: {script}");
        let purified = result.unwrap();
        println!("  Input:    {script}");
        println!(
            "  Purified: {}",
            purified.lines().next().unwrap_or(&purified)
        );
    }
    println!("  Purification pipeline verified\n");
}

/// 5. validate_or_purify() fast path and fallback.
fn demo_validate_or_purify() {
    println!("--- 5. Validate-or-Purify ---");

    // Fast path: valid script returned unchanged
    let script = "echo test";
    let result = validate_or_purify(script).unwrap();
    assert_eq!(result, script);
    println!("  Valid script: returned unchanged (fast path)");

    // Purifiable script
    let script = "echo hello; echo world";
    let result = validate_or_purify(script);
    assert!(result.is_ok(), "purifiable script must succeed");
    println!("  Multi-command: purified OK");
    println!("  Validate-or-purify verified\n");
}

/// 6. Provider-generated scripts pass I8.
fn demo_provider_scripts() {
    println!("--- 6. Provider Script Validation ---");

    // Sandbox build scripts (what derivation_exec generates)
    let sandbox_scripts = [
        "cp -r /inputs/base /out/base",
        "make && make install",
        "tar xzf /inputs/archive.tar.gz -C /out",
    ];

    for script in &sandbox_scripts {
        let result = validate_script(script);
        assert!(result.is_ok(), "sandbox script must validate: {script}");
    }
    println!("  3 sandbox build scripts: all validated");

    // GC commands
    let gc_scripts = [
        "rm -rf /var/forjar/store/deadbeef",
        "du -sb /var/forjar/store/abc123",
    ];

    for script in &gc_scripts {
        let result = validate_script(script);
        assert!(result.is_ok(), "GC script must validate: {script}");
    }
    println!("  2 GC scripts: all validated");

    // Cache transport
    let cache_scripts = [
        "rsync -az /var/forjar/store/abc user@cache:/store/",
        "b3sum /var/forjar/store/abc/content/*",
    ];

    for script in &cache_scripts {
        let result = validate_script(script);
        assert!(result.is_ok(), "cache script must validate: {script}");
    }
    println!("  2 cache scripts: all validated");
    println!("  All execution bridge scripts pass I8\n");
}

/// 7. lint_error_count() quick check.
fn demo_error_count() {
    println!("--- 7. Error Count ---");

    assert_eq!(lint_error_count("echo hello"), 0);
    println!("  'echo hello': 0 errors");

    assert_eq!(lint_error_count("#!/bin/sh\necho ok"), 0);
    println!("  '#!/bin/sh\\necho ok': 0 errors");

    // Complex script
    let complex = r#"
set -euo pipefail
STAGING=$(mktemp -d)
trap 'rm -rf $STAGING' EXIT
echo "building in $STAGING"
"#;
    assert_eq!(lint_error_count(complex), 0);
    println!("  Complex script: 0 errors");
    println!("  Error count verified\n");
}
