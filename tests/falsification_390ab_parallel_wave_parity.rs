//! Refs #390-A / #390-B: the parallel wave path must behave like the sequential one.
//!
//! THE FLAW THIS CLOSES.
//!
//! `machine_wave.rs` ran resources concurrently and then diverged from
//! `resource_ops.rs` in two ways nobody had asserted against:
//!
//! **#390-A — no run log at all.** It never called `run_capture`, so under
//! `--parallel` a failing task's transcript was DESTROYED rather than merely
//! hidden from the console. Measured A/B with only `policy.parallel_resources`
//! flipped: sequential wrote 8 files including a full `=== STDOUT ===` section;
//! parallel produced no `runs/` directory at all. That is why #390's reporter
//! could not find their diagnostics "anywhere in the full raw apply log".
//!
//! The mechanical cause was structural: the script was built and dropped inside
//! the spawn closure (`apply_script(..).and_then(|script| ..)`), so nothing
//! outside the thread had the text `run_capture` needs.
//!
//! **#390-B — no post-apply verification.** The success arm went straight to
//! `record_success`, so FJ-2731 (declared `output_artifacts`) and FJ-2732 (the
//! host reports the declared state) silently did not run. Two configs identical
//! but for `policy.parallel_resources` could report converged and failed — and
//! the population is every plain `type: task`, because `task::check_script`
//! falls through to `verdict::always_diverged("task=pending")` when there is
//! neither a `completion_check` nor `output_artifacts`.
//!
//! WHAT THIS TEST MUST NOT BECOME. Asserting only that parallel *fails* would
//! pass against a build that fails everything. Every case here therefore runs the
//! SAME config both ways and asserts the two agree — the divergence is the defect,
//! so parity is the property.
//!
//! AND IT MUST ACTUALLY REACH THE WAVE PATH. `machine.rs` selects it with
//! `use_parallel && machine_changes.len() > 1`, so a ONE-resource config silently
//! runs sequentially no matter what the policy says. The first version of this
//! file used one resource: all four tests passed against a build with both fixes
//! reverted, because none of them ever executed the code under test. Every config
//! here therefore declares TWO resources, and `the_wave_path_is_actually_taken`
//! guards the guard.

use std::process::Command;

fn config(_dir: &std::path::Path, parallel: bool, body: &str) -> String {
    let policy = if parallel {
        "policy: { parallel_resources: true }"
    } else {
        "policy: { parallel_resources: false }"
    };
    format!(
        r#"version: '1.0'
name: wave
{policy}
machines:
  local:
    hostname: localhost
    addr: localhost
    transport: local
resources:
{body}
"#
    )
}

fn apply(dir: &std::path::Path, name: &str, cfg: &str) -> (String, bool) {
    let path = dir.join(format!("{name}.yaml"));
    std::fs::write(&path, cfg).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_forjar"))
        .current_dir(dir)
        .args(["apply", "--yes", "-f"])
        .arg(&path)
        .args(["--state-dir", name])
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

/// TWO tasks — the wave path needs more than one change to be selected at all.
/// Each writes a marker to stdout and has a check that always fails.
fn failing_task(dir: &std::path::Path) -> String {
    let one = |n: &str| {
        format!(
            "  {n}:\n    machine: local\n    type: task\n    working_dir: {}\n    \
             command: |\n      echo \"WAVE_STDOUT_MARKER_{n}\"\n    completion_check: |\n      \
             test -e /nonexistent-wave\n",
            dir.display()
        )
    };
    format!("{}{}", one("a"), one("b"))
}

/// Two tasks with the given command/check, so the wave path is reachable.
fn two_tasks(dir: &std::path::Path, command: &str, check: Option<&str>) -> String {
    let one = |n: &str| {
        let c = match check {
            Some(c) => format!("    completion_check: |\n      {c}\n"),
            None => String::new(),
        };
        format!(
            "  {n}:\n    machine: local\n    type: task\n    working_dir: {}\n    \
             command: |\n      {command}\n{c}",
            dir.display()
        )
    };
    format!("{}{}", one("a"), one("b"))
}

#[test]
fn the_parallel_path_writes_a_run_log() {
    // #390-A. Without the fix `runs/` does not exist at all under --parallel.
    let dir = tempfile::tempdir().unwrap();
    let body = failing_task(dir.path());
    let (_out, _ok) = apply(dir.path(), "par", &config(dir.path(), true, &body));

    let runs = dir.path().join("par/local/runs");
    assert!(
        runs.exists(),
        "the parallel path wrote NO run directory — a failed task's transcript \
         was destroyed, not merely hidden"
    );
    let found = walk(&runs);
    assert!(
        found.iter().any(|f| f.to_string_lossy().ends_with(".log")),
        "no .log file under {runs:?}; found {found:?}"
    );
    let body_text: String = found
        .iter()
        .filter(|f| f.to_string_lossy().ends_with(".log"))
        .map(|f| std::fs::read_to_string(f).unwrap_or_default())
        .collect();
    assert!(
        body_text.contains("WAVE_STDOUT_MARKER_a"),
        "the run log exists but does not contain the task's stdout"
    );
}

#[test]
fn both_schedulers_write_a_run_log() {
    // PARITY, not just presence. The sequential path is the reference.
    let dir = tempfile::tempdir().unwrap();
    let body = failing_task(dir.path());
    apply(dir.path(), "seq", &config(dir.path(), false, &body));
    apply(dir.path(), "par", &config(dir.path(), true, &body));

    let seq = walk(&dir.path().join("seq/local/runs"));
    let par = walk(&dir.path().join("par/local/runs"));
    assert!(
        !seq.is_empty(),
        "the sequential reference wrote nothing — the premise is broken"
    );
    assert!(
        !par.is_empty(),
        "sequential wrote {} files, parallel wrote 0",
        seq.len()
    );
}

#[test]
fn a_task_that_does_not_reach_its_declared_state_fails_under_parallel_too() {
    // #390-B. A plain `type: task` with no completion_check and no
    // output_artifacts lands in `always_diverged("task=pending")`, so the HOST
    // check must fail it. Under --parallel that check never ran.
    let dir = tempfile::tempdir().unwrap();
    let body = two_tasks(dir.path(), "true", None);

    let (_so, seq_ok) = apply(dir.path(), "seq", &config(dir.path(), false, &body));
    let (_po, par_ok) = apply(dir.path(), "par", &config(dir.path(), true, &body));

    assert_eq!(
        seq_ok, par_ok,
        "the SAME config reported success={seq_ok} sequentially and success={par_ok} \
         in parallel. Post-apply verification is being skipped on one path."
    );
}

#[test]
fn a_converging_task_still_converges_under_parallel() {
    // THE CONTROL. Without it, "fail everything in parallel" passes the test
    // above. A task whose check genuinely passes must still succeed both ways.
    let dir = tempfile::tempdir().unwrap();
    let body = two_tasks(dir.path(), "true", Some("true"));
    let (_so, seq_ok) = apply(dir.path(), "seq", &config(dir.path(), false, &body));
    let (po, par_ok) = apply(dir.path(), "par", &config(dir.path(), true, &body));
    assert!(seq_ok, "the sequential reference must converge");
    assert!(
        par_ok,
        "a genuinely converging task must not be failed by the new check.\n{po}"
    );
}

#[test]
fn the_wave_path_is_actually_taken() {
    // GUARD THE GUARD. `machine.rs` picks the wave path only when there is more
    // than one change, so a one-resource config runs sequentially however the
    // policy is set — and every assertion above would then pass against a build
    // with the fixes removed, which is exactly what happened the first time.
    let dir = tempfile::tempdir().unwrap();
    let cfg = config(dir.path(), true, &failing_task(dir.path()));
    assert!(
        cfg.matches("    type: task").count() > 1,
        "the fixture must declare more than one resource or the wave path is never entered"
    );
    let (out, _) = apply(dir.path(), "par", &cfg);
    assert!(
        out.contains("WAVE_STDOUT_MARKER_a") && out.contains("WAVE_STDOUT_MARKER_b"),
        "both resources must have executed\n{out}"
    );
}

/// NOTE: `Path::ends_with` matches whole COMPONENTS, not string suffixes, so
/// `p.ends_with(".log")` is false for `a.create.log`. Callers compare on
/// `to_string_lossy()` — the first version of this test failed for that reason
/// while the product was already correct.
fn walk(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(dir) else {
        return out;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            out.extend(walk(&p));
        } else {
            out.push(p);
        }
    }
    out
}
