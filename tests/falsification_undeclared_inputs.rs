//! GH-244: detecting a task that reads a file it never declared.
//!
//! `staleness_reason` decides entirely from the DECLARED set, so a change to an
//! undeclared input yields `None` and `plan`/`check`/`drift`/`apply` all report
//! a clean, converged stack over a stale artifact. Under-declaration converts
//! "no build system" into "a build system that lies", which is strictly harder
//! to detect than having no cache at all.
//!
//! The mechanism here is sandboxing-by-omission: run once from a full copy of
//! `working_dir`, once from a tree containing only the glob-expanded
//! `task_inputs`, and compare. It needs no privileges, no ptrace and no
//! LD_PRELOAD — the three mechanisms GH-244 weighs and calls "inherently
//! best-effort".
//!
//! What it CANNOT see: a read of `/usr/share/fonts` or a tool version, because
//! those exist in the scratch tree too. That is GH-244 option (c), and a
//! `data:` source of `type: command` already covers it. Nothing here should be
//! read as "the declaration is now proven complete".

use forjar::core::types::{MachineTarget, Resource, ResourceType};
use forjar::core::verify::{verify_hermetic, Verdict};
use std::path::Path;

fn task(dir: &Path, command: &str, inputs: &[&str], artifacts: &[&str]) -> Resource {
    Resource {
        resource_type: ResourceType::Task,
        machine: MachineTarget::Single("local".to_string()),
        command: Some(command.to_string()),
        working_dir: Some(dir.display().to_string()),
        task_inputs: inputs.iter().map(|s| (*s).to_string()).collect(),
        output_artifacts: artifacts.iter().map(|s| (*s).to_string()).collect(),
        cache: true,
        ..Default::default()
    }
}

/// Run the recipe the way apply would, and return the recorded output hash.
fn apply_and_record(r: &Resource, dir: &Path) -> String {
    std::process::Command::new("bash")
        .arg("-c")
        .arg(r.command.as_deref().unwrap())
        .current_dir(dir)
        .status()
        .unwrap();
    forjar::core::task::hash_outputs_in(&r.output_artifacts, dir)
        .unwrap()
        .unwrap()
}

#[test]
fn an_undeclared_project_file_is_detected() {
    // THE CASE. `render.sh` reads BOTH slide.txt (declared) and theme.txt
    // (NOT declared). Editing theme.txt today produces staleness_reason=None
    // and a green plan over a stale artifact.
    let dir = tempfile::tempdir().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("slide.txt"), "SLIDE\n").unwrap();
    std::fs::write(dir.path().join("theme.txt"), "DARK\n").unwrap();

    let r = task(
        dir.path(),
        "cat slide.txt theme.txt > out.txt",
        &["slide.txt"], // theme.txt deliberately omitted
        &["out.txt"],
    );
    let recorded = apply_and_record(&r, dir.path());

    let outcome = verify_hermetic("render", &r, Some(&recorded), scratch.path());
    assert!(
        matches!(outcome.verdict, Verdict::UndeclaredInput { .. }),
        "reading an undeclared project file must be detected, got {:?}",
        outcome.verdict
    );
    assert!(outcome.verdict.is_failure());
}

#[test]
fn a_fully_declared_recipe_is_clean() {
    // The gate must be passable, or it trains people to ignore it.
    let dir = tempfile::tempdir().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("slide.txt"), "SLIDE\n").unwrap();
    std::fs::write(dir.path().join("theme.txt"), "DARK\n").unwrap();

    let r = task(
        dir.path(),
        "cat slide.txt theme.txt > out.txt",
        &["slide.txt", "theme.txt"], // both declared this time
        &["out.txt"],
    );
    let recorded = apply_and_record(&r, dir.path());

    let outcome = verify_hermetic("render", &r, Some(&recorded), scratch.path());
    assert_eq!(
        outcome.verdict,
        Verdict::Reproduced,
        "a fully-declared recipe must come back clean, got {:?}",
        outcome.verdict
    );
}

#[test]
fn nondeterminism_is_reported_as_divergence_not_as_under_declaration() {
    // THE DISCRIMINATION THAT MATTERS. A non-deterministic generator fails BOTH
    // runs. Reporting that as an undeclared input would blame the declaration
    // for a generator's own instability — which is exactly the misdiagnosis
    // this feature exists to replace.
    let dir = tempfile::tempdir().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("in.txt"), "IN\n").unwrap();

    let r = task(
        dir.path(),
        "date +%s%N > out.txt",
        &["in.txt"],
        &["out.txt"],
    );
    let recorded = apply_and_record(&r, dir.path());

    let outcome = verify_hermetic("gen", &r, Some(&recorded), scratch.path());
    assert!(
        matches!(outcome.verdict, Verdict::Diverged { .. }),
        "a non-deterministic recipe must stay `Diverged`, not be blamed on the \
         declaration: {:?}",
        outcome.verdict
    );
}

#[test]
fn a_recipe_that_cannot_run_without_the_undeclared_file_names_the_failure() {
    // The sharper form: the recipe does not merely produce different bytes, it
    // fails outright. The message must say WHY, not just "diverged".
    let dir = tempfile::tempdir().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), "A\n").unwrap();
    std::fs::write(dir.path().join("required.txt"), "R\n").unwrap();

    let r = task(
        dir.path(),
        "set -e; cat required.txt > /dev/null; cp a.txt out.txt",
        &["a.txt"], // required.txt omitted
        &["out.txt"],
    );
    let recorded = apply_and_record(&r, dir.path());

    let outcome = verify_hermetic("needs", &r, Some(&recorded), scratch.path());
    match outcome.verdict {
        Verdict::UndeclaredInput { hermetic } => assert!(
            hermetic.contains("task_inputs"),
            "the message must point at the declaration: {hermetic}"
        ),
        other => panic!("expected UndeclaredInput, got {other:?}"),
    }
}

// ── End to end, through the binary ──────────────────────────────────────
//
// These exist because the unit tests above passed while `--check-declared-inputs`
// did NOTHING: the flag was parsed, the value was bound, and the call site still
// called the non-hermetic path. A feature can be correct and unreachable, and
// only a test that drives the binary can tell the difference.

use std::process::Command;

fn forjar() -> Command {
    Command::new(env!("CARGO_BIN_EXE_forjar"))
}

/// A project whose recipe reads `theme.txt` without declaring it.
fn under_declared_project(dir: &Path, declare_theme: bool) -> std::path::PathBuf {
    let work = dir.join("work");
    std::fs::create_dir_all(&work).unwrap();
    std::fs::write(work.join("slide.txt"), "SLIDE\n").unwrap();
    std::fs::write(work.join("theme.txt"), "DARK\n").unwrap();
    let inputs = if declare_theme {
        r#"["slide.txt", "theme.txt"]"#
    } else {
        r#"["slide.txt"]"#
    };
    let cfg = dir.join("forjar.yaml");
    std::fs::write(
        &cfg,
        format!(
            r#"version: "1.0"
name: ui
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
resources:
  render:
    type: task
    machine: local
    working_dir: "{}"
    cache: true
    task_inputs: {inputs}
    output_artifacts: ["out.txt"]
    command: |
      cat slide.txt theme.txt > out.txt
"#,
            work.display()
        ),
    )
    .unwrap();
    cfg
}

fn apply(cfg: &Path, state: &Path) {
    let out = forjar()
        .args([
            "apply",
            "-f",
            cfg.to_str().unwrap(),
            "--state-dir",
            state.to_str().unwrap(),
            "--no-tripwire",
            "--yes",
        ])
        .output()
        .expect("apply runs");
    assert!(
        out.status.success(),
        "apply failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn verify(cfg: &Path, state: &Path, extra: &[&str]) -> (bool, String) {
    let mut args = vec![
        "verify",
        "-f",
        cfg.to_str().unwrap(),
        "--state-dir",
        state.to_str().unwrap(),
    ];
    args.extend_from_slice(extra);
    let out = forjar().args(&args).output().expect("verify runs");
    (
        out.status.success(),
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
    )
}

#[test]
fn the_flag_actually_changes_the_verdict() {
    // THE WIRING TEST. Without the flag this reproduces; with it, the
    // under-declaration is detected. If the flag were ignored — as it was on
    // the first implementation — both runs would agree and this fails.
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    std::fs::create_dir_all(&state).unwrap();
    let cfg = under_declared_project(dir.path(), false);
    apply(&cfg, &state);

    let (plain_ok, plain) = verify(&cfg, &state, &[]);
    assert!(plain_ok, "the full-tree run must reproduce:\n{plain}");

    let (hermetic_ok, hermetic) = verify(&cfg, &state, &["--check-declared-inputs"]);
    assert!(
        !hermetic_ok,
        "--check-declared-inputs must fail on an under-declared recipe:\n{hermetic}"
    );
    assert!(
        hermetic.contains("UNDECLARED"),
        "the verdict must name the problem:\n{hermetic}"
    );
    assert!(
        hermetic.contains("theme.txt"),
        "the message must name the file that was not declared:\n{hermetic}"
    );
}

#[test]
fn a_fully_declared_project_passes_the_flag() {
    // The gate must be passable with the flag on, or nobody will turn it on.
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    std::fs::create_dir_all(&state).unwrap();
    let cfg = under_declared_project(dir.path(), true);
    apply(&cfg, &state);

    let (ok, out) = verify(&cfg, &state, &["--check-declared-inputs"]);
    assert!(ok, "a fully-declared recipe must pass:\n{out}");
    assert!(out.contains("1 reproduced"), "{out}");
}
