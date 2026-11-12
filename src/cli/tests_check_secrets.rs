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
