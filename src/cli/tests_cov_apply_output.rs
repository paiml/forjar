//! Coverage tests for cli/apply_output.rs — count_results, print functions, timing.

use super::apply_output::*;
use crate::core::types;

fn make_result(machine: &str, converged: u32, unchanged: u32, failed: u32) -> types::ApplyResult {
    types::ApplyResult {
        machine: machine.into(),
        resources_converged: converged,
        resources_unchanged: unchanged,
        resources_failed: failed,
        resource_reports: Vec::new(),
        total_duration: std::time::Duration::from_secs_f64(1.5),
    }
}

fn make_result_with_reports(
    machine: &str,
    converged: u32,
    failed: u32,
) -> types::ApplyResult {
    let mut reports = Vec::new();
    for i in 0..converged {
        reports.push(types::ResourceReport {
            resource_id: format!("pkg-{i}"),
            resource_type: "package".into(),
            status: "converged".into(),
            duration_seconds: 0.5,
            exit_code: Some(0),
            hash: Some("abc123".into()),
            error: None,
        });
    }
    for i in 0..failed {
        reports.push(types::ResourceReport {
            resource_id: format!("fail-{i}"),
            resource_type: "service".into(),
            status: "failed".into(),
            duration_seconds: 0.1,
            exit_code: Some(1),
            hash: None,
            error: Some("exit code 1".into()),
        });
    }
    types::ApplyResult {
        machine: machine.into(),
        resources_converged: converged,
        resources_unchanged: 0,
        resources_failed: failed,
        resource_reports: reports,
        total_duration: std::time::Duration::from_secs_f64(2.0),
    }
}

// ── count_results ──

#[test]
fn count_results_empty() {
    let (c, u, f) = count_results(&[]);
    assert_eq!((c, u, f), (0, 0, 0));
}

#[test]
fn count_results_single() {
    let results = vec![make_result("web1", 5, 3, 1)];
    let (c, u, f) = count_results(&results);
    assert_eq!((c, u, f), (5, 3, 1));
}

#[test]
fn count_results_multi() {
    let results = vec![
        make_result("web1", 5, 3, 1),
        make_result("db1", 2, 0, 0),
        make_result("lb1", 1, 1, 2),
    ];
    let (c, u, f) = count_results(&results);
    assert_eq!((c, u, f), (8, 4, 3));
}

// ── print_events_output ──

#[test]
fn events_output_empty() {
    let r = print_events_output(&[]);
    assert!(r.is_ok());
}

#[test]
fn events_output_with_reports() {
    let results = vec![make_result_with_reports("web1", 2, 1)];
    let r = print_events_output(&results);
    assert!(r.is_ok());
}

// ── print_resource_report ──

#[test]
fn resource_report_empty() {
    print_resource_report(&[]);
}

#[test]
fn resource_report_with_data() {
    let results = vec![make_result_with_reports("web1", 3, 1)];
    print_resource_report(&results);
}

#[test]
fn resource_report_unchanged() {
    let mut r = make_result_with_reports("web1", 1, 0);
    r.resource_reports.push(types::ResourceReport {
        resource_id: "cfg".into(),
        resource_type: "file".into(),
        status: "unchanged".into(),
        duration_seconds: 0.0,
        exit_code: Some(0),
        hash: Some("def456".into()),
        error: None,
    });
    print_resource_report(&[r]);
}

// ── print_timing ──

#[test]
fn timing_output() {
    let parse = std::time::Duration::from_secs_f64(0.15);
    let apply = std::time::Duration::from_secs_f64(3.5);
    let total = std::time::Duration::from_secs_f64(3.65);
    print_timing(parse, apply, total);
}

// ── print_apply_summary ──

fn minimal_config() -> types::ForjarConfig {
    serde_yaml_ng::from_str(
        "version: '1.0'\nname: test-stack\nmachines: {}\nresources: {}\n"
    ).unwrap()
}

#[test]
fn summary_text_success() {
    let config = minimal_config();
    let results = vec![make_result("web1", 3, 2, 0)];
    let r = print_apply_summary(&config, &results, 3, 2, 0, std::time::Duration::from_secs(1), false);
    assert!(r.is_ok());
}

#[test]
fn summary_text_with_failures() {
    let config = minimal_config();
    let results = vec![make_result("web1", 3, 2, 1)];
    let r = print_apply_summary(&config, &results, 3, 2, 1, std::time::Duration::from_secs(1), false);
    assert!(r.is_ok());
}

#[test]
fn summary_json_success() {
    let config = minimal_config();
    let results = vec![make_result("web1", 3, 2, 0)];
    let r = print_apply_summary(&config, &results, 3, 2, 0, std::time::Duration::from_secs(1), true);
    assert!(r.is_ok());
}

#[test]
fn summary_json_empty() {
    let config = minimal_config();
    let r = print_apply_summary(&config, &[], 0, 0, 0, std::time::Duration::from_secs(0), true);
    assert!(r.is_ok());
}

#[test]
fn summary_multi_machine() {
    let config = minimal_config();
    let results = vec![
        make_result("web1", 5, 1, 0),
        make_result("db1", 2, 0, 1),
    ];
    let r = print_apply_summary(&config, &results, 7, 1, 1, std::time::Duration::from_secs(2), false);
    assert!(r.is_ok());
}
