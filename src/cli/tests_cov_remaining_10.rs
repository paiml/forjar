//! Coverage tests: show.rs, store_cache, store_ops, store_archive gaps (FJ-1372).

#![allow(unused_imports)]
use super::show::*;
use super::store_archive::*;
use super::store_cache::*;
use super::store_ops::*;
use std::path::{Path, PathBuf};

fn write_cfg(dir: &Path, yaml: &str) -> PathBuf {
    let p = dir.join("forjar.yaml");
    std::fs::write(&p, yaml).unwrap();
    p
}

fn two_resource_cfg() -> &'static str {
    r#"version: "1.0"
name: compare-src
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
resources:
  pkg:
    type: package
    machine: m1
    provider: apt
    packages: [curl]
  cfg:
    type: file
    machine: m1
    path: /etc/app.conf
    content: "hello"
"#
}

fn changed_cfg() -> &'static str {
    r#"version: "1.0"
name: compare-dst
machines:
  m1:
    hostname: box
    addr: 1.2.3.4
resources:
  cfg:
    type: file
    machine: m1
    path: /etc/app.conf
    content: "changed"
  svc:
    type: file
    machine: m1
    path: /etc/svc.conf
    content: "new-service"
"#
}

// ── cmd_compare ─────────────────────────────────────────────

#[test]
fn compare_text_added_removed_changed() {
    let d1 = tempfile::tempdir().unwrap();
    let d2 = tempfile::tempdir().unwrap();
    let f1 = write_cfg(d1.path(), two_resource_cfg());
    let f2 = write_cfg(d2.path(), changed_cfg());
    assert!(cmd_compare(&f1, &f2, false).is_ok());
}

#[test]
fn compare_json_output() {
    let d1 = tempfile::tempdir().unwrap();
    let d2 = tempfile::tempdir().unwrap();
    let f1 = write_cfg(d1.path(), two_resource_cfg());
    let f2 = write_cfg(d2.path(), changed_cfg());
    assert!(cmd_compare(&f1, &f2, true).is_ok());
}

#[test]
fn compare_identical_configs() {
    let d1 = tempfile::tempdir().unwrap();
    let d2 = tempfile::tempdir().unwrap();
    let f1 = write_cfg(d1.path(), two_resource_cfg());
    let f2 = write_cfg(d2.path(), two_resource_cfg());
    assert!(cmd_compare(&f1, &f2, false).is_ok());
}

#[test]
fn compare_invalid_file() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(d.path(), two_resource_cfg());
    assert!(cmd_compare(&f, Path::new("/nonexistent.yaml"), false).is_err());
}

// ── cmd_template ────────────────────────────────────────────

#[test]
fn template_text_with_vars() {
    let d = tempfile::tempdir().unwrap();
    let recipe = d.path().join("recipe.yaml");
    std::fs::write(
        &recipe,
        "name: {{inputs.app}}\nport: {{inputs.port}}\n",
    )
    .unwrap();
    assert!(cmd_template(&recipe, &["app=web".to_string(), "port=8080".to_string()], false).is_ok());
}

#[test]
fn template_json_output() {
    let d = tempfile::tempdir().unwrap();
    let recipe = d.path().join("recipe.yaml");
    std::fs::write(&recipe, "name: {{inputs.svc}}\n").unwrap();
    assert!(cmd_template(&recipe, &["svc=api".to_string()], true).is_ok());
}

#[test]
fn template_no_vars() {
    let d = tempfile::tempdir().unwrap();
    let recipe = d.path().join("recipe.yaml");
    std::fs::write(&recipe, "static: content\n").unwrap();
    assert!(cmd_template(&recipe, &[], false).is_ok());
}

#[test]
fn template_invalid_file() {
    assert!(cmd_template(Path::new("/nonexistent.yaml"), &[], false).is_err());
}

// ── cmd_policy ──────────────────────────────────────────────

#[test]
fn policy_deny_violation_returns_err() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(
        d.path(),
        r#"version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  cfg:
    type: file
    machine: m1
    path: /etc/app.conf
policies:
  - type: require
    message: "files must have owner"
    resource_type: file
    field: owner
"#,
    );
    let result = cmd_policy(&f, false);
    assert!(result.is_err());
}

#[test]
fn policy_deny_violation_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(
        d.path(),
        r#"version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  cfg:
    type: file
    machine: m1
    path: /etc/app.conf
policies:
  - type: require
    message: "files must have owner"
    resource_type: file
    field: owner
"#,
    );
    let result = cmd_policy(&f, true);
    assert!(result.is_err());
}

#[test]
fn policy_warn_only_passes() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(
        d.path(),
        r#"version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources:
  cfg:
    type: file
    machine: m1
    path: /etc/app.conf
policies:
  - type: warn
    message: "files should have owner"
    resource_type: file
    field: owner
"#,
    );
    assert!(cmd_policy(&f, false).is_ok());
}

// ── cmd_explain edge cases ──────────────────────────────────

#[test]
fn explain_with_deps_text() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(
        d.path(),
        r#"version: "1.0"
name: explain-test
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  base:
    type: file
    machine: local
    path: /tmp/base.txt
    content: "base"
  app:
    type: file
    machine: local
    path: /tmp/app.txt
    content: "app"
    depends_on: [base]
"#,
    );
    assert!(cmd_explain(&f, "app", false).is_ok());
}

#[test]
fn explain_ssh_transport() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(
        d.path(),
        r#"version: "1.0"
name: explain-ssh
machines:
  remote:
    hostname: web-1
    addr: 10.0.0.5
    ssh_key: /home/deploy/.ssh/id_ed25519
resources:
  cfg:
    type: file
    machine: remote
    path: /etc/app.conf
    content: "prod"
"#,
    );
    assert!(cmd_explain(&f, "cfg", false).is_ok());
}

#[test]
fn explain_ssh_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(
        d.path(),
        r#"version: "1.0"
name: explain-ssh
machines:
  remote:
    hostname: web-1
    addr: 10.0.0.5
    ssh_key: /home/deploy/.ssh/id_ed25519
resources:
  cfg:
    type: file
    machine: remote
    path: /etc/app.conf
    content: "prod"
    tags: [web]
    resource_group: frontend
"#,
    );
    assert!(cmd_explain(&f, "cfg", true).is_ok());
}

// ── cmd_output edge cases ───────────────────────────────────

#[test]
fn output_no_outputs_json() {
    let d = tempfile::tempdir().unwrap();
    let f = write_cfg(
        d.path(),
        r#"version: "1.0"
name: test
machines:
  m1:
    hostname: m1
    addr: 1.2.3.4
resources: {}
"#,
    );
    assert!(cmd_output(&f, None, true).is_ok());
}

// ── store_cache extra coverage ──────────────────────────────

#[test]
fn cache_verify_with_entries_text() {
    let d = tempfile::tempdir().unwrap();
    let store = d.path().join("store");
    // Create a store entry with content (hash won't match → failed verification)
    let entry = store.join("aabbccdd00112233445566778899aabbccddeeff0011223344556677889900aa");
    std::fs::create_dir_all(entry.join("content")).unwrap();
    std::fs::write(entry.join("content/test.txt"), "data").unwrap();

    let result = cmd_cache_verify(&store, false);
    // Hash mismatch → failed > 0 → Err
    assert!(result.is_err());
}

#[test]
fn cache_verify_with_entries_json() {
    let d = tempfile::tempdir().unwrap();
    let store = d.path().join("store");
    let entry = store.join("1122334455667788990011223344556677889900aabbccddeeff00112233aabb");
    std::fs::create_dir_all(entry.join("content")).unwrap();
    std::fs::write(entry.join("content/file"), "hello").unwrap();

    let result = cmd_cache_verify(&store, true);
    assert!(result.is_err());
}

#[test]
fn cache_verify_skips_gc_roots() {
    let d = tempfile::tempdir().unwrap();
    let store = d.path().join("store");
    std::fs::create_dir_all(store.join(".gc-roots")).unwrap();

    let result = cmd_cache_verify(&store, false);
    assert!(result.is_ok());
}

#[test]
fn cache_verify_skips_non_dirs() {
    let d = tempfile::tempdir().unwrap();
    let store = d.path().join("store");
    std::fs::create_dir_all(&store).unwrap();
    std::fs::write(store.join("README"), "ignore this").unwrap();

    let result = cmd_cache_verify(&store, false);
    assert!(result.is_ok());
}

#[test]
fn cache_list_with_gc_roots_dir() {
    let d = tempfile::tempdir().unwrap();
    let store = d.path().join("store");
    std::fs::create_dir_all(&store).unwrap();
    std::fs::create_dir_all(store.join(".gc-roots")).unwrap();
    // .gc-roots should be skipped in listing
    assert!(cmd_cache_list(&store, false).is_ok());
}

#[test]
fn cache_list_entry_no_meta() {
    let d = tempfile::tempdir().unwrap();
    let store = d.path().join("store");
    let entry = store.join("deadbeef00112233445566778899aabbccddeeff0011223344556677889900aa");
    std::fs::create_dir_all(entry.join("content")).unwrap();
    // No meta.yaml → provider/arch should be "unknown"
    assert!(cmd_cache_list(&store, false).is_ok());
}

#[test]
fn cache_list_entry_no_meta_json() {
    let d = tempfile::tempdir().unwrap();
    let store = d.path().join("store");
    let entry = store.join("deadbeef00112233445566778899aabbccddeeff0011223344556677889900aa");
    std::fs::create_dir_all(entry.join("content")).unwrap();
    assert!(cmd_cache_list(&store, true).is_ok());
}

// ── store_ops extra coverage ────────────────────────────────

#[test]
fn store_gc_with_gc_roots_dir() {
    let d = tempfile::tempdir().unwrap();
    let store = d.path().join("store");
    std::fs::create_dir_all(store.join(".gc-roots")).unwrap();
    let state = d.path().join("state");
    std::fs::create_dir_all(&state).unwrap();
    assert!(cmd_store_gc(&store, &state, true, None, 5, false).is_ok());
}

#[test]
fn store_gc_with_lock_hashes() {
    let d = tempfile::tempdir().unwrap();
    let store = d.path().join("store");
    std::fs::create_dir_all(&store).unwrap();
    let state = d.path().join("state");
    std::fs::create_dir_all(&state).unwrap();
    // Write a lock file to exercise collect_lock_hashes
    std::fs::write(
        state.join("forjar.inputs.lock.yaml"),
        r#"schema: "1.0"
generated_at: "2026-03-02T10:00:00Z"
pins:
  curl:
    name: curl
    provider: apt
    version: "7.88.1"
    hash: "blake3:aaa111"
"#,
    )
    .unwrap();
    assert!(cmd_store_gc(&store, &state, true, None, 5, false).is_ok());
}

#[test]
fn store_list_without_provider() {
    let d = tempfile::tempdir().unwrap();
    let store = d.path().join("store");
    let entry = store.join("aaaa111122223333444455556666777788889999000011112222333344445555");
    std::fs::create_dir_all(entry.join("content")).unwrap();
    // No meta.yaml → unknown provider
    assert!(cmd_store_list(&store, false, false).is_ok());
}

#[test]
fn store_gc_dry_run_text_with_entries() {
    let d = tempfile::tempdir().unwrap();
    let store = d.path().join("store");
    let entry = store.join("dead111122223333444455556666777788889999000011112222333344445555");
    std::fs::create_dir_all(entry.join("content")).unwrap();
    std::fs::write(entry.join("content/data"), "some data").unwrap();
    std::fs::write(
        entry.join("meta.yaml"),
        r#"schema: "1.0"
store_hash: "blake3:dead111122223333444455556666777788889999000011112222333344445555"
recipe_hash: "blake3:0000"
input_hashes: []
arch: "x86_64"
provider: "apt"
created_at: "2026-01-01T00:00:00Z"
generator: "forjar 1.0"
references: []
"#,
    )
    .unwrap();
    let state = d.path().join("state");
    std::fs::create_dir_all(&state).unwrap();
    // Entry is dead (no roots) → dry_run reports it
    assert!(cmd_store_gc(&store, &state, true, None, 5, false).is_ok());
}

// ── store_archive extra coverage ────────────────────────────

#[test]
fn archive_pack_with_real_entry() {
    let d = tempfile::tempdir().unwrap();
    let store = d.path().join("store");
    let hash = "aaaa111122223333444455556666777788889999000011112222333344445555";
    let entry = store.join(hash);
    std::fs::create_dir_all(entry.join("content")).unwrap();
    std::fs::write(entry.join("content/hello.txt"), "hello world").unwrap();
    std::fs::write(
        entry.join("meta.yaml"),
        r#"schema: "1.0"
store_hash: "blake3:aaaa111122223333444455556666777788889999000011112222333344445555"
recipe_hash: "blake3:treehash1234"
input_hashes: []
arch: "x86_64"
provider: "apt"
created_at: "2026-03-02T10:00:00Z"
generator: "forjar 1.0"
references: []
"#,
    )
    .unwrap();
    let out = d.path().join("output.far");
    let result = cmd_archive_pack(&format!("blake3:{hash}"), &store, Some(&out));
    assert!(result.is_ok());
    assert!(out.exists());
}

#[test]
fn archive_pack_default_output_name() {
    let d = tempfile::tempdir().unwrap();
    let store = d.path().join("store");
    let hash = "bbbb222233334444555566667777888899990000111122223333444455556666";
    let entry = store.join(hash);
    std::fs::create_dir_all(entry.join("content")).unwrap();
    std::fs::write(
        entry.join("meta.yaml"),
        r#"schema: "1.0"
store_hash: "blake3:bbbb222233334444555566667777888899990000111122223333444455556666"
recipe_hash: "blake3:0000"
input_hashes: []
arch: "x86_64"
provider: "cargo"
created_at: "2026-03-02T10:00:00Z"
generator: "forjar 1.0"
references: []
"#,
    )
    .unwrap();
    // Pack without explicit output → generates <hash>.far in cwd
    let result = cmd_archive_pack(&format!("blake3:{hash}"), &store, None);
    assert!(result.is_ok());
    // Clean up the generated file
    let _ = std::fs::remove_file(format!("{hash}.far"));
}

#[test]
fn archive_pack_no_meta() {
    let d = tempfile::tempdir().unwrap();
    let store = d.path().join("store");
    let hash = "cccc333344445555666677778888999900001111222233334444555566667777";
    let entry = store.join(hash);
    std::fs::create_dir_all(entry.join("content")).unwrap();
    // No meta.yaml → should error
    let result = cmd_archive_pack(&format!("blake3:{hash}"), &store, None);
    assert!(result.is_err());
}

#[test]
fn archive_pack_roundtrip_inspect() {
    let d = tempfile::tempdir().unwrap();
    let store = d.path().join("store");
    let hash = "dddd444455556666777788889999000011112222333344445555666677778888";
    let entry = store.join(hash);
    std::fs::create_dir_all(entry.join("content")).unwrap();
    std::fs::write(entry.join("content/data.bin"), vec![0u8; 64]).unwrap();
    std::fs::write(
        entry.join("meta.yaml"),
        r#"schema: "1.0"
store_hash: "blake3:dddd444455556666777788889999000011112222333344445555666677778888"
recipe_hash: "blake3:treehash"
input_hashes: []
arch: "aarch64"
provider: "nix"
created_at: "2026-03-02T10:00:00Z"
generator: "forjar 1.0"
references: []
"#,
    )
    .unwrap();
    let far_out = d.path().join("roundtrip.far");
    cmd_archive_pack(&format!("blake3:{hash}"), &store, Some(&far_out)).unwrap();

    // Now inspect + verify the packed FAR
    assert!(cmd_archive_inspect(&far_out, false).is_ok());
    assert!(cmd_archive_inspect(&far_out, true).is_ok());
    assert!(cmd_archive_verify(&far_out, false).is_ok());
    assert!(cmd_archive_verify(&far_out, true).is_ok());
}
