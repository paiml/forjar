//! Refs #680 (the apply half of #674): `apply -r a` must not resolve the
//! templates of a resource outside its scope.
//!
//! `plan -r a --output-dir` did: it resolved every resource, so an unset
//! `{{secrets.x}}` in an unrelated resource `b` failed the plan of `a`. Nothing
//! measured whether `apply -r a` does the same. This runs the real binary over
//! a local transport in a tempdir, with a file secret provider whose key is
//! absent.
//!
//! What keeps apply scoped is `resolve_selection` in `cmd_apply_scoped`: it
//! narrows `config.resources` to `a` and its `depends_on` closure before
//! anything resolves a template. Resolving every resource ahead of that call,
//! or dropping the call, turns this test red. A whole-file resolve inside the
//! executor does not, because by then `b` is no longer in the config.

use std::path::Path;
use std::process::Command;

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

/// `dep` <- `a` (plain), and `b`, whose content is a secret the provider
/// cannot resolve.
fn fixture(dir: &Path) -> String {
    std::fs::create_dir_all(dir.join("secrets")).unwrap();
    let cfg = dir.join("forjar.yaml");
    std::fs::write(
        &cfg,
        format!(
            r#"version: '1.0'
name: scoped-secret
secrets:
  provider: file
  path: "{secrets}"
machines:
  local:
    hostname: localhost
    addr: localhost
    transport: local
resources:
  dep:
    type: file
    machine: local
    path: {dep}
    content: "needed"
  a:
    type: file
    machine: local
    path: {a}
    content: "plain"
    depends_on: [dep]
  b:
    type: file
    machine: local
    path: {b}
    content: "{{{{secrets.unset-key}}}}"
"#,
            secrets = dir.join("secrets").display(),
            dep = dir.join("dep.txt").display(),
            a = dir.join("a.txt").display(),
            b = dir.join("b.txt").display(),
        ),
    )
    .unwrap();
    cfg.display().to_string()
}

#[test]
fn falsify_680_apply_r_a_runs_with_b_secret_unset() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = fixture(dir.path());

    let (out, ok) = run(
        dir.path(),
        &["apply", "--yes", "-f", &cfg, "--state-dir", "st", "-r", "a"],
    );
    assert!(ok, "apply -r a must not need b's secret:\n{out}");
    let read = |f: &str| std::fs::read_to_string(dir.path().join(f)).ok();
    assert_eq!(read("a.txt").as_deref(), Some("plain"), "{out}");
    assert_eq!(
        read("dep.txt").as_deref(),
        Some("needed"),
        "apply -r a dropped its dependency:\n{out}"
    );
    assert!(
        !dir.path().join("b.txt").exists(),
        "apply -r a rendered b:\n{out}"
    );
}

/// Control: the fixture really does carry an unresolvable secret. An unscoped
/// apply reaches `b` and must fail on it, naming the key.
#[test]
fn falsify_680_control_unscoped_apply_fails_on_b() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = fixture(dir.path());

    let (out, ok) = run(
        dir.path(),
        &["apply", "--yes", "-f", &cfg, "--state-dir", "st"],
    );
    assert!(
        !ok,
        "an unscoped apply must fail on b's unset secret:\n{out}"
    );
    assert!(
        out.contains("unset-key"),
        "the failure must name the key:\n{out}"
    );
    assert!(
        !dir.path().join("b.txt").exists(),
        "b was written without its secret:\n{out}"
    );
}
