//! FJ-3209/3308/3306/095/113/115/051/114: Policy, safety, and supply chain falsification.
//!
//! Demonstrates Popperian rejection criteria for:
//! - Policy boundary testing (assert/deny/require/require_tag boundaries)
//! - Secret audit trail (JSONL lifecycle events)
//! - Namespace isolation (env_clear + secret injection)
//! - Reproducible build verification (env/profile checks)
//! - Ferrocene certification evidence (ISO 26262 / DO-178C)
//! - Flight-grade execution model (no_std, bounded topo sort)
//! - MC/DC coverage analysis (AND/OR pair generation)
//! - DO-330 tool qualification data package
//!
//! Usage: cargo run --example policy_safety_falsification

use forjar::core::compliance_pack::{ComplianceCheck, CompliancePack, ComplianceRule};
use forjar::core::do330::{generate_qualification_package, ToolQualLevel};
use forjar::core::ferrocene::{check_source_compliance, generate_evidence, SafetyStandard};
use forjar::core::flight_grade::{check_compliance, fg_topo_sort, FgPlan, FgResource};
use forjar::core::mcdc::{build_decision, generate_mcdc_and, generate_mcdc_or};
use forjar::core::policy_boundary::{format_boundary_results, test_boundaries};
use forjar::core::repro_build::{check_cargo_profile, check_environment};
use forjar::core::secret_audit::{
    append_audit, audit_summary, format_audit_summary, make_discard_event, make_inject_event,
    make_resolve_event, make_rotate_event, read_audit,
};
use forjar::core::secret_namespace::{
    execute_isolated, format_result, verify_no_leak, NamespaceConfig,
};

fn main() {
    println!("Forjar Policy / Safety / Supply Chain Falsification");
    println!("{}", "=".repeat(60));

    // ── FJ-3209: Policy Boundary Testing ──
    println!("\n[FJ-3209] Policy Boundary Testing:");

    let pack = CompliancePack {
        name: "boundary-demo".into(),
        version: "1.0".into(),
        framework: "CIS".into(),
        description: None,
        rules: vec![
            ComplianceRule {
                id: "R1".into(),
                title: "Root owner".into(),
                description: None,
                severity: "error".into(),
                controls: vec!["CIS 1.1".into()],
                check: ComplianceCheck::Assert {
                    resource_type: "file".into(),
                    field: "owner".into(),
                    expected: "root".into(),
                },
            },
            ComplianceRule {
                id: "D1".into(),
                title: "No world-writable".into(),
                description: None,
                severity: "error".into(),
                controls: vec![],
                check: ComplianceCheck::Deny {
                    resource_type: "file".into(),
                    field: "mode".into(),
                    pattern: "777".into(),
                },
            },
        ],
    };

    let bt_result = test_boundaries(&pack);
    let bt_ok = bt_result.all_passed();
    println!(
        "  Assert + Deny boundary tests: {} {}",
        if bt_ok { "all pass" } else { "failures" },
        if bt_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(bt_ok);
    println!("{}", format_boundary_results(&bt_result));

    // ── FJ-3308: Secret Audit Trail ──
    println!("\n[FJ-3308] Secret Audit Trail:");

    let dir = tempfile::tempdir().unwrap();
    append_audit(
        dir.path(),
        &make_resolve_event("db_pass", "file", "h1", Some("web-01")),
    )
    .unwrap();
    append_audit(
        dir.path(),
        &make_inject_event("db_pass", "file", "h1", "ns-apply"),
    )
    .unwrap();
    append_audit(dir.path(), &make_discard_event("db_pass", "h1")).unwrap();
    append_audit(
        dir.path(),
        &make_rotate_event("db_pass", "file", "h1", "h2"),
    )
    .unwrap();

    let events = read_audit(dir.path()).unwrap();
    let summary = audit_summary(&events);
    let lifecycle_ok = events.len() == 4 && summary.unique_keys == 1;
    println!(
        "  Full lifecycle (4 events, 1 key): {} {}",
        if lifecycle_ok { "yes" } else { "no" },
        if lifecycle_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(lifecycle_ok);
    println!("{}", format_audit_summary(&summary));

    // ── FJ-3306: Namespace Isolation ──
    println!("\n[FJ-3306] Namespace Isolation:");

    let ns_config = NamespaceConfig {
        namespace_id: "ns-demo-1".into(),
        audit_enabled: false,
        inherit_env: vec!["PATH".into()],
        ..Default::default()
    };
    let secrets = vec![forjar::core::ephemeral::ResolvedEphemeral {
        key: "SECRET_TOKEN".into(),
        value: "hidden-value".into(),
        hash: blake3::hash(b"hidden-value").to_hex().to_string(),
    }];

    let ns_result =
        execute_isolated(&ns_config, &secrets, "sh", &["-c", "echo $SECRET_TOKEN"]).unwrap();
    let ns_ok = ns_result.success
        && ns_result.stdout.trim() == "hidden-value"
        && verify_no_leak("SECRET_TOKEN");
    println!(
        "  Secret in child, not in parent: {} {}",
        if ns_ok { "yes" } else { "no" },
        if ns_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(ns_ok);
    println!("  {}", format_result(&ns_result));

    // ── FJ-095: Reproducible Build ──
    println!("\n[FJ-095] Reproducible Build Checks:");

    let env_checks = check_environment();
    println!("  Environment checks: {}", env_checks.len());

    let good_toml = "[profile.release]\npanic = \"abort\"\nlto = true\ncodegen-units = 1\n";
    let profile_checks = check_cargo_profile(good_toml);
    let profile_ok = profile_checks.iter().all(|c| c.passed);
    println!(
        "  Good Cargo profile passes: {} {}",
        if profile_ok { "yes" } else { "no" },
        if profile_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(profile_ok);

    // ── FJ-113: Ferrocene Certification ──
    println!("\n[FJ-113] Ferrocene Certification Evidence:");

    let evidence = generate_evidence(SafetyStandard::Iso26262, "binary-hash", "source-hash");
    let ev_ok = evidence.standard == SafetyStandard::Iso26262
        && !evidence.build_flags.is_empty()
        && !evidence.forbidden_features.is_empty();
    println!(
        "  ISO 26262 evidence generated: {} {}",
        if ev_ok { "yes" } else { "no" },
        if ev_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(ev_ok);

    let clean_source = "fn main() { println!(\"hello\"); }\n";
    let violations = check_source_compliance(clean_source);
    let clean_ok = violations.is_empty();
    println!(
        "  Clean source has no violations: {} {}",
        if clean_ok { "yes" } else { "no" },
        if clean_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(clean_ok);

    // ── FJ-115: Flight-Grade Execution ──
    println!("\n[FJ-115] Flight-Grade Execution:");

    let fg_report = check_compliance(100, 10);
    let fg_ok = fg_report.compliant && fg_report.no_dynamic_alloc && fg_report.bounded_loops;
    println!(
        "  100 resources, depth 10 compliant: {} {}",
        if fg_ok { "yes" } else { "no" },
        if fg_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(fg_ok);

    let mut plan = FgPlan::empty();
    plan.resources[0] = FgResource::empty();
    plan.resources[0].id = 0;
    plan.resources[1] = FgResource::empty();
    plan.resources[1].id = 1;
    plan.resources[1].deps[0] = 0;
    plan.resources[1].dep_count = 1;
    plan.count = 2;
    let sort_ok = fg_topo_sort(&mut plan).is_ok() && plan.order[0] == 0 && plan.order[1] == 1;
    println!(
        "  Topo sort A→B correct: {} {}",
        if sort_ok { "yes" } else { "no" },
        if sort_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(sort_ok);

    // ── FJ-051: MC/DC ──
    println!("\n[FJ-051] MC/DC Coverage Analysis:");

    let and_d = build_decision("a && b && c", &["a", "b", "c"]);
    let and_report = generate_mcdc_and(&and_d);
    let and_ok = and_report.pairs.len() == 3 && and_report.coverage_achievable;
    println!(
        "  AND(a,b,c): 3 pairs, achievable: {} {}",
        if and_ok { "yes" } else { "no" },
        if and_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(and_ok);

    let or_d = build_decision("x || y", &["x", "y"]);
    let or_report = generate_mcdc_or(&or_d);
    let or_ok = or_report.pairs.len() == 2 && or_report.coverage_achievable;
    println!(
        "  OR(x,y): 2 pairs, achievable: {} {}",
        if or_ok { "yes" } else { "no" },
        if or_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(or_ok);

    // ── FJ-114: DO-330 ──
    println!("\n[FJ-114] DO-330 Tool Qualification:");

    let pkg = generate_qualification_package("1.1.1", ToolQualLevel::Tql5);
    let pkg_ok = pkg.qualification_complete
        && pkg.total_requirements == pkg.verified_requirements
        && pkg.coverage_evidence.iter().all(|c| c.satisfied);
    println!(
        "  TQL-5 package complete: {} {}",
        if pkg_ok { "yes" } else { "no" },
        if pkg_ok { "✓" } else { "✗ FALSIFIED" }
    );
    assert!(pkg_ok);

    println!("\n{}", "=".repeat(60));
    println!("All policy/safety/supply chain criteria survived.");
}
