//! Coverage tests for check.rs — verbose, tag filter, resource filter, no-check-script.

use std::path::Path;

fn write_config(dir: &Path, yaml: &str) -> std::path::PathBuf {
    let file = dir.join("forjar.yaml");
    std::fs::write(&file, yaml).unwrap();
    file
}

// ── verbose mode ────────────────────────────────────────────────────

#[test]
fn check_verbose_mode() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("verbose-test.txt");
    std::fs::write(&target, "hello").unwrap();
    let file = write_config(
        dir.path(),
        &format!(
            r#"
version: "1.0"
name: verbose-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: local
    path: {}
    content: hello
"#,
            target.display()
        ),
    );
    let result = super::check::cmd_check(&file, None, None, None, false, true);
    assert!(result.is_ok());
}

#[test]
fn check_verbose_json() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("verbose-json.txt");
    std::fs::write(&target, "hello").unwrap();
    let file = write_config(
        dir.path(),
        &format!(
            r#"
version: "1.0"
name: verbose-json
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: local
    path: {}
    content: hello
"#,
            target.display()
        ),
    );
    let result = super::check::cmd_check(&file, None, None, None, true, true);
    assert!(result.is_ok());
}

// ── tag filter ──────────────────────────────────────────────────────

#[test]
fn check_tag_filter_match() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("tag-match.txt");
    std::fs::write(&target, "hello").unwrap();
    let file = write_config(
        dir.path(),
        &format!(
            r#"
version: "1.0"
name: tag-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: local
    path: {}
    content: hello
    tags: [web, critical]
"#,
            target.display()
        ),
    );
    let result = super::check::cmd_check(&file, None, None, Some("web"), false, false);
    assert!(result.is_ok());
}

#[test]
fn check_tag_filter_no_match() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("tag-no-match.txt");
    std::fs::write(&target, "hello").unwrap();
    let file = write_config(
        dir.path(),
        &format!(
            r#"
version: "1.0"
name: tag-no-match
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: local
    path: {}
    content: hello
    tags: [database]
"#,
            target.display()
        ),
    );
    // tag "web" doesn't match "database" → all skipped
    let result = super::check::cmd_check(&file, None, None, Some("web"), false, false);
    assert!(result.is_ok());
}

#[test]
fn check_tag_filter_json() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: tag-json
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: local
    path: /tmp/forjar-check-tag-json.txt
    content: hello
    tags: [app]
"#,
    );
    let result = super::check::cmd_check(&file, None, None, Some("app"), true, false);
    assert!(result.is_ok());
}

// ── resource filter ─────────────────────────────────────────────────

#[test]
fn check_resource_filter_match() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("res-match.txt");
    std::fs::write(&target, "hello").unwrap();
    let file = write_config(
        dir.path(),
        &format!(
            r#"
version: "1.0"
name: res-filter
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  target-cfg:
    type: file
    machine: local
    path: {}
    content: hello
  other-cfg:
    type: file
    machine: local
    path: /tmp/forjar-check-other.txt
    content: other
"#,
            target.display()
        ),
    );
    let result =
        super::check::cmd_check(&file, None, Some("target-cfg"), None, false, false);
    assert!(result.is_ok());
}

#[test]
fn check_resource_filter_no_match() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: res-no-match
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: local
    path: /tmp/forjar-check-no-match.txt
    content: hello
"#,
    );
    // resource "nonexistent" doesn't match "cfg" → all filtered out
    let result =
        super::check::cmd_check(&file, None, Some("nonexistent"), None, false, false);
    assert!(result.is_ok());
}

// ── machine filter ──────────────────────────────────────────────────

#[test]
fn check_machine_filter_no_match() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: machine-no-match
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: local
    path: /tmp/forjar-check-mf.txt
    content: hello
"#,
    );
    // machine "other" doesn't match "local" → all skipped
    let result =
        super::check::cmd_check(&file, Some("other"), None, None, false, false);
    assert!(result.is_ok());
}

// ── empty resources ─────────────────────────────────────────────────

#[test]
fn check_empty_resources() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        "version: \"1.0\"\nname: empty\nmachines: {}\nresources: {}\n",
    );
    let result = super::check::cmd_check(&file, None, None, None, false, false);
    assert!(result.is_ok());
}

#[test]
fn check_empty_resources_json() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        "version: \"1.0\"\nname: empty\nmachines: {}\nresources: {}\n",
    );
    let result = super::check::cmd_check(&file, None, None, None, true, false);
    assert!(result.is_ok());
}

// ── combined filters ────────────────────────────────────────────────

#[test]
fn check_combined_tag_and_machine_filter() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("combined.txt");
    std::fs::write(&target, "hello").unwrap();
    let file = write_config(
        dir.path(),
        &format!(
            r#"
version: "1.0"
name: combined
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: local
    path: {}
    content: hello
    tags: [web]
"#,
            target.display()
        ),
    );
    let result =
        super::check::cmd_check(&file, Some("local"), None, Some("web"), false, false);
    assert!(result.is_ok());
}

// ── check_resource_filters helper ───────────────────────────────────

#[test]
fn resource_filter_no_filters() {
    let resource: crate::core::types::Resource =
        serde_yaml_ng::from_str("type: file\npath: /tmp/x\ncontent: x\n").unwrap();
    let (skip, count) = super::check::check_resource_filters("r1", &resource, None, None);
    assert!(!skip);
    assert!(!count);
}

#[test]
fn resource_filter_name_match() {
    let resource: crate::core::types::Resource =
        serde_yaml_ng::from_str("type: file\npath: /tmp/x\ncontent: x\n").unwrap();
    let (skip, count) =
        super::check::check_resource_filters("cfg", &resource, Some("cfg"), None);
    assert!(!skip);
    assert!(!count);
}

#[test]
fn resource_filter_name_mismatch() {
    let resource: crate::core::types::Resource =
        serde_yaml_ng::from_str("type: file\npath: /tmp/x\ncontent: x\n").unwrap();
    let (skip, count) =
        super::check::check_resource_filters("cfg", &resource, Some("other"), None);
    assert!(skip);
    assert!(!count);
}

#[test]
fn resource_filter_tag_match() {
    let resource: crate::core::types::Resource =
        serde_yaml_ng::from_str("type: file\npath: /tmp/x\ncontent: x\ntags: [web, app]\n")
            .unwrap();
    let (skip, count) =
        super::check::check_resource_filters("cfg", &resource, None, Some("web"));
    assert!(!skip);
    assert!(!count);
}

#[test]
fn resource_filter_tag_mismatch() {
    let resource: crate::core::types::Resource =
        serde_yaml_ng::from_str("type: file\npath: /tmp/x\ncontent: x\ntags: [db]\n").unwrap();
    let (skip, count) =
        super::check::check_resource_filters("cfg", &resource, None, Some("web"));
    assert!(skip);
    assert!(count);
}

// ── skip_machine helper ─────────────────────────────────────────────

#[test]
fn skip_machine_no_filter() {
    let resource: crate::core::types::Resource =
        serde_yaml_ng::from_str("type: file\npath: /tmp/x\ncontent: x\n").unwrap();
    let machine = super::check::localhost_machine();
    assert!(!super::check::skip_machine("local", None, &resource, &machine));
}

#[test]
fn skip_machine_filter_match() {
    let resource: crate::core::types::Resource =
        serde_yaml_ng::from_str("type: file\npath: /tmp/x\ncontent: x\n").unwrap();
    let machine = super::check::localhost_machine();
    assert!(!super::check::skip_machine(
        "local",
        Some("local"),
        &resource,
        &machine
    ));
}

#[test]
fn skip_machine_filter_mismatch() {
    let resource: crate::core::types::Resource =
        serde_yaml_ng::from_str("type: file\npath: /tmp/x\ncontent: x\n").unwrap();
    let machine = super::check::localhost_machine();
    assert!(super::check::skip_machine(
        "local",
        Some("other"),
        &resource,
        &machine
    ));
}

#[test]
fn skip_machine_arch_mismatch() {
    let resource: crate::core::types::Resource =
        serde_yaml_ng::from_str("type: file\npath: /tmp/x\ncontent: x\narch: [aarch64]\n")
            .unwrap();
    let machine = super::check::localhost_machine(); // arch is x86_64
    assert!(super::check::skip_machine(
        "local",
        None,
        &resource,
        &machine
    ));
}

#[test]
fn skip_machine_arch_match() {
    let resource: crate::core::types::Resource =
        serde_yaml_ng::from_str("type: file\npath: /tmp/x\ncontent: x\narch: [x86_64]\n")
            .unwrap();
    let machine = super::check::localhost_machine();
    assert!(!super::check::skip_machine(
        "local",
        None,
        &resource,
        &machine
    ));
}
