//! Example: Policy boundary testing (FJ-3209)
//!
//! Demonstrates mutation testing for compliance packs by generating
//! boundary configurations that exercise each policy rule.
//!
//! ```bash
//! cargo run --example policy_boundary
//! ```

use forjar::core::cis_ubuntu_pack::cis_ubuntu_2204_pack;
use forjar::core::compliance_pack::{ComplianceCheck, CompliancePack, ComplianceRule};
use forjar::core::policy_boundary::{
    format_boundary_results, generate_boundary_configs, test_boundaries,
};

fn main() {
    println!("=== Policy Boundary Testing (FJ-3209) ===\n");

    // 1. Simple pack with all rule types
    println!("1. Testing custom pack with all rule types:");
    let pack = CompliancePack {
        name: "boundary-demo".into(),
        version: "1.0".into(),
        framework: "DEMO".into(),
        description: Some("Demonstration pack for boundary testing".into()),
        rules: vec![
            ComplianceRule {
                id: "DEMO-001".into(),
                title: "Files must be owned by root".into(),
                description: None,
                severity: "error".into(),
                controls: vec!["CIS 6.1.1".into()],
                check: ComplianceCheck::Assert {
                    resource_type: "file".into(),
                    field: "owner".into(),
                    expected: "root".into(),
                },
            },
            ComplianceRule {
                id: "DEMO-002".into(),
                title: "No world-writable files".into(),
                description: None,
                severity: "error".into(),
                controls: vec!["CIS 5.3.1".into()],
                check: ComplianceCheck::Deny {
                    resource_type: "file".into(),
                    field: "mode".into(),
                    pattern: "777".into(),
                },
            },
            ComplianceRule {
                id: "DEMO-003".into(),
                title: "Services must have owner".into(),
                description: None,
                severity: "warning".into(),
                controls: vec![],
                check: ComplianceCheck::Require {
                    resource_type: "service".into(),
                    field: "owner".into(),
                },
            },
            ComplianceRule {
                id: "DEMO-004".into(),
                title: "Resources must be tagged".into(),
                description: None,
                severity: "info".into(),
                controls: vec![],
                check: ComplianceCheck::RequireTag {
                    tag: "managed".into(),
                },
            },
        ],
    };

    let configs = generate_boundary_configs(&pack);
    println!(
        "   Generated {} boundary configs for {} rules",
        configs.len(),
        pack.rules.len()
    );

    for config in &configs {
        let pass_str = if config.expected_pass { "PASS" } else { "FAIL" };
        println!(
            "   [{pass_str}] {}: {}",
            config.target_rule_id, config.description
        );
    }

    println!("\n2. Running boundary tests:");
    let result = test_boundaries(&pack);
    println!("{}", format_boundary_results(&result));

    // 3. Test against real CIS Ubuntu pack
    println!("\n3. CIS Ubuntu 22.04 boundary testing:");
    let cis_pack = cis_ubuntu_2204_pack();
    let cis_configs = generate_boundary_configs(&cis_pack);
    println!(
        "   Generated {} boundary configs for {} CIS rules",
        cis_configs.len(),
        cis_pack.rules.len()
    );

    let cis_result = test_boundaries(&cis_pack);
    println!("{}", format_boundary_results(&cis_result));

    if cis_result.all_passed() {
        println!("\n   All CIS boundary tests PASSED — no vacuous rules detected.");
    } else {
        println!(
            "\n   {} CIS boundary tests FAILED",
            cis_result.failure_count()
        );
    }

    println!("\nDone.");
}
