//! Coverage tests for fleet_ops.rs — inventory with local/container machines.

use std::path::Path;

fn write_config(dir: &Path, yaml: &str) -> std::path::PathBuf {
    let file = dir.join("forjar.yaml");
    std::fs::write(&file, yaml).unwrap();
    file
}

// ── cmd_inventory ────────────────────────────────────────────────────

#[test]
fn inventory_local_text() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: inv
machines:
  web:
    hostname: web
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: web
    path: /tmp/cfg
    content: "hello"
"#,
    );
    let result = super::fleet_ops::cmd_inventory(&file, false);
    assert!(result.is_ok(), "failed: {result:?}");
}

#[test]
fn inventory_local_json() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: inv
machines:
  web:
    hostname: web
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: web
    path: /tmp/cfg
    content: "hello"
"#,
    );
    let result = super::fleet_ops::cmd_inventory(&file, true);
    assert!(result.is_ok(), "failed: {result:?}");
}

#[test]
fn inventory_localhost() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: inv
machines:
  local:
    hostname: local
    addr: localhost
resources:
  cfg:
    type: file
    machine: local
    path: /tmp/x
    content: "x"
"#,
    );
    let result = super::fleet_ops::cmd_inventory(&file, false);
    assert!(result.is_ok(), "failed: {result:?}");
}

#[test]
fn inventory_container_machine() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: inv
machines:
  ctr:
    hostname: ctr
    addr: container
    container:
      image: ubuntu:22.04
      runtime: docker
resources:
  app:
    type: file
    machine: ctr
    path: /tmp/app
    content: "x"
"#,
    );
    let result = super::fleet_ops::cmd_inventory(&file, false);
    assert!(result.is_ok(), "failed: {result:?}");
}

#[test]
fn inventory_container_transport() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: inv
machines:
  dock:
    hostname: dock
    addr: 10.0.0.5
    transport: container
    container:
      image: ubuntu:22.04
      runtime: docker
resources: {}
"#,
    );
    let result = super::fleet_ops::cmd_inventory(&file, true);
    assert!(result.is_ok(), "failed: {result:?}");
}

#[test]
fn inventory_empty_machines() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        "version: \"1.0\"\nname: empty\nmachines: {}\nresources: {}\n",
    );
    let result = super::fleet_ops::cmd_inventory(&file, false);
    assert!(result.is_ok(), "failed: {result:?}");
}

#[test]
fn inventory_multi_machine_local() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: multi
machines:
  web:
    hostname: web
    addr: 127.0.0.1
  db:
    hostname: db
    addr: 127.0.0.1
  cache:
    hostname: cache
    addr: localhost
resources:
  cfg-a:
    type: file
    machine: web
    path: /tmp/a
    content: "a"
  cfg-b:
    type: file
    machine: db
    path: /tmp/b
    content: "b"
  cfg-c:
    type: file
    machine: cache
    path: /tmp/c
    content: "c"
"#,
    );
    let result = super::fleet_ops::cmd_inventory(&file, false);
    assert!(result.is_ok(), "failed: {result:?}");
}

#[test]
fn inventory_multi_machine_json() {
    let dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: multi_json
machines:
  web:
    hostname: web
    addr: 127.0.0.1
  ctr:
    hostname: ctr
    addr: container
    container:
      image: ubuntu:22.04
      runtime: docker
resources:
  app:
    type: file
    machine: web
    path: /tmp/app
    content: "x"
"#,
    );
    let result = super::fleet_ops::cmd_inventory(&file, true);
    assert!(result.is_ok(), "failed: {result:?}");
}

// ── cmd_retry_failed ─────────────────────────────────────────────────

#[test]
fn retry_failed_no_event_logs() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: retry
machines:
  web:
    hostname: web
    addr: 127.0.0.1
resources:
  cfg:
    type: file
    machine: web
    path: /tmp/cfg
    content: "hello"
"#,
    );
    let result = super::fleet_ops::cmd_retry_failed(&file, state_dir.path(), &[], None);
    assert!(result.is_ok(), "failed: {result:?}");
}

// ── cmd_rolling ──────────────────────────────────────────────────────

#[test]
fn rolling_empty_machines() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        "version: \"1.0\"\nname: roll\nmachines: {}\nresources: {}\n",
    );
    let result = super::fleet_ops::cmd_rolling(&file, state_dir.path(), 2, &[], None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("no machines defined"));
}

// ── cmd_canary ───────────────────────────────────────────────────────

#[test]
fn canary_nonexistent_machine() {
    let dir = tempfile::tempdir().unwrap();
    let state_dir = tempfile::tempdir().unwrap();
    let file = write_config(
        dir.path(),
        r#"
version: "1.0"
name: canary
machines:
  web:
    hostname: web
    addr: 127.0.0.1
resources: {}
"#,
    );
    let result =
        super::fleet_ops::cmd_canary(&file, state_dir.path(), "nonexistent", false, &[], None);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found in config"));
}
