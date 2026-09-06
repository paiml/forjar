//! forjar#469 (PMAT-161): ONE state dir, N stacks, keyed by config name.
//!
//! THE DEFECT, measured 2026-09-05 on 1.24.0 → 1.25.2 (found by paiml/infra#442).
//! paiml/infra keeps one `state/` dir for six machine manifests
//! (`machines/<m>/forjar.yaml`, each with its own `name:` and its own machine),
//! and every `machines/*/Makefile` applies with `--state-dir state`. The lock
//! file has always been keyed by config name for its machine sections, but the
//! STAMP — "which stack last applied here" — was a single value, so every apply
//! re-stamped the dir as a different stack:
//!
//! ```text
//! $ forjar apply -f machines/mini/forjar.yaml --state-dir state
//! warning: state dir state was last applied by stack 'forjar-clean-room';
//!          this apply re-stamps it as 'mini' … `forjar undo` refuses this combination
//! ```
//!
//! Every one of those applies touched only its own lock section. The warning
//! was a false positive on EVERY apply of a layout forjar supports, and the
//! `undo` it threatened refused that layout for real.
//!
//! WHAT THIS SUITE PINS. The stamp is a map, `stacks: {name -> StackStamp}`,
//! and both `apply` (warns) and `undo` (refuses) ask ONE question of it —
//! `state::stack_conflict`, so they cannot disagree about what "wrong stack"
//! means. N names with their own machines are silent and undoable; the two
//! conditions that are genuinely dangerous still fire:
//!
//! * the SAME name from a DIFFERENT `-f` (GH-377's case), and
//! * a machine another stack owns, because generations and per-machine locks
//!   are keyed by machine name alone.
//!
//! Every assertion here is at the binary level, because the defect was only
//! visible as stderr text and a lock file on disk.

use std::path::{Path, PathBuf};
use std::process::Command;

use forjar::core::state;

fn forjar() -> Command {
    Command::new(env!("CARGO_BIN_EXE_forjar"))
}

/// One stack, in its own directory, with its OWN machine name and its own file
/// resource — the paiml/infra shape. `machines/<m>/forjar.yaml` differs from
/// its neighbours in exactly these three ways.
fn write_stack(root: &Path, dir: &str, name: &str, machine: &str, content: &str) -> PathBuf {
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
  {machine}:
    hostname: localhost
    addr: 127.0.0.1
    transport: local
resources:
  {name}_file:
    type: file
    machine: {machine}
    path: {}
    content: "{content}\n"
"#,
            d.join("marker.txt").display()
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

fn undo(cfg: &Path, state: &Path) -> (i32, String) {
    run(&[
        "undo",
        "-f",
        &cfg.display().to_string(),
        "--state-dir",
        &state.display().to_string(),
        "--yes",
    ])
}

/// Every `warning:` line, whatever printed it. The point of forjar#469 is that
/// a supported layout produces NONE — a warning an operator sees on every
/// correct apply is a warning they stop reading.
///
/// ONE exclusion, and it is not a stack signal: named snapshots carry a
/// second-resolution timestamp, so two applies inside the same second collide
/// on the name and the second prints "pre-apply snapshot failed … already
/// exists". A test that drives five applies in three seconds hits that; an
/// operator does not. Excluded by its text so anything else still fails here.
fn warnings(out: &str) -> Vec<String> {
    out.lines()
        .filter(|l| l.trim_start().starts_with("warning:"))
        .filter(|l| !l.contains("pre-apply snapshot failed"))
        .map(str::to_string)
        .collect()
}

fn lock_of(state: &Path) -> forjar::core::types::GlobalLock {
    state::load_global_lock(state).unwrap().unwrap()
}

fn machine_lock_bytes(state: &Path, machine: &str) -> Vec<u8> {
    std::fs::read(state.join(machine).join("state.lock.yaml")).unwrap()
}

fn marker(root: &Path, dir: &str) -> String {
    std::fs::read_to_string(root.join(dir).join("marker.txt")).unwrap()
}

/// alpha/bravo/charlie, distinct machines, distinct resources, ONE state dir.
struct Fleet {
    _dir: tempfile::TempDir,
    root: PathBuf,
    state: PathBuf,
    bravo: PathBuf,
}

fn fleet() -> (Fleet, Vec<String>) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let state = root.join("state");
    let alpha = write_stack(&root, "alpha", "alpha", "mini", "one");
    let bravo = write_stack(&root, "bravo", "bravo", "lambda-labs", "one");
    let charlie = write_stack(&root, "charlie", "charlie", "clean-room", "one");

    let mut warned = Vec::new();
    for cfg in [&alpha, &bravo, &charlie] {
        let (rc, out) = apply(cfg, &state);
        assert_eq!(rc, 0, "apply of {} failed:\n{out}", cfg.display());
        warned.extend(warnings(&out));
    }
    (
        Fleet {
            _dir: dir,
            root,
            state,
            bravo,
        },
        warned,
    )
}

/// (a) THE BLOCKER. Three configs, one state dir, zero warnings — and the lock
/// holds three stamps, not one that keeps being overwritten.
#[test]
fn three_stacks_through_one_state_dir_warn_about_nothing() {
    let (f, warned) = fleet();
    assert!(
        warned.is_empty(),
        "the supported six-stack layout warned on a correct apply (forjar#469):\n{}",
        warned.join("\n"),
    );

    let lock = lock_of(&f.state);
    let mut names: Vec<&str> = lock.stacks.keys().map(String::as_str).collect();
    names.sort_unstable();
    assert_eq!(
        names,
        ["alpha", "bravo", "charlie"],
        "each config must have its OWN stamp; the single stamp is what made every \
         apply look like a different stack"
    );
    assert_eq!(lock.stamp_for("alpha").unwrap().machines, ["mini"]);
    assert_eq!(lock.stamp_for("bravo").unwrap().machines, ["lambda-labs"]);
    assert_eq!(lock.stamp_for("charlie").unwrap().machines, ["clean-room"]);
}

/// (a) continued: `undo` of ONE stack is not refused, reverts that stack, and
/// leaves the other two sections byte-identical — then that stack re-applies
/// without a warning.
///
/// The re-apply is not decoration. `undo` replays the target generation from a
/// config staged in a temp sibling file; a stamp that recorded THAT path would
/// pin the stack to a file deleted seconds later, and the operator's next real
/// apply would look like a different `-f` — reintroducing the false positive
/// through the back door.
#[test]
fn undoing_one_stack_leaves_the_other_two_untouched() {
    let (f, _) = fleet();
    // Two more bravo applies, so the generation `undo` targets is bravo's own.
    write_stack(&f.root, "bravo", "bravo", "lambda-labs", "two");
    assert_eq!(apply(&f.bravo, &f.state).0, 0);
    write_stack(&f.root, "bravo", "bravo", "lambda-labs", "three");
    assert_eq!(apply(&f.bravo, &f.state).0, 0);

    let before_alpha = machine_lock_bytes(&f.state, "mini");
    let before_charlie = machine_lock_bytes(&f.state, "clean-room");
    let stamps_before = lock_of(&f.state);

    let (rc, out) = undo(&f.bravo, &f.state);
    assert_eq!(
        rc, 0,
        "undo of one stack in a shared state dir was refused:\n{out}"
    );
    assert_eq!(
        marker(&f.root, "bravo"),
        "two\n",
        "undo did not revert its own stack:\n{out}"
    );
    assert_eq!(marker(&f.root, "alpha"), "one\n", "undo touched alpha");
    assert_eq!(marker(&f.root, "charlie"), "one\n", "undo touched charlie");
    assert_eq!(
        machine_lock_bytes(&f.state, "mini"),
        before_alpha,
        "undo of bravo rewrote alpha's machine lock:\n{out}"
    );
    assert_eq!(
        machine_lock_bytes(&f.state, "clean-room"),
        before_charlie,
        "undo of bravo rewrote charlie's machine lock:\n{out}"
    );

    let after = lock_of(&f.state);
    assert_eq!(after.stamp_for("alpha"), stamps_before.stamp_for("alpha"));
    assert_eq!(
        after.stamp_for("charlie"),
        stamps_before.stamp_for("charlie")
    );

    let (rc, out) = apply(&f.bravo, &f.state);
    assert_eq!(rc, 0, "re-apply after undo failed:\n{out}");
    assert!(
        warnings(&out).is_empty(),
        "the re-apply after an undo warned — the replay stamped the stack with the \
         staged temp config:\n{out}"
    );
}

/// (b) The GH-377 case survives, narrowed: the SAME name from a DIFFERENT `-f`.
/// `apply` warns (it does what its arguments say); `undo` refuses (its plan and
/// its work would be about different stacks).
#[test]
fn the_same_name_from_another_config_file_warns_and_undo_refuses() {
    let (f, _) = fleet();
    // A copy of alpha at another path, with its own resource — the wrong-stack
    // case: one name, two configs.
    let copy = write_stack(&f.root, "alpha-copy", "alpha", "mini", "one");

    let (rc, out) = undo(&copy, &f.state);
    assert_ne!(rc, 0, "undo from a different -f for the same name:\n{out}");
    assert!(
        out.contains("was last applied from"),
        "the refusal must carry the same sentence apply warns with; got:\n{out}"
    );
    assert!(
        out.contains(&f.state.display().to_string()),
        "the refusal must print the absolute state dir; got:\n{out}"
    );

    let (rc, out) = apply(&copy, &f.state);
    assert_eq!(rc, 0, "apply must not refuse, only warn:\n{out}");
    assert!(
        out.contains("was last applied from"),
        "apply must still warn on the same name from another -f; got:\n{out}"
    );
}

/// (c) The machine-ownership half. Generations and per-machine locks are keyed
/// by machine name ALONE, so a second stack writing another stack's machine
/// overwrites its history — the same wrong-stack condition, in a different
/// disguise.
#[test]
fn a_machine_another_stack_owns_warns_and_undo_refuses() {
    let (f, _) = fleet();
    // delta is a new NAME (so nothing above fires) that names alpha's machine.
    let delta = write_stack(&f.root, "delta", "delta", "mini", "one");

    let (rc, out) = apply(&delta, &f.state);
    assert_eq!(rc, 0, "apply must not refuse, only warn:\n{out}");
    assert!(
        out.contains("which stack 'alpha' owns"),
        "apply must warn that delta writes alpha's machine; got:\n{out}"
    );

    let (rc, out) = undo(&delta, &f.state);
    assert_ne!(
        rc, 0,
        "undo replayed generations keyed by a machine another stack owns:\n{out}"
    );
    assert!(
        out.contains("which stack 'alpha' owns"),
        "the refusal must carry the same sentence apply warns with; got:\n{out}"
    );
}

/// (d) MIGRATION. A dir carrying the pre-#469 single stamp — written here by
/// hand, in the old format, because a fixture generated by this binary could
/// not be the format that shipped — migrates on the first apply that reads it,
/// silently, and comes out as 1.1 with the old stamp under its own name.
#[test]
fn a_legacy_single_stamp_dir_migrates_on_the_next_apply() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let state = root.join("state");
    let alpha = write_stack(&root, "alpha", "alpha", "mini", "one");
    assert_eq!(apply(&alpha, &state).0, 0);

    let legacy = r#"schema: "1.0"
name: alpha
last_apply: "2026-01-01T00:00:00Z"
generator: forjar 1.24.0
machines:
  mini:
    resources: 1
    converged: 1
    failed: 0
    last_apply: "2026-01-01T00:00:00Z"
"#;
    std::fs::write(state.join("forjar.lock.yaml"), legacy).unwrap();
    // The sidecar is BLAKE3 of the bytes we just replaced. A stale one is a
    // hard apply failure by design (FJ-1270), and re-sealing is not what this
    // test is about; a MISSING sidecar is the documented soft case.
    let _ = std::fs::remove_file(state.join("forjar.lock.yaml.b3"));

    let (rc, out) = apply(&alpha, &state);
    assert_eq!(rc, 0, "apply against a legacy state dir failed:\n{out}");
    assert!(
        warnings(&out).is_empty(),
        "the upgrade itself must be quiet — a migrated stamp records no file, and \
         'origin unknown' is not evidence of a mismatch:\n{out}"
    );

    let lock = lock_of(&state);
    assert_eq!(lock.schema, "1.1", "the migration must be persisted");
    assert_eq!(
        lock.stamp_for("alpha").unwrap().machines,
        ["mini"],
        "the old single stamp must become the entry for its own name"
    );
}

/// (e) `status` attributes each machine to the stack that wrote it, from the
/// stacks map — not all of them to whichever stack happened to apply last.
#[test]
fn status_names_each_machine_under_its_own_stack() {
    let (f, _) = fleet();
    let (rc, out) = run(&["status", "--state-dir", &f.state.display().to_string()]);
    assert_eq!(rc, 0, "status failed:\n{out}");

    for (machine, stack) in [
        ("mini", "alpha"),
        ("lambda-labs", "bravo"),
        ("clean-room", "charlie"),
    ] {
        assert_eq!(
            stack_shown_for(&out, machine).as_deref(),
            Some(stack),
            "status must name {machine} under stack {stack}; got:\n{out}"
        );
    }
}

/// The stack `status` printed under a machine's block, if any.
fn stack_shown_for(out: &str, machine: &str) -> Option<String> {
    let mut lines = out
        .lines()
        .skip_while(|l| !l.starts_with(&format!("Machine: {machine} ")));
    lines.next()?;
    lines
        .take_while(|l| !l.starts_with("Machine: "))
        .find_map(|l| l.trim().strip_prefix("Stack: ").map(str::to_string))
}

/// (f) OVER-CORRECTION GUARD. One config, one state dir — the overwhelmingly
/// common case — is untouched by all of the above: no warning, undo allowed,
/// and `status` prints no per-stack attribution at all.
#[test]
fn a_single_stack_dir_behaves_exactly_as_before() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let state = root.join("state");
    let alpha = write_stack(&root, "alpha", "alpha", "mini", "one");

    for content in ["one", "two"] {
        write_stack(&root, "alpha", "alpha", "mini", content);
        let (rc, out) = apply(&alpha, &state);
        assert_eq!(rc, 0, "apply failed:\n{out}");
        assert!(
            warnings(&out).is_empty(),
            "single-stack apply warned:\n{out}"
        );
    }

    let (rc, out) = undo(&alpha, &state);
    assert_eq!(rc, 0, "a normal single-stack undo was refused:\n{out}");
    assert_eq!(marker(&root, "alpha"), "one\n", "the undo did not revert");

    let (rc, out) = run(&["status", "--state-dir", &state.display().to_string()]);
    assert_eq!(rc, 0, "status failed:\n{out}");
    assert!(
        !out.contains("Stack: ") && !out.contains("Stacks: "),
        "a single-stack dir must print exactly what it printed before #469:\n{out}"
    );
}
