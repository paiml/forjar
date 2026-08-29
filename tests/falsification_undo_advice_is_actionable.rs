//! `forjar undo` refused with advice that provably does not work.
//!
//! THE DEFECT.
//!
//! On a config that does not set `policy.snapshot_generations`, `apply`
//! records no generation — ever. `undo` then fails with:
//!
//! ```text
//! $ forjar apply -f forjar.yaml --state-dir state --yes   # rc=0, 1 converged
//! $ forjar undo  -f forjar.yaml --state-dir state
//! error: no generations found — run `forjar apply` first  # rc=1
//! $ forjar apply -f forjar.yaml --state-dir state --yes   # rc=0
//! $ forjar apply -f forjar.yaml --state-dir state --yes   # rc=0
//! $ forjar undo  -f forjar.yaml --state-dir state
//! error: no generations found — run `forjar apply` first  # rc=1, unchanged
//! ```
//!
//! The refusal itself is correct — there really are no generations. What is
//! wrong is the remedy. `forjar apply` is the one thing that cannot help: the
//! generation is written by `maybe_auto_snapshot`, which returns early unless
//! `policy.snapshot_generations` is set. An operator who follows the message
//! re-applies forever and the message never changes. The knob that would
//! change it is never named, and nothing in the CLI docs mentions it either.
//!
//! WHY THESE TESTS DRIVE THE ADVICE, NOT THE STRING.
//!
//! Asserting the wording would pin whatever text is written next. What makes
//! an error honest is that following it changes the outcome, so the oracle
//! here is behavioural: do what the message says, then measure whether `undo`
//! moves. `misdirection_repro` proves the OLD advice does not move it (three
//! applies, same error), and `the_named_remedy_actually_works` proves the NEW
//! advice does.
//!
//! These also close a vacuum in `contracts/destroy-undo-roundtrip-v1.yaml`.
//! Its three falsification tests call `create_generation` directly on a
//! hand-built state dir, and every `cmd_undo` test fabricates
//! `state/generations/N` with `create_dir_all` plus a hand-made `current`
//! symlink. Not one drove `apply` into `undo`, so the contract held over a
//! directory layout only the tests ever produced — and stayed green through a
//! release in which no real apply produced it.
//!
//! Scope: "undo is reachable and runs", not "undo reverts the machine". The
//! latter was a separate confirmed defect
//! (`undo-reports-rollback-but-machine-is-unchanged` in docs/cli-defects.json:
//! re-convergence re-applied the CURRENT config), fixed for GH-376 and now
//! owned by tests/falsification_undo_actually_undoes.rs, which asserts the
//! BYTES at the declared path. Keep the scopes apart: this file must keep
//! passing on the strength of the ADVICE alone, so it deliberately does not
//! assert what the host holds.

use std::path::{Path, PathBuf};
use std::process::Command;

fn forjar() -> Command {
    Command::new(env!("CARGO_BIN_EXE_forjar"))
}

/// A one-resource config. `generations` writes `policy.snapshot_generations`.
fn write_config(dir: &Path, generations: Option<u32>, content: &str) -> PathBuf {
    let cfg = dir.join("forjar.yaml");
    let policy = match generations {
        Some(n) => format!("policy:\n  snapshot_generations: {n}\n"),
        None => String::new(),
    };
    let target = dir.join("hello.txt");
    std::fs::write(
        &cfg,
        format!(
            r#"version: "1.0"
name: undo-advice
{policy}machines:
  local:
    hostname: localhost
    addr: localhost
    transport: local
resources:
  hello:
    type: file
    machine: local
    path: {}
    content: "{content}\n"
"#,
            target.display()
        ),
    )
    .unwrap();
    cfg
}

/// Run a forjar subcommand, returning (exit code, stdout + stderr merged).
fn run(args: &[&str]) -> (i32, String) {
    let out = forjar().args(args).output().unwrap();
    let mut merged = String::from_utf8_lossy(&out.stdout).into_owned();
    merged.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.code().unwrap_or(-1), merged)
}

fn apply(cfg: &Path, state: &Path) -> (i32, String) {
    run(&[
        "apply",
        "-f",
        &cfg.display().to_string(),
        "--state-dir",
        &state.display().to_string(),
        "--yes",
    ])
}

fn undo(cfg: &Path, state: &Path, extra: &[&str]) -> (i32, String) {
    let cfg = cfg.display().to_string();
    let state = state.display().to_string();
    let mut args = vec!["undo", "-f", &cfg, "--state-dir", &state];
    args.extend_from_slice(extra);
    run(&args)
}

/// The blocker, verbatim: three successful applies, and `undo` never moves.
///
/// The behavioural half is the loop — re-applying is exactly what the old
/// message told the operator to do, and it changes nothing. The message must
/// therefore name the knob that does.
#[test]
fn misdirection_repro() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    let cfg = write_config(dir.path(), None, "ONE");

    let mut messages = Vec::new();
    for round in 1..=3 {
        let (rc, out) = apply(&cfg, &state);
        assert_eq!(rc, 0, "apply #{round} must succeed, got:\n{out}");
        let (rc, out) = undo(&cfg, &state, &[]);
        assert_ne!(rc, 0, "undo must refuse when nothing recorded a generation");
        messages.push(out);
    }

    // Falsifies the old advice: the remedy it names was applied three times.
    assert_eq!(
        messages[0], messages[2],
        "re-applying changed undo's answer, so the config records generations \
         after all — this repro no longer reproduces the defect"
    );

    let msg = &messages[2];
    assert!(
        msg.contains("snapshot_generations"),
        "undo refused without naming the setting that would enable generations. \
         Re-applying provably does not help; an error that sends the operator \
         back to `apply` misdirects. Got:\n{msg}"
    );
}

/// The remedy the new message names must actually work, in the number of
/// applies the message implies — not one more.
///
/// Before the fix this needed THREE applies: `maybe_auto_snapshot` skipped the
/// first because the state dir did not exist yet, so the operator who did what
/// the error said still hit `cannot undo 1 generation(s): only 0 exist`.
#[test]
fn the_named_remedy_actually_works() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    let cfg = write_config(dir.path(), Some(10), "ONE");

    let (rc, out) = apply(&cfg, &state);
    assert_eq!(rc, 0, "apply #1 failed:\n{out}");
    let (rc, out) = run(&[
        "generation",
        "list",
        "--state-dir",
        &state.display().to_string(),
    ]);
    assert_eq!(rc, 0, "generation list failed:\n{out}");
    assert!(
        !out.contains("No generations."),
        "the FIRST apply under `snapshot_generations` must record a generation; \
         got:\n{out}"
    );

    let (rc, out) = apply(&cfg, &state);
    assert_eq!(rc, 0, "apply #2 failed:\n{out}");

    let (rc, out) = undo(&cfg, &state, &["--dry-run"]);
    assert_eq!(
        rc, 0,
        "undo must be reachable after two applies with generations enabled; got:\n{out}"
    );
    assert!(
        out.contains("Undo: generation 1 → 0"),
        "undo must target the previous generation; got:\n{out}"
    );
}

/// End-to-end, through the CLI: three applies of changing content, then
/// `undo --yes` reaches the rollback.
///
/// This is the leg `destroy-undo-roundtrip-v1` never had — every test behind
/// it built `state/generations/N` by hand. It asserts undo is REACHABLE from a
/// real apply and restores the target generation's state; that the machine
/// itself reverts is asserted in falsification_undo_actually_undoes.rs.
#[test]
fn apply_then_undo_reaches_the_rollback() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");

    // The content must differ per apply: identical locks produce an empty diff
    // and undo short-circuits with "No changes" before it rolls anything back.
    for content in ["ONE", "TWO", "THREE"] {
        let cfg = write_config(dir.path(), Some(10), content);
        let (rc, out) = apply(&cfg, &state);
        assert_eq!(rc, 0, "apply of {content} failed:\n{out}");
    }
    let cfg = dir.path().join("forjar.yaml");

    let (rc, out) = undo(&cfg, &state, &["--dry-run"]);
    assert_eq!(rc, 0, "undo --dry-run failed:\n{out}");
    assert!(
        out.contains("hello"),
        "undo must diff the resource between generations; got:\n{out}"
    );

    let (rc, out) = undo(&cfg, &state, &["--yes"]);
    assert_eq!(rc, 0, "undo --yes failed:\n{out}");
    assert!(
        out.contains("Rolled back to generation 1"),
        "undo --yes must restore the target generation; got:\n{out}"
    );
}

/// The opposite error: generations ARE enabled, none recorded yet. Here
/// `run forjar apply` is true advice and must survive — a fix that always
/// blames the config would be the same defect pointed the other way.
#[test]
fn enabled_but_nothing_applied_yet_still_says_apply() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    let cfg = write_config(dir.path(), Some(10), "ONE");

    let (rc, out) = undo(&cfg, &state, &[]);
    assert_ne!(rc, 0, "undo must refuse with no generations; got:\n{out}");
    assert!(
        out.contains("forjar apply"),
        "with generations enabled, re-applying IS the remedy; got:\n{out}"
    );
    assert!(
        !out.contains("snapshot_generations"),
        "must not tell the operator to set a policy they already set; got:\n{out}"
    );
}
