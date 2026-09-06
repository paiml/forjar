//! Coverage tests for check.rs — filters, helpers, JSON formatting.
use std::path::Path;
fn write_config(dir: &Path, yaml: &str) -> std::path::PathBuf {
    let file = dir.join("forjar.yaml");
    std::fs::write(&file, yaml).unwrap();
    file
}

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
    let result = super::check::cmd_check(&file, None, None, None, std::path::Path::new("state"), false, true);
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
    let result = super::check::cmd_check(&file, None, None, None, std::path::Path::new("state"), true, true);
    assert!(result.is_ok());
}

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
    let result = super::check::cmd_check(&file, None, None, Some("web"), std::path::Path::new("state"), false, false);
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
    // PMAT-160: `check` resolves its selectors through the same resolver
    // `apply` does, so a tag naming nothing is now the FJ-2723 error rather
    // than "0 pass, 0 fail" at exit 0.
    let err = super::check::cmd_check(&file, None, None, Some("web"), std::path::Path::new("state"), false, false)
        .expect_err("a tag matching nothing is a typo, not an empty success");
    assert!(err.contains("web"), "{err}");
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
    let result = super::check::cmd_check(&file, None, None, Some("app"), std::path::Path::new("state"), true, false);
    // FJ-2720: the tag filter selected the resource and it is not converged,
    // so the honest verdict is a failure. The filter working is what this test
    // is for — an Ok here would mean check passed something that does not exist.
    assert!(result.is_err(), "tagged but unconverged resource must fail check");
}

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
        super::check::cmd_check(&file, None, Some("target-cfg"), None, std::path::Path::new("state"), false, false);
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
    // PMAT-160: refused, not emptied — see `check_tag_filter_no_match`.
    let err =
        super::check::cmd_check(&file, None, Some("nonexistent"), None, std::path::Path::new("state"), false, false)
            .expect_err("a -r matching nothing is a typo, not an empty success");
    assert!(err.contains("nonexistent"), "{err}");
    assert!(err.contains("cfg"), "the error must name what IS there: {err}");
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
        super::check::cmd_check(&file, Some("other"), None, None, std::path::Path::new("state"), false, false);
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
    let result = super::check::cmd_check(&file, None, None, None, std::path::Path::new("state"), false, false);
    assert!(result.is_ok());
}

#[test]
fn check_empty_resources_json() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        "version: \"1.0\"\nname: empty\nmachines: {}\nresources: {}\n",
    );
    let result = super::check::cmd_check(&file, None, None, None, std::path::Path::new("state"), true, false);
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
        super::check::cmd_check(&file, Some("local"), None, Some("web"), std::path::Path::new("state"), false, false);
    assert!(result.is_ok());
}

// ── check_resource_filters helper ───────────────────────────────────
