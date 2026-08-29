//! `forjar undo` did not undo.
//!
//! THE DEFECT, measured on 1.20.1 and again on 1.22.0. Three applies of one
//! `file` resource whose content changes each time, with
//! `policy.snapshot_generations: 10`:
//!
//! ```text
//! $ forjar undo -f CFG --state-dir ST --yes
//! Undo: generation 2 → 1
//! Rolled back to generation 1
//! Re-applying config to converge to generation 1...
//! box: 1 converged, 0 unchanged, 0 failed
//! Apply complete: 1 converged, 0 unchanged.        # rc=0
//! $ cat demo.txt
//! v3                                               # <- expected v2
//! ```
//!
//! `cmd_undo` rolled the LOCK back to the target generation and then called
//! `cmd_apply` on the CURRENT config, which immediately re-converged the host
//! to the very state the rollback had just walked away from. Every visible
//! signal said it worked. The exit code was 0, "Rolled back to generation 1"
//! was true of the state dir, and "1 converged" is what `--force` prints
//! whether or not it wrote different bytes.
//!
//! It could not have worked. `create_generation` stored only a BLAKE3
//! `config_hash`; the config BODY was never snapshotted, and `git_ref` — the
//! only other config trace — is printed and never resolved, and would miss
//! uncommitted edits anyway. Undo had no way to know what the target
//! generation's desired state WAS, so it used the one config it had.
//!
//! WHY THE ORACLE IS THE BYTES AT THE PATH. Every assertion here reads the file
//! the config declares. The summary line is the defect's own output: an undo
//! that reverted nothing printed "1 converged" and exited 0, so a test that
//! trusted the summary would have passed over the bug for two releases — which
//! is exactly what happened. `tests/falsification_undo_advice_is_actionable.rs`
//! scoped itself to "undo is reachable and runs" and stayed green throughout;
//! the lib tests built `generations/N` by hand and never drove an apply.
//!
//! Note the CONTROL (`undo_that_changes_nothing_leaves_the_file_untouched`).
//! "Always re-apply the target generation" would revert correctly and still be
//! wrong: it would rewrite files an operator was told nothing would change. The
//! control pins the file's mtime, which only survives if undo took the no-op
//! path.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

fn forjar() -> Command {
    Command::new(env!("CARGO_BIN_EXE_forjar"))
}

/// One `file` resource on the loopback machine, its content the version marker.
fn write_config(dir: &Path, content: &str) -> PathBuf {
    let cfg = dir.join("forjar.yaml");
    std::fs::write(
        &cfg,
        format!(
            r#"version: "1.0"
name: undo-repro
policy:
  snapshot_generations: 10
machines:
  box:
    hostname: localhost
    addr: 127.0.0.1
    transport: local
resources:
  demo_file:
    type: file
    machine: box
    path: {}
    content: "{content}\n"
"#,
            dir.join("demo.txt").display()
        ),
    )
    .unwrap();
    cfg
}

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

/// THE ORACLE: the bytes the declared path actually holds.
fn declared(dir: &Path) -> String {
    std::fs::read_to_string(dir.join("demo.txt")).unwrap()
}

fn mtime(p: &Path) -> SystemTime {
    std::fs::metadata(p).unwrap().modified().unwrap()
}

/// Apply `versions` in order, leaving the config at the last one.
fn apply_series(dir: &Path, state: &Path, versions: &[&str]) -> PathBuf {
    let mut cfg = dir.join("forjar.yaml");
    for v in versions {
        cfg = write_config(dir, v);
        let (rc, out) = apply(&cfg, state);
        assert_eq!(rc, 0, "apply of {v} failed:\n{out}");
        assert_eq!(declared(dir), format!("{v}\n"), "apply of {v} did not land");
    }
    cfg
}

/// The blocker, verbatim: v1 → v2 → v3, then `undo --yes` must leave v2.
#[test]
fn undo_returns_the_file_to_the_previous_version() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    let cfg = apply_series(dir.path(), &state, &["v1", "v2", "v3"]);

    let (rc, out) = undo(&cfg, &state, &["--yes"]);
    assert_eq!(rc, 0, "undo --yes failed:\n{out}");

    assert_eq!(
        declared(dir.path()),
        "v2\n",
        "undo left the host at the CURRENT config's state. It re-applied the config \
         `-f` names instead of the one the target generation recorded, so the rollback \
         was immediately undone by the re-convergence. Output was:\n{out}"
    );
}

/// Undo is a walk backwards, not a one-shot. Two undos reach v1.
///
/// This is what fails if the replay is allowed to append a generation: `current`
/// would land past the state the host is in, so the second undo would target the
/// state the first had just restored and report there was nothing to do.
#[test]
fn undo_is_repeatable() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    let cfg = apply_series(dir.path(), &state, &["v1", "v2", "v3"]);

    let (rc, out) = undo(&cfg, &state, &["--yes"]);
    assert_eq!(rc, 0, "first undo failed:\n{out}");
    let (rc, out) = undo(&cfg, &state, &["--yes"]);
    assert_eq!(rc, 0, "second undo failed:\n{out}");

    assert_eq!(
        declared(dir.path()),
        "v1\n",
        "the second undo did not step back another generation; got:\n{out}"
    );
}

/// The state lock must follow the host, not just the file.
#[test]
fn undo_leaves_the_lock_holding_the_target_generations_hash() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    let cfg = apply_series(dir.path(), &state, &["v1", "v2", "v3"]);

    // After three applies generations 0..2 exist and `current` is 2, so
    // `undo --generations 1` targets generation 1 — the state apply #2 produced.
    let target_lock = state.join("generations/1/box/state.lock.yaml");
    let target = std::fs::read_to_string(&target_lock).expect("generation 1 must hold a lock");

    let (rc, out) = undo(&cfg, &state, &["--yes"]);
    assert_eq!(rc, 0, "undo failed:\n{out}");

    let live = std::fs::read_to_string(state.join("box/state.lock.yaml")).unwrap();
    let hash_of = |s: &str| {
        s.lines()
            .find(|l| l.trim_start().starts_with("hash:"))
            .unwrap_or("<none>")
            .trim()
            .to_string()
    };
    assert_eq!(
        hash_of(&live),
        hash_of(&target),
        "the live lock does not describe generation 1's state after undo"
    );
}

/// CONTROL. Two identical applies, then undo: there is genuinely nothing to
/// revert, and undo must say so without touching the file.
///
/// A fix that unconditionally re-applied the target generation would revert the
/// earlier cases correctly and still fail here — it would rewrite a file after
/// telling the operator nothing would change. The mtime is the discriminator;
/// the content alone cannot tell the two apart.
#[test]
fn undo_that_changes_nothing_leaves_the_file_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    let cfg = apply_series(dir.path(), &state, &["same", "same"]);

    let target = dir.path().join("demo.txt");
    let before = mtime(&target);

    let (rc, out) = undo(&cfg, &state, &["--yes"]);
    assert_eq!(rc, 0, "an undo with nothing to do must succeed:\n{out}");
    assert_eq!(declared(dir.path()), "same\n", "content changed:\n{out}");
    assert_eq!(
        mtime(&target),
        before,
        "undo rewrote a file it had nothing to change. Output was:\n{out}"
    );
    assert!(
        !out.contains("Rolled back to generation"),
        "undo rolled the state dir back for a no-op; got:\n{out}"
    );
}

/// A generation recorded by forjar 1.22.0 or earlier has no config body. Undo
/// must REFUSE — never fall back to the current config, which is the defect.
#[test]
fn undo_refuses_a_generation_that_recorded_no_config() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    let cfg = apply_series(dir.path(), &state, &["v1", "v2", "v3"]);

    // Reproduce an old state dir exactly: the generation exists, holds its lock,
    // and has no recorded config.
    std::fs::remove_file(state.join("generations/1/.applied-config.yaml")).unwrap();

    let (rc, out) = undo(&cfg, &state, &["--yes"]);
    assert_ne!(
        rc, 0,
        "undo must refuse a generation it cannot replay, not fall back to the \
         current config; got:\n{out}"
    );
    assert!(
        out.contains("records no config"),
        "the refusal must name the missing snapshot; got:\n{out}"
    );
    assert_eq!(
        declared(dir.path()),
        "v3\n",
        "a refused undo must not touch the host; got:\n{out}"
    );
}

/// GH-376: a machine the target generation never recorded used to vanish from
/// the diff entirely. `compute_undo_diff` iterated the TARGET's machines only,
/// so a machine present in the live state but absent from the target was never
/// visited, `changes` came back empty, and undo printed "No changes between
/// generation X and Y" and exited 0 over a real difference — the loudest
/// possible change reported as no change at all.
#[test]
fn undo_does_not_hide_a_machine_the_target_generation_lacks() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    let cfg = apply_series(dir.path(), &state, &["v1", "v2", "v3"]);

    std::fs::remove_dir_all(state.join("generations/1/box")).unwrap();

    let (rc, out) = undo(&cfg, &state, &["--dry-run"]);
    assert_eq!(rc, 0, "undo --dry-run failed:\n{out}");
    assert!(
        out.contains("demo_file"),
        "the machine missing from the target generation was dropped from the diff \
         instead of being reported; got:\n{out}"
    );
    assert!(
        !out.contains("No changes between"),
        "undo claimed there was nothing to do; got:\n{out}"
    );
}

/// An empty diff against a target generation that recorded no machine state is
/// not a success. "No changes between generation X and Y" plus exit 0 was a
/// silent success for a request undo had not fulfilled: it neither knew nor
/// could verify what the host was supposed to hold.
#[test]
fn undo_refuses_a_target_generation_with_no_recorded_state() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    let cfg = apply_series(dir.path(), &state, &["v1", "v2", "v3"]);

    // A generation whose machine state was lost, against a state dir whose own
    // lock is gone too — nothing on either side to compare, so the old code's
    // diff was empty and it returned Ok.
    std::fs::remove_dir_all(state.join("generations/1/box")).unwrap();
    std::fs::remove_dir_all(state.join("box")).unwrap();

    let (rc, out) = undo(&cfg, &state, &["--yes"]);
    assert_ne!(
        rc, 0,
        "undo reported success for a generation whose state it never had; got:\n{out}"
    );
    assert!(
        out.contains("records no machine state"),
        "the refusal must say why; got:\n{out}"
    );
    assert_eq!(declared(dir.path()), "v3\n", "host must be untouched");
}

/// `--dry-run` must describe the undo that `--yes` would perform, and change
/// nothing. It is the only preview an operator gets before a destructive step.
#[test]
fn undo_dry_run_previews_without_touching_the_host() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    let cfg = apply_series(dir.path(), &state, &["v1", "v2", "v3"]);

    let (rc, out) = undo(&cfg, &state, &["--dry-run"]);
    assert_eq!(rc, 0, "undo --dry-run failed:\n{out}");
    assert!(
        out.contains("demo_file"),
        "the preview must name the resource; got:\n{out}"
    );
    assert_eq!(declared(dir.path()), "v3\n", "dry run changed the host");
}
