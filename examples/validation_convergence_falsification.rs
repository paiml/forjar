//! FJ-2501/2504/2600/2603: Validation, LSP, convergence, and sandbox falsification.
//!
//! Demonstrates Popperian rejection criteria for:
//! - Format validation (mode, port, path, owner, cron)
//! - LSP diagnostics (YAML validation, unknown ensure values)
//! - Convergence testing (result model, summary aggregation)
//! - Sandbox isolation (level properties, config validation, presets)
//!
//! Usage: cargo run --example validation_convergence_falsification

use forjar::cli::lsp::{validate_yaml, DiagnosticSeverity, LspServer};
use forjar::core::parser::{check_unknown_fields, parse_config, validate_config};
use forjar::core::store::convergence_runner::{ConvergenceResult, ConvergenceSummary};
use forjar::core::store::sandbox::{
    blocks_network, enforces_fs_isolation, preset_profile, SandboxLevel,
};

fn main() {
    println!("Forjar Validation, Convergence & Sandbox Falsification");
    println!("{}", "=".repeat(58));

    // ── FJ-2501: Format Validation ──
    println!("\n[FJ-2501] Format Validation:");

    let good_yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: m
    path: /etc/app.conf
    mode: "0644"
    owner: www-data
    content: "key: value"
"#;
    let config = parse_config(good_yaml).unwrap();
    let errors = validate_config(&config);
    let format_ok = !errors.iter().any(|e| {
        e.message.contains("mode")
            || e.message.contains("port")
            || e.message.contains("path")
            || e.message.contains("owner")
    });
    println!(
        "  Valid mode/path/owner accepted: {} {}",
        if format_ok { "yes" } else { "no" },
        if format_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(format_ok);

    let bad_yaml = r#"
version: "1.0"
name: test
resources:
  cfg:
    type: file
    path: /etc/app.conf
    mode: "0999"
    content: "x"
"#;
    let bad_config = parse_config(bad_yaml).unwrap();
    let bad_errors = validate_config(&bad_config);
    let reject_ok = bad_errors.iter().any(|e| e.message.contains("mode"));
    println!(
        "  Invalid mode '0999' rejected: {} {}",
        if reject_ok { "yes" } else { "no" },
        if reject_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(reject_ok);

    // ── FJ-2500: Unknown Fields ──
    println!("\n[FJ-2500] Unknown Field Detection:");

    let unknown_yaml = r#"
version: "1.0"
name: test
resources:
  pkg:
    type: package
    packages: [curl]
    bogus_field: true
"#;
    let warnings = check_unknown_fields(unknown_yaml);
    let unk_ok = !warnings.is_empty();
    println!(
        "  Unknown field 'bogus_field' flagged: {} {}",
        if unk_ok { "yes" } else { "no" },
        if unk_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(unk_ok);

    // ── FJ-2504: LSP Diagnostics ──
    println!("\n[FJ-2504] LSP Diagnostics:");

    let diags = validate_yaml("{ invalid: [yaml");
    let parse_err_ok = !diags.is_empty() && diags[0].severity == DiagnosticSeverity::Error;
    println!(
        "  Invalid YAML → Error diagnostic: {} {}",
        if parse_err_ok { "yes" } else { "no" },
        if parse_err_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(parse_err_ok);

    let mut server = LspServer::new();
    let init = serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "rootUri": "file:///tmp" }
    });
    let resp = server.handle_message(&init);
    let init_ok = resp.is_some() && server.initialized;
    println!(
        "  LSP initialize handshake: {} {}",
        if init_ok { "ok" } else { "failed" },
        if init_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(init_ok);

    // ── FJ-2600: Convergence Model ──
    println!("\n[FJ-2600] Convergence Testing Model:");

    let pass_result = ConvergenceResult {
        resource_id: "nginx".into(),
        resource_type: "package".into(),
        converged: true,
        idempotent: true,
        preserved: true,
        duration_ms: 50,
        error: None,
    };
    let fail_result = ConvergenceResult {
        resource_id: "broken".into(),
        resource_type: "file".into(),
        converged: true,
        idempotent: false,
        preserved: true,
        duration_ms: 30,
        error: None,
    };
    let pass_ok = pass_result.passed() && !fail_result.passed();
    println!(
        "  Pass/fail detection: {} {}",
        if pass_ok { "correct" } else { "wrong" },
        if pass_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(pass_ok);

    let summary = ConvergenceSummary::from_results(&[pass_result, fail_result]);
    let sum_ok = summary.total == 2 && summary.passed == 1 && summary.idempotency_failures == 1;
    println!(
        "  Summary: {}/{} passed, {} idem failures  {} {}",
        summary.passed,
        summary.total,
        summary.idempotency_failures,
        if sum_ok { "✓" } else { "✗" },
        if sum_ok { "" } else { "FALSIFIED" }
    );
    assert!(sum_ok);

    // ── FJ-2603: Sandbox Properties ──
    println!("\n[FJ-2603] Sandbox Isolation:");

    let net_ok = blocks_network(SandboxLevel::Full)
        && !blocks_network(SandboxLevel::NetworkOnly)
        && !blocks_network(SandboxLevel::Minimal);
    println!(
        "  Full blocks network, others don't: {} {}",
        if net_ok { "yes" } else { "no" },
        if net_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(net_ok);

    let fs_ok = enforces_fs_isolation(SandboxLevel::Full)
        && enforces_fs_isolation(SandboxLevel::NetworkOnly)
        && enforces_fs_isolation(SandboxLevel::Minimal)
        && !enforces_fs_isolation(SandboxLevel::None);
    println!(
        "  FS isolation (Full/Network/Minimal yes, None no): {} {}",
        if fs_ok { "yes" } else { "no" },
        if fs_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(fs_ok);

    let presets = ["full", "network-only", "minimal", "gpu"];
    let preset_ok = presets.iter().all(|p| preset_profile(p).is_some())
        && preset_profile("nonexistent").is_none();
    println!(
        "  All 4 presets exist, unknown returns None: {} {}",
        if preset_ok { "yes" } else { "no" },
        if preset_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(preset_ok);

    println!("\n{}", "=".repeat(58));
    println!("All validation, convergence & sandbox criteria survived.");
}
