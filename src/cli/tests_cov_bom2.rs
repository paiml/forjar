//! Coverage tests for cli/sbom.rs and cli/cbom.rs.

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
    hash: abc123def456
    applied_at: "2025-01-01T00:00:00Z"
    duration_seconds: 2.5
"#;

fn write_temp_config(yaml: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    std::io::Write::write_all(&mut f, yaml.as_bytes()).unwrap();
    std::io::Write::flush(&mut f).unwrap();
    f
}

fn setup_state(dir: &std::path::Path) {
    std::fs::create_dir_all(dir.join("web1")).unwrap();
    std::fs::write(dir.join("web1/state.lock.yaml"), LOCK_WEB1).unwrap();
}

const CONFIG: &str = "version: '1.0'\nname: test\nmachines:\n  web1:\n    hostname: web1\n    addr: 127.0.0.1\nresources:\n  nginx:\n    type: package\n    provider: apt\n    packages:\n      - nginx\n";

// ── cmd_sbom ──

#[test]
fn sbom_text() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    let cfg = write_temp_config(CONFIG);
    let r = super::sbom::cmd_sbom(cfg.path(), d.path(), false);
    assert!(r.is_ok());
}

#[test]
fn sbom_json() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    let cfg = write_temp_config(CONFIG);
    let r = super::sbom::cmd_sbom(cfg.path(), d.path(), true);
    assert!(r.is_ok());
}

#[test]
fn sbom_missing_config() {
    let d = tempfile::tempdir().unwrap();
    let r = super::sbom::cmd_sbom(std::path::Path::new("/nonexistent"), d.path(), false);
    assert!(r.is_err());
}

#[test]
fn sbom_no_state() {
    let d = tempfile::tempdir().unwrap();
    let cfg = write_temp_config(CONFIG);
    let r = super::sbom::cmd_sbom(cfg.path(), d.path(), false);
    assert!(r.is_ok());
}

// ── cmd_cbom ──

#[test]
fn cbom_text() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    let cfg = write_temp_config(CONFIG);
    let r = super::cbom::cmd_cbom(cfg.path(), d.path(), false);
    assert!(r.is_ok());
}

#[test]
fn cbom_json() {
    let d = tempfile::tempdir().unwrap();
    setup_state(d.path());
    let cfg = write_temp_config(CONFIG);
    let r = super::cbom::cmd_cbom(cfg.path(), d.path(), true);
    assert!(r.is_ok());
}

#[test]
fn cbom_missing_config() {
    let d = tempfile::tempdir().unwrap();
    let r = super::cbom::cmd_cbom(std::path::Path::new("/nonexistent"), d.path(), false);
    assert!(r.is_err());
}

#[test]
fn cbom_no_state() {
    let d = tempfile::tempdir().unwrap();
    let cfg = write_temp_config(CONFIG);
    let r = super::cbom::cmd_cbom(cfg.path(), d.path(), false);
    assert!(r.is_ok());
}
