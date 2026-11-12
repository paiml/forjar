//! forjar#996: `apply --check` resolves `{{secrets.*}}` through the manifest's
//! `secrets:` provider, as `apply` and `apply --dry-run` do.
//!
//! Each case runs a REAL check on localhost. The secret key is unique to this
//! file, so no `FORJAR_SECRET_*` variable of the host can stand in for it.

use super::check::cmd_check;

fn manifest(dir: &std::path::Path, expected: &str) -> std::path::PathBuf {
    let secrets = dir.join("secrets");
    std::fs::create_dir_all(&secrets).unwrap();
    std::fs::write(secrets.join("fj996_demo"), "s3cr3t-996\n").unwrap();
    let file = dir.join("forjar.yaml");
    std::fs::write(
        &file,
        format!(
            "version: \"1.0\"\n\
             name: check-secrets\n\
             secrets:\n\
             \x20 provider: file\n\
             \x20 path: \"{}\"\n\
             machines:\n\
             \x20 local:\n\
             \x20   hostname: local\n\
             \x20   addr: 127.0.0.1\n\
             resources:\n\
             \x20 guarded:\n\
             \x20   type: task\n\
             \x20   machine: local\n\
             \x20   command: \"true\"\n\
             \x20   completion_check: \"test '{{{{secrets.fj996_demo}}}}' = '{expected}'\"\n",
            secrets.display()
        ),
    )
    .unwrap();
    file
}

fn check(file: &std::path::Path, dir: &std::path::Path) -> Result<(), String> {
    cmd_check(file, None, None, None, &dir.join("state"), false, false)
}

#[test]
fn check_resolves_a_secret_through_the_manifest_file_provider() {
    let dir = tempfile::tempdir().unwrap();
    let file = manifest(dir.path(), "s3cr3t-996");
    check(&file, dir.path()).expect("the file provider holds the secret the check compares");
}

#[test]
fn check_fails_on_the_resolved_value_not_on_a_missing_env_var() {
    let dir = tempfile::tempdir().unwrap();
    let file = manifest(dir.path(), "another-value");
    let err = check(&file, dir.path()).expect_err("the resolved secret differs from the expected");
    assert!(
        !err.contains("FORJAR_SECRET_"),
        "the check asked the environment instead of the declared provider: {err}"
    );
}

/// The reported shape: a `file` resource whose content is a provider secret.
#[test]
fn check_of_a_file_resource_with_secret_content_uses_the_provider() {
    let dir = tempfile::tempdir().unwrap();
    let secrets = dir.path().join("secrets");
    std::fs::create_dir_all(&secrets).unwrap();
    std::fs::write(secrets.join("fj996_file"), "s3cr3t-996\n").unwrap();
    let out = dir.path().join("out.txt");
    std::fs::write(&out, "s3cr3t-996").unwrap();
    let file = dir.path().join("forjar.yaml");
    std::fs::write(
        &file,
        format!(
            "version: \"1.0\"\n\
             name: check-secrets-file\n\
             secrets:\n\
             \x20 provider: file\n\
             \x20 path: \"{}\"\n\
             machines:\n\
             \x20 local:\n\
             \x20   hostname: local\n\
             \x20   addr: 127.0.0.1\n\
             resources:\n\
             \x20 out:\n\
             \x20   type: file\n\
             \x20   machine: local\n\
             \x20   path: \"{}\"\n\
             \x20   content: \"{{{{secrets.fj996_file}}}}\"\n",
            secrets.display(),
            out.display()
        ),
    )
    .unwrap();
    if let Err(e) = check(&file, dir.path()) {
        assert!(
            !e.contains("FORJAR_SECRET_"),
            "the check asked the environment instead of the declared provider: {e}"
        );
    }
}

/// One failing check whose script echoes the secret to stderr; returns the
/// detail the check keeps (what text mode prints and `--json` serializes).
fn leaked_detail(sensitive: bool) -> String {
    use super::check_run::{run_single_check, Redaction};
    use crate::core::{codegen, resolver, types::ForjarConfig};
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("fj996_echo"), "s3cr3t-996\n").unwrap();
    let config: ForjarConfig = serde_yaml_ng::from_str(&format!(
        "version: \"1.0\"\n\
         name: check-redaction\n\
         secrets:\n\
         \x20 provider: file\n\
         \x20 path: \"{}\"\n\
         machines:\n\
         \x20 local:\n\
         \x20   hostname: local\n\
         \x20   addr: 127.0.0.1\n\
         resources:\n\
         \x20 guarded:\n\
         \x20   type: task\n\
         \x20   machine: local\n\
         \x20   sensitive: {sensitive}\n\
         \x20   command: \"true\"\n\
         \x20   completion_check: \"echo marker-996 {{{{secrets.fj996_echo}}}} >&2; exit 1\"\n",
        dir.path().display()
    ))
    .expect("fixture parses");
    let resource = &config.resources["guarded"];
    let resolved = resolver::resolve_resource_templates_with_secrets(
        resource,
        &config.params,
        &config.machines,
        &config.secrets,
    )
    .expect("the file provider resolves");
    let script = codegen::check_script(&resolved).expect("a task with a completion_check");
    assert!(script.contains("s3cr3t-996"), "the fixture must carry the secret");
    let redaction = Redaction::for_resource(resource, &config.secrets);
    let (result, passed) = run_single_check(
        &config.machines["local"],
        &script,
        "guarded",
        "local",
        true,
        &redaction,
    );
    assert!(!passed, "the check script exits 1");
    result.detail
}

#[test]
fn a_failing_check_does_not_keep_the_secret_it_echoed() {
    let detail = leaked_detail(false);
    assert!(detail.contains("marker-996"), "the rest of stderr is kept: {detail}");
    assert!(!detail.contains("s3cr3t-996"), "the secret reached the detail");
}

#[test]
fn a_sensitive_resource_has_its_check_output_withheld() {
    let detail = leaked_detail(true);
    assert!(!detail.contains("s3cr3t-996"), "the secret reached the detail");
    assert!(!detail.contains("marker-996"), "a sensitive resource's output is withheld whole: {detail}");
}
