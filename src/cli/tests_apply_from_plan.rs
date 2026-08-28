//! Unit tests for the plan-file apply path (Refs #358).

use super::*;

#[test]
fn a_sealed_plan_may_legitimately_report_no_changes() {
    assert!(check_empty_plan_is_trustworthy(true).is_ok());
}

#[test]
fn an_unsealed_plan_reporting_no_changes_is_refused() {
    let err = check_empty_plan_is_trustworthy(false).unwrap_err();
    assert!(err.contains(plan_file::FORMAT_V1), "{err}");
    assert!(err.contains(plan_file::FORMAT_V2), "{err}");
    assert!(err.contains("unauthenticated counter"), "{err}");
}

#[test]
fn prepare_config_reports_a_missing_file_rather_than_panicking() {
    let err = prepare_config(Path::new("/nonexistent/forjar.yaml"), None, None).unwrap_err();
    assert!(!err.is_empty());
}
