//! PMAT-564 (forjar#564, paiml/infra#605 first signature): a drift run that
//! inspected NONE of the resources it was asked about declines, and never
//! grades resources from a manifest it was not given.
//!
//! MEASURED, forjar 1.30.0, yoga, 2026-09-15 11:51Z:
//!
//! ```text
//! $ forjar drift -f machines/yoga/forjar-ephemeral.yaml
//! Checking yoga (93 resources)...
//!   inspected 2 of 103 resource(s) in scope: file 2
//!   skipped 101: declared here, absent from the lock 10, in the lock, not in
//!   the config 65, no hash recorded in the lock 24, not converged in the lock 2
//!   DRIFTED: gitconfig on yoga (/home/noah/.gitconfig content changed)
//!   DRIFTED: zshrc on yoga (/home/noah/.zshrc content changed)
//! Drift detected: 2 resource(s)
//! rc=0
//! ```
//!
//! "declared here, absent from the lock 10" is every resource the manifest
//! declares. Inspected: 0 of them. The two it DID grade belong to
//! `machines/yoga/forjar.yaml`, a manifest this invocation did not name — and
//! the same census line calls "in the lock, not in the config" a SKIP
//! category, so grading two such resources contradicts its own census.
//!
//! The mechanism: the file detector walks the LOCK and never asks whether the
//! config declares the entry; the non-file detector does ask and skips
//! `NotInConfig`. A verdict about resources nobody asked about, over zero
//! coverage of the ones they did, exit 0 — the forjar#488 shape, and the
//! fleet's "0 violations over 0 files" signature.
//!
//! Exit vocabulary: `0 accept · 1 reject · 2 decline · 3 error`. Zero
//! coverage is a decline.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const FORJAR: &str = env!("CARGO_BIN_EXE_forjar");

/// A manifest declaring `file` resources with inline content on a local box.
fn manifest(name: &str, files: &[(&str, &Path)]) -> String {
    let mut s = format!(
        "version: \"1.0\"\nname: {name}\nmachines:\n  box:\n    hostname: box\n    addr: 127.0.0.1\nresources:\n"
    );
    for (id, path) in files {
        s.push_str(&format!(
            "  {id}:\n    type: file\n    machine: box\n    path: {}\n    content: \"declared {id}\\n\"\n",
            path.display()
        ));
    }
    s
}

struct Sandbox {
    dir: tempfile::TempDir,
    state: PathBuf,
}

fn run(args: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::new(FORJAR)
        .args(args)
        .arg("--no-color")
        .output()
        .expect("forjar must run");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

impl Sandbox {
    /// Manifest A applied: the lock holds `a-one` and `a-two`.
    fn with_stack_a_applied() -> (Self, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        let a_one = dir.path().join("a-one.txt");
        let a_two = dir.path().join("a-two.txt");
        let a = dir.path().join("a.yaml");
        fs::write(
            &a,
            manifest("stack-a", &[("a-one", &a_one), ("a-two", &a_two)]),
        )
        .unwrap();
        let (code, out, err) = run(&[
            "apply",
            "-f",
            a.to_str().unwrap(),
            "--state-dir",
            state.to_str().unwrap(),
            "--yes",
        ]);
        assert_eq!(code, Some(0), "stack A must converge:\n{out}\n{err}");
        let lock = fs::read_to_string(state.join("box").join("state.lock.yaml")).unwrap();
        assert!(lock.contains("a-one") && lock.contains("a-two"), "{lock}");
        (Self { dir, state }, a_one)
    }

    fn write(&self, name: &str, body: &str) -> PathBuf {
        let p = self.dir.path().join(name);
        fs::write(&p, body).unwrap();
        p
    }

    fn drift(&self, cfg: &Path, extra: &[&str]) -> (Option<i32>, String, String) {
        let mut args = vec![
            "drift",
            "-f",
            cfg.to_str().unwrap(),
            "--state-dir",
            self.state.to_str().unwrap(),
        ];
        args.extend(extra.iter().copied());
        run(&args)
    }

    /// Manifest B: three resources on the same box, none ever applied.
    fn stack_b(&self) -> PathBuf {
        let b1 = self.dir.path().join("b-one.txt");
        let b2 = self.dir.path().join("b-two.txt");
        let b3 = self.dir.path().join("b-three.txt");
        self.write(
            "b.yaml",
            &manifest(
                "stack-b",
                &[("b-one", &b1), ("b-two", &b2), ("b-three", &b3)],
            ),
        )
    }
}

/// THE REGRESSION. Every resource B declares is absent from the lock; the
/// run must decline, name the count, and exit 2.
#[test]
fn a_manifest_none_of_whose_resources_are_locked_is_declined_with_the_count() {
    let (sb, _) = Sandbox::with_stack_a_applied();
    let b = sb.stack_b();
    let (code, out, err) = sb.drift(&b, &[]);
    assert_eq!(
        code,
        Some(2),
        "0 of 3 declared resources were inspected and the run did not decline.\nstdout:\n{out}\nstderr:\n{err}"
    );
    assert!(
        err.contains("declined: inspected 0 of 3 declared"),
        "the decline must name the count:\nstderr:\n{err}\nstdout:\n{out}"
    );
    assert!(err.contains("no lock holds them"), "{err}");
}

/// A resource in the lock but not in the config is SKIPPED, never graded —
/// even when it has drifted, which is the case that misled the operator.
#[test]
fn another_manifests_resources_are_never_graded() {
    let (sb, a_one) = Sandbox::with_stack_a_applied();
    fs::write(&a_one, "tampered\n").unwrap();
    let b = sb.stack_b();
    let (_, out, err) = sb.drift(&b, &[]);
    assert!(
        !out.contains("DRIFTED: a-one"),
        "a-one belongs to stack A; drift -f b.yaml graded it:\n{out}\n{err}"
    );
    assert!(!out.contains("Drift detected"), "{out}");
}

/// The census must not contradict itself: a locked resource the config does
/// not declare appears under "in the lock, not in the config" and nowhere
/// under inspected — graded or skipped, never both.
#[test]
fn a_locked_resource_outside_the_config_is_skipped_not_inspected() {
    let (sb, _) = Sandbox::with_stack_a_applied();
    let b = sb.stack_b();
    let (_, out, _) = sb.drift(&b, &["--json"]);
    let report: serde_json::Value = serde_json::Deserializer::from_str(&out)
        .into_iter::<serde_json::Value>()
        .next()
        .expect("a JSON document")
        .unwrap_or_else(|e| panic!("bad JSON ({e}):\n{out}"));
    assert_eq!(report["resources_inspected"].as_u64(), Some(0), "{report}");
    let reasons = &report["census"][0]["skipped_by_reason"];
    assert_eq!(
        reasons["in the lock, not in the config"].as_u64(),
        Some(2),
        "a-one and a-two must be skipped as not-in-config:\n{report}"
    );
    assert_eq!(
        reasons["declared here, absent from the lock"].as_u64(),
        Some(3),
        "{report}"
    );
}

/// THE CONTROL. A manifest with one locked resource is inspected, not
/// declined — and its tampered file is a real reject.
#[test]
fn a_partially_locked_manifest_is_inspected_not_declined() {
    let (sb, a_one) = Sandbox::with_stack_a_applied();
    let c_new = sb.dir.path().join("c-new.txt");
    let c = sb.write(
        "c.yaml",
        &manifest("stack-c", &[("a-one", &a_one), ("c-new", &c_new)]),
    );
    let (code, out, err) = sb.drift(&c, &[]);
    assert_eq!(
        code,
        Some(0),
        "converged a-one, unlocked c-new: not a decline\n{out}\n{err}"
    );
    assert!(out.contains("inspected 1 of"), "{out}");

    fs::write(&a_one, "tampered\n").unwrap();
    let (code, out, _) = sb.drift(&c, &[]);
    assert_eq!(
        code,
        Some(1),
        "a graded, drifted resource is a reject:\n{out}"
    );
    assert!(out.contains("DRIFTED: a-one"), "{out}");
}

/// The manifest that was applied is not declined either: full coverage.
#[test]
fn the_applied_manifest_is_fully_inspected() {
    let (sb, _) = Sandbox::with_stack_a_applied();
    let a = sb.dir.path().join("a.yaml");
    let (code, out, err) = sb.drift(&a, &[]);
    assert_eq!(code, Some(0), "{out}\n{err}");
    assert!(out.contains("inspected 2 of 2"), "{out}");
}

/// `--json` declines too, with the same code — the renderer does not own it.
#[test]
fn json_output_declines_with_the_same_code() {
    let (sb, _) = Sandbox::with_stack_a_applied();
    let b = sb.stack_b();
    let (code, _, err) = sb.drift(&b, &["--json"]);
    assert_eq!(code, Some(2), "{err}");
    assert!(err.contains("declined: inspected 0 of 3 declared"), "{err}");
}
