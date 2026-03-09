//! Coverage tests for store_ops.rs — store list, gc, diff operations.

// ── cmd_store_list ──────────────────────────────────────────────────

#[test]
fn store_list_empty() {
    let store_dir = tempfile::tempdir().unwrap();
    let result = super::store_ops::cmd_store_list(store_dir.path(), false, false);
    assert!(result.is_ok());
}

#[test]
fn store_list_empty_json() {
    let store_dir = tempfile::tempdir().unwrap();
    let result = super::store_ops::cmd_store_list(store_dir.path(), false, true);
    assert!(result.is_ok());
}

#[test]
fn store_list_with_entries_no_meta() {
    let store_dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(store_dir.path().join("abc123")).unwrap();
    std::fs::create_dir_all(store_dir.path().join("def456")).unwrap();
    let result = super::store_ops::cmd_store_list(store_dir.path(), false, false);
    assert!(result.is_ok());
}

#[test]
fn store_list_with_entries_show_provider() {
    let store_dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(store_dir.path().join("abc123")).unwrap();
    let result = super::store_ops::cmd_store_list(store_dir.path(), true, false);
    assert!(result.is_ok());
}

#[test]
fn store_list_with_entries_json() {
    let store_dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(store_dir.path().join("abc123")).unwrap();
    let result = super::store_ops::cmd_store_list(store_dir.path(), true, true);
    assert!(result.is_ok());
}

#[test]
fn store_list_with_meta() {
    let store_dir = tempfile::tempdir().unwrap();
    let entry = store_dir.path().join("abc123");
    std::fs::create_dir_all(&entry).unwrap();
    std::fs::write(
        entry.join("meta.yaml"),
        r#"schema: "1"
store_hash: "blake3:abc123"
recipe_hash: "blake3:def456"
input_hashes: []
arch: "x86_64"
provider: "apt"
created_at: "2026-03-08T10:00:00Z"
generator: "forjar 1.0.0"
references: []
"#,
    )
    .unwrap();
    let result = super::store_ops::cmd_store_list(store_dir.path(), true, false);
    assert!(result.is_ok());
}

#[test]
fn store_list_with_meta_json() {
    let store_dir = tempfile::tempdir().unwrap();
    let entry = store_dir.path().join("abc123");
    std::fs::create_dir_all(&entry).unwrap();
    std::fs::write(
        entry.join("meta.yaml"),
        r#"schema: "1"
store_hash: "blake3:abc123"
recipe_hash: "blake3:def456"
input_hashes: []
arch: "x86_64"
provider: "cargo"
created_at: "2026-03-08T10:00:00Z"
generator: "forjar 1.0.0"
references: []
"#,
    )
    .unwrap();
    let result = super::store_ops::cmd_store_list(store_dir.path(), true, true);
    assert!(result.is_ok());
}

#[test]
fn store_list_skips_gc_roots() {
    let store_dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(store_dir.path().join(".gc-roots")).unwrap();
    std::fs::create_dir_all(store_dir.path().join("abc123")).unwrap();
    let result = super::store_ops::cmd_store_list(store_dir.path(), false, false);
    assert!(result.is_ok());
}

// ── cmd_store_gc ────────────────────────────────────────────────────

#[test]
fn store_gc_empty_dry_run() {
    let store_dir = tempfile::tempdir().unwrap();
    let state_dir = tempfile::tempdir().unwrap();
    let result =
        super::store_ops::cmd_store_gc(store_dir.path(), state_dir.path(), true, None, 3, false);
    assert!(result.is_ok());
}

#[test]
fn store_gc_empty_dry_run_json() {
    let store_dir = tempfile::tempdir().unwrap();
    let state_dir = tempfile::tempdir().unwrap();
    let result =
        super::store_ops::cmd_store_gc(store_dir.path(), state_dir.path(), true, None, 3, true);
    assert!(result.is_ok());
}

#[test]
fn store_gc_empty_sweep() {
    let store_dir = tempfile::tempdir().unwrap();
    let state_dir = tempfile::tempdir().unwrap();
    let result =
        super::store_ops::cmd_store_gc(store_dir.path(), state_dir.path(), false, None, 3, false);
    assert!(result.is_ok());
}

#[test]
fn store_gc_empty_sweep_json() {
    let store_dir = tempfile::tempdir().unwrap();
    let state_dir = tempfile::tempdir().unwrap();
    let result =
        super::store_ops::cmd_store_gc(store_dir.path(), state_dir.path(), false, None, 3, true);
    assert!(result.is_ok());
}

#[test]
fn store_gc_with_dead_entry_dry_run() {
    let store_dir = tempfile::tempdir().unwrap();
    let state_dir = tempfile::tempdir().unwrap();
    let entry = store_dir.path().join("deadbeef");
    std::fs::create_dir_all(&entry).unwrap();
    std::fs::write(entry.join("data"), b"some content").unwrap();
    let result =
        super::store_ops::cmd_store_gc(store_dir.path(), state_dir.path(), true, None, 3, false);
    assert!(result.is_ok());
}

#[test]
fn store_gc_with_dead_entry_dry_run_json() {
    let store_dir = tempfile::tempdir().unwrap();
    let state_dir = tempfile::tempdir().unwrap();
    let entry = store_dir.path().join("deadbeef");
    std::fs::create_dir_all(&entry).unwrap();
    std::fs::write(entry.join("data"), b"some content").unwrap();
    let result =
        super::store_ops::cmd_store_gc(store_dir.path(), state_dir.path(), true, None, 3, true);
    assert!(result.is_ok());
}

#[test]
fn store_gc_with_dead_entry_sweep() {
    let store_dir = tempfile::tempdir().unwrap();
    let state_dir = tempfile::tempdir().unwrap();
    let entry = store_dir.path().join("deadbeef");
    std::fs::create_dir_all(&entry).unwrap();
    std::fs::write(entry.join("data"), b"some content").unwrap();
    let result =
        super::store_ops::cmd_store_gc(store_dir.path(), state_dir.path(), false, None, 3, false);
    assert!(result.is_ok());
}

#[test]
fn store_gc_with_dead_entry_sweep_json() {
    let store_dir = tempfile::tempdir().unwrap();
    let state_dir = tempfile::tempdir().unwrap();
    let entry = store_dir.path().join("deadbeef");
    std::fs::create_dir_all(&entry).unwrap();
    std::fs::write(entry.join("data"), b"some content").unwrap();
    let result =
        super::store_ops::cmd_store_gc(store_dir.path(), state_dir.path(), false, None, 3, true);
    assert!(result.is_ok());
}

#[test]
fn store_gc_older_than() {
    let store_dir = tempfile::tempdir().unwrap();
    let state_dir = tempfile::tempdir().unwrap();
    let result = super::store_ops::cmd_store_gc(
        store_dir.path(),
        state_dir.path(),
        true,
        Some(30),
        3,
        false,
    );
    assert!(result.is_ok());
}

#[test]
fn store_gc_with_gc_roots_dir() {
    let store_dir = tempfile::tempdir().unwrap();
    let state_dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(store_dir.path().join(".gc-roots")).unwrap();
    let result =
        super::store_ops::cmd_store_gc(store_dir.path(), state_dir.path(), true, None, 3, false);
    assert!(result.is_ok());
}

#[test]
fn store_gc_with_lockfile() {
    let store_dir = tempfile::tempdir().unwrap();
    let state_dir = tempfile::tempdir().unwrap();
    std::fs::write(
        state_dir.path().join("forjar.inputs.lock.yaml"),
        r#"schema: "1"
pins:
  nginx:
    provider: apt
    hash: "blake3:abc123"
"#,
    )
    .unwrap();
    let result =
        super::store_ops::cmd_store_gc(store_dir.path(), state_dir.path(), true, None, 3, false);
    assert!(result.is_ok());
}

// ── cmd_store_diff ──────────────────────────────────────────────────

#[test]
fn store_diff_missing_entry() {
    let store_dir = tempfile::tempdir().unwrap();
    let result = super::store_ops::cmd_store_diff("blake3:nonexistent", store_dir.path(), false);
    assert!(result.is_err());
}

#[test]
fn store_diff_no_provenance() {
    let store_dir = tempfile::tempdir().unwrap();
    let entry = store_dir.path().join("abc123");
    std::fs::create_dir_all(&entry).unwrap();
    std::fs::write(
        entry.join("meta.yaml"),
        r#"schema: "1"
store_hash: "blake3:abc123"
recipe_hash: "blake3:def456"
input_hashes: []
arch: "x86_64"
provider: "apt"
created_at: "2026-03-08T10:00:00Z"
generator: "forjar 1.0.0"
references: []
"#,
    )
    .unwrap();
    let result = super::store_ops::cmd_store_diff("blake3:abc123", store_dir.path(), false);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("no provenance"));
}

#[test]
fn store_diff_with_provenance_text() {
    let store_dir = tempfile::tempdir().unwrap();
    let entry = store_dir.path().join("abc123");
    std::fs::create_dir_all(&entry).unwrap();
    std::fs::write(
        entry.join("meta.yaml"),
        r#"schema: "1"
store_hash: "blake3:abc123"
recipe_hash: "blake3:def456"
input_hashes: []
arch: "x86_64"
provider: "apt"
created_at: "2026-03-08T10:00:00Z"
generator: "forjar 1.0.0"
references: []
provenance:
  origin_provider: apt
  origin_ref: "nginx=1.24.0-1"
  origin_hash: "blake3:origin123"
"#,
    )
    .unwrap();
    let result = super::store_ops::cmd_store_diff("blake3:abc123", store_dir.path(), false);
    assert!(result.is_ok());
}

#[test]
fn store_diff_with_provenance_json() {
    let store_dir = tempfile::tempdir().unwrap();
    let entry = store_dir.path().join("abc123");
    std::fs::create_dir_all(&entry).unwrap();
    std::fs::write(
        entry.join("meta.yaml"),
        r#"schema: "1"
store_hash: "blake3:abc123"
recipe_hash: "blake3:def456"
input_hashes: []
arch: "x86_64"
provider: "apt"
created_at: "2026-03-08T10:00:00Z"
generator: "forjar 1.0.0"
references: []
provenance:
  origin_provider: apt
  origin_ref: "nginx=1.24.0-1"
  origin_hash: "blake3:origin123"
"#,
    )
    .unwrap();
    let result = super::store_ops::cmd_store_diff("blake3:abc123", store_dir.path(), true);
    assert!(result.is_ok());
}

#[test]
fn store_diff_hash_without_prefix() {
    let store_dir = tempfile::tempdir().unwrap();
    let entry = store_dir.path().join("abc123");
    std::fs::create_dir_all(&entry).unwrap();
    std::fs::write(
        entry.join("meta.yaml"),
        r#"schema: "1"
store_hash: "blake3:abc123"
recipe_hash: "blake3:def456"
input_hashes: []
arch: "x86_64"
provider: "cargo"
created_at: "2026-03-08T10:00:00Z"
generator: "forjar 1.0.0"
references: []
provenance:
  origin_provider: cargo
  origin_ref: "serde@1.0.200"
  origin_hash: "blake3:origin456"
"#,
    )
    .unwrap();
    // Pass hash without blake3: prefix
    let result = super::store_ops::cmd_store_diff("abc123", store_dir.path(), false);
    assert!(result.is_ok());
}

// ── cmd_store_sync (dry-run only) ───────────────────────────────────

#[test]
fn store_sync_dry_run_text() {
    let store_dir = tempfile::tempdir().unwrap();
    let entry = store_dir.path().join("abc123");
    std::fs::create_dir_all(&entry).unwrap();
    std::fs::write(
        entry.join("meta.yaml"),
        r#"schema: "1"
store_hash: "blake3:abc123"
recipe_hash: "blake3:def456"
input_hashes: []
arch: "x86_64"
provider: "apt"
created_at: "2026-03-08T10:00:00Z"
generator: "forjar 1.0.0"
references: []
provenance:
  origin_provider: apt
  origin_ref: "nginx=1.24.0-1"
  origin_hash: "blake3:origin123"
"#,
    )
    .unwrap();
    let result =
        super::store_ops::cmd_store_sync("blake3:abc123", store_dir.path(), false, false);
    assert!(result.is_ok());
}

#[test]
fn store_sync_dry_run_json() {
    let store_dir = tempfile::tempdir().unwrap();
    let entry = store_dir.path().join("abc123");
    std::fs::create_dir_all(&entry).unwrap();
    std::fs::write(
        entry.join("meta.yaml"),
        r#"schema: "1"
store_hash: "blake3:abc123"
recipe_hash: "blake3:def456"
input_hashes: []
arch: "x86_64"
provider: "apt"
created_at: "2026-03-08T10:00:00Z"
generator: "forjar 1.0.0"
references: []
provenance:
  origin_provider: apt
  origin_ref: "nginx=1.24.0-1"
  origin_hash: "blake3:origin123"
"#,
    )
    .unwrap();
    let result = super::store_ops::cmd_store_sync("blake3:abc123", store_dir.path(), false, true);
    assert!(result.is_ok());
}

#[test]
fn store_sync_dry_run_with_derivation() {
    let store_dir = tempfile::tempdir().unwrap();
    let entry = store_dir.path().join("abc123");
    std::fs::create_dir_all(&entry).unwrap();
    std::fs::write(
        entry.join("meta.yaml"),
        r#"schema: "1"
store_hash: "blake3:abc123"
recipe_hash: "blake3:def456"
input_hashes: []
arch: "x86_64"
provider: "apt"
created_at: "2026-03-08T10:00:00Z"
generator: "forjar 1.0.0"
references: []
provenance:
  origin_provider: apt
  origin_ref: "nginx=1.24.0-1"
  origin_hash: "blake3:origin123"
  derived_from: "blake3:base000"
"#,
    )
    .unwrap();
    let result =
        super::store_ops::cmd_store_sync("blake3:abc123", store_dir.path(), false, false);
    assert!(result.is_ok());
}
