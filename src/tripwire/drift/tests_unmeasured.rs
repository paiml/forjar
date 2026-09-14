//! forjar#549: the reader, the census and the tripwire class for UNMEASURED.

use super::census::DriftCensus;
use super::file::{content_verdict, listing_digest};
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

// forjar#549, the second query: a directory is digested from a listing.

#[test]
fn a_directory_listing_that_never_came_back_is_unmeasured_not_clean() {
    let stderr = "ssh: connect to host 203.0.113.9 port 22: Connection timed out";
    let unanswered = classify(&machine("203.0.113.9"), exited(255, stderr));
    let f = content_verdict("conf", "/srv/site", "blake3:x", listing_digest(unanswered))
        .expect("a listing nobody read must not be a clean verdict");
    assert!(f.is_unmeasured(), "{}", f.actual_hash);
    assert!(f.detail.contains("203.0.113.9"), "{}", f.detail);
}

#[test]
fn a_directory_listing_that_failed_is_neither_clean_nor_unmeasured() {
    let stderr = "ls: cannot open directory '/srv/site': Permission denied";
    let refused = classify(&machine("203.0.113.9"), exited(2, stderr));
    let f = content_verdict("conf", "/srv/site", "blake3:x", listing_digest(refused))
        .expect("a listing the host refused must not be a clean verdict");
    assert_eq!(f.actual_hash, "ERROR");
    assert!(f.detail.contains("Permission denied"), "{}", f.detail);
}

#[test]
fn a_directory_listing_that_answered_is_compared_like_any_digest() {
    let listing = || {
        classify(
            &machine("203.0.113.9"),
            Ok(ExecOutput {
                exit_code: 0,
                stdout: "total 0\n".to_string(),
                stderr: String::new(),
            }),
        )
    };
    let digest = crate::tripwire::hasher::hash_string_or_sentinel("total 0\n");
    assert!(content_verdict("conf", "/srv/site", &digest, listing_digest(listing())).is_none());
    let changed = content_verdict(
        "conf",
        "/srv/site",
        "blake3:other",
        listing_digest(listing()),
    )
    .expect("a different listing is drift");
    assert_eq!(changed.actual_hash, digest);
}

// forjar#549: every detector reads through `read`, not only the file detector.

#[test]
fn an_unanswered_completion_check_is_unmeasured_not_a_failed_guard() {
    let guard = crate::core::types::Resource {
        resource_type: ResourceType::Task,
        machine: crate::core::types::MachineTarget::Single("web".to_string()),
        command: Some("exit 1".to_string()),
        completion_check: Some("true".to_string()),
        ..Default::default()
    };
    let f = super::task_check::check_task_drift("guard", &guard, &machine("203.0.113.9"))
        .expect("a guard nobody evaluated must not read as satisfied");
    assert!(f.is_unmeasured(), "{}: {}", f.actual_hash, f.detail);
    assert!(f.resource_type == ResourceType::Task);
}

#[test]
fn an_unanswered_docker_inspect_is_unmeasured_not_an_error() {
    let f = super::image::check_image_drift("img", "web", "sha256:abc", &machine("203.0.113.9"))
        .expect("an image nobody inspected must not read as deployed");
    assert!(f.is_unmeasured(), "{}: {}", f.actual_hash, f.detail);
    assert!(f.resource_type == ResourceType::Image);
}

// forjar#549, round two: the properties a review lane found no test for.

#[test]
fn a_remote_script_that_exits_255_over_ssh_is_read_as_unmeasured() {
    // The documented cost of the rule: ssh cannot tell its own 255 from the
    // script's, so the reading errs toward "not known", never toward an answer.
    let script_said = exited(255, "deploy-guard: refusing to run twice");
    assert!(why(classify(&machine("203.0.113.9"), script_said)).is_some());
}

#[test]
fn an_unanswered_state_query_is_unmeasured_not_an_error() {
    let service = super::tests_full::make_service_resource(Some("nginx"));
    let rl = crate::core::types::ResourceLock {
        resource_type: ResourceType::Service,
        status: crate::core::types::ResourceStatus::Converged,
        applied_at: None,
        duration_seconds: None,
        hash: "blake3:desired".to_string(),
        observed: None,
        details: std::collections::HashMap::new(),
    };
    let f =
        super::check_nonfile_drift("svc", &rl, &service, &machine("203.0.113.9"), "blake3:live")
            .expect("a service nobody queried must not read as converged");
    assert!(f.is_unmeasured(), "{}: {}", f.actual_hash, f.detail);
}
