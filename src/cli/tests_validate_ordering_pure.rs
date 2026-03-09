//! Coverage tests for validate_ordering.rs pure functions.

use super::validate_ordering::*;
use crate::core::types;

fn minimal_config(yaml: &str) -> types::ForjarConfig {
    serde_yaml_ng::from_str(yaml).unwrap()
}

const BASE: &str = "version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\n";

// ── find_ordering_issues ────────────────────────────────────────────

#[test]
fn ordering_no_deps_no_issues() {
    let cfg = minimal_config(&format!(
        "{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n"
    ));
    assert!(find_ordering_issues(&cfg).is_empty());
}

#[test]
fn ordering_valid_deps() {
    let cfg = minimal_config(&format!(
        "{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    depends_on: [b]\n  b:\n    machine: m1\n    type: file\n    path: /b\n    content: b\n"
    ));
    assert!(find_ordering_issues(&cfg).is_empty());
}

#[test]
fn ordering_missing_dep() {
    let cfg = minimal_config(&format!(
        "{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    depends_on: [ghost]\n"
    ));
    let issues = find_ordering_issues(&cfg);
    assert_eq!(issues.len(), 1);
    assert!(issues[0].1.contains("non-existent"));
    assert!(issues[0].1.contains("ghost"));
}

#[test]
fn ordering_self_dependency() {
    let cfg = minimal_config(&format!(
        "{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    depends_on: [a]\n"
    ));
    let issues = find_ordering_issues(&cfg);
    assert!(issues.iter().any(|(_, r)| r.contains("self-dependency")));
}

#[test]
fn ordering_multiple_issues_sorted() {
    let cfg = minimal_config(&format!(
        "{BASE}resources:\n  z:\n    machine: m1\n    type: file\n    path: /z\n    content: z\n    depends_on: [missing]\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    depends_on: [a]\n"
    ));
    let issues = find_ordering_issues(&cfg);
    assert!(issues.len() >= 2);
    // Should be sorted by resource name
    assert!(issues[0].0 <= issues[1].0);
}

// ── find_missing_tags ───────────────────────────────────────────────

#[test]
fn missing_tags_all_have_tags() {
    let cfg = minimal_config(&format!(
        "{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    tags: [web]\n"
    ));
    assert!(find_missing_tags(&cfg).is_empty());
}

#[test]
fn missing_tags_some_missing() {
    let cfg = minimal_config(&format!(
        "{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n  b:\n    machine: m1\n    type: file\n    path: /b\n    content: b\n    tags: [core]\n"
    ));
    let missing = find_missing_tags(&cfg);
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].0, "a");
}

#[test]
fn missing_tags_empty_resources() {
    let cfg = minimal_config(&format!("{BASE}resources: {{}}\n"));
    assert!(find_missing_tags(&cfg).is_empty());
}

// ── find_naming_violations ──────────────────────────────────────────

#[test]
fn naming_valid_names() {
    let cfg = minimal_config(&format!(
        "{BASE}resources:\n  my-app:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n"
    ));
    assert!(find_naming_violations(&cfg).is_empty());
}

#[test]
fn naming_spaces() {
    let yaml = format!(
        "{BASE}resources:\n  'bad name':\n    machine: m1\n    type: file\n    path: /a\n    content: a\n"
    );
    let cfg = minimal_config(&yaml);
    let v = find_naming_violations(&cfg);
    assert!(v.iter().any(|(_, r)| r.contains("spaces")));
}

#[test]
fn naming_uppercase() {
    let cfg = minimal_config(&format!(
        "{BASE}resources:\n  MyApp:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n"
    ));
    let v = find_naming_violations(&cfg);
    assert!(v.iter().any(|(_, r)| r.contains("uppercase")));
}

#[test]
fn naming_double_underscore() {
    let cfg = minimal_config(&format!(
        "{BASE}resources:\n  bad__name:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n"
    ));
    let v = find_naming_violations(&cfg);
    assert!(v.iter().any(|(_, r)| r.contains("double underscore")));
}

// ── find_dependency_asymmetries ─────────────────────────────────────

#[test]
fn asymmetry_no_deps() {
    let cfg = minimal_config(&format!(
        "{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n"
    ));
    assert!(find_dependency_asymmetries(&cfg).is_empty());
}

#[test]
fn asymmetry_one_way_dep() {
    let cfg = minimal_config(&format!(
        "{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    depends_on: [b]\n  b:\n    machine: m1\n    type: file\n    path: /b\n    content: b\n"
    ));
    let asym = find_dependency_asymmetries(&cfg);
    assert_eq!(asym.len(), 1);
    assert_eq!(asym[0], ("a".to_string(), "b".to_string()));
}

#[test]
fn asymmetry_mutual_dep() {
    let cfg = minimal_config(&format!(
        "{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    depends_on: [b]\n  b:\n    machine: m1\n    type: file\n    path: /b\n    content: b\n    depends_on: [a]\n"
    ));
    // Mutual deps are symmetric, no asymmetry
    assert!(find_dependency_asymmetries(&cfg).is_empty());
}

// ── find_circular_aliases ───────────────────────────────────────────

#[test]
fn circular_none() {
    let cfg = minimal_config(&format!(
        "{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    depends_on: [b]\n  b:\n    machine: m1\n    type: file\n    path: /b\n    content: b\n"
    ));
    assert!(find_circular_aliases(&cfg).is_empty());
}

#[test]
fn circular_found() {
    let cfg = minimal_config(&format!(
        "{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    depends_on: [b]\n  b:\n    machine: m1\n    type: file\n    path: /b\n    content: b\n    depends_on: [a]\n"
    ));
    let cycles = find_circular_aliases(&cfg);
    assert_eq!(cycles.len(), 1);
    assert_eq!(cycles[0], ("a".to_string(), "b".to_string()));
}

// ── find_depth_limit_violations / compute_depth ─────────────────────

#[test]
fn depth_no_deps() {
    let cfg = minimal_config(&format!(
        "{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n"
    ));
    assert!(find_depth_limit_violations(&cfg, 5).is_empty());
}

#[test]
fn depth_within_limit() {
    let cfg = minimal_config(&format!(
        "{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    depends_on: [b]\n  b:\n    machine: m1\n    type: file\n    path: /b\n    content: b\n    depends_on: [c]\n  c:\n    machine: m1\n    type: file\n    path: /c\n    content: c\n"
    ));
    // Depth of a = 2, limit = 5
    assert!(find_depth_limit_violations(&cfg, 5).is_empty());
}

#[test]
fn depth_exceeds_limit() {
    // Chain: a→b→c→d→e→f (depth 5 for a)
    let cfg = minimal_config(&format!(
        "{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    depends_on: [b]\n  b:\n    machine: m1\n    type: file\n    path: /b\n    content: b\n    depends_on: [c]\n  c:\n    machine: m1\n    type: file\n    path: /c\n    content: c\n    depends_on: [d]\n  d:\n    machine: m1\n    type: file\n    path: /d\n    content: d\n    depends_on: [e]\n  e:\n    machine: m1\n    type: file\n    path: /e\n    content: e\n    depends_on: [f]\n  f:\n    machine: m1\n    type: file\n    path: /f\n    content: f\n"
    ));
    // Depth of a = 5, limit = 3
    let violations = find_depth_limit_violations(&cfg, 3);
    assert!(!violations.is_empty());
    assert!(violations.iter().any(|(n, _)| n == "a"));
}

#[test]
fn compute_depth_single() {
    let cfg = minimal_config(&format!(
        "{BASE}resources:\n  solo:\n    machine: m1\n    type: file\n    path: /s\n    content: s\n"
    ));
    let d = compute_depth(&cfg, "solo", &mut std::collections::HashSet::new());
    assert_eq!(d, 0);
}

#[test]
fn compute_depth_chain() {
    let cfg = minimal_config(&format!(
        "{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    depends_on: [b]\n  b:\n    machine: m1\n    type: file\n    path: /b\n    content: b\n    depends_on: [c]\n  c:\n    machine: m1\n    type: file\n    path: /c\n    content: c\n"
    ));
    let d = compute_depth(&cfg, "a", &mut std::collections::HashSet::new());
    assert_eq!(d, 2);
}

#[test]
fn compute_depth_nonexistent() {
    let cfg = minimal_config(&format!("{BASE}resources: {{}}\n"));
    let d = compute_depth(&cfg, "ghost", &mut std::collections::HashSet::new());
    assert_eq!(d, 0);
}

#[test]
fn compute_depth_cycle_protection() {
    // a→b→a cycle — should not infinite loop
    let cfg = minimal_config(&format!(
        "{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    depends_on: [b]\n  b:\n    machine: m1\n    type: file\n    path: /b\n    content: b\n    depends_on: [a]\n"
    ));
    let d = compute_depth(&cfg, "a", &mut std::collections::HashSet::new());
    // Cycle detected via visited set, returns safe value
    assert!(d <= 2);
}

// ── cmd_validate_check_resource_dependency_ordering ──────────────────

#[test]
fn cmd_ordering_valid_text() {
    let f = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        f.path(),
        format!("{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n"),
    )
    .unwrap();
    assert!(cmd_validate_check_resource_dependency_ordering(f.path(), false).is_ok());
}

#[test]
fn cmd_ordering_valid_json() {
    let f = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        f.path(),
        format!("{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n"),
    )
    .unwrap();
    assert!(cmd_validate_check_resource_dependency_ordering(f.path(), true).is_ok());
}

#[test]
fn cmd_ordering_with_issues_text() {
    let f = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        f.path(),
        format!("{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    depends_on: [ghost]\n"),
    )
    .unwrap();
    assert!(cmd_validate_check_resource_dependency_ordering(f.path(), false).is_ok());
}

// ── cmd_validate_check_resource_tag_completeness ─────────────────────

#[test]
fn cmd_tags_all_present_text() {
    let f = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        f.path(),
        format!("{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    tags: [web]\n"),
    )
    .unwrap();
    assert!(cmd_validate_check_resource_tag_completeness(f.path(), false).is_ok());
}

#[test]
fn cmd_tags_missing_text() {
    let f = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        f.path(),
        format!("{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n"),
    )
    .unwrap();
    assert!(cmd_validate_check_resource_tag_completeness(f.path(), false).is_ok());
}

#[test]
fn cmd_tags_json() {
    let f = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        f.path(),
        format!("{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n"),
    )
    .unwrap();
    assert!(cmd_validate_check_resource_tag_completeness(f.path(), true).is_ok());
}

// ── cmd_validate_check_resource_naming_standards ─────────────────────

#[test]
fn cmd_naming_valid_text() {
    let f = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        f.path(),
        format!("{BASE}resources:\n  my-app:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n"),
    )
    .unwrap();
    assert!(cmd_validate_check_resource_naming_standards(f.path(), false).is_ok());
}

#[test]
fn cmd_naming_violations_text() {
    let f = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        f.path(),
        format!("{BASE}resources:\n  Bad__Name:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n"),
    )
    .unwrap();
    assert!(cmd_validate_check_resource_naming_standards(f.path(), false).is_ok());
}

#[test]
fn cmd_naming_json() {
    let f = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        f.path(),
        format!("{BASE}resources:\n  Bad__Name:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n"),
    )
    .unwrap();
    assert!(cmd_validate_check_resource_naming_standards(f.path(), true).is_ok());
}

// ── cmd_validate_check_resource_dependency_symmetry ──────────────────

#[test]
fn cmd_symmetry_no_asymmetry() {
    let f = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        f.path(),
        format!("{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n"),
    )
    .unwrap();
    assert!(cmd_validate_check_resource_dependency_symmetry(f.path(), false).is_ok());
}

#[test]
fn cmd_symmetry_with_asymmetry_json() {
    let f = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        f.path(),
        format!("{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    depends_on: [b]\n  b:\n    machine: m1\n    type: file\n    path: /b\n    content: b\n"),
    )
    .unwrap();
    assert!(cmd_validate_check_resource_dependency_symmetry(f.path(), true).is_ok());
}

// ── cmd_validate_check_resource_circular_alias ───────────────────────

#[test]
fn cmd_circular_none_text() {
    let f = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        f.path(),
        format!("{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n"),
    )
    .unwrap();
    assert!(cmd_validate_check_resource_circular_alias(f.path(), false).is_ok());
}

#[test]
fn cmd_circular_found_json() {
    let f = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        f.path(),
        format!("{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n    depends_on: [b]\n  b:\n    machine: m1\n    type: file\n    path: /b\n    content: b\n    depends_on: [a]\n"),
    )
    .unwrap();
    assert!(cmd_validate_check_resource_circular_alias(f.path(), true).is_ok());
}

// ── cmd_validate_check_resource_dependency_depth_limit ───────────────

#[test]
fn cmd_depth_within_limit_text() {
    let f = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        f.path(),
        format!("{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n"),
    )
    .unwrap();
    assert!(cmd_validate_check_resource_dependency_depth_limit(f.path(), false).is_ok());
}

#[test]
fn cmd_depth_json() {
    let f = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        f.path(),
        format!("{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n"),
    )
    .unwrap();
    assert!(cmd_validate_check_resource_dependency_depth_limit(f.path(), true).is_ok());
}

// ── cmd_validate_check_resource_unused_params ────────────────────────

#[test]
fn cmd_unused_params_all_used() {
    let f = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        f.path(),
        format!("{BASE}params:\n  port: 8080\nresources:\n  cfg:\n    machine: m1\n    type: file\n    path: /a\n    content: \"{{{{port}}}}\"\n"),
    )
    .unwrap();
    assert!(cmd_validate_check_resource_unused_params(f.path(), false).is_ok());
}

#[test]
fn cmd_unused_params_some_unused() {
    let f = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        f.path(),
        format!("{BASE}params:\n  port: 8080\n  unused: val\nresources:\n  cfg:\n    machine: m1\n    type: file\n    path: /a\n    content: \"{{{{port}}}}\"\n"),
    )
    .unwrap();
    assert!(cmd_validate_check_resource_unused_params(f.path(), false).is_ok());
}

#[test]
fn cmd_unused_params_json() {
    let f = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        f.path(),
        format!("{BASE}params:\n  unused: val\nresources:\n  cfg:\n    machine: m1\n    type: file\n    path: /a\n    content: hi\n"),
    )
    .unwrap();
    assert!(cmd_validate_check_resource_unused_params(f.path(), true).is_ok());
}

// ── cmd_validate_check_resource_content_hash_consistency ─────────────

#[test]
fn cmd_hash_consistency_no_checksums() {
    let f = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        f.path(),
        format!("{BASE}resources:\n  cfg:\n    machine: m1\n    type: file\n    path: /a\n    content: hi\n"),
    )
    .unwrap();
    assert!(cmd_validate_check_resource_content_hash_consistency(f.path(), false).is_ok());
}

#[test]
fn cmd_hash_consistency_json() {
    let f = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(
        f.path(),
        format!("{BASE}resources:\n  cfg:\n    machine: m1\n    type: file\n    path: /a\n    content: hi\n"),
    )
    .unwrap();
    assert!(cmd_validate_check_resource_content_hash_consistency(f.path(), true).is_ok());
}
