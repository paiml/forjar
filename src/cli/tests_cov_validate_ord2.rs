//! Coverage tests for cli/validate_ordering_ext.rs — naming, idempotency, content size, fan limit, gpu backend, when condition.

fn write_temp_config(yaml: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    std::io::Write::write_all(&mut f, yaml.as_bytes()).unwrap();
    std::io::Write::flush(&mut f).unwrap();
    f
}

const SIMPLE_CONFIG: &str = "version: '1.0'\nname: test\nmachines:\n  web1:\n    hostname: web1\n    addr: 127.0.0.1\nresources:\n  nginx:\n    type: package\n";
const MULTI_RESOURCE: &str = "version: '1.0'\nname: test\nmachines:\n  web1:\n    hostname: web1\n    addr: 127.0.0.1\nresources:\n  nginx:\n    type: package\n  my-config:\n    type: file\n    path: /etc/app.conf\n    content: hello\n  my-svc:\n    type: service\n    requires:\n      - nginx\n      - my-config\n";

// ── cmd_validate_check_resource_naming_convention_strict ──

#[test]
fn naming_strict_text() {
    let cfg = write_temp_config(SIMPLE_CONFIG);
    let r = super::validate_ordering_ext::cmd_validate_check_resource_naming_convention_strict(cfg.path(), false);
    assert!(r.is_ok());
}

#[test]
fn naming_strict_json() {
    let cfg = write_temp_config(SIMPLE_CONFIG);
    let r = super::validate_ordering_ext::cmd_validate_check_resource_naming_convention_strict(cfg.path(), true);
    assert!(r.is_ok());
}

#[test]
fn naming_strict_missing() {
    let r = super::validate_ordering_ext::cmd_validate_check_resource_naming_convention_strict(
        std::path::Path::new("/nonexistent"), false,
    );
    assert!(r.is_err());
}

#[test]
fn naming_strict_multi() {
    let cfg = write_temp_config(MULTI_RESOURCE);
    let r = super::validate_ordering_ext::cmd_validate_check_resource_naming_convention_strict(cfg.path(), false);
    assert!(r.is_ok());
}

// ── cmd_validate_check_resource_idempotency_annotations ──

#[test]
fn idempotency_text() {
    let cfg = write_temp_config(SIMPLE_CONFIG);
    let r = super::validate_ordering_ext::cmd_validate_check_resource_idempotency_annotations(cfg.path(), false);
    assert!(r.is_ok());
}

#[test]
fn idempotency_json() {
    let cfg = write_temp_config(SIMPLE_CONFIG);
    let r = super::validate_ordering_ext::cmd_validate_check_resource_idempotency_annotations(cfg.path(), true);
    assert!(r.is_ok());
}

#[test]
fn idempotency_multi() {
    let cfg = write_temp_config(MULTI_RESOURCE);
    let r = super::validate_ordering_ext::cmd_validate_check_resource_idempotency_annotations(cfg.path(), false);
    assert!(r.is_ok());
}

// ── cmd_validate_check_resource_content_size_limit ──

#[test]
fn content_size_text() {
    let cfg = write_temp_config(SIMPLE_CONFIG);
    let r = super::validate_ordering_ext::cmd_validate_check_resource_content_size_limit(cfg.path(), false);
    assert!(r.is_ok());
}

#[test]
fn content_size_json() {
    let cfg = write_temp_config(SIMPLE_CONFIG);
    let r = super::validate_ordering_ext::cmd_validate_check_resource_content_size_limit(cfg.path(), true);
    assert!(r.is_ok());
}

#[test]
fn content_size_with_content() {
    let cfg = write_temp_config(MULTI_RESOURCE);
    let r = super::validate_ordering_ext::cmd_validate_check_resource_content_size_limit(cfg.path(), false);
    assert!(r.is_ok());
}

// ── cmd_validate_check_resource_dependency_fan_limit ──

#[test]
fn fan_limit_text() {
    let cfg = write_temp_config(SIMPLE_CONFIG);
    let r = super::validate_ordering_ext::cmd_validate_check_resource_dependency_fan_limit(cfg.path(), false);
    assert!(r.is_ok());
}

#[test]
fn fan_limit_json() {
    let cfg = write_temp_config(SIMPLE_CONFIG);
    let r = super::validate_ordering_ext::cmd_validate_check_resource_dependency_fan_limit(cfg.path(), true);
    assert!(r.is_ok());
}

#[test]
fn fan_limit_with_deps() {
    let cfg = write_temp_config(MULTI_RESOURCE);
    let r = super::validate_ordering_ext::cmd_validate_check_resource_dependency_fan_limit(cfg.path(), false);
    assert!(r.is_ok());
}

// ── cmd_validate_check_resource_gpu_backend_consistency ──

#[test]
fn gpu_backend_text() {
    let cfg = write_temp_config(SIMPLE_CONFIG);
    let r = super::validate_ordering_ext::cmd_validate_check_resource_gpu_backend_consistency(cfg.path(), false);
    assert!(r.is_ok());
}

#[test]
fn gpu_backend_json() {
    let cfg = write_temp_config(SIMPLE_CONFIG);
    let r = super::validate_ordering_ext::cmd_validate_check_resource_gpu_backend_consistency(cfg.path(), true);
    assert!(r.is_ok());
}

// ── cmd_validate_check_resource_when_condition_syntax ──

#[test]
fn when_condition_text() {
    let cfg = write_temp_config(SIMPLE_CONFIG);
    let r = super::validate_ordering_ext::cmd_validate_check_resource_when_condition_syntax(cfg.path(), false);
    assert!(r.is_ok());
}

#[test]
fn when_condition_json() {
    let cfg = write_temp_config(SIMPLE_CONFIG);
    let r = super::validate_ordering_ext::cmd_validate_check_resource_when_condition_syntax(cfg.path(), true);
    assert!(r.is_ok());
}

#[test]
fn when_condition_multi() {
    let cfg = write_temp_config(MULTI_RESOURCE);
    let r = super::validate_ordering_ext::cmd_validate_check_resource_when_condition_syntax(cfg.path(), false);
    assert!(r.is_ok());
}
