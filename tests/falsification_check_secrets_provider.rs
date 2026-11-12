//! Refs #996: `forjar check` must resolve `{{secrets.*}}` through the
//! manifest's `secrets:` provider, as `apply` does, and must not print what
//! it resolved.
//!
//! Check resolved templates with the environment-only resolver, so a secret
//! held by a `provider: file` failed the whole check as unresolved. The unit
//! tests call `cmd_check` in-process; nothing ran the binary. This runs the
//! real `forjar check` over a local transport in a tempdir, with a file
//! provider that holds a distinctive marker value.
//!
//! What resolves the secret is `resolve_resource_templates_with_secrets` in
//! `cmd_check_selected` (src/cli/check.rs). Putting back the env-only
//! `resolve_resource_templates` turns both tests red. What keeps the value
//! out of the output is `Redaction` in `check_run.rs`; dropping it turns the
//! second red.

use std::path::Path;
use std::process::Command;

const MARKER: &str = "fj996-e2e-marker-value";

fn run(dir: &Path, args: &[&str]) -> (String, bool) {
    let out = Command::new(env!("CARGO_BIN_EXE_forjar"))
        .current_dir(dir)
        .args(args)
        .output()
        .expect("forjar must run");
    (
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
        out.status.success(),
    )
}

/// One task whose `completion_check` is `check_body`, a secret provider that
/// holds `fj996_e2e = MARKER`.
fn fixture(dir: &Path, check_body: &str) -> String {
    std::fs::create_dir_all(dir.join("secrets")).unwrap();
    std::fs::write(dir.join("secrets").join("fj996_e2e"), format!("{MARKER}\n")).unwrap();
    let cfg = dir.join("forjar.yaml");
    std::fs::write(
        &cfg,
        format!(
            r#"version: '1.0'
name: check-secrets-e2e
secrets:
  provider: file
  path: "{secrets}"
machines:
  local:
    hostname: localhost
    addr: localhost
    transport: local
resources:
  guarded:
    type: task
    machine: local
    command: "true"
    completion_check: "{check_body}"
"#,
            secrets = dir.join("secrets").display(),
        ),
    )
    .unwrap();
    cfg.display().to_string()
}

#[test]
fn falsify_996_check_resolves_the_secret_from_the_file_provider() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = fixture(
        dir.path(),
        &format!("test '{{{{secrets.fj996_e2e}}}}' = '{MARKER}'"),
    );

    let (out, ok) = run(dir.path(), &["check", "-f", &cfg, "--state-dir", "st"]);
    assert!(ok, "check must resolve the provider's secret:\n{out}");
    assert!(
        !out.contains("FORJAR_SECRET_") && !out.contains("unresolved"),
        "check asked the environment instead of the provider:\n{out}"
    );
    assert!(!out.contains(MARKER), "check printed the secret:\n{out}");
}

#[test]
fn falsify_996_failing_check_does_not_print_the_secret_it_echoed() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = fixture(
        dir.path(),
        "echo seen-996 {{secrets.fj996_e2e}} >&2; exit 1",
    );

    for extra in [&[][..], &["--json"][..]] {
        let mut args = vec!["check", "-f", &cfg, "--state-dir", "st"];
        args.extend_from_slice(extra);
        let (out, ok) = run(dir.path(), &args);
        assert!(!ok, "the check script exits 1 ({extra:?}):\n{out}");
        assert!(
            out.contains("seen-996"),
            "the rest of the failure output is kept ({extra:?}):\n{out}"
        );
        assert!(!out.contains(MARKER), "secret leaked ({extra:?}):\n{out}");
    }
}
