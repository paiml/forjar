//! A task's artifact check must look where the task actually wrote.
//!
//! `output_artifacts` are declared RELATIVE TO `working_dir` — that is where the
//! command runs and where it produces them. The check emitted `[ -e 'foo.srt' ]`
//! with no `cd`, testing a path relative to wherever the check happened to be
//! invoked from, which is a different filesystem location than the one the
//! resource wrote to.
//!
//! This was inert while nothing on the apply path consulted `check_script`.
//! FJ-2732 made the executor verify against the host after every successful
//! apply, and the mismatch surfaced immediately — on `main`, not in review:
//!
//!     JIDOKA: local/transcribe failed — apply exited 0 but the host does not
//!     report the declared state (check exit 1). task=pending:narration.srt
//!
//! The file was there. The check looked somewhere else.
//!
//! Worth recording how it was found: PR #264 (output-equivalence predicates) and
//! PR #269 (post-apply verification) were each green on their own branch and red
//! once both were on main. Neither review could have caught it, because the
//! defect is in their interaction — a latent wrong answer in one, and the first
//! caller ever to ask the question in the other.

use forjar::core::types::{MachineTarget, Resource, ResourceType};

fn task_with_artifact(working_dir: Option<&str>, artifact: &str) -> Resource {
    Resource {
        resource_type: ResourceType::Task,
        machine: MachineTarget::Single("local".into()),
        command: Some(format!("printf 'x' > {artifact}")),
        working_dir: working_dir.map(str::to_string),
        output_artifacts: vec![artifact.to_string()],
        ..Default::default()
    }
}

/// Execute the emitted check from a DIFFERENT cwd than `working_dir`, which is
/// the situation the executor is always in.
fn check_passes_from_elsewhere(r: &Resource, cwd: &std::path::Path) -> bool {
    let script = forjar::resources::task::check_script(r);
    std::process::Command::new("bash")
        .arg("-c")
        .arg(&script)
        .current_dir(cwd)
        .output()
        .expect("bash must run")
        .status
        .success()
}

#[test]
fn an_artifact_under_working_dir_is_found_from_another_cwd() {
    // THE REGRESSION. The artifact exists exactly where the task would have
    // written it; the check must find it without being run from there.
    let dir = tempfile::tempdir().unwrap();
    let work = dir.path().join("work");
    std::fs::create_dir_all(&work).unwrap();
    std::fs::write(work.join("narration.srt"), "ASR draft\n").unwrap();

    let r = task_with_artifact(Some(work.to_str().unwrap()), "narration.srt");
    let elsewhere = dir.path().join("elsewhere");
    std::fs::create_dir_all(&elsewhere).unwrap();

    assert!(
        check_passes_from_elsewhere(&r, &elsewhere),
        "the artifact is present under working_dir; a check run from another \
         directory must still find it.\nscript:\n{}",
        forjar::resources::task::check_script(&r)
    );
}

#[test]
fn a_genuinely_missing_artifact_still_fails() {
    // The fix must not make the check vacuous: resolving under working_dir is
    // only useful if a missing artifact is still detected there.
    let dir = tempfile::tempdir().unwrap();
    let work = dir.path().join("work");
    std::fs::create_dir_all(&work).unwrap();
    // nothing written

    let r = task_with_artifact(Some(work.to_str().unwrap()), "narration.srt");
    assert!(
        !check_passes_from_elsewhere(&r, dir.path()),
        "a missing artifact must still report pending"
    );
}

#[test]
fn a_decoy_in_the_cwd_does_not_satisfy_the_check() {
    // The sharpest form: the artifact is ABSENT under working_dir but a file of
    // the same name sits in the invoking directory. The old relative check
    // passed on this — reporting converged because of an unrelated file.
    let dir = tempfile::tempdir().unwrap();
    let work = dir.path().join("work");
    std::fs::create_dir_all(&work).unwrap();
    let decoy = dir.path().join("decoy");
    std::fs::create_dir_all(&decoy).unwrap();
    std::fs::write(decoy.join("narration.srt"), "not the artifact\n").unwrap();

    let r = task_with_artifact(Some(work.to_str().unwrap()), "narration.srt");
    assert!(
        !check_passes_from_elsewhere(&r, &decoy),
        "a same-named file in the invoking directory must not satisfy a check \
         about an artifact under working_dir"
    );
}

#[test]
fn without_working_dir_the_relative_behaviour_is_unchanged() {
    // No working_dir means "." — the previous semantics, preserved.
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("out.txt"), "x").unwrap();

    let r = task_with_artifact(None, "out.txt");
    assert!(
        check_passes_from_elsewhere(&r, dir.path()),
        "with no working_dir the artifact is relative to cwd, as before"
    );
}
