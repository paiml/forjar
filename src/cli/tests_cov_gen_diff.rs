//! Coverage tests for cli/generation.rs — cmd_generation_diff.

const LOCK_WEB1: &str = r#"schema: "1"
machine: web1
hostname: web1
generated_at: "2025-01-01T00:00:00Z"
generator: forjar-test
blake3_version: "1.0"
resources:
  nginx:
    type: package
    status: converged
    hash: abc123
    applied_at: "2025-01-01T00:00:00Z"
    duration_seconds: 1.0
  config:
    type: file
    status: converged
    hash: def456
    applied_at: "2025-01-01T00:00:00Z"
    duration_seconds: 0.5
"#;

const LOCK_WEB1_V2: &str = r#"schema: "1"
machine: web1
hostname: web1
generated_at: "2025-01-02T00:00:00Z"
generator: forjar-test
blake3_version: "1.0"
resources:
  nginx:
    type: package
    status: converged
    hash: abc123
    applied_at: "2025-01-02T00:00:00Z"
    duration_seconds: 1.0
  config:
    type: file
    status: converged
    hash: ghi789
    applied_at: "2025-01-02T00:00:00Z"
    duration_seconds: 0.5
  redis:
    type: package
    status: converged
    hash: jkl012
    applied_at: "2025-01-02T00:00:00Z"
    duration_seconds: 2.0
"#;

fn setup_gens(dir: &std::path::Path) {
    let gen_dir = dir.join("generations");
    // gen 1
    let g1 = gen_dir.join("1").join("web1");
    std::fs::create_dir_all(&g1).unwrap();
    std::fs::write(g1.join("state.lock.yaml"), LOCK_WEB1).unwrap();
    // gen 2 — modified config hash, added redis
    let g2 = gen_dir.join("2").join("web1");
    std::fs::create_dir_all(&g2).unwrap();
    std::fs::write(g2.join("state.lock.yaml"), LOCK_WEB1_V2).unwrap();
}

#[test]
fn gen_diff_text() {
    let d = tempfile::tempdir().unwrap();
    setup_gens(d.path());
    let r = super::generation::cmd_generation_diff(d.path(), 1, 2, false);
    assert!(r.is_ok());
}

#[test]
fn gen_diff_json() {
    let d = tempfile::tempdir().unwrap();
    setup_gens(d.path());
    let r = super::generation::cmd_generation_diff(d.path(), 1, 2, true);
    assert!(r.is_ok());
}

#[test]
fn gen_diff_identical() {
    let d = tempfile::tempdir().unwrap();
    setup_gens(d.path());
    let r = super::generation::cmd_generation_diff(d.path(), 1, 1, false);
    assert!(r.is_ok());
}

#[test]
fn gen_diff_missing_from() {
    let d = tempfile::tempdir().unwrap();
    setup_gens(d.path());
    let r = super::generation::cmd_generation_diff(d.path(), 99, 1, false);
    assert!(r.is_err());
    assert!(r.unwrap_err().contains("99"));
}

#[test]
fn gen_diff_missing_to() {
    let d = tempfile::tempdir().unwrap();
    setup_gens(d.path());
    let r = super::generation::cmd_generation_diff(d.path(), 1, 99, false);
    assert!(r.is_err());
    assert!(r.unwrap_err().contains("99"));
}

#[test]
fn gen_diff_no_gens_dir() {
    let d = tempfile::tempdir().unwrap();
    let r = super::generation::cmd_generation_diff(d.path(), 1, 2, false);
    assert!(r.is_err());
}

#[test]
fn gen_diff_reverse() {
    let d = tempfile::tempdir().unwrap();
    setup_gens(d.path());
    // Diff in reverse direction should show opposite actions
    let r = super::generation::cmd_generation_diff(d.path(), 2, 1, false);
    assert!(r.is_ok());
}
