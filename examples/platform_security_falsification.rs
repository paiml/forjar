#![allow(clippy::field_reassign_with_default)]
//! FJ-2101/2300/2301/2604: Security, container build, and observability falsification.
//!
//! Runnable example demonstrating Popperian rejection criteria for:
//! - OCI layer building determinism and dual-digest correctness
//! - Security scanner detecting IaC security smells
//! - Path deny policy enforcement
//! - Mutation testing score model
//! - Observability log truncation and verbosity levels
//!
//! Usage: cargo run --example platform_security_falsification

use forjar::core::security_scanner::{scan, severity_counts};
use forjar::core::store::layer_builder::{build_layer, compute_dual_digest, LayerEntry};
use forjar::core::types::{
    ForjarConfig, LogTruncation, MutationScore, OciLayerConfig, PathPolicy, Resource, ResourceType,
    VerbosityLevel,
};

fn main() {
    println!("Forjar Platform Falsification — Security, OCI, Observability");
    println!("{}", "=".repeat(62));

    // ── FJ-2102: OCI Layer Determinism ──
    println!("\n[FJ-2102] OCI Layer Determinism:");

    let entries = vec![
        LayerEntry::dir("etc/", 0o755),
        LayerEntry::file("etc/config.yaml", b"key: value\n", 0o644),
    ];
    let config = OciLayerConfig::default();
    let (r1, d1) = build_layer(&entries, &config).unwrap();
    let (r2, d2) = build_layer(&entries, &config).unwrap();
    let det_ok = r1.digest == r2.digest && d1 == d2;
    println!(
        "  Same inputs → same digest: {} {}",
        r1.digest,
        if det_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(det_ok);

    let e2 = vec![LayerEntry::file("etc/config.yaml", b"different\n", 0o644)];
    let (r3, _) = build_layer(&e2, &config).unwrap();
    let diff_ok = r1.digest != r3.digest;
    println!(
        "  Different content → different digest: {} {}",
        if diff_ok { "true" } else { "false" },
        if diff_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(diff_ok);

    // ── FJ-2101: Dual Digest ──
    println!("\n[FJ-2101] Dual Digest (BLAKE3 + SHA-256):");
    let dd = compute_dual_digest(b"content for hashing");
    println!("  blake3: {}...", &dd.blake3[..16]);
    println!("  sha256: {}...", &dd.sha256[..16]);
    println!("  OCI:    {}", dd.oci_digest());
    println!("  Forjar: {}", dd.forjar_digest());
    assert!(dd.oci_digest().starts_with("sha256:"));
    assert!(dd.forjar_digest().starts_with("blake3:"));
    println!("  Format check: ✓");

    // ── FJ-2300: Security Scanner ──
    println!("\n[FJ-2300] Security Scanner:");

    let mut config_secrets = ForjarConfig::default();
    let mut r = Resource::default();
    r.resource_type = ResourceType::File;
    r.content = Some("password=mysecret123".into());
    r.mode = Some("0777".into());
    config_secrets.resources.insert("bad".into(), r);

    let findings = scan(&config_secrets);
    let (crit, high, med, low) = severity_counts(&findings);
    println!(
        "  Hardcoded secret + 0777: {} findings (C:{crit} H:{high} M:{med} L:{low})",
        findings.len()
    );
    let has_ss1 = findings.iter().any(|f| f.rule_id == "SS-1");
    let has_ss3 = findings.iter().any(|f| f.rule_id == "SS-3");
    println!(
        "  SS-1 (hardcoded secret):  {} {}",
        if has_ss1 { "detected" } else { "missed" },
        if has_ss1 { "✓" } else { "✗ FALSIFIED" }
    );
    println!(
        "  SS-3 (world-accessible):  {} {}",
        if has_ss3 { "detected" } else { "missed" },
        if has_ss3 { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(has_ss1);
    assert!(has_ss3);

    // Clean config must have no SS-1
    let mut clean_config = ForjarConfig::default();
    let mut safe = Resource::default();
    safe.resource_type = ResourceType::File;
    safe.mode = Some("0600".into());
    safe.content = Some("clean config data".into());
    clean_config.resources.insert("safe".into(), safe);
    let clean_findings = scan(&clean_config);
    let no_ss1 = !clean_findings.iter().any(|f| f.rule_id == "SS-1");
    println!(
        "  Clean config → no SS-1:   {} {}",
        if no_ss1 { "clean" } else { "dirty" },
        if no_ss1 { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(no_ss1);

    // ── FJ-2300: Path Deny Policy ──
    println!("\n[FJ-2300] Path Deny Policy:");
    let policy = PathPolicy {
        deny_paths: vec!["/etc/shadow".into(), "/root/.ssh/*".into()],
    };
    let checks = [
        ("/etc/shadow", true),
        ("/root/.ssh/id_rsa", true),
        ("/etc/nginx.conf", false),
        ("/var/log/syslog", false),
    ];
    for (path, expected) in &checks {
        let denied = policy.is_denied(path);
        let ok = denied == *expected;
        println!(
            "  {} → {} {}",
            path,
            if denied { "DENIED" } else { "allowed" },
            if ok { "✓" } else { "✗ FALSIFIED" }
        );
        assert!(ok);
    }

    // ── FJ-2604: Mutation Score Model ──
    println!("\n[FJ-2604] Mutation Score Model:");
    let scores = [
        (100, 100, 'A'),
        (90, 100, 'A'),
        (89, 100, 'B'),
        (80, 100, 'B'),
        (60, 100, 'C'),
        (59, 100, 'F'),
        (0, 100, 'F'),
    ];
    for (detected, total, expected) in &scores {
        let s = MutationScore {
            total: *total,
            detected: *detected,
            survived: total - detected,
            errored: 0,
        };
        let ok = s.grade() == *expected;
        println!(
            "  {detected}/{total} = {:.0}% → {} {}",
            s.score_pct(),
            s.grade(),
            if ok { "✓" } else { "✗ FALSIFIED" }
        );
        assert!(ok);
    }

    // ── FJ-2301: Observability ──
    println!("\n[FJ-2301] Observability:");

    // Verbosity ordering
    let v_ok = VerbosityLevel::Normal < VerbosityLevel::Verbose
        && VerbosityLevel::Verbose < VerbosityLevel::VeryVerbose
        && VerbosityLevel::VeryVerbose < VerbosityLevel::Trace;
    println!(
        "  Verbosity ordering:       {} {}",
        if v_ok { "strict" } else { "broken" },
        if v_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(v_ok);

    // Log truncation
    let trunc = LogTruncation {
        first_bytes: 10,
        last_bytes: 10,
    };
    let small = "short text";
    let big = "AAAAAAAAAA_very_long_middle_part_BBBBBBBBBB";
    let small_ok = trunc.truncate(small) == small;
    let big_result = trunc.truncate(big);
    let big_ok = big_result.starts_with("AAAAAAAAAA") && big_result.contains("TRUNCATED");
    println!(
        "  Small log preserved:      {} {}",
        if small_ok { "yes" } else { "no" },
        if small_ok { "✓" } else { "✗ FALSIFIED" }
    );
    println!(
        "  Big log truncated:        {} {}",
        if big_ok { "yes" } else { "no" },
        if big_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(small_ok);
    assert!(big_ok);

    println!("\n{}", "=".repeat(62));
    println!("All security/OCI/observability falsification criteria survived.");
}
