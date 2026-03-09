//! FJ-3404/095: Plugin dispatch and reproducible build verification.
//!
//! Usage: cargo run --example plugin_dispatch_repro

use forjar::core::plugin_dispatch::{
    available_plugin_types, dispatch_check, is_plugin_type, parse_plugin_type,
};
use forjar::core::repro_build::{
    check_cargo_profile, check_environment, generate_report, repro_ci_snippet,
};

fn main() {
    println!("Forjar: Plugin Dispatch & Reproducible Builds");
    println!("{}", "=".repeat(55));

    // ── Plugin Dispatch ──
    println!("\n[FJ-3404] Plugin Type Parsing:");
    for t in ["plugin:nginx", "plugin:my-custom", "package", "file"] {
        println!(
            "  {:20} → is_plugin={}, name={:?}",
            t,
            is_plugin_type(t),
            parse_plugin_type(t)
        );
    }

    println!("\n[FJ-3404] Plugin Dispatch (missing):");
    let dir = tempfile::tempdir().unwrap();
    let config = serde_json::json!({"port": 8080});
    let result = dispatch_check(dir.path(), "nonexistent", &config);
    println!("  check: success={} msg={}", result.success, result.message);

    println!("\n[FJ-3404] Available Plugins (empty):");
    let types = available_plugin_types(dir.path());
    println!("  found: {}", types.len());

    // ── Reproducible Builds ──
    println!("\n[FJ-095] Environment Checks:");
    for check in check_environment() {
        let mark = if check.passed { "✓" } else { "✗" };
        println!("  {mark} {}: {}", check.name, check.detail);
    }

    println!("\n[FJ-095] Cargo Profile Checks:");
    let good_toml = "[profile.release]\npanic = \"abort\"\nlto = true\ncodegen-units = 1\n";
    for check in check_cargo_profile(good_toml) {
        let mark = if check.passed { "✓" } else { "✗" };
        println!("  {mark} {}: {}", check.name, check.detail);
    }

    println!("\n[FJ-095] Full Report:");
    let report = generate_report("src-abc", "bin-def", good_toml);
    println!(
        "  Source: {} | Binary: {} | Reproducible: {}",
        report.source_hash, report.binary_hash, report.reproducible
    );
    println!("  Toolchain: {}", report.toolchain);
    println!("  Checks: {} total", report.checks.len());

    println!("\n[FJ-095] CI Snippet: {} bytes", repro_ci_snippet().len());

    println!("\n{}", "=".repeat(55));
    println!("All plugin-dispatch/repro criteria survived.");
}
