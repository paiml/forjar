//! Coverage tests for validate_paths_b.rs — cron, env refs, naming, counts, paths.

use super::validate_paths_b::*;
use std::io::Write;

const BASE: &str = "version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\n";

fn write_cfg(yaml: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(yaml.as_bytes()).unwrap();
    f.flush().unwrap();
    f
}

// ── find_mount_conflicts ────────────────────────────────────────────

#[test]
fn mount_no_conflicts() {
    let paths = vec![
        ("a".into(), "/mnt/a".into()),
        ("b".into(), "/mnt/b".into()),
    ];
    assert!(find_mount_conflicts(&paths).is_empty());
}

#[test]
fn mount_exact_conflict() {
    let paths = vec![
        ("a".into(), "/data".into()),
        ("b".into(), "/data".into()),
    ];
    let c = find_mount_conflicts(&paths);
    assert_eq!(c.len(), 1);
}

#[test]
fn mount_parent_child_conflict() {
    let paths = vec![
        ("a".into(), "/data".into()),
        ("b".into(), "/data/sub".into()),
    ];
    let c = find_mount_conflicts(&paths);
    assert_eq!(c.len(), 1);
}

#[test]
fn mount_empty_list() {
    assert!(find_mount_conflicts(&[]).is_empty());
}

// ── cmd_validate_check_cron_syntax ──────────────────────────────────

#[test]
fn cron_valid() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  job:\n    machine: m1\n    type: cron\n    schedule: '0 5 * * *'\n    command: echo hi\n"
    ));
    assert!(cmd_validate_check_cron_syntax(f.path(), false).is_ok());
}

#[test]
fn cron_invalid_fields() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  job:\n    machine: m1\n    type: cron\n    schedule: '60 25 32 13 8'\n    command: echo hi\n"
    ));
    assert!(cmd_validate_check_cron_syntax(f.path(), false).is_ok());
}

#[test]
fn cron_wrong_field_count() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  job:\n    machine: m1\n    type: cron\n    schedule: '0 5 *'\n    command: echo hi\n"
    ));
    assert!(cmd_validate_check_cron_syntax(f.path(), false).is_ok());
}

#[test]
fn cron_json() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  job:\n    machine: m1\n    type: cron\n    schedule: '0 5 * * *'\n    command: echo hi\n"
    ));
    assert!(cmd_validate_check_cron_syntax(f.path(), true).is_ok());
}

#[test]
fn cron_range() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  job:\n    machine: m1\n    type: cron\n    schedule: '0-30 1-5 * * 1-5'\n    command: echo hi\n"
    ));
    assert!(cmd_validate_check_cron_syntax(f.path(), false).is_ok());
}

#[test]
fn cron_step() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  job:\n    machine: m1\n    type: cron\n    schedule: '*/15 * * * *'\n    command: echo hi\n"
    ));
    assert!(cmd_validate_check_cron_syntax(f.path(), false).is_ok());
}

// ── cmd_validate_check_env_refs ─────────────────────────────────────

#[test]
fn env_refs_no_refs() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  cfg:\n    machine: m1\n    type: file\n    path: /a\n    content: plain text\n"
    ));
    assert!(cmd_validate_check_env_refs(f.path(), false).is_ok());
}

#[test]
fn env_refs_json() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  cfg:\n    machine: m1\n    type: file\n    path: /a\n    content: plain\n"
    ));
    assert!(cmd_validate_check_env_refs(f.path(), true).is_ok());
}

// ── cmd_validate_check_resource_names ────────────────────────────────

#[test]
fn names_kebab_case_valid() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  my-app:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n"
    ));
    assert!(cmd_validate_check_resource_names(f.path(), false, "kebab-case").is_ok());
}

#[test]
fn names_kebab_case_violation() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  MyApp:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n"
    ));
    assert!(cmd_validate_check_resource_names(f.path(), false, "kebab-case").is_ok());
}

#[test]
fn names_prefix_match() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  web-nginx:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n"
    ));
    assert!(cmd_validate_check_resource_names(f.path(), false, "web-").is_ok());
}

#[test]
fn names_prefix_violation() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  db-postgres:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n"
    ));
    assert!(cmd_validate_check_resource_names(f.path(), false, "web-").is_ok());
}

#[test]
fn names_json() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  MyApp:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n"
    ));
    assert!(cmd_validate_check_resource_names(f.path(), true, "kebab-case").is_ok());
}

// ── cmd_validate_check_resource_count ────────────────────────────────

#[test]
fn count_within_limit() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n"
    ));
    assert!(cmd_validate_check_resource_count(f.path(), false, 10).is_ok());
}

#[test]
fn count_over_limit() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n  b:\n    machine: m1\n    type: file\n    path: /b\n    content: b\n  c:\n    machine: m1\n    type: file\n    path: /c\n    content: c\n"
    ));
    assert!(cmd_validate_check_resource_count(f.path(), false, 1).is_ok());
}

#[test]
fn count_json() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /a\n    content: a\n"
    ));
    assert!(cmd_validate_check_resource_count(f.path(), true, 10).is_ok());
}

// ── cmd_validate_check_duplicate_paths ───────────────────────────────

#[test]
fn dup_paths_none() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /etc/a.conf\n    content: a\n  b:\n    machine: m1\n    type: file\n    path: /etc/b.conf\n    content: b\n"
    ));
    assert!(cmd_validate_check_duplicate_paths(f.path(), false).is_ok());
}

#[test]
fn dup_paths_found() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /etc/shared.conf\n    content: a\n  b:\n    machine: m1\n    type: file\n    path: /etc/shared.conf\n    content: b\n"
    ));
    assert!(cmd_validate_check_duplicate_paths(f.path(), false).is_err());
}

#[test]
fn dup_paths_json() {
    let f = write_cfg(&format!(
        "{BASE}resources:\n  a:\n    machine: m1\n    type: file\n    path: /etc/shared.conf\n    content: a\n  b:\n    machine: m1\n    type: file\n    path: /etc/shared.conf\n    content: b\n"
    ));
    assert!(cmd_validate_check_duplicate_paths(f.path(), true).is_ok());
}
