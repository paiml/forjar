//! Coverage tests for cli/show.rs — cmd_show, cmd_explain, cmd_compare, cmd_policy, cmd_output.

fn write_temp_config(yaml: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    std::io::Write::write_all(&mut f, yaml.as_bytes()).unwrap();
    std::io::Write::flush(&mut f).unwrap();
    f
}

const CONFIG: &str = "version: '1.0'\nname: test\nmachines:\n  web1:\n    hostname: web1\n    addr: 127.0.0.1\nresources:\n  nginx:\n    type: package\n    provider: apt\n    packages:\n      - nginx\n  my-config:\n    type: file\n    path: /etc/app.conf\n    content: hello\n    depends_on:\n      - nginx\n";

const CONFIG2: &str = "version: '1.0'\nname: test2\nmachines:\n  web1:\n    hostname: web1\n    addr: 127.0.0.1\nresources:\n  nginx:\n    type: package\n    provider: apt\n    packages:\n      - nginx\n      - nginx-extras\n";

const CONFIG_WITH_POLICY: &str = "version: '1.0'\nname: test\npolicy:\n  require_tags: false\n  max_resources: 100\nmachines:\n  web1:\n    hostname: web1\n    addr: 127.0.0.1\nresources:\n  nginx:\n    type: package\n    provider: apt\n    packages:\n      - nginx\n";

const CONFIG_WITH_OUTPUTS: &str = "version: '1.0'\nname: test\nmachines:\n  web1:\n    hostname: web1\n    addr: 127.0.0.1\nresources:\n  nginx:\n    type: package\n    provider: apt\n    packages:\n      - nginx\noutputs:\n  nginx_version:\n    value: \"1.24\"\n  server_name:\n    value: web1\n";

// ── cmd_show ──

#[test]
fn show_text() {
    let cfg = write_temp_config(CONFIG);
    let r = super::show::cmd_show(cfg.path(), None, false);
    assert!(r.is_ok());
}

#[test]
fn show_json() {
    let cfg = write_temp_config(CONFIG);
    let r = super::show::cmd_show(cfg.path(), None, true);
    assert!(r.is_ok());
}

#[test]
fn show_resource_filter() {
    let cfg = write_temp_config(CONFIG);
    let r = super::show::cmd_show(cfg.path(), Some("nginx"), false);
    assert!(r.is_ok());
}

#[test]
fn show_resource_filter_json() {
    let cfg = write_temp_config(CONFIG);
    let r = super::show::cmd_show(cfg.path(), Some("nginx"), true);
    assert!(r.is_ok());
}

#[test]
fn show_resource_not_found() {
    let cfg = write_temp_config(CONFIG);
    let r = super::show::cmd_show(cfg.path(), Some("nonexistent"), false);
    assert!(r.is_err());
}

#[test]
fn show_missing_file() {
    let r = super::show::cmd_show(std::path::Path::new("/nonexistent/f.yaml"), None, false);
    assert!(r.is_err());
}

// ── cmd_explain ──

#[test]
fn explain_text() {
    let cfg = write_temp_config(CONFIG);
    let r = super::show::cmd_explain(cfg.path(), "nginx", false);
    assert!(r.is_ok());
}

#[test]
fn explain_json() {
    let cfg = write_temp_config(CONFIG);
    let r = super::show::cmd_explain(cfg.path(), "nginx", true);
    assert!(r.is_ok());
}

#[test]
fn explain_with_deps() {
    let cfg = write_temp_config(CONFIG);
    let r = super::show::cmd_explain(cfg.path(), "my-config", false);
    assert!(r.is_ok());
}

#[test]
fn explain_not_found() {
    let cfg = write_temp_config(CONFIG);
    let r = super::show::cmd_explain(cfg.path(), "nonexistent", false);
    assert!(r.is_err());
}

// ── cmd_compare ──

#[test]
fn compare_text() {
    let cfg1 = write_temp_config(CONFIG);
    let cfg2 = write_temp_config(CONFIG2);
    let r = super::show::cmd_compare(cfg1.path(), cfg2.path(), false);
    assert!(r.is_ok());
}

#[test]
fn compare_json() {
    let cfg1 = write_temp_config(CONFIG);
    let cfg2 = write_temp_config(CONFIG2);
    let r = super::show::cmd_compare(cfg1.path(), cfg2.path(), true);
    assert!(r.is_ok());
}

#[test]
fn compare_identical() {
    let cfg1 = write_temp_config(CONFIG);
    let cfg2 = write_temp_config(CONFIG);
    let r = super::show::cmd_compare(cfg1.path(), cfg2.path(), false);
    assert!(r.is_ok());
}

// ── cmd_policy ──

#[test]
fn policy_text() {
    let cfg = write_temp_config(CONFIG_WITH_POLICY);
    let r = super::show::cmd_policy(cfg.path(), false);
    assert!(r.is_ok());
}

#[test]
fn policy_json() {
    let cfg = write_temp_config(CONFIG_WITH_POLICY);
    let r = super::show::cmd_policy(cfg.path(), true);
    assert!(r.is_ok());
}

#[test]
fn policy_no_policy_section() {
    let cfg = write_temp_config(CONFIG);
    let r = super::show::cmd_policy(cfg.path(), false);
    assert!(r.is_ok());
}

// ── cmd_output ──

#[test]
fn output_text() {
    let cfg = write_temp_config(CONFIG_WITH_OUTPUTS);
    let r = super::show::cmd_output(cfg.path(), None, false);
    assert!(r.is_ok());
}

#[test]
fn output_json() {
    let cfg = write_temp_config(CONFIG_WITH_OUTPUTS);
    let r = super::show::cmd_output(cfg.path(), None, true);
    assert!(r.is_ok());
}

#[test]
fn output_key_filter() {
    let cfg = write_temp_config(CONFIG_WITH_OUTPUTS);
    let r = super::show::cmd_output(cfg.path(), Some("nginx_version"), false);
    assert!(r.is_ok());
}

#[test]
fn output_no_outputs() {
    let cfg = write_temp_config(CONFIG);
    let r = super::show::cmd_output(cfg.path(), None, false);
    assert!(r.is_ok());
}
