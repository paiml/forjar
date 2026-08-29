//! `forjar undo` joined one stack's config to another stack's state dir.
//!
//! THE DEFECT, measured on 1.22.0. `-f/--file` defaults to `forjar.yaml` in the
//! CWD; `--state-dir` is a separate argument. Nothing checked that the two
//! named the same stack. Run from a directory holding stack B's config:
//!
//! ```text
//! $ cd stackB && forjar undo --state-dir ../stateA --yes
//! Undo: generation 2 → 1
//!   ~ aaa_file (box): will be updated     # <- a resource stack B does not declare
//! Rolled back to generation 1
//! box: 1 converged, 0 unchanged, 0 failed # <- and it converged stack B's zzz_file
//! $ ls stackB/zzz.txt                     # created, against stack A's state
//! $ grep name: stateA/forjar.lock.yaml
//! name: stack-bravo                       # <- A's state dir re-stamped as B's
//! ```
//!
//! Exit 0, no warning. The plan it PRINTED was computed from stack A's
//! generations; the work it DID was stack B's resources. Those are different
//! stacks. In the forjar repo checkout the same invocation would have run a
//! ~1600-entry `type: package` apt resource against the host.
//!
//! The re-apply then re-stamped `forjar.lock.yaml` with the intruder's name,
//! destroying the only evidence that the two dirs had ever belonged to
//! different stacks — which is why the guard has to run BEFORE the re-apply,
//! and why these tests assert on the lock's `name:` as well as on the files.
//!
//! THE SIGNAL. `state/forjar.lock.yaml` carries `name:`, copied verbatim from
//! the config's required top-level `name` on every apply. It is the only stack
//! identity the state dir records that survives an edit to the config, and it
//! was already there.
//!
//! SCOPE, DELIBERATELY NARROW. `undo` and `undo --resume` REFUSE. `apply` only
//! warns: it does exactly what its two arguments say, it runs unattended in
//! every CI job and cron, and state dirs already carrying a foreign name exist
//! in the wild — promoting apply to a refusal would break them on upgrade.
//! `one_stack_editing_and_undoing_is_never_refused` and
//! `a_state_dir_with_no_global_lock_is_not_refused` are the over-correction
//! guards: a guard that fires on a normal cycle, or on a state dir that legally
//! has no lock (`rollback --generation 0` produces one), would be worse than
//! the defect.

use std::path::{Path, PathBuf};
use std::process::Command;

fn forjar() -> Command {
    Command::new(env!("CARGO_BIN_EXE_forjar"))
}

/// A one-resource stack in its own directory, named and applied independently.
fn write_stack(root: &Path, dir: &str, name: &str, resource: &str, content: &str) -> PathBuf {
    let d = root.join(dir);
    std::fs::create_dir_all(&d).unwrap();
    let cfg = d.join("forjar.yaml");
    std::fs::write(
        &cfg,
        format!(
            r#"version: "1.0"
name: {name}
policy:
  snapshot_generations: 10
machines:
  box:
    hostname: localhost
    addr: 127.0.0.1
    transport: local
resources:
  {resource}:
    type: file
    machine: box
    path: {}
    content: "{content}\n"
"#,
            d.join(format!("{resource}.txt")).display()
        ),
    )
    .unwrap();
    cfg
}

/// Run with an explicit working directory — the whole point is that `-f`
/// defaults to `forjar.yaml` in the CWD.
fn run_in(cwd: &Path, args: &[&str]) -> (i32, String) {
    let out = forjar().current_dir(cwd).args(args).output().unwrap();
    let mut merged = String::from_utf8_lossy(&out.stdout).into_owned();
    merged.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.code().unwrap_or(-1), merged)
}

fn apply(cfg: &Path, state: &Path) -> (i32, String) {
    let dir = cfg.parent().unwrap();
    run_in(
        dir,
        &[
            "apply",
            "-f",
            &cfg.display().to_string(),
            "--state-dir",
            &state.display().to_string(),
            "--yes",
        ],
    )
}

fn lock_name(state: &Path) -> String {
    std::fs::read_to_string(state.join("forjar.lock.yaml"))
        .unwrap()
        .lines()
        .find(|l| l.starts_with("name:"))
        .unwrap()
        .to_string()
}

/// Stack A applied twice into `stateA`; stack B's config exists but is unapplied.
struct TwoStacks {
    _dir: tempfile::TempDir,
    root: PathBuf,
    state_a: PathBuf,
    cfg_a: PathBuf,
    dir_b: PathBuf,
}

fn two_stacks() -> TwoStacks {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let state_a = root.join("stateA");
    let cfg_a = write_stack(&root, "A", "stack-alpha", "aaa_file", "one");
    write_stack(&root, "B", "stack-bravo", "zzz_file", "one");
    for _ in 0..2 {
        let (rc, out) = apply(&cfg_a, &state_a);
        assert_eq!(rc, 0, "stack A apply failed:\n{out}");
    }
    TwoStacks {
        _dir: dir,
        dir_b: root.join("B"),
        root,
        state_a,
        cfg_a,
    }
}

/// The blocker, verbatim: stack B's CWD config against stack A's state dir.
#[test]
fn undo_refuses_a_state_dir_owned_by_another_stack() {
    let s = two_stacks();
    let (rc, out) = run_in(
        &s.dir_b,
        &[
            "undo",
            "--state-dir",
            &s.state_a.display().to_string(),
            "--yes",
        ],
    );

    assert_ne!(rc, 0, "undo joined two stacks and exited 0:\n{out}");
    assert!(
        !s.dir_b.join("zzz_file.txt").exists(),
        "undo applied stack B's resource against stack A's state:\n{out}"
    );
    assert_eq!(
        lock_name(&s.state_a),
        "name: stack-alpha",
        "the re-apply re-stamped stack A's state dir with the intruder's name — the \
         guard must run BEFORE the re-apply:\n{out}"
    );
    assert!(
        out.contains("different stack")
            && out.contains("stack-alpha")
            && out.contains("stack-bravo"),
        "the refusal must name both stacks; got:\n{out}"
    );
    assert!(
        out.contains(&s.state_a.display().to_string()),
        "the refusal must print the absolute state dir — `-f` defaulting to a bare \
         relative `forjar.yaml` is what made this invisible; got:\n{out}"
    );
}

/// `--dry-run` must refuse too. Its diff would be read from the other stack's
/// generations, so it is not a preview of anything that could happen.
#[test]
fn undo_dry_run_refuses_a_state_dir_owned_by_another_stack() {
    let s = two_stacks();
    let (rc, out) = run_in(
        &s.dir_b,
        &[
            "undo",
            "--state-dir",
            &s.state_a.display().to_string(),
            "--dry-run",
        ],
    );
    assert_ne!(rc, 0, "a mismatched dry run printed a plan:\n{out}");
    assert!(out.contains("different stack"), "got:\n{out}");
}

/// `--resume` is the identical bypass one flag away: it reads
/// `state_dir/<machine>/undo-progress.yaml` for the machines named in the CWD
/// config, and machine names (`local`, `web`, `box`) collide constantly.
#[test]
fn undo_resume_refuses_a_state_dir_owned_by_another_stack() {
    let s = two_stacks();
    let (rc, out) = run_in(
        &s.dir_b,
        &[
            "undo",
            "--resume",
            "--state-dir",
            &s.state_a.display().to_string(),
            "--yes",
        ],
    );
    assert_ne!(rc, 0, "undo --resume joined two stacks:\n{out}");
    assert!(
        out.contains("different stack"),
        "the mismatch must outrank 'no partial undo found'; got:\n{out}"
    );
}

/// OVER-CORRECTION GUARD. Editing a config and undoing the edit is the normal
/// cycle. The stack name does not change when the config does, so a guard keyed
/// on anything else — the config hash, say — would refuse every second apply.
#[test]
fn one_stack_editing_and_undoing_is_never_refused() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    let target = dir.path().join("A/aaa_file.txt");

    let cfg = write_stack(dir.path(), "A", "stack-alpha", "aaa_file", "one");
    let (rc, out) = apply(&cfg, &state);
    assert_eq!(rc, 0, "apply #1 failed:\n{out}");
    write_stack(dir.path(), "A", "stack-alpha", "aaa_file", "two");
    let (rc, out) = apply(&cfg, &state);
    assert_eq!(rc, 0, "apply #2 failed:\n{out}");
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "two\n");

    let (rc, out) = run_in(
        dir.path(),
        &[
            "undo",
            "-f",
            &cfg.display().to_string(),
            "--state-dir",
            &state.display().to_string(),
            "--yes",
        ],
    );
    assert_eq!(rc, 0, "a normal edit-apply-undo cycle was refused:\n{out}");
    assert_eq!(
        std::fs::read_to_string(&target).unwrap(),
        "one\n",
        "the undo was allowed but did not revert:\n{out}"
    );
}

/// OVER-CORRECTION GUARD. A state dir with no `forjar.lock.yaml` is reachable
/// in normal use — `rollback --generation 0` restores a generation that
/// predates the first global lock and leaves the dir without one. Absence must
/// ALLOW; only a name that is present and different refuses.
#[test]
fn a_state_dir_with_no_global_lock_is_not_refused() {
    let s = two_stacks();
    std::fs::remove_file(s.state_a.join("forjar.lock.yaml")).unwrap();

    let (rc, out) = run_in(
        &s.dir_b,
        &[
            "undo",
            "--state-dir",
            &s.state_a.display().to_string(),
            "--dry-run",
        ],
    );
    assert!(
        !out.contains("different stack"),
        "a lock-less state dir was refused as foreign; that bricks a state dir \
         `forjar rollback --generation 0` legitimately produces. rc={rc}, got:\n{out}"
    );
}

/// A stack that was genuinely renamed must have a way out, and it must not be a
/// new `--force` flag: one apply re-stamps the state dir, and undo works again.
#[test]
fn a_renamed_stack_is_unblocked_by_one_apply() {
    let s = two_stacks();
    let renamed = std::fs::read_to_string(&s.cfg_a)
        .unwrap()
        .replace("name: stack-alpha", "name: stack-renamed");
    std::fs::write(&s.cfg_a, renamed).unwrap();

    let (rc, out) = run_in(
        &s.root,
        &[
            "undo",
            "-f",
            &s.cfg_a.display().to_string(),
            "--state-dir",
            &s.state_a.display().to_string(),
            "--yes",
        ],
    );
    assert_ne!(rc, 0, "a renamed stack must refuse first:\n{out}");
    assert!(
        out.contains("run `forjar apply` once to re-stamp"),
        "the refusal must name the remedy; got:\n{out}"
    );

    let (rc, out) = apply(&s.cfg_a, &s.state_a);
    assert_eq!(rc, 0, "the remedy itself failed:\n{out}");
    assert!(
        out.contains("was last applied by stack 'stack-alpha'"),
        "apply must warn that it is re-stamping the dir — that warning is the only \
         signal an operator gets that two configs share one state dir; got:\n{out}"
    );

    let (rc, out) = run_in(
        &s.root,
        &[
            "undo",
            "-f",
            &s.cfg_a.display().to_string(),
            "--state-dir",
            &s.state_a.display().to_string(),
            "--yes",
        ],
    );
    assert_eq!(rc, 0, "undo still refused after the named remedy:\n{out}");
}
