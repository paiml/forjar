//! GH-247 tests. The load-bearing one is `never_writes_the_declared_output_path`
//! — the rest describe behaviour, that one describes the promise.

use super::*;
use crate::core::types::{MachineTarget, ResourceType};

fn task(dir: &Path, command: &str, artifacts: &[&str]) -> Resource {
    Resource {
        resource_type: ResourceType::Task,
        machine: MachineTarget::Single("local".to_string()),
        command: Some(command.to_string()),
        working_dir: Some(dir.display().to_string()),
        output_artifacts: artifacts.iter().map(|s| (*s).to_string()).collect(),
        cache: true,
        ..Default::default()
    }
}

/// Hash the artifacts as they currently sit in `dir` — i.e. what apply recorded.
fn recorded(resource: &Resource, dir: &Path) -> String {
    hash_outputs_in(&resource.output_artifacts, dir)
        .expect("hashable")
        .expect("some artifact exists")
}

#[test]
fn a_deterministic_recipe_reproduces() {
    let work = tempfile::tempdir().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let r = task(work.path(), "printf 'STABLE\\n' > out.txt", &["out.txt"]);

    // Produce the artifact the way apply would, and record its hash.
    std::process::Command::new("bash")
        .arg("-c")
        .arg("printf 'STABLE\\n' > out.txt")
        .current_dir(work.path())
        .status()
        .unwrap();
    let rec = recorded(&r, work.path());

    let out = verify_resource("t", &r, Some(&rec), scratch.path());
    assert_eq!(out.verdict, Verdict::Reproduced, "{:?}", out.verdict);
}

#[test]
fn a_nondeterministic_recipe_diverges() {
    // The shape the issue is about: a generator opaque to static analysis. No
    // static gate can see inside ffmpeg or an LLM call either.
    let work = tempfile::tempdir().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let cmd = "date +%s%N > out.txt";
    let r = task(work.path(), cmd, &["out.txt"]);

    std::process::Command::new("bash")
        .arg("-c")
        .arg(cmd)
        .current_dir(work.path())
        .status()
        .unwrap();
    let rec = recorded(&r, work.path());

    let out = verify_resource("t", &r, Some(&rec), scratch.path());
    assert!(
        matches!(out.verdict, Verdict::Diverged { .. }),
        "a timestamp generator must not report as reproduced: {:?}",
        out.verdict
    );
    assert!(out.verdict.is_failure());
}

#[test]
fn never_writes_the_declared_output_path() {
    // THE PROMISE. `#247` states it as the hard requirement: on match OR
    // mismatch, the declared output must not be touched. A verify that can
    // clobber the artifact is useless for the case it exists to serve —
    // checking an expensive, human-corrected output.
    let work = tempfile::tempdir().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let out_path = work.path().join("out.txt");

    // A human-corrected artifact, and a recipe that would overwrite it.
    std::fs::write(&out_path, "HUMAN CORRECTED\n").unwrap();
    let r = task(work.path(), "date +%s%N > out.txt", &["out.txt"]);
    let rec = recorded(&r, work.path());
    let before = std::fs::read_to_string(&out_path).unwrap();
    let mtime_before = std::fs::metadata(&out_path).unwrap().modified().unwrap();

    let outcome = verify_resource("t", &r, Some(&rec), scratch.path());
    assert!(
        matches!(outcome.verdict, Verdict::Diverged { .. }),
        "precondition: this recipe must diverge, else the test proves nothing"
    );

    let after = std::fs::read_to_string(&out_path).unwrap();
    assert_eq!(
        before, after,
        "verify overwrote the declared output — the one thing it must never do"
    );
    assert_eq!(
        mtime_before,
        std::fs::metadata(&out_path).unwrap().modified().unwrap(),
        "verify touched the declared output's mtime"
    );
}

#[test]
fn the_previous_output_is_not_visible_to_the_regenerated_run() {
    // A recipe that no-ops when its output already exists would "reproduce"
    // perfectly if the scratch copy inherited the old artifact. Excluding the
    // declared artifacts from the copy is what makes the check mean anything.
    let work = tempfile::tempdir().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let cmd = "[ -f out.txt ] || printf 'FRESH\\n' > out.txt";
    let r = task(work.path(), cmd, &["out.txt"]);

    std::fs::write(work.path().join("out.txt"), "STALE\n").unwrap();
    let rec = recorded(&r, work.path());

    let outcome = verify_resource("t", &r, Some(&rec), scratch.path());
    assert!(
        matches!(outcome.verdict, Verdict::Diverged { .. }),
        "the stale artifact leaked into the scratch tree, so a no-op recipe \
         looked reproducible: {:?}",
        outcome.verdict
    );
    assert!(
        !scratch.path().join("out.txt").exists() || {
            std::fs::read_to_string(scratch.path().join("out.txt")).unwrap() == "FRESH\n"
        }
    );
}

#[test]
fn a_failing_command_is_not_reported_as_reproduced() {
    let work = tempfile::tempdir().unwrap();
    let scratch = tempfile::tempdir().unwrap();
    std::fs::write(work.path().join("out.txt"), "X\n").unwrap();
    let r = task(work.path(), "exit 3", &["out.txt"]);
    let rec = recorded(&r, work.path());

    let outcome = verify_resource("t", &r, Some(&rec), scratch.path());
    assert!(
        matches!(outcome.verdict, Verdict::CommandFailed { .. }),
        "{:?}",
        outcome.verdict
    );
    assert!(
        outcome.verdict.is_failure(),
        "an artifact whose recipe no longer runs is not reproducible"
    );
}

#[test]
fn preconditions_are_enumerated_rather_than_a_bare_none() {
    let work = tempfile::tempdir().unwrap();
    let mut r = task(work.path(), "true", &["out.txt"]);

    r.command = None;
    assert_eq!(verifiability(&r, Some("h")), Some(SkipReason::NoCommand));

    r.command = Some("true".into());
    r.output_artifacts.clear();
    assert_eq!(
        verifiability(&r, Some("h")),
        Some(SkipReason::NoOutputArtifacts)
    );

    r.output_artifacts = vec!["out.txt".into()];
    assert_eq!(verifiability(&r, None), Some(SkipReason::NoRecordedHash));

    r.working_dir = Some("/definitely/not/here".into());
    assert_eq!(
        verifiability(&r, Some("h")),
        Some(SkipReason::WorkingDirUnavailable)
    );

    r.working_dir = Some(work.path().display().to_string());
    assert_eq!(verifiability(&r, Some("h")), None);
}

#[test]
fn a_skip_is_not_a_gate_failure_but_a_divergence_is() {
    assert!(!Verdict::Skipped(SkipReason::NoCommand).is_failure());
    assert!(!Verdict::Reproduced.is_failure());
    assert!(Verdict::Diverged {
        recorded: "a".into(),
        regenerated: None
    }
    .is_failure());
    assert!(Verdict::CommandFailed { status: "x".into() }.is_failure());
}
