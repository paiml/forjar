//! PMAT-565 (forjar#565, paiml/infra#605 third signature): the per-machine
//! lock names the binary that WROTE it, on every write.
//!
//! MEASURED, four fleet locks under one 1.30.0 binary, 2026-09-15:
//!
//! ```text
//! state/yoga/state.lock.yaml   generator: forjar 1.1.1    generated_at: 2026-09-15T09:19:01Z
//! state/gx10/state.lock.yaml   generator: forjar 1.13.1   generated_at: 2026-09-15T09:18:56Z
//! ```
//!
//! Two files rewritten in the same minute by the same binary, claiming two
//! different, long-gone writers. `generator` was stamped ONCE by `new_lock`
//! and never touched again; `generated_at` was refreshed on every apply. The
//! pair is a claim no version of forjar could have made — the artifact lies
//! about itself, the same class as exit-0-over-red.
//!
//! The fix lives in the ONE writer, `state::save_lock`: every write stamps
//! `generator` with the writing binary and, the first time, preserves the
//! value it is replacing as `created_by`, so the creator is named rather
//! than erased. `forjar lock --restamp` walks a state dir and writes every
//! lock through it once, so the fleet converges in one run.

use forjar::core::state::{load_lock, new_lock, save_lock};
use std::fs;
use std::path::Path;
use std::process::Command;

const FORJAR: &str = env!("CARGO_BIN_EXE_forjar");
const FAKE: &str = "forjar 0.0.0-fake-first-writer";

/// What the binary says it is — the only acceptable `generator`.
fn writer() -> String {
    let out = Command::new(FORJAR)
        .arg("--version")
        .output()
        .expect("forjar --version");
    let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
    assert!(v.starts_with("forjar "), "{v}");
    v
}

fn read_lock_text(state: &Path, machine: &str) -> String {
    fs::read_to_string(state.join(machine).join("state.lock.yaml")).unwrap()
}

/// THE REGRESSION. A lock whose in-memory `generator` is a fake first-writer
/// string is written; the FILE must carry the real writer, and the fake must
/// survive only as `created_by`.
#[test]
fn a_write_stamps_the_real_writer_and_keeps_the_creator() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    let mut lock = new_lock("box", "box");
    lock.generator = FAKE.to_string();

    save_lock(&state, &lock).unwrap();

    let back = load_lock(&state, "box").unwrap().expect("lock exists");
    assert_eq!(
        back.generator,
        writer(),
        "the file must name the binary that wrote it, not whatever the struct carried"
    );
    assert_eq!(
        back.created_by.as_deref(),
        Some(FAKE),
        "the value being replaced is the creator, and it must be kept, not erased"
    );
}

/// A second write by the same binary leaves the creator alone and keeps the
/// writer current: the creator is set once, the writer every time.
#[test]
fn the_creator_is_set_once_and_the_writer_every_time() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    let mut lock = new_lock("box", "box");
    lock.generator = FAKE.to_string();
    save_lock(&state, &lock).unwrap();

    let mut second = load_lock(&state, "box").unwrap().unwrap();
    second.generator = "forjar 9.9.9-somebody-else".to_string();
    save_lock(&state, &second).unwrap();

    let back = load_lock(&state, "box").unwrap().unwrap();
    assert_eq!(back.generator, writer());
    assert_eq!(
        back.created_by.as_deref(),
        Some(FAKE),
        "the creator must not roll"
    );
}

/// A pre-existing lock file with no `created_by` (every lock on the fleet
/// today) still loads, and its stale `generator` becomes its `created_by`
/// on the first write.
#[test]
fn a_legacy_lock_migrates_its_stale_generator_into_created_by() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    fs::create_dir_all(state.join("yoga")).unwrap();
    fs::write(
        state.join("yoga").join("state.lock.yaml"),
        "schema: '1.0'\nmachine: yoga\nhostname: yoga\ngenerated_at: 2026-09-15T09:19:01Z\ngenerator: forjar 1.1.1\nblake3_version: '1.8'\nresources: {}\n",
    )
    .unwrap();

    let legacy = load_lock(&state, "yoga").unwrap().unwrap();
    assert_eq!(legacy.generator, "forjar 1.1.1");
    assert_eq!(
        legacy.created_by, None,
        "the fleet's locks carry no created_by yet"
    );

    save_lock(&state, &legacy).unwrap();
    let back = load_lock(&state, "yoga").unwrap().unwrap();
    assert_eq!(back.generator, writer());
    assert_eq!(back.created_by.as_deref(), Some("forjar 1.1.1"));
}

/// The apply path writes through the same writer: after `forjar apply`, the
/// lock on disk names this binary, whatever it said before.
#[test]
fn apply_rewrites_the_writer() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    let target = dir.path().join("t.txt");
    let cfg = dir.path().join("forjar.yaml");
    fs::write(
        &cfg,
        format!(
            "version: \"1.0\"\nname: w\nmachines:\n  box:\n    hostname: box\n    addr: 127.0.0.1\nresources:\n  f:\n    type: file\n    machine: box\n    path: {}\n    content: \"x\\n\"\n",
            target.display()
        ),
    )
    .unwrap();
    let ok = Command::new(FORJAR)
        .args([
            "apply",
            "-f",
            cfg.to_str().unwrap(),
            "--state-dir",
            state.to_str().unwrap(),
            "--yes",
        ])
        .output()
        .unwrap();
    assert!(
        ok.status.success(),
        "{}",
        String::from_utf8_lossy(&ok.stderr)
    );

    // Forge a legacy lock the way the fleet's carry one — a stale writer and
    // no `created_by` at all — then apply again.
    let text = read_lock_text(&state, "box")
        .replace(
            &format!("generator: {}", writer()),
            "generator: forjar 1.1.1",
        )
        .lines()
        .filter(|l| !l.starts_with("created_by:"))
        .map(|l| format!("{l}\n"))
        .collect::<String>();
    assert!(
        text.contains("generator: forjar 1.1.1") && !text.contains("created_by"),
        "{text}"
    );
    fs::write(state.join("box").join("state.lock.yaml"), &text).unwrap();
    // The integrity sidecar no longer matches the forged file; reseal it so the
    // second apply is refused for nothing but what this test is about.
    let reseal = Command::new(FORJAR)
        .args([
            "reseal",
            "--file",
            state.join("box").join("state.lock.yaml").to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        reseal.status.success(),
        "{}",
        String::from_utf8_lossy(&reseal.stderr)
    );

    let again = Command::new(FORJAR)
        .args([
            "apply",
            "-f",
            cfg.to_str().unwrap(),
            "--state-dir",
            state.to_str().unwrap(),
            "--yes",
        ])
        .output()
        .unwrap();
    assert!(
        again.status.success(),
        "{}",
        String::from_utf8_lossy(&again.stderr)
    );
    let after = read_lock_text(&state, "box");
    assert!(
        after.contains(&format!("generator: {}", writer())),
        "{after}"
    );
    assert!(after.contains("created_by: forjar 1.1.1"), "{after}");
}

/// THE ONE-SHOT. `forjar lock --restamp` rewrites every lock under the state
/// dir through the writer, names each change, and `--dry-run` writes nothing.
#[test]
fn lock_restamp_converges_every_lock_in_one_run() {
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    for (m, g) in [
        ("yoga", "forjar 1.1.1"),
        ("gx10", "forjar 1.13.1"),
        ("intel", "forjar 1.27.0"),
    ] {
        fs::create_dir_all(state.join(m)).unwrap();
        fs::write(
            state.join(m).join("state.lock.yaml"),
            format!("schema: '1.0'\nmachine: {m}\nhostname: {m}\ngenerated_at: 2026-09-10T07:15:25Z\ngenerator: {g}\nblake3_version: '1.8'\nresources: {{}}\n"),
        )
        .unwrap();
    }

    let dry = Command::new(FORJAR)
        .args([
            "lock",
            "--restamp",
            "--dry-run",
            "--state-dir",
            state.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        dry.status.success(),
        "{}",
        String::from_utf8_lossy(&dry.stderr)
    );
    let dry_out = String::from_utf8_lossy(&dry.stdout);
    assert!(
        dry_out.contains("yoga") && dry_out.contains("forjar 1.1.1"),
        "{dry_out}"
    );
    assert!(
        read_lock_text(&state, "yoga").contains("generator: forjar 1.1.1"),
        "--dry-run must write nothing"
    );

    let run = Command::new(FORJAR)
        .args(["lock", "--restamp", "--state-dir", state.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let out = String::from_utf8_lossy(&run.stdout);
    assert!(out.contains("3 lock(s) restamped"), "{out}");
    for (m, g) in [
        ("yoga", "forjar 1.1.1"),
        ("gx10", "forjar 1.13.1"),
        ("intel", "forjar 1.27.0"),
    ] {
        let back = load_lock(&state, m).unwrap().unwrap();
        assert_eq!(back.generator, writer(), "{m}");
        assert_eq!(back.created_by.as_deref(), Some(g), "{m}");
        assert!(
            state.join(m).join("state.lock.yaml.b3").exists(),
            "{m}: the sidecar must be rewritten with the lock"
        );
    }

    // Idempotent: a second run changes nothing and says so.
    let again = Command::new(FORJAR)
        .args(["lock", "--restamp", "--state-dir", state.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(again.status.success());
    assert!(
        String::from_utf8_lossy(&again.stdout).contains("0 lock(s) restamped"),
        "{}",
        String::from_utf8_lossy(&again.stdout)
    );
}
