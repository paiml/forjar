//! FJ-012: Core executor tests — machine filter.

use super::test_fixtures::*;
use super::*;

#[test]
fn test_fj012_machine_filter() {
    let yaml = r#"
version: "1.0"
name: test
machines:
  a:
    hostname: a
    addr: 127.0.0.1
  b:
    hostname: b
    addr: 127.0.0.1
resources:
  r1:
    type: file
    machine: a
    path: /tmp/forjar-test-filter-a.txt
    content: "a"
  r2:
    type: file
    machine: b
    path: /tmp/forjar-test-filter-b.txt
    content: "b"
"#;
    let config: ForjarConfig = serde_yaml_ng::from_str(yaml).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let cfg = ApplyConfig {
        config: &config,
        state_dir: dir.path(),
        force: false,
        dry_run: false,
        machine_filter: Some("a"),
        resource_filter: None,
        tag_filter: None,
        group_filter: None,
        timeout_secs: None,
        force_unlock: false,
        progress: false,
        retry: 0,
        parallel: None,
        resource_timeout: None,
        rollback_on_failure: false,
        max_parallel: None,
        trace: false,
        run_id: None,
        refresh: false,
        force_tag: None,
    };
    let results = apply(&cfg).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].machine, "a");

    let _ = std::fs::remove_file("/tmp/forjar-test-filter-a.txt");
    let _ = std::fs::remove_file("/tmp/forjar-test-filter-b.txt");
}
