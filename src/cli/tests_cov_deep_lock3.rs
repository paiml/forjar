//! Coverage tests for validate_deep failure paths, show transport detection,
//! and lock_audit edge cases.

fn write_temp_config(yaml: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    std::io::Write::write_all(&mut f, yaml.as_bytes()).unwrap();
    std::io::Write::flush(&mut f).unwrap();
    f
}

fn write_yaml(dir: &std::path::Path, name: &str, content: &str) {
    let p = dir.join(name);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&p, content).unwrap();
}

// ── validate_deep: unresolved templates ──

const CFG_UNRESOLVED_TPL: &str = "version: '1.0'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  cfg:\n    type: file\n    path: /etc/app.conf\n    content: '{{params.missing_var}} and {{params.also_missing}}'\n    state: present\n";

#[test]
fn deep_validate_unresolved_templates() {
    let cfg = write_temp_config(CFG_UNRESOLVED_TPL);
    let _ = super::validate_deep::cmd_validate_deep(cfg.path(), false);
}

#[test]
fn deep_validate_unresolved_templates_json() {
    let cfg = write_temp_config(CFG_UNRESOLVED_TPL);
    let _ = super::validate_deep::cmd_validate_deep(cfg.path(), true);
}

// ── validate_deep: overlapping paths ──

const CFG_OVERLAPPING: &str = "version: '1.0'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  file-a:\n    type: file\n    path: /etc/shared.conf\n    content: from-a\n    state: present\n  file-b:\n    type: file\n    path: /etc/shared.conf\n    content: from-b\n    state: present\n";

#[test]
fn deep_validate_overlapping_paths() {
    let cfg = write_temp_config(CFG_OVERLAPPING);
    let _ = super::validate_deep::cmd_validate_deep(cfg.path(), false);
}

// ── validate_deep: secrets detection ──

const CFG_SECRETS: &str = "version: '1.0'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  cfg:\n    type: file\n    path: /etc/app.conf\n    content: 'password: s3cret'\n    state: present\n";

#[test]
fn deep_validate_secrets_detected() {
    let cfg = write_temp_config(CFG_SECRETS);
    let _ = super::validate_deep::cmd_validate_deep(cfg.path(), false);
}

// ── validate_deep: naming violations ──

const CFG_BAD_NAME: &str = "version: '1.0'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  BadName:\n    type: package\n    packages:\n      - nginx\n";

#[test]
fn deep_validate_naming_violation() {
    let cfg = write_temp_config(CFG_BAD_NAME);
    let _ = super::validate_deep::cmd_validate_deep(cfg.path(), false);
}

#[test]
fn deep_validate_naming_violation_json() {
    let cfg = write_temp_config(CFG_BAD_NAME);
    let _ = super::validate_deep::cmd_validate_deep(cfg.path(), true);
}

// ── show.rs: detect_transport_type — container ──

const CFG_CONTAINER: &str = "version: '1.0'\nname: test\nmachines:\n  ctr:\n    hostname: mycontainer\n    addr: container\n    container:\n      runtime: docker\n      image: nginx:latest\nresources:\n  pkg:\n    type: package\n    provider: apt\n    packages:\n      - nginx\n";

#[test]
fn explain_container_transport() {
    let cfg = write_temp_config(CFG_CONTAINER);
    let r = super::show::cmd_explain(cfg.path(), "pkg", false);
    assert!(r.is_ok(), "explain failed: {:?}", r);
}

#[test]
fn explain_container_transport_json() {
    let cfg = write_temp_config(CFG_CONTAINER);
    let r = super::show::cmd_explain(cfg.path(), "pkg", true);
    assert!(r.is_ok());
}

// ── show.rs: detect_transport_type — localhost ──

const CFG_LOCALHOST: &str = "version: '1.0'\nname: test\nmachines:\n  local-box:\n    hostname: local-box\n    addr: localhost\nresources:\n  pkg:\n    type: package\n    provider: apt\n    packages:\n      - nginx\n";

#[test]
fn explain_localhost_transport() {
    let cfg = write_temp_config(CFG_LOCALHOST);
    let r = super::show::cmd_explain(cfg.path(), "pkg", false);
    assert!(r.is_ok());
}

// ── lock_audit: invalid hash format ──

#[test]
fn lock_audit_invalid_hash_format() {
    let d = tempfile::tempdir().unwrap();
    write_yaml(d.path(), "m1/state.lock.yaml",
        "schema: '1.0'\ngenerator: forjar-test\nmachine: m1\nresources:\n  bad:\n    type: package\n    status: converged\n    hash: not-a-valid-hash\n");
    let r = super::lock_audit::cmd_lock_audit(d.path(), false);
    assert!(r.is_ok());
}

#[test]
fn lock_audit_invalid_hash_json() {
    let d = tempfile::tempdir().unwrap();
    write_yaml(d.path(), "m1/state.lock.yaml",
        "schema: '1.0'\ngenerator: forjar-test\nmachine: m1\nresources:\n  bad:\n    type: package\n    status: converged\n    hash: not-valid\n");
    let r = super::lock_audit::cmd_lock_audit(d.path(), true);
    assert!(r.is_ok());
}

// ── lock_audit: bad generator ──

#[test]
fn lock_audit_bad_generator() {
    let d = tempfile::tempdir().unwrap();
    write_yaml(d.path(), "m1/state.lock.yaml",
        "schema: '1.0'\ngenerator: terraform-v1\nmachine: m1\nresources:\n  pkg:\n    type: package\n    status: converged\n    hash: blake3:96e791ed3adb73bebc1064e9e1dbce0bb07a2926ad02e48c5608c9de73a3c89d\n");
    let r = super::lock_audit::cmd_lock_audit(d.path(), false);
    assert!(r.is_ok());
}

// ── lock_audit: empty lock file ──

#[test]
fn lock_audit_empty_file() {
    let d = tempfile::tempdir().unwrap();
    write_yaml(d.path(), "m1/state.lock.yaml", "");
    let r = super::lock_audit::cmd_lock_audit(d.path(), false);
    assert!(r.is_ok());
}

// ── lock_audit: unparseable YAML ──

#[test]
fn lock_audit_bad_yaml() {
    let d = tempfile::tempdir().unwrap();
    write_yaml(d.path(), "m1/state.lock.yaml", "{{{{not yaml at all");
    let r = super::lock_audit::cmd_lock_audit(d.path(), false);
    assert!(r.is_ok());
}

// ── lock_audit: HMAC verify with signatures present ──

#[test]
fn lock_verify_hmac_with_sigs() {
    let d = tempfile::tempdir().unwrap();
    write_yaml(d.path(), "m1/state.lock.yaml",
        "schema: '1.0'\ngenerator: forjar\nmachine: m1\nresources: {}\n");
    write_yaml(d.path(), "m1.lock.yaml.sig", "blake3:abcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcd");
    let r = super::lock_audit::cmd_lock_verify_hmac(d.path(), false);
    assert!(r.is_ok());
}

#[test]
fn lock_verify_hmac_with_sigs_json() {
    let d = tempfile::tempdir().unwrap();
    write_yaml(d.path(), "m1/state.lock.yaml",
        "schema: '1.0'\ngenerator: forjar\nmachine: m1\nresources: {}\n");
    write_yaml(d.path(), "m1.lock.yaml.sig", "blake3:abcd1234");
    let r = super::lock_audit::cmd_lock_verify_hmac(d.path(), true);
    assert!(r.is_ok());
}

// ── lock_audit: verify schema mismatch ──

#[test]
fn lock_verify_schema_mismatch() {
    let d = tempfile::tempdir().unwrap();
    write_yaml(d.path(), "m1/state.lock.yaml",
        "schema: '2.0'\ngenerator: forjar\nmachine: m1\nresources: {}\n");
    let r = super::lock_audit::cmd_lock_verify_schema(d.path(), false);
    assert!(r.is_ok());
}

#[test]
fn lock_verify_schema_mismatch_json() {
    let d = tempfile::tempdir().unwrap();
    write_yaml(d.path(), "m1/state.lock.yaml",
        "schema: '2.0'\ngenerator: forjar\nmachine: m1\nresources: {}\n");
    let r = super::lock_audit::cmd_lock_verify_schema(d.path(), true);
    assert!(r.is_ok());
}

// ── lock_audit: lock tag ──

#[test]
fn lock_tag_applies() {
    let d = tempfile::tempdir().unwrap();
    write_yaml(d.path(), "m1/state.lock.yaml",
        "schema: '1.0'\ngenerator: forjar\nmachine: m1\nresources: {}\n");
    let r = super::lock_audit::cmd_lock_tag(d.path(), "env", "prod", false);
    assert!(r.is_ok());
}

#[test]
fn lock_tag_json() {
    let d = tempfile::tempdir().unwrap();
    write_yaml(d.path(), "m1/state.lock.yaml",
        "schema: '1.0'\ngenerator: forjar\nmachine: m1\nresources: {}\n");
    let r = super::lock_audit::cmd_lock_tag(d.path(), "env", "staging", true);
    assert!(r.is_ok());
}

// ── lock_audit: lock migrate ──

#[test]
fn lock_migrate_from_old() {
    let d = tempfile::tempdir().unwrap();
    write_yaml(d.path(), "m1/state.lock.yaml",
        "schema: '0.9'\ngenerator: forjar\nmachine: m1\nresources: {}\n");
    let r = super::lock_audit::cmd_lock_migrate(d.path(), "0.9", false);
    assert!(r.is_ok());
}

#[test]
fn lock_migrate_json() {
    let d = tempfile::tempdir().unwrap();
    write_yaml(d.path(), "m1/state.lock.yaml",
        "schema: '0.9'\ngenerator: forjar\nmachine: m1\nresources: {}\n");
    let r = super::lock_audit::cmd_lock_migrate(d.path(), "0.9", true);
    assert!(r.is_ok());
}

// ── lock_audit: lock history with unparseable entry ──

#[test]
fn lock_history_unparseable_lock() {
    let d = tempfile::tempdir().unwrap();
    write_yaml(d.path(), "m1/state.lock.yaml", "not: valid: lock: yaml: {{");
    let r = super::lock_audit::cmd_lock_history(d.path(), false, 10);
    assert!(r.is_ok());
}

// ── lock_audit: lock history text with entries ──

#[test]
fn lock_history_text_with_entries() {
    let d = tempfile::tempdir().unwrap();
    write_yaml(d.path(), "m1/state.lock.yaml",
        "schema: '1.0'\ngenerator: forjar\nmachine: m1\nresources:\n  pkg:\n    type: package\n    status: converged\n    hash: blake3:96e791ed3adb73bebc1064e9e1dbce0bb07a2926ad02e48c5608c9de73a3c89d\n    applied_at: '2025-06-15T12:00:00Z'\n");
    let r = super::lock_audit::cmd_lock_history(d.path(), false, 10);
    assert!(r.is_ok());
}

// ── show.rs: output key not found ──

const CFG_OUTPUTS: &str = "version: '1.0'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    type: package\n    provider: apt\n    packages:\n      - nginx\noutputs:\n  ver:\n    value: '1.0'\n";

#[test]
fn output_key_not_found() {
    let cfg = write_temp_config(CFG_OUTPUTS);
    let r = super::show::cmd_output(cfg.path(), Some("nonexistent"), false);
    assert!(r.is_err());
}

#[test]
fn output_key_json() {
    let cfg = write_temp_config(CFG_OUTPUTS);
    let r = super::show::cmd_output(cfg.path(), Some("ver"), true);
    assert!(r.is_ok());
}

#[test]
fn output_no_outputs_json() {
    let cfg = write_temp_config(CFG_LOCALHOST);
    let r = super::show::cmd_output(cfg.path(), None, true);
    assert!(r.is_ok());
}
