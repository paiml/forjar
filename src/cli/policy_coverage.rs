//! FJ-3208: `forjar policy-coverage` — policy rule coverage report.
//!
//! A RENDERER. The calculation is `core::policy_coverage::compute_coverage`,
//! and this file must not compute anything it does not — that separation is
//! the point of paiml/forjar#356.
//!
//! What used to be here was a second calculation (`build_report`) that answered
//! a different question from the module of the same name in `core`, and both
//! printed "1 of 2" over the same fixture while pointing at opposite resources.
//! See the header of `src/core/policy_coverage/mod.rs`.
//!
//! `--json` prints `serde_json::to_value(&coverage)` — the same bytes the
//! `policy-coverage` MCP verb returns, because the verb's output type IS
//! `core::policy_coverage::PolicyCoverage`. Not "the same shape": the same
//! type.

use crate::core::policy_coverage::{self, PolicyCoverage};
use std::path::Path;

/// Run `forjar policy-coverage` — analyze policy rule coverage.
pub(crate) fn cmd_policy_coverage(file: &Path, json: bool) -> Result<(), String> {
    let config = super::helpers::parse_and_validate(file)?;
    let report = policy_coverage::compute_coverage(&config);

    if json {
        print_json(&report);
    } else {
        print_table(&report);
    }
    Ok(())
}

/// One `name  count` block, printed only when it has rows.
fn print_counts(heading: &str, rows: &std::collections::BTreeMap<String, usize>) {
    if rows.is_empty() {
        return;
    }
    println!("{heading}");
    for (k, n) in rows {
        println!("  {k:<12} {n}");
    }
    println!();
}

fn print_table(r: &PolicyCoverage) {
    println!("Policy Coverage Report");
    println!("======================");
    println!();
    println!(
        "Rules: {} total, {} triggered, {} untriggered",
        r.total_rules,
        r.rules_triggered,
        r.untriggered_rules.len()
    );
    println!(
        "Resources: {} total, {} clean (no violations)",
        r.total_resources, r.clean_resources
    );
    // COVERED is not CLEAN, and printing only the second is what let the two
    // calculations diverge unnoticed: a resource no rule scopes to is clean
    // because nothing ever looked at it.
    println!(
        "Coverage:  {:.1}% ({}/{} resources in the scope of at least one rule)",
        r.coverage_percent, r.covered_resources, r.total_resources
    );
    println!();

    print_counts("By rule type:", &r.by_type);
    print_counts("By severity:", &r.by_severity);
    print_counts("By resource scope:", &r.by_resource_scope);

    if !r.compliance_frameworks.is_empty() {
        println!("Compliance frameworks:");
        for (f, n) in &r.compliance_frameworks {
            println!("  {f:<12} {n} rule(s)");
        }
        println!();
    }

    if !r.uncovered.is_empty() {
        println!("Uncovered resources (no rule scopes to them):");
        for id in &r.uncovered {
            println!("  {id}");
        }
        println!();
    }

    if !r.untriggered_rules.is_empty() {
        println!("Untriggered rules (no violations found):");
        for id in &r.untriggered_rules {
            println!("  {id}");
        }
    }
}

fn print_json(r: &PolicyCoverage) {
    // NOT a hand-written `json!` projection. One was here, and it was how the
    // CLI and the MCP verb came to publish different field sets over the same
    // question.
    println!(
        "{}",
        serde_json::to_string_pretty(&policy_coverage::coverage_to_json(r))
            .unwrap_or_else(|_| "{}".to_string())
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::*;

    fn cfg(resources: &[(&str, ResourceType)], policies: Vec<PolicyRule>) -> ForjarConfig {
        let mut c = ForjarConfig {
            policies,
            ..Default::default()
        };
        for (name, t) in resources {
            c.resources.insert(
                (*name).to_string(),
                Resource {
                    resource_type: t.clone(),
                    ..Default::default()
                },
            );
        }
        c
    }

    fn require_owner(scope: Option<&str>, id: &str) -> PolicyRule {
        PolicyRule {
            rule_type: PolicyRuleType::Require,
            message: "need owner".into(),
            id: Some(id.into()),
            resource_type: scope.map(str::to_string),
            tag: None,
            field: Some("owner".into()),
            condition_field: None,
            condition_value: None,
            max_count: None,
            min_count: None,
            severity: None,
            remediation: None,
            compliance: vec![ComplianceMapping {
                framework: "cis".into(),
                control: "5.1".into(),
            }],
        }
    }

    #[test]
    fn print_table_no_panic_on_an_empty_report() {
        let r = policy_coverage::compute_coverage(&ForjarConfig::default());
        print_table(&r);
        print_json(&r);
    }

    #[test]
    fn print_table_renders_every_section() {
        let r = policy_coverage::compute_coverage(&cfg(
            &[("conf", ResourceType::File), ("pkg", ResourceType::Package)],
            vec![require_owner(Some("file"), "P-001")],
        ));
        print_table(&r);
        assert_eq!(r.uncovered, vec!["pkg"]);
        assert_eq!(r.by_resource_scope.get("file"), Some(&1));
        assert_eq!(r.compliance_frameworks.get("cis"), Some(&1));
    }

    /// The renderer must not reshape the document. `--json` prints exactly
    /// `coverage_to_json`, which is exactly `serde_json::to_value` of the
    /// report — the same value the MCP verb returns.
    #[test]
    fn the_json_renderer_prints_the_calculation_verbatim() {
        let config = cfg(
            &[("conf", ResourceType::File)],
            vec![require_owner(None, "P-any")],
        );
        let report = policy_coverage::compute_coverage(&config);
        assert_eq!(
            policy_coverage::coverage_to_json(&report),
            serde_json::to_value(&report).unwrap()
        );
    }

    #[test]
    fn cmd_coverage_valid_config() {
        let dir = tempfile::tempdir().unwrap();
        let config = "version: \"1.0\"\nname: test\nmachines:\n  m1:\n    addr: localhost\n    hostname: m1\nresources:\n  cfg:\n    type: file\n    path: /etc/app.conf\n    content: \"key=val\"\npolicies:\n  - type: require\n    message: needs owner\n    field: owner\n";
        std::fs::write(dir.path().join("forjar.yaml"), config).unwrap();
        let result = cmd_policy_coverage(&dir.path().join("forjar.yaml"), true);
        assert!(result.is_ok(), "failed: {:?}", result.err());
    }

    #[test]
    fn cmd_coverage_table_form_runs() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("forjar.yaml"),
            "version: \"1.0\"\nname: test\nmachines:\n  m1:\n    addr: localhost\n    hostname: m1\nresources:\n  cfg:\n    type: file\n    path: /etc/app.conf\n    content: \"key=val\"\n",
        )
        .unwrap();
        assert!(cmd_policy_coverage(&dir.path().join("forjar.yaml"), false).is_ok());
    }
}
