//! Tests: Coverage for apply_variants and apply_output.

#![allow(unused_imports)]
#![allow(dead_code)]
use super::apply_output::*;
use super::apply_variants::*;
use super::helpers::*;
use super::helpers_state::*;
use super::test_fixtures::*;
use crate::core::types;
use std::io::Write;
use std::time::Duration;

#[cfg(test)]
mod tests {
    use super::*;

    fn write_temp_config(yaml: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(yaml.as_bytes()).unwrap();
        f.flush().unwrap();
        f
    }

    fn write_yaml(dir: &std::path::Path, name: &str, content: &str) -> std::path::PathBuf {
        let p = dir.join(name);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&p, content).unwrap();
        p
    }

    fn minimal_config_yaml() -> &'static str {
        "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n"
    }

    fn two_machine_config_yaml() -> &'static str {
        "version: \"1.0\"\nname: t\nmachines:\n  alpha:\n    hostname: alpha\n    addr: 127.0.0.1\n  beta:\n    hostname: beta\n    addr: 127.0.0.2\nresources:\n  a:\n    type: file\n    machine: alpha\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: beta\n    path: /tmp/b\n    content: b\n"
    }

    fn make_apply_result(
        machine: &str,
        converged: u32,
        unchanged: u32,
        failed: u32,
    ) -> types::ApplyResult {
        types::ApplyResult {
            machine: machine.to_string(),
            resources_converged: converged,
            resources_unchanged: unchanged,
            resources_failed: failed,
            total_duration: Duration::from_millis(150),
            resource_reports: vec![],
        }
    }

    fn make_apply_result_with_reports(
        machine: &str,
        converged: u32,
        unchanged: u32,
        failed: u32,
        reports: Vec<types::ResourceReport>,
    ) -> types::ApplyResult {
        types::ApplyResult {
            machine: machine.to_string(),
            resources_converged: converged,
            resources_unchanged: unchanged,
            resources_failed: failed,
            total_duration: Duration::from_millis(200),
            resource_reports: reports,
        }
    }

    fn make_resource_report(
        id: &str,
        rtype: &str,
        status: &str,
        dur: f64,
    ) -> types::ResourceReport {
        types::ResourceReport {
            resource_id: id.to_string(),
            resource_type: rtype.to_string(),
            status: status.to_string(),
            duration_seconds: dur,
            exit_code: Some(0),
            hash: Some("blake3:abc123".to_string()),
            error: None,
        }
    }

    fn make_failed_resource_report(id: &str, rtype: &str, err: &str) -> types::ResourceReport {
        types::ResourceReport {
            resource_id: id.to_string(),
            resource_type: rtype.to_string(),
            status: "failed".to_string(),
            duration_seconds: 0.01,
            exit_code: Some(1),
            hash: None,
            error: Some(err.to_string()),
        }
    }

    // ================================================================
    // cmd_apply_canary_machine tests
    // ================================================================

    #[test]
    fn test_canary_machine_not_found_returns_error() {
        let f = write_temp_config(minimal_config_yaml());
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let result = cmd_apply_canary_machine(f.path(), &state_dir, "nonexistent", &[], None);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("nonexistent"),
            "error should mention the missing machine name: {}",
            err
        );
        assert!(
            err.contains("not found"),
            "error should say 'not found': {}",
            err
        );
    }

    #[test]
    fn test_canary_machine_not_found_lists_available() {
        let f = write_temp_config(two_machine_config_yaml());
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let result = cmd_apply_canary_machine(f.path(), &state_dir, "missing", &[], None);
        assert!(result.is_err());
        let err = result.unwrap_err();
        // Should list available machines
        assert!(
            err.contains("alpha") || err.contains("beta"),
            "error should list available machines: {}",
            err
        );
    }

    #[test]
    fn test_canary_machine_invalid_config_returns_error() {
        let f = write_temp_config("invalid: yaml: content");
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let result = cmd_apply_canary_machine(f.path(), &state_dir, "m", &[], None);
        assert!(result.is_err());
    }

    #[test]
    fn test_canary_machine_empty_config_returns_error() {
        let f = write_temp_config("");
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let result = cmd_apply_canary_machine(f.path(), &state_dir, "any", &[], None);
        assert!(result.is_err());
    }

    #[test]
    fn test_canary_machine_nonexistent_file_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let result = cmd_apply_canary_machine(
            std::path::Path::new("/tmp/does-not-exist-forjar-test.yaml"),
            &state_dir,
            "m",
            &[],
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_canary_machine_with_timeout_not_found() {
        let f = write_temp_config(minimal_config_yaml());
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let result = cmd_apply_canary_machine(f.path(), &state_dir, "bogus", &[], Some(5));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("bogus"));
    }

    #[test]
    fn test_canary_machine_with_params_not_found() {
        let f = write_temp_config(minimal_config_yaml());
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let params = vec!["key=value".to_string()];
        let result = cmd_apply_canary_machine(f.path(), &state_dir, "nope", &params, None);
        assert!(result.is_err());
    }

    // ================================================================
    // cmd_apply_dry_run_cost tests
    // ================================================================

    #[test]
    fn test_dry_run_cost_valid_config_no_state() {
        let f = write_temp_config(minimal_config_yaml());
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let result = cmd_apply_dry_run_cost(f.path(), &state_dir, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_dry_run_cost_valid_config_with_machine_filter() {
        let f = write_temp_config(minimal_config_yaml());
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let result = cmd_apply_dry_run_cost(f.path(), &state_dir, Some("m"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_dry_run_cost_with_existing_state() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("forjar.yaml");
        std::fs::write(&config_path, minimal_config_yaml()).unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        // Create a state lock so the planner sees existing state
        make_state_dir_with_lock(
            &state_dir,
            "m",
            vec![("a", "blake3:old", types::ResourceStatus::Converged)],
        );

        let result = cmd_apply_dry_run_cost(&config_path, &state_dir, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_dry_run_cost_invalid_config() {
        let f = write_temp_config("not valid yaml for forjar");
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let result = cmd_apply_dry_run_cost(f.path(), &state_dir, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_dry_run_cost_empty_config() {
        let f = write_temp_config("");
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let result = cmd_apply_dry_run_cost(f.path(), &state_dir, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_dry_run_cost_nonexistent_file() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let result = cmd_apply_dry_run_cost(
            std::path::Path::new("/tmp/no-such-forjar-config-xyz.yaml"),
            &state_dir,
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_dry_run_cost_two_machines() {
        let f = write_temp_config(two_machine_config_yaml());
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let result = cmd_apply_dry_run_cost(f.path(), &state_dir, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_dry_run_cost_filter_nonexistent_machine() {
        let f = write_temp_config(minimal_config_yaml());
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        // Filtering by a machine not in config still works, just shows nothing to change
        let result = cmd_apply_dry_run_cost(f.path(), &state_dir, Some("nonexistent"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_dry_run_cost_with_deps() {
        let yaml = "version: \"1.0\"\nname: deps\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  base:\n    type: file\n    machine: m\n    path: /tmp/base\n    content: base\n  child:\n    type: file\n    machine: m\n    path: /tmp/child\n    content: child\n    depends_on: [base]\n";
        let f = write_temp_config(yaml);
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let result = cmd_apply_dry_run_cost(f.path(), &state_dir, None);
        assert!(result.is_ok());
    }

    // ================================================================
    // print_events_output tests
    // ================================================================

    #[test]
    fn test_print_events_output_empty_results() {
        let results: Vec<types::ApplyResult> = vec![];
        let result = print_events_output(&results);
        assert!(result.is_ok());
    }

    #[test]
    fn test_print_events_output_single_machine_no_reports() {
        let results = vec![make_apply_result("web", 2, 1, 0)];
        let result = print_events_output(&results);
        assert!(result.is_ok());
    }

    #[test]
    fn test_print_events_output_single_machine_with_converged_report() {
        let reports = vec![make_resource_report("cfg", "file", "converged", 0.1)];
        let results = vec![make_apply_result_with_reports("web", 1, 0, 0, reports)];
        let result = print_events_output(&results);
        assert!(result.is_ok());
    }

    #[test]
    fn test_print_events_output_single_machine_with_failed_report() {
        let reports = vec![make_failed_resource_report("pkg", "package", "apt failed")];
        let results = vec![make_apply_result_with_reports("web", 0, 0, 1, reports)];
        let result = print_events_output(&results);
        assert!(result.is_ok());
    }

    #[test]
    fn test_print_events_output_single_machine_with_unchanged_report() {
        let reports = vec![make_resource_report("svc", "service", "unchanged", 0.05)];
        let results = vec![make_apply_result_with_reports("web", 0, 1, 0, reports)];
        let result = print_events_output(&results);
        assert!(result.is_ok());
    }

    #[test]
    fn test_print_events_output_multiple_machines() {
        let reports1 = vec![
            make_resource_report("cfg", "file", "converged", 0.1),
            make_resource_report("app", "file", "unchanged", 0.05),
        ];
        let reports2 = vec![make_failed_resource_report("db", "package", "timeout")];
        let results = vec![
            make_apply_result_with_reports("web", 1, 1, 0, reports1),
            make_apply_result_with_reports("db", 0, 0, 1, reports2),
        ];
        let result = print_events_output(&results);
        assert!(result.is_ok());
    }

    #[test]
    fn test_print_events_output_mixed_statuses() {
        let reports = vec![
            make_resource_report("r1", "file", "converged", 0.2),
            make_resource_report("r2", "service", "unchanged", 0.01),
            make_failed_resource_report("r3", "package", "not found"),
        ];
        let results = vec![make_apply_result_with_reports("srv", 1, 1, 1, reports)];
        let result = print_events_output(&results);
        assert!(result.is_ok());
    }

    #[test]
    fn test_print_events_output_many_machines() {
        let results: Vec<types::ApplyResult> = (0..5)
            .map(|i| {
                let rpts = vec![make_resource_report(
                    &format!("r{}", i),
                    "file",
                    "converged",
                    0.1 * i as f64,
                )];
                make_apply_result_with_reports(&format!("m{}", i), 1, 0, 0, rpts)
            })
            .collect();
        let result = print_events_output(&results);
        assert!(result.is_ok());
    }

    #[test]
    fn test_print_events_output_report_with_none_fields() {
        let report = types::ResourceReport {
            resource_id: "x".to_string(),
            resource_type: "file".to_string(),
            status: "converged".to_string(),
            duration_seconds: 0.0,
            exit_code: None,
            hash: None,
            error: None,
        };
        let results = vec![make_apply_result_with_reports("m", 1, 0, 0, vec![report])];
        let result = print_events_output(&results);
        assert!(result.is_ok());
    }
}
