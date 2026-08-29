//! Unit tests for `core::compliance_pack`.
//!
//! Extracted from the module so neither file clears the repo's 500-line
//! ceiling. The load-bearing ones are the `list_packs` cases: a directory it
//! cannot read must not be reported as a directory with no packs in it.

use super::*;

#[test]
fn parse_compliance_pack() {
    let yaml = r#"
name: test-pack
version: "1.0.0"
framework: CIS
description: "Test compliance pack"
rules:
  - id: "CIS-1.1"
    title: "Ensure root login disabled"
    severity: error
    controls: ["CIS 1.1.1"]
    type: assert
    resource_type: file
    field: owner
    expected: root
"#;
    let pack = parse_pack(yaml).unwrap();
    assert_eq!(pack.name, "test-pack");
    assert_eq!(pack.framework, "CIS");
    assert_eq!(pack.rules.len(), 1);
    assert_eq!(pack.rules[0].id, "CIS-1.1");
}

#[test]
fn parse_pack_deny_rule() {
    let yaml = r#"
name: deny-test
version: "1.0.0"
framework: SOC2
rules:
  - id: "SOC2-1"
    title: "No world-writable files"
    type: deny
    resource_type: file
    field: mode
    pattern: "777"
"#;
    let pack = parse_pack(yaml).unwrap();
    assert_eq!(pack.rules[0].id, "SOC2-1");
}

#[test]
fn evaluate_assert_passing() {
    let mut resources = HashMap::new();
    let mut fields = HashMap::new();
    fields.insert("type".into(), "file".into());
    fields.insert("owner".into(), "root".into());
    resources.insert("nginx-conf".into(), fields);

    let pack = CompliancePack {
        name: "test".into(),
        version: "1.0".into(),
        framework: "CIS".into(),
        description: None,
        rules: vec![ComplianceRule {
            id: "R1".into(),
            title: "Root owner".into(),
            description: None,
            severity: "error".into(),
            controls: vec!["CIS 1.1".into()],
            check: ComplianceCheck::Assert {
                resource_type: "file".into(),
                field: "owner".into(),
                expected: "root".into(),
            },
        }],
    };

    let result = evaluate_pack(&pack, &resources);
    assert_eq!(result.passed_count(), 1);
    assert_eq!(result.failed_count(), 0);
    assert!((result.pass_rate() - 100.0).abs() < f64::EPSILON);
}

#[test]
fn evaluate_assert_failing() {
    let mut resources = HashMap::new();
    let mut fields = HashMap::new();
    fields.insert("type".into(), "file".into());
    fields.insert("owner".into(), "nobody".into());
    resources.insert("bad-file".into(), fields);

    let (passed, _msg) = check_assert(&resources, "file", "owner", "root");
    assert!(!passed);
}

#[test]
fn evaluate_deny() {
    let mut resources = HashMap::new();
    let mut fields = HashMap::new();
    fields.insert("type".into(), "file".into());
    fields.insert("mode".into(), "777".into());
    resources.insert("bad-file".into(), fields);

    let (passed, _msg) = check_deny(&resources, "file", "mode", "777");
    assert!(!passed);
}

#[test]
fn evaluate_require() {
    let mut resources = HashMap::new();
    let mut fields = HashMap::new();
    fields.insert("type".into(), "file".into());
    resources.insert("no-owner".into(), fields);

    let (passed, _msg) = check_require(&resources, "file", "owner");
    assert!(!passed);
}

#[test]
fn evaluate_require_tag() {
    let mut resources = HashMap::new();
    let mut fields = HashMap::new();
    fields.insert("tags".into(), "config,web".into());
    resources.insert("r1".into(), fields);

    let (passed, _) = check_require_tag(&resources, "config");
    assert!(passed);

    let (passed, _) = check_require_tag(&resources, "security");
    assert!(!passed);
}

#[test]
fn list_packs_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let packs = list_packs(dir.path()).expect("an empty directory lists fine");
    assert!(packs.is_empty());
}

/// A directory that is not there declares no packs. This is the case
/// `forjar apply --policy-check` hits by default (`--policy-dir` defaults
/// to `policies`, which most projects do not have), so it must stay `Ok`.
#[test]
fn a_missing_directory_lists_no_packs_and_is_not_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let packs = list_packs(&dir.path().join("no-such-dir"))
        .expect("an absent policy directory declares no packs; it is not a failure");
    assert!(packs.is_empty());
}

/// A path that EXISTS and is not a directory cannot be listed, and saying
/// "no packs" about it is the same lie as saying it about an unreadable one.
#[test]
fn a_policy_dir_that_is_actually_a_file_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("policies");
    std::fs::write(&f, "name: not-a-directory").unwrap();
    let err = list_packs(&f)
        .expect_err("pointing --policy-dir at a file must not read as 'zero packs, compliant'");
    assert!(
        err.contains("policies"),
        "the error must name the path: {err}"
    );
}

/// The B2 unit: an unreadable directory is NOT an empty one.
///
/// Degrades to a no-op under a uid that can read a `chmod 000` directory
/// (root, CAP_DAC_OVERRIDE): the mode is the mechanism, not the property.
#[test]
#[cfg(unix)]
fn an_unreadable_directory_is_an_error_not_an_empty_listing() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let locked = dir.path().join("locked");
    std::fs::create_dir(&locked).unwrap();
    std::fs::write(locked.join("strict.yaml"), "name: strict").unwrap();
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();

    let listed = list_packs(&locked);
    let restore = std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755));

    if listed.as_ref().is_ok_and(|p| p == &["strict"]) {
        restore.unwrap();
        return;
    }
    let err = listed.expect_err(
        "an unreadable policy directory answered a listing — every pack inside it \
             would vanish and the gate would report compliant",
    );
    assert!(
        err.contains("locked"),
        "the error must name the path: {err}"
    );
    restore.unwrap();
}

#[test]
fn list_packs_with_files() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("cis.yaml"), "name: cis").unwrap();
    std::fs::write(dir.path().join("stig.yml"), "name: stig").unwrap();
    std::fs::write(dir.path().join("readme.txt"), "not a pack").unwrap();
    let packs = list_packs(dir.path()).expect("a readable directory lists");
    assert_eq!(packs, vec!["cis", "stig"]);
}

#[test]
fn load_pack_from_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("pack.yaml");
    std::fs::write(
        &path,
        r#"
name: file-pack
version: "1.0"
framework: STIG
rules: []
"#,
    )
    .unwrap();
    let pack = load_pack(&path).unwrap();
    assert_eq!(pack.name, "file-pack");
}

#[test]
fn pack_eval_empty() {
    let result = PackEvalResult {
        pack_name: "empty".into(),
        results: vec![],
    };
    assert_eq!(result.passed_count(), 0);
    assert_eq!(result.failed_count(), 0);
    assert!((result.pass_rate() - 100.0).abs() < f64::EPSILON);
}

#[test]
fn script_check_passes() {
    let (passed, _) = check_script("true");
    assert!(passed);
}

#[test]
fn script_check_fails() {
    let (passed, _) = check_script("false");
    assert!(!passed);
}
