//! Tests for mutation_types.rs — mutation operators, results, scores, reports.

use super::mutation_types::*;

#[test]
fn mutation_operator_display() {
    assert_eq!(MutationOperator::DeleteFile.to_string(), "delete_file");
    assert_eq!(MutationOperator::StopService.to_string(), "stop_service");
    assert_eq!(
        MutationOperator::RemovePackage.to_string(),
        "remove_package"
    );
}

#[test]
fn mutation_operator_description() {
    assert_eq!(
        MutationOperator::DeleteFile.description(),
        "Remove a managed file"
    );
}

#[test]
fn mutation_operator_applicable_types() {
    assert_eq!(MutationOperator::DeleteFile.applicable_types(), &["file"]);
    assert_eq!(
        MutationOperator::StopService.applicable_types(),
        &["service"]
    );
    assert_eq!(
        MutationOperator::RemovePackage.applicable_types(),
        &["package"]
    );
}

#[test]
fn mutation_operator_serde_roundtrip() {
    for op in [
        MutationOperator::DeleteFile,
        MutationOperator::ModifyContent,
        MutationOperator::StopService,
        MutationOperator::RemovePackage,
        MutationOperator::KillProcess,
    ] {
        let json = serde_json::to_string(&op).unwrap();
        let parsed: MutationOperator = serde_json::from_str(&json).unwrap();
        assert_eq!(op, parsed);
    }
}

#[test]
fn mutation_result_killed() {
    let r = MutationResult {
        resource_id: "nginx-config".into(),
        resource_type: "file".into(),
        operator: MutationOperator::DeleteFile,
        detected: true,
        reconverged: Some(true),
        duration_ms: 150,
        error: None,
    };
    assert!(r.is_killed());
    assert!(!r.is_survived());
    let display = r.to_string();
    assert!(display.contains("KILLED"));
    assert!(display.contains("nginx-config"));
}

#[test]
fn mutation_result_survived() {
    let r = MutationResult {
        resource_id: "curl-pkg".into(),
        resource_type: "package".into(),
        operator: MutationOperator::RemovePackage,
        detected: false,
        reconverged: None,
        duration_ms: 200,
        error: None,
    };
    assert!(r.is_survived());
    assert!(!r.is_killed());
    assert!(r.to_string().contains("SURVIVED"));
}

#[test]
fn mutation_score_perfect() {
    let score = MutationScore {
        total: 10,
        detected: 10,
        survived: 0,
        errored: 0,
    };
    assert!((score.score_pct() - 100.0).abs() < 0.01);
    assert_eq!(score.grade(), 'A');
}

#[test]
fn mutation_score_grade_boundaries() {
    let grade_a = MutationScore {
        total: 10,
        detected: 9,
        survived: 1,
        errored: 0,
    };
    assert_eq!(grade_a.grade(), 'A');

    let grade_b = MutationScore {
        total: 10,
        detected: 8,
        survived: 2,
        errored: 0,
    };
    assert_eq!(grade_b.grade(), 'B');

    let grade_c = MutationScore {
        total: 10,
        detected: 6,
        survived: 4,
        errored: 0,
    };
    assert_eq!(grade_c.grade(), 'C');

    let grade_f = MutationScore {
        total: 10,
        detected: 5,
        survived: 5,
        errored: 0,
    };
    assert_eq!(grade_f.grade(), 'F');
}

#[test]
fn mutation_score_empty() {
    let score = MutationScore::default();
    assert!((score.score_pct() - 100.0).abs() < 0.01);
    assert_eq!(score.grade(), 'A');
}

#[test]
fn mutation_score_display() {
    let score = MutationScore {
        total: 20,
        detected: 17,
        survived: 3,
        errored: 0,
    };
    let s = score.to_string();
    assert!(s.contains("85%"));
    assert!(s.contains("Grade B"));
    assert!(s.contains("17/20"));
}

#[test]
fn type_mutation_summary_display() {
    let s = TypeMutationSummary {
        resource_type: "file".into(),
        total: 12,
        detected: 12,
    };
    assert!((s.detection_pct() - 100.0).abs() < 0.01);
    let display = s.to_string();
    assert!(display.contains("file"));
    assert!(display.contains("12/12"));
}

#[test]
fn mutation_report_from_results() {
    let results = vec![
        MutationResult {
            resource_id: "config".into(),
            resource_type: "file".into(),
            operator: MutationOperator::DeleteFile,
            detected: true,
            reconverged: Some(true),
            duration_ms: 100,
            error: None,
        },
        MutationResult {
            resource_id: "config".into(),
            resource_type: "file".into(),
            operator: MutationOperator::ModifyContent,
            detected: true,
            reconverged: Some(true),
            duration_ms: 120,
            error: None,
        },
        MutationResult {
            resource_id: "nginx".into(),
            resource_type: "package".into(),
            operator: MutationOperator::RemovePackage,
            detected: false,
            reconverged: None,
            duration_ms: 200,
            error: None,
        },
    ];
    let report = MutationReport::from_results(results);
    assert_eq!(report.score.total, 3);
    assert_eq!(report.score.detected, 2);
    assert_eq!(report.score.survived, 1);
    assert_eq!(report.undetected.len(), 1);
    assert_eq!(report.undetected[0].resource_id, "nginx");
    assert_eq!(report.by_type.len(), 2);
}

#[test]
fn mutation_report_format_summary() {
    let results = vec![MutationResult {
        resource_id: "svc".into(),
        resource_type: "service".into(),
        operator: MutationOperator::StopService,
        detected: false,
        reconverged: None,
        duration_ms: 50,
        error: None,
    }];
    let report = MutationReport::from_results(results);
    let text = report.format_summary();
    assert!(text.contains("Grade F"));
    assert!(text.contains("Undetected"));
    assert!(text.contains("svc"));
}
