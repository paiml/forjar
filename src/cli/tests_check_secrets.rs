//! forjar#1011 (infra#4181): `apply --check` resolves `{{secrets.*}}`
//! through the manifest's `secrets:` provider, exactly as `apply` does.
//!
//! The provider is `file` over a temp dir, so no real sops or env var is needed:
//! a check that ignores the manifest's provider falls back to the env provider
//! and fails `secret 'demo' not found`.

fn secret_fixture(dir: &std::path::Path) -> std::path::PathBuf {
    let secrets = dir.join("secrets");
    std::fs::create_dir_all(&secrets).unwrap();
    std::fs::write(secrets.join("forjar_p0_check_demo"), "dummy-secret-value\n").unwrap();
    let file = dir.join("forjar.yaml");
    std::fs::write(
        &file,
        format!(
            r#"
version: "1.0"
name: check-secrets
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
secrets:
  provider: file
  path: {}
resources:
  r:
    type: file
    machine: local
    path: {}
    content: "{{{{secrets.forjar_p0_check_demo}}}}"
"#,
            secrets.display(),
            dir.join("out.txt").display()
        ),
    )
    .unwrap();
    file
}

#[test]
fn apply_check_resolves_secrets_through_the_manifest_provider() {
    let d = tempfile::tempdir().unwrap();
    let file = secret_fixture(d.path());
    let config = super::helpers::parse_and_validate(&file).unwrap();
    // The output file is absent, so the check is red — but it must be red for
    // drift, never for an unresolvable secret.
    let err = super::check::cmd_check_selected(&config, None, None, d.path(), true, false)
        .err()
        .unwrap_or_default();
    assert!(!err.contains("not found"), "secret not resolved: {err}");
}
