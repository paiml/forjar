//! Refs #704: `apply -r <leaf> --dry-run` must list the repair of a drifted
//! `depends_on` prerequisite that the real `apply -r <leaf>` then performs.
//!
//! The lock records the prerequisite converged; its live check fails. The
//! drift gate sees that, but a dry run does not write the lock (#310), so the
//! plan used to read the stale lock and print "no changes" for a resource the
//! apply would go on to repair. The fix overlays the gate's findings on the
//! locks the dry-run plan reads.
//!
//! This drives the real binary over the local transport in a tempdir and
//! reads only what a user reads: the `--dry-run` stdout (its `--json` body and
//! its text body) and the bytes of the state dir. Removing the overlay from
//! the dry-run plan turns the drift rows red; the converged row stays green.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

const LEAF: &str = "leaf-file";

struct Fixture {
    dir: tempfile::TempDir,
    cfg: String,
}

impl Fixture {
    fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }
}

/// leaf-file -> base-file              (file prerequisite)
/// leaf-file -> pkg-upgrade            (task; completion check = marker exists)
/// other-file                          (outside the `-r leaf-file` selection)
fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let p = |n: &str| dir.path().join(n).display().to_string();
    let cfg = dir.path().join("forjar.yaml");
    std::fs::write(
        &cfg,
        format!(
            r#"version: "1.0"
name: dry-run-drift-repair
machines:
  local:
    hostname: localhost
    addr: 127.0.0.1
    transport: local
resources:
  base-file:
    type: file
    machine: local
    path: {base}
    content: "base declared"
  pkg-upgrade:
    type: task
    machine: local
    command: "echo ran >> '{runlog}' && touch '{marker}'"
    completion_check: "[ -f '{marker}' ]"
  leaf-file:
    type: file
    machine: local
    path: {leaf}
    content: "leaf declared"
    depends_on: [base-file, pkg-upgrade]
  other-file:
    type: file
    machine: local
    path: {other}
    content: "other declared"
policy:
  tripwire: true
"#,
            base = p("base.txt"),
            leaf = p("leaf.txt"),
            other = p("other.txt"),
            runlog = p("upgrade.runs"),
            marker = p("upgraded.marker"),
        ),
    )
    .unwrap();
    let f = Fixture {
        cfg: cfg.display().to_string(),
        dir,
    };
    // Converge everything, so the lock records every resource converged.
    let (out, ok) = run(&f, &["apply", "--yes", "-f", &f.cfg, "--state-dir", "st"]);
    assert!(ok, "the initial apply must converge:\n{out}");
    assert_eq!(runs(&f), 1, "the initial apply runs the task once:\n{out}");
    f
}

fn run(f: &Fixture, args: &[&str]) -> (String, bool) {
    let out = Command::new(env!("CARGO_BIN_EXE_forjar"))
        .current_dir(f.dir.path())
        .env("NO_COLOR", "1")
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

/// How many times the task's command has run.
fn runs(f: &Fixture) -> usize {
    std::fs::read_to_string(f.path("upgrade.runs"))
        .map(|s| s.lines().count())
        .unwrap_or(0)
}

/// Every file under the state dir, with its bytes.
fn state_bytes(f: &Fixture) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(dir: &Path, acc: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for e in std::fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                walk(&p, acc);
            } else {
                acc.insert(p.clone(), std::fs::read(&p).unwrap());
            }
        }
    }
    let mut acc = BTreeMap::new();
    walk(&f.path("st"), &mut acc);
    acc
}

/// `apply -r leaf-file --dry-run --json`: resource id -> action, from stdout.
fn dry_run_json(f: &Fixture) -> (BTreeMap<String, String>, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_forjar"))
        .current_dir(f.dir.path())
        .env("NO_COLOR", "1")
        .args([
            "apply",
            "--yes",
            "-f",
            &f.cfg,
            "--state-dir",
            "st",
            "-r",
            LEAF,
            "--dry-run",
            "--json",
        ])
        .output()
        .expect("forjar must run");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let all = format!("{stdout}{}", String::from_utf8_lossy(&out.stderr));
    assert!(out.status.success(), "the dry run must succeed:\n{all}");
    let start = stdout
        .find('{')
        .unwrap_or_else(|| panic!("no JSON body on stdout:\n{all}"));
    let v: serde_json::Value = serde_json::Deserializer::from_str(&stdout[start..])
        .into_iter::<serde_json::Value>()
        .next()
        .unwrap_or_else(|| panic!("no JSON body on stdout:\n{all}"))
        .unwrap_or_else(|e| panic!("dry-run JSON does not parse ({e}):\n{all}"));
    assert_eq!(v["dry_run"], true, "{all}");
    let actions = v["changes"]
        .as_array()
        .unwrap_or_else(|| panic!("no changes array:\n{all}"))
        .iter()
        .map(|c| {
            (
                c["resource"].as_str().unwrap().to_string(),
                c["action"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    (actions, all)
}

/// `apply -r leaf-file --dry-run` text body: the lines that list an action.
fn dry_run_text(f: &Fixture) -> String {
    let (out, ok) = run(
        f,
        &[
            "apply",
            "--yes",
            "-f",
            &f.cfg,
            "--state-dir",
            "st",
            "-r",
            LEAF,
            "--dry-run",
        ],
    );
    assert!(ok, "the text dry run must succeed:\n{out}");
    out
}

fn action<'a>(actions: &'a BTreeMap<String, String>, id: &str) -> &'a str {
    actions.get(id).map(String::as_str).unwrap_or("<absent>")
}

/// (a) a file prerequisite drifted: the dry run lists its repair.
#[test]
fn falsify_704_a_drifted_file_dependency_is_listed() {
    let f = fixture();
    std::fs::write(f.path("base.txt"), "tampered").unwrap();

    let (actions, out) = dry_run_json(&f);
    assert_eq!(
        action(&actions, "base-file"),
        "update",
        "the dry run hides the repair of a drifted dependency:\n{out}"
    );
    assert_eq!(
        std::fs::read_to_string(f.path("base.txt")).unwrap(),
        "tampered",
        "the dry run repaired the file:\n{out}"
    );

    // The text body says the same thing: `~ base-file`.
    let text = dry_run_text(&f);
    assert!(
        text.lines().any(|l| l.contains("~ base-file on local")),
        "the text dry run does not list base-file as an update:\n{text}"
    );

    // And the apply really does repair it, so the preview was owed.
    let (out, ok) = run(
        &f,
        &[
            "apply",
            "--yes",
            "-f",
            &f.cfg,
            "--state-dir",
            "st",
            "-r",
            LEAF,
        ],
    );
    assert!(ok, "{out}");
    assert_eq!(
        std::fs::read_to_string(f.path("base.txt")).unwrap(),
        "base declared",
        "apply -r leaf did not repair the dependency the dry run listed:\n{out}"
    );
}

/// (b) the incident shape: an upgrade-style task prerequisite whose completion
/// check fails. The dry run lists it and does not run its command.
#[test]
fn falsify_704_b_task_dependency_with_failing_check_is_listed() {
    let f = fixture();
    std::fs::remove_file(f.path("upgraded.marker")).unwrap();

    let (actions, out) = dry_run_json(&f);
    assert_eq!(
        action(&actions, "pkg-upgrade"),
        "update",
        "the dry run hides the task the apply would run:\n{out}"
    );
    assert_eq!(runs(&f), 1, "the dry run ran the task's command:\n{out}");

    let (out, ok) = run(
        &f,
        &[
            "apply",
            "--yes",
            "-f",
            &f.cfg,
            "--state-dir",
            "st",
            "-r",
            LEAF,
        ],
    );
    assert!(ok, "{out}");
    assert_eq!(
        runs(&f),
        2,
        "apply -r leaf did not run the task the dry run listed:\n{out}"
    );
}

/// (c) fully converged: the dry run lists no change at all.
#[test]
fn falsify_704_c_converged_lists_no_change() {
    let f = fixture();

    let (actions, out) = dry_run_json(&f);
    assert!(
        actions.contains_key(LEAF),
        "the selected resource is missing from the plan:\n{out}"
    );
    let changed: Vec<_> = actions.iter().filter(|(_, a)| *a != "noop").collect();
    assert!(
        changed.is_empty(),
        "a converged config previews changes {changed:?}:\n{out}"
    );
    assert!(
        dry_run_text(&f).contains("0 to add, 0 to change, 0 to destroy"),
        "the converged text summary is not all zeros"
    );
}

/// (d) drift outside the selection is neither listed nor repaired.
#[test]
fn falsify_704_d_drift_outside_selection_is_not_listed() {
    let f = fixture();
    std::fs::write(f.path("other.txt"), "tampered").unwrap();
    std::fs::write(f.path("base.txt"), "tampered").unwrap();

    let (actions, out) = dry_run_json(&f);
    assert_eq!(action(&actions, "base-file"), "update", "{out}");
    assert_eq!(
        action(&actions, "other-file"),
        "<absent>",
        "a resource outside -r leaf-file is in the dry run:\n{out}"
    );
}

/// (e) two dry runs write no lock: every state-dir byte is identical, and
/// both plan the same. Run with drift on both prerequisites.
#[test]
fn falsify_704_e_repeated_dry_runs_write_no_lock() {
    let f = fixture();
    std::fs::write(f.path("base.txt"), "tampered").unwrap();
    std::fs::remove_file(f.path("upgraded.marker")).unwrap();
    let before = state_bytes(&f);
    assert!(!before.is_empty(), "the converged apply wrote no state");

    let (first, out1) = dry_run_json(&f);
    let (second, out2) = dry_run_json(&f);
    assert_eq!(
        state_bytes(&f),
        before,
        "a dry run changed the state dir:\n{out1}\n{out2}"
    );
    assert_eq!(
        first, second,
        "two dry runs planned differently:\n{out1}\n{out2}"
    );
    assert_eq!(action(&first, "base-file"), "update", "{out1}");
    assert_eq!(action(&first, "pkg-upgrade"), "update", "{out1}");
    assert_eq!(runs(&f), 1, "a dry run ran the task's command:\n{out1}");
}
