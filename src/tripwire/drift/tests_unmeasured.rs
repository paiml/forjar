//! forjar#549: the reader, the census and the tripwire class for UNMEASURED.

use super::census::DriftCensus;
use super::unmeasured::{census_unmeasured, classify, Reading};
use super::{DriftFinding, DriftReport, SkipReason, UNMEASURED};
use crate::core::error::{classify_untyped, ErrorClass, DRIFT_UNMEASURED_MARKER};
use crate::core::types::{Machine, ResourceType};
use crate::transport::ExecOutput;

fn machine(addr: &str) -> Machine {
    Machine {
        hostname: "web".to_string(),
        addr: addr.to_string(),
        user: "root".to_string(),
        arch: "x86_64".to_string(),
        ssh_key: None,
        roles: vec![],
        transport: None,
        container: None,
        pepita: None,
        cost: 0,
        allowed_operators: vec![],
    }
}

fn exited(code: i32, stderr: &str) -> Result<ExecOutput, String> {
    Ok(ExecOutput {
        exit_code: code,
        stdout: String::new(),
        stderr: stderr.to_string(),
    })
}

fn why(reading: Reading) -> Option<String> {
    match reading {
        Reading::Unmeasured(why) => Some(why),
        Reading::Answered(_) => None,
    }
}

#[test]
fn ssh_exit_255_is_unmeasured_and_names_the_host() {
    let stderr = "ssh: connect to host 203.0.113.9 port 22: Connection timed out";
    let why = why(classify(&machine("203.0.113.9"), exited(255, stderr)))
        .expect("ssh's own failure must not be read as the target's answer");
    assert!(why.contains("203.0.113.9"), "{why}");
}

#[test]
fn a_transport_error_is_unmeasured() {
    let err = "transport timeout: script on 'web' exceeded 60s limit".to_string();
    let why = why(classify(&machine("203.0.113.9"), Err(err)));
    assert!(why.is_some_and(|w| w.contains("exceeded 60s")));
}

#[test]
fn an_ssh_target_that_answered_is_an_answer_whatever_it_said() {
    // `cat` on a missing file: the host was reached, and MISSING is true.
    let m = machine("203.0.113.9");
    assert!(why(classify(&m, exited(1, "cat: /x: No such file"))).is_none());
    assert!(why(classify(&m, exited(0, ""))).is_none());
}

#[test]
fn a_local_script_exiting_255_is_its_own_answer() {
    // No ssh in the path, so 255 is the script's status, not a transport failure.
    assert!(why(classify(&machine("127.0.0.1"), exited(255, ""))).is_none());
}

#[test]
fn an_unmeasured_finding_is_distinguishable_from_every_drift_verdict() {
    let u = DriftFinding::unmeasured("conf", ResourceType::File, "blake3:abc", "why".into());
    assert!(u.is_unmeasured());
    assert_eq!(u.actual_hash, UNMEASURED);
    for actual in ["MISSING", "ERROR", "completion_check: FAIL", "blake3:def"] {
        let d = DriftFinding {
            actual_hash: actual.to_string(),
            ..u.clone()
        };
        assert!(!d.is_unmeasured(), "{actual} was read as unmeasured");
    }
}

#[test]
fn the_census_never_counts_an_unanswered_query_as_inspected_or_skipped() {
    let mut census = DriftCensus::new();
    census.inspected("conf", &ResourceType::File);
    census.inspected("pkg", &ResourceType::Package);
    census.skipped("svc", &ResourceType::Service, SkipReason::NotConverged);
    let findings = vec![DriftFinding::unmeasured(
        "conf",
        ResourceType::File,
        "h",
        "why".into(),
    )];
    census_unmeasured(&findings, &mut census);

    assert_eq!(census.in_scope(), 3);
    assert_eq!(census.inspected_total(), 1, "only pkg was answered");
    assert_eq!(census.unmeasured_total(), 1);
    assert_eq!(census.skipped_total(), 1);
    assert_eq!(
        census.in_scope(),
        census.inspected_total() + census.unmeasured_total() + census.skipped_total()
    );
    assert_eq!(census.unmeasured_ids(), vec!["conf"]);
    assert_eq!(census.inspected_by_type().get("file"), None);
    let json = census.to_json();
    assert_eq!(json["unmeasured"], 1, "{json}");
    assert_eq!(json["inspected"], 1, "{json}");
    let lines = census.summary_lines();
    assert!(
        lines.iter().any(|l| l.starts_with("unmeasured 1: conf")),
        "{lines:?}"
    );
}

#[test]
fn a_report_marks_its_census_and_keeps_the_finding() {
    let mut census = DriftCensus::new();
    census.inspected("conf", &ResourceType::File);
    let finding = DriftFinding::unmeasured("conf", ResourceType::File, "h", "why".into());
    let report = DriftReport::new(vec![finding], census);
    assert_eq!(report.census.unmeasured_total(), 1);
    assert_eq!(report.census.inspected_total(), 0);
    assert_eq!(report.findings.len(), 1, "apply still sees the finding");
}

#[test]
fn the_tripwire_marker_exits_with_the_connection_class() {
    let message = format!("{DRIFT_UNMEASURED_MARKER}: 1 resource(s)");
    assert_eq!(classify_untyped(&message), ErrorClass::Connection);
    assert_ne!(
        classify_untyped("1 drift finding(s)"),
        ErrorClass::Connection
    );
}
