//! Refs #674: `plan -r a --output-dir D` failed with "secret '…' not found"
//! because ANOTHER resource referenced a secret, and it wrote every
//! resource's scripts into D. An export is for a human to read, so it must
//! hold the plan's selection only and must never carry a secret's value.
//!
//! This runs the real binary over a local transport in a tempdir, with a file
//! secret provider: `known-key` has a value on disk, `unset-key` has none.
//!
//! What keeps the export right is `export_scripts` in `src/cli/print_helpers.rs`:
//! it skips resources outside the plan's selection, and it resolves templates
//! with the `plan-export-redacted` provider, which writes each secret as
//! `FORJAR_REDACTED_SECRET_<key>` and leaves an `ENC[age,...]` literal as
//! written (`resolve_template_with_secrets` in `src/core/resolver/template.rs`).
//! Exporting the whole config, resolving with the config's own provider, or
//! decrypting an age literal turns these tests red.

use std::path::Path;
use std::process::Command;

const SECRET_VALUE: &str = "fj674-the-value-must-never-be-exported";

/// A well-formed age marker (`ENC[age,<base64>]`, 20+ chars of base64).
const AGE_LITERAL: &str = "ENC[age,YWdlLWVuY3J5cHRpb24ub3JnL3YxCg==]";

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

/// `a` (a plain file), and three tasks whose command carries a secret: `b`
/// (unset), `c` (its value is on disk) and `d` (an age literal). A task's
/// command reaches the exported apply script as written, so the script shows
/// what was resolved.
fn fixture(dir: &Path) -> String {
    std::fs::create_dir_all(dir.join("secrets")).unwrap();
    std::fs::write(dir.join("secrets").join("known-key"), SECRET_VALUE).unwrap();
    let cfg = dir.join("forjar.yaml");
    std::fs::write(
        &cfg,
        format!(
            r#"version: '1.0'
name: export-secret
secrets:
  provider: file
  path: "{secrets}"
machines:
  local:
    hostname: localhost
    addr: localhost
    transport: local
resources:
  a:
    type: file
    machine: local
    path: {a}
    content: "plain"
  b:
    type: task
    machine: local
    command: "post --token {{{{secrets.unset-key}}}}"
  c:
    type: task
    machine: local
    command: "post --token {{{{secrets.known-key}}}}"
  d:
    type: task
    machine: local
    command: "post --token {age}"
"#,
            secrets = dir.join("secrets").display(),
            a = dir.join("a.txt").display(),
            age = AGE_LITERAL,
        ),
    )
    .unwrap();
    cfg.display().to_string()
}

fn exported(out: &Path) -> Vec<(String, String)> {
    let mut files: Vec<(String, String)> = std::fs::read_dir(out)
        .map(|rd| {
            rd.filter_map(Result::ok)
                .map(|e| {
                    (
                        e.file_name().to_string_lossy().into_owned(),
                        std::fs::read_to_string(e.path()).unwrap_or_default(),
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
}

#[test]
fn falsify_674_plan_r_a_exports_only_a_with_b_secret_unset() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = fixture(dir.path());

    let (out, ok) = run(
        dir.path(),
        &[
            "plan",
            "-f",
            &cfg,
            "--state-dir",
            "st",
            "-r",
            "a",
            "--output-dir",
            "x",
        ],
    );
    assert!(
        ok,
        "plan -r a --output-dir must not need b's secret:\n{out}"
    );
    let files = exported(&dir.path().join("x"));
    let names: Vec<&str> = files.iter().map(|(n, _)| n.as_str()).collect();
    assert!(
        names.contains(&"a.apply.sh"),
        "a was not exported: {names:?}\n{out}"
    );
    assert!(
        names.iter().all(|n| n.starts_with("a.")),
        "plan -r a exported resources outside its selection: {names:?}"
    );
}

#[test]
fn falsify_674_unscoped_export_names_secrets_never_resolves_them() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = fixture(dir.path());

    let (out, ok) = run(
        dir.path(),
        &["plan", "-f", &cfg, "--state-dir", "st", "--output-dir", "x"],
    );
    assert!(ok, "an export must not need any secret's value:\n{out}");
    let files = exported(&dir.path().join("x"));
    let script = |name: &str| {
        files
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, body)| body.as_str())
            .unwrap_or_else(|| panic!("{name} was not exported"))
    };
    assert!(script("b.apply.sh").contains("FORJAR_REDACTED_SECRET_unset-key"));
    assert!(script("c.apply.sh").contains("FORJAR_REDACTED_SECRET_known-key"));
    for (name, body) in &files {
        assert!(
            !body.contains(SECRET_VALUE),
            "{name} carries the secret's value"
        );
    }
}

/// An age literal is ciphertext the export shows as written. Decrypting it
/// would put the plaintext in a script; a build without `encryption` refused
/// the whole export instead.
#[test]
fn falsify_674_export_leaves_age_literal_as_written() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = fixture(dir.path());

    let (out, ok) = run(
        dir.path(),
        &[
            "plan",
            "-f",
            &cfg,
            "--state-dir",
            "st",
            "-r",
            "d",
            "--output-dir",
            "x",
        ],
    );
    assert!(ok, "an export must not decrypt an age literal:\n{out}");
    let script = std::fs::read_to_string(dir.path().join("x").join("d.apply.sh"))
        .unwrap_or_else(|e| panic!("d was not exported: {e}\n{out}"));
    assert!(
        script.contains(&format!("post --token {AGE_LITERAL}")),
        "the age literal was not written as it stands:\n{script}"
    );
}
