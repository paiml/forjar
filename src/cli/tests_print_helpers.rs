//! Tests: Plan printing and diff display helpers.

#![allow(unused_imports)]
use super::helpers::*;
use super::helpers_state::*;
use super::helpers_time::*;
use super::print_helpers::*;
use crate::core::types::ProvenanceEvent;
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use crate::transport;
use crate::tripwire::{anomaly, drift, eventlog, tracer};
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fj017_print_plan_update_and_destroy_symbols() {
        // Exercises the Update (~) and Destroy (-) match arms in print_plan
        let plan = types::ExecutionPlan {
            name: "symbol-test".to_string(),
            changes: vec![
                types::PlannedChange {
                    resource_id: "r1".to_string(),
                    machine: "m1".to_string(),
                    resource_type: types::ResourceType::File,
                    action: types::PlanAction::Update,
                    description: "update /etc/conf".to_string(),
                },
                types::PlannedChange {
                    resource_id: "r2".to_string(),
                    machine: "m1".to_string(),
                    resource_type: types::ResourceType::File,
                    action: types::PlanAction::Destroy,
                    description: "destroy /tmp/old".to_string(),
                },
            ],
            execution_order: vec!["r1".to_string(), "r2".to_string()],
            to_create: 0,
            to_update: 1,
            to_destroy: 1,
            unchanged: 0,
            unprobed: Vec::new(),
        };
        // Just verify it doesn't panic — output goes to stdout
        print_plan(&plan, None, None, 0);
    }

    /// forjar#497, the suppression at zero. An unconditional banner is noise,
    /// and noise is how a disclosure stops being read.
    #[test]
    fn an_empty_census_produces_no_disclosure() {
        assert!(unprobed_disclosure(&[]).is_none());
    }

    /// The wording is load-bearing: the entry names the (resource, machine)
    /// pair, the sentence says MEASURE rather than "drifted", and it names the
    /// instrument that can answer.
    #[test]
    fn a_census_entry_names_the_pair_the_measurement_and_the_remedy() {
        let census = vec![types::UnprobedResource {
            resource_id: "build".to_string(),
            machine: "box".to_string(),
            reason: "this host does not answer for machine box".to_string(),
        }];
        let msg = unprobed_disclosure(&census).expect("a non-empty census discloses");
        assert!(msg.contains("did not measure"), "{msg}");
        assert!(msg.contains("build@box"), "{msg}");
        assert!(msg.contains("forjar drift"), "{msg}");
        assert!(
            !msg.contains("drifted"),
            "the count is of what was NOT MEASURED, never of what drifted: {msg}"
        );
    }

    #[test]
    fn test_fj132_export_scripts_creates_files() {
        let dir = tempfile::tempdir().unwrap();
        let output_dir = dir.path().join("scripts");
        let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  my-pkg:
    type: package
    machine: m
    provider: apt
    packages: [curl]
  my-file:
    type: file
    machine: m
    path: /etc/test.conf
    content: "hello"
"#;
        let config: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        export_scripts(&config, &output_dir).unwrap();
        assert!(output_dir.join("my-pkg.check.sh").exists());
        assert!(output_dir.join("my-pkg.apply.sh").exists());
        assert!(output_dir.join("my-file.check.sh").exists());
        assert!(output_dir.join("my-file.apply.sh").exists());
    }

    #[test]
    fn test_fj132_export_scripts_sanitizes_slashes() {
        let dir = tempfile::tempdir().unwrap();
        let output_dir = dir.path().join("scripts");
        let yaml = r#"
version: "1.0"
name: test
machines:
  m:
    hostname: m
    addr: 127.0.0.1
resources:
  web/config:
    type: file
    machine: m
    path: /etc/nginx/nginx.conf
    content: "server {}"
"#;
        let config: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        export_scripts(&config, &output_dir).unwrap();
        // Slashes should be replaced with --
        assert!(output_dir.join("web--config.check.sh").exists());
        assert!(output_dir.join("web--config.apply.sh").exists());
    }

    #[test]
    fn test_fj255_print_content_diff_create() {
        // Should not panic on Create action
        print_content_diff("line1\nline2\nline3", &types::PlanAction::Create, None);
    }

    #[test]
    fn test_fj255_print_content_diff_update() {
        print_content_diff("updated content", &types::PlanAction::Update, None);
    }

    #[test]
    fn test_fj255_print_content_diff_truncation() {
        // 60 lines — should truncate at 50
        let content: String = (1..=60)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        print_content_diff(&content, &types::PlanAction::Create, None);
    }

    #[test]
    fn test_fj255_print_content_diff_empty() {
        print_content_diff("", &types::PlanAction::Create, None);
    }

    // FJ-274: Unified diff tests

    #[test]
    fn test_fj274_print_unified_diff_added_lines() {
        print_unified_diff("a", "a\nb\nc");
    }

    #[test]
    fn test_fj274_print_unified_diff_removed_lines() {
        print_unified_diff("a\nb\nc", "a");
    }

    #[test]
    fn test_fj297_export_scripts_metadata_header() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        std::fs::write(
            &config_path,
            r#"
version: "1.0"
name: header-test
machines:
  local:
    hostname: local
    addr: 127.0.0.1
resources:
  web-cfg:
    type: file
    machine: local
    path: /tmp/fj297/test.txt
    content: "hello"
    resource_group: frontend
    tags: [web, critical]
    depends_on: []
"#,
        )
        .unwrap();

        let config = parser::parse_and_validate(&config_path).unwrap();
        let out_dir = dir.path().join("scripts");
        export_scripts(&config, &out_dir).unwrap();

        // Check that the apply script has metadata header
        let apply = std::fs::read_to_string(out_dir.join("web-cfg.apply.sh")).unwrap();
        assert!(apply.contains("# forjar: web-cfg (header-test)"));
        assert!(apply.contains("# machine: local"));
        assert!(apply.contains("# type: file"));
        assert!(apply.contains("# group: frontend"));
        assert!(apply.contains("# tags: web, critical"));
    }

    // ── FJ-303: status --summary ──
}
