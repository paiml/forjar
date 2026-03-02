//! Tests: Coverage for apply_post_actions, count_results, dry_run_output, summary, reports (part 2).

#![allow(unused_imports)]
use super::apply_output::*;
use super::apply_variants::*;
use super::helpers::*;
use crate::core::types;
use std::io::Write;
use std::time::Duration;

#[cfg(test)]
mod tests {
    use super::*;

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

    // ================================================================
    // send_apply_webhook (via apply_post_actions) tests
    // ================================================================

    #[test]
    fn test_apply_post_actions_no_webhook_no_autocommit() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let config: types::ForjarConfig = serde_yaml_ng::from_str(minimal_config_yaml()).unwrap();
        let results = vec![make_apply_result("m", 1, 0, 0)];
        let t_total = std::time::Instant::now();

        let result = apply_post_actions(
            &state_dir, &config, &results, 1, false, false, None, &t_total,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_apply_post_actions_with_webhook_invalid_url() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let config: types::ForjarConfig = serde_yaml_ng::from_str(minimal_config_yaml()).unwrap();
        let results = vec![make_apply_result("m", 1, 0, 0)];
        let t_total = std::time::Instant::now();

        let result = apply_post_actions(
            &state_dir,
            &config,
            &results,
            1,
            false,
            false,
            Some("http://127.0.0.1:1/webhook"),
            &t_total,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_apply_post_actions_with_webhook_verbose() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let config: types::ForjarConfig = serde_yaml_ng::from_str(minimal_config_yaml()).unwrap();
        let results = vec![make_apply_result("m", 0, 1, 0)];
        let t_total = std::time::Instant::now();

        let result = apply_post_actions(
            &state_dir,
            &config,
            &results,
            0,
            false,
            true,
            Some("http://127.0.0.1:1/hook"),
            &t_total,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_apply_post_actions_empty_results() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let config: types::ForjarConfig = serde_yaml_ng::from_str(minimal_config_yaml()).unwrap();
        let results: Vec<types::ApplyResult> = vec![];
        let t_total = std::time::Instant::now();

        let result = apply_post_actions(
            &state_dir, &config, &results, 0, false, false, None, &t_total,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_apply_post_actions_with_failures() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let config: types::ForjarConfig = serde_yaml_ng::from_str(minimal_config_yaml()).unwrap();
        let results = vec![make_apply_result("m", 0, 0, 2)];
        let t_total = std::time::Instant::now();

        let result = apply_post_actions(
            &state_dir, &config, &results, 0, false, false, None, &t_total,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_apply_post_actions_webhook_multiple_results() {
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();

        let yaml = two_machine_config_yaml();
        let config: types::ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
        let results = vec![
            make_apply_result("alpha", 1, 0, 0),
            make_apply_result("beta", 0, 1, 0),
        ];
        let t_total = std::time::Instant::now();

        let result = apply_post_actions(
            &state_dir,
            &config,
            &results,
            1,
            false,
            false,
            Some("http://127.0.0.1:1/notify"),
            &t_total,
        );
        assert!(result.is_ok());
    }

    // ================================================================
    // count_results tests
    // ================================================================

    #[test]
    fn test_count_results_empty() {
        let (c, u, f) = count_results(&[]);
        assert_eq!(c, 0);
        assert_eq!(u, 0);
        assert_eq!(f, 0);
    }

    #[test]
    fn test_count_results_single() {
        let results = vec![make_apply_result("m", 3, 2, 1)];
        let (c, u, f) = count_results(&results);
        assert_eq!(c, 3);
        assert_eq!(u, 2);
        assert_eq!(f, 1);
    }

    #[test]
    fn test_count_results_multiple() {
        let results = vec![
            make_apply_result("a", 2, 1, 0),
            make_apply_result("b", 1, 3, 2),
        ];
        let (c, u, f) = count_results(&results);
        assert_eq!(c, 3);
        assert_eq!(u, 4);
        assert_eq!(f, 2);
    }

    // ================================================================
    // apply_dry_run_output tests
    // ================================================================

    #[test]
    fn test_apply_dry_run_output_text_mode() {
        let config: types::ForjarConfig = serde_yaml_ng::from_str(minimal_config_yaml()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        assert!(apply_dry_run_output(&config, &state_dir, None, None, false).is_ok());
    }

    #[test]
    fn test_apply_dry_run_output_json_mode() {
        let config: types::ForjarConfig = serde_yaml_ng::from_str(minimal_config_yaml()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        assert!(apply_dry_run_output(&config, &state_dir, None, None, true).is_ok());
    }

    #[test]
    fn test_apply_dry_run_output_json_with_machine_filter() {
        let config: types::ForjarConfig = serde_yaml_ng::from_str(minimal_config_yaml()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        assert!(apply_dry_run_output(&config, &state_dir, Some("m"), None, true).is_ok());
    }

    #[test]
    fn test_apply_dry_run_output_with_tag_filter() {
        let config: types::ForjarConfig = serde_yaml_ng::from_str(minimal_config_yaml()).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let state_dir = dir.path().join("state");
        std::fs::create_dir_all(&state_dir).unwrap();
        assert!(apply_dry_run_output(&config, &state_dir, None, Some("web"), false).is_ok());
    }

    // ================================================================
    // print_apply_summary tests
    // ================================================================

    #[test]
    fn test_print_apply_summary_text_no_failures() {
        let config: types::ForjarConfig = serde_yaml_ng::from_str(minimal_config_yaml()).unwrap();
        let results = vec![make_apply_result("m", 1, 0, 0)];
        assert!(print_apply_summary(
            &config,
            &results,
            1,
            0,
            0,
            Duration::from_millis(100),
            false
        )
        .is_ok());
    }

    #[test]
    fn test_print_apply_summary_text_with_failures() {
        let config: types::ForjarConfig = serde_yaml_ng::from_str(minimal_config_yaml()).unwrap();
        let results = vec![make_apply_result("m", 0, 0, 1)];
        assert!(
            print_apply_summary(&config, &results, 0, 0, 1, Duration::from_millis(50), false)
                .is_ok()
        );
    }

    #[test]
    fn test_print_apply_summary_json_mode() {
        let config: types::ForjarConfig = serde_yaml_ng::from_str(minimal_config_yaml()).unwrap();
        let results = vec![make_apply_result("m", 2, 1, 0)];
        assert!(
            print_apply_summary(&config, &results, 2, 1, 0, Duration::from_millis(200), true)
                .is_ok()
        );
    }

    // ================================================================
    // print_resource_report tests
    // ================================================================

    #[test]
    fn test_print_resource_report_empty() {
        let results: Vec<types::ApplyResult> = vec![];
        print_resource_report(&results);
    }

    #[test]
    fn test_print_resource_report_mixed() {
        let reports = vec![
            make_resource_report("cfg", "file", "converged", 0.2),
            make_failed_resource_report("pkg", "package", "install error"),
            make_resource_report("svc", "service", "unchanged", 0.01),
        ];
        let results = vec![make_apply_result_with_reports("m", 1, 1, 1, reports)];
        print_resource_report(&results);
    }

    // ================================================================
    // print_timing tests
    // ================================================================

    #[test]
    fn test_print_timing_basic() {
        print_timing(
            Duration::from_millis(10),
            Duration::from_millis(500),
            Duration::from_millis(510),
        );
    }

    #[test]
    fn test_print_timing_zero_durations() {
        print_timing(Duration::ZERO, Duration::ZERO, Duration::ZERO);
    }
}
