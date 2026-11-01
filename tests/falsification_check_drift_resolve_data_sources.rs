//! `check` and `drift` rendered templates without ever resolving the config's
//! `data:` sources, so a healthy, declared source surfaced as the misleading
//! `unknown data source: <key>`. Both must resolve sources first, as `plan`
//! and `apply` do.

use std::fs;
use std::process::Command;

/// Config with `data:` sources; `extra` is extra `data:` YAML (2-space indented).
fn setup(
    data_yaml: &str,
) -> (
    tempfile::TempDir,
    std::path::PathBuf,
    std::path::PathBuf,
    std::path::PathBuf,
) {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg = dir.path().join("forjar.yaml");
    let out_file = dir.path().join("out.txt");
    fs::write(
        &cfg,
        format!(
            "version: \"1.0\"\nname: ds\nmachines: {{ local: {{ hostname: localhost, addr: 127.0.0.1 }} }}\n\
data:\n{data_yaml}\
resources:\n\x20 r: {{ type: file, machine: local, path: {}, content: \"v={{{{data.sha}}}}\\n\" }}\n",
            out_file.display()
        ),
    )
    .unwrap();
    let state = dir.path().join("state");
    (dir, cfg, state, out_file)
}

fn forjar(args: &[&str], cfg: &std::path::Path, state: &std::path::Path) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_forjar"))
        .args(args)
        .arg("-f")
        .arg(cfg)
        .arg("--state-dir")
        .arg(state)
        .output()
        .expect("forjar starts");
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.success(), s)
}

/// Apply, then run `verb`; returns (verb succeeded, combined output, out.txt).
fn run(verb: &[&str], data_value: &str) -> (bool, String, String) {
    let (_dir, cfg, state, out_file) = setup(&format!(
        "  sha: {{ type: command, value: \"{data_value}\" }}\n"
    ));
    let (applied, aout) = forjar(&["apply", "--yes"], &cfg, &state);
    if data_value.starts_with("echo") {
        assert!(applied, "apply must succeed so the verb has a lock: {aout}");
    }
    let (ok, out) = forjar(verb, &cfg, &state);
    (ok, out, fs::read_to_string(out_file).unwrap_or_default())
}

#[test]
fn check_resolves_a_declared_data_source() {
    let (ok, out, written) = run(&["check"], "echo abc123");
    assert_eq!(written, "v=abc123\n", "apply rendered the source: {out}");
    assert!(ok, "check of a converged healthy config must pass: {out}");
    assert!(!out.contains("unknown data source"), "{out}");
}

#[test]
fn apply_check_resolves_a_declared_data_source() {
    let (ok, out, _) = run(&["apply", "--check"], "echo abc123");
    assert!(
        ok,
        "apply --check of a converged healthy config must pass: {out}"
    );
    assert!(!out.contains("unknown data source"), "{out}");
}

#[test]
fn drift_resolves_a_declared_data_source() {
    let (ok, out, _) = run(&["drift"], "echo abc123");
    assert!(ok, "drift of a converged healthy config must pass: {out}");
    assert!(!out.contains("unknown data source"), "{out}");
}

/// A source that really fails is reported as itself, not as "unknown".
#[test]
fn check_names_the_real_failure_of_a_broken_source() {
    let (ok, out, _) = run(&["check"], "exit 127");
    assert!(!ok, "{out}");
    assert!(
        out.contains("exit 127") || out.contains("(exit 127)"),
        "{out}"
    );
    assert!(!out.contains("unknown data source"), "{out}");
}

/// drift warns and carries on past a broken source, and a HEALTHY source that
/// comes after it must still resolve (the first failure used to abort the
/// loop, so the healthy one surfaced as "unknown data source: sha").
#[test]
fn drift_keeps_resolving_healthy_sources_after_a_broken_one() {
    let (_dir, cfg, state, _out) = setup(
        "  broken: { type: command, value: \"exit 127\" }\n  sha: { type: command, value: \"echo abc123\" }\n",
    );
    let (_ok, out) = forjar(&["drift"], &cfg, &state);
    assert!(out.contains("broken"), "the broken source is named: {out}");
    assert!(
        !out.contains("unknown data source: sha"),
        "the healthy source after the broken one must resolve: {out}"
    );
}
