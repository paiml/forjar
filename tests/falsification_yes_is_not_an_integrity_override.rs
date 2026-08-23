//! `--yes` must mean "do not prompt" and nothing else.
//!
//! THE DEFECT.
//!
//! `apply` verifies every lock file against its BLAKE3 `.b3` sidecar before it
//! does anything (FJ-1270). That gate was written like this:
//!
//! ```ignore
//! if state::integrity::has_errors(&issues) && !yes {
//!     return Err("state integrity check failed — use --yes to override ...")
//! }
//! ```
//!
//! `--yes` is documented as "FJ-286: Skip confirmation prompt (CI mode)" and is
//! MANDATORY for any non-interactive apply — there is no other way to get past
//! the `[y/N]` prompt. So every scheduled apply on the fleet, every apply in a
//! CI job, every `ExecStart=/usr/local/bin/forjar apply --yes` unit in the book,
//! ran with tamper detection switched off. The one population that cannot
//! eyeball a warning is exactly the population that never had the gate.
//!
//! Measured before the fix, on a two-resource config whose machine lock had one
//! comment line appended (valid YAML, wrong hash):
//!
//! ```text
//! $ forjar apply -f forjar.yaml --state-dir state --yes
//! ERROR: integrity check failed for state/localhost/state.lock.yaml: expected 4a6…, got 1c9…
//! localhost: 1 converged, 1 unchanged, 0 failed (0.0s)
//! Apply complete: 1 converged, 1 unchanged.
//! rc=0
//! ```
//!
//! It printed `ERROR`, converged against the tampered state, and exited 0. A CI
//! job gating on the exit code sees a green apply.
//!
//! WHY NO REPLACEMENT OVERRIDE FLAG.
//!
//! Recovery is real: a lock and its sidecar can legitimately diverge (a
//! hand-edited lock, a `git checkout` that restored one but not the other). But
//! the recovery command already exists and is narrower — `forjar reseal` blesses
//! a lock whose contents the operator has decided are good, as one deliberate,
//! auditable act. A `--ignore-state-integrity` flag would be the weaker control:
//! it asserts nothing about the state it waves through, and it is exactly the
//! kind of flag a CI job acquires permanently after one bad night, which is how
//! the gate was lost the first time.
//!
//! Two measurements settled it. `forjar reseal --all` on a tampered lock exits 0
//! and the next `apply --yes` converges (asserted below, so the refusal's advice
//! is tested and not merely written). And for the corrupt-YAML case an override
//! buys nothing anyway: the lock fails to parse at load
//! (`error: invalid lock file …`) whether or not the check ran.
//!
//! WHY THESE DRIVE THE BINARY.
//!
//! The defect lived in the wiring between one flag and an unrelated gate, not
//! in the gate's own logic — `verify_state_integrity` was correct throughout and
//! its unit tests all passed. Only an end-to-end apply can observe that the
//! verdict was computed, printed, and then discarded.

use std::fs;
use std::path::Path;
use std::process::Command;

fn forjar() -> Command {
    Command::new(env!("CARGO_BIN_EXE_forjar"))
}

/// A config of `task` resources, each touching its own marker file.
///
/// Two resources rather than one: the second is added only for the post-tamper
/// apply, so a marker that appears is proof the apply EXECUTED rather than
/// proof it merely re-read its lock. A plain apply of an already-converged
/// resource does no work, which would make "nothing happened" ambiguous.
fn write_config(dir: &Path, resources: &[&str]) -> std::path::PathBuf {
    let cfg = dir.join("forjar.yaml");
    let mut yaml = String::from(
        r#"version: "1.0"
name: integrity-override
machines:
  localhost:
    hostname: localhost
    addr: localhost
resources:
"#,
    );
    for name in resources {
        let marker = dir.join(format!("{name}.marker"));
        yaml.push_str(&format!(
            "  {name}:\n    type: task\n    machine: localhost\n    command: \"touch {m}\"\n    completion_check: \"test -f {m}\"\n",
            m = marker.display()
        ));
    }
    fs::write(&cfg, yaml).expect("write config");
    cfg
}

/// Run an apply and return (combined output, exit-zero).
fn apply(cfg: &Path, state: &Path, extra: &[&str]) -> (String, bool) {
    let mut c = forjar();
    c.arg("apply")
        .arg("-f")
        .arg(cfg)
        .arg("--state-dir")
        .arg(state);
    for e in extra {
        c.arg(e);
    }
    let out = c.output().expect("forjar must run");
    let mut s = String::from_utf8_lossy(&out.stdout).to_string();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.success())
}

/// Converge `first`, then corrupt the machine lock so it no longer matches its
/// `.b3` sidecar, and widen the config so a second apply has real work to do.
///
/// Returns (config path, state dir, the marker the second resource would write).
fn tampered_state(dir: &Path) -> (std::path::PathBuf, std::path::PathBuf, std::path::PathBuf) {
    let state = dir.join("state");
    fs::create_dir_all(&state).unwrap();

    let cfg = write_config(dir, &["first"]);
    let (out, ok) = apply(&cfg, &state, &["--yes"]);
    assert!(ok, "the setup apply must converge:\n{out}");

    // Tamper: still valid YAML, so this is a HASH failure and not a parse
    // failure — the case the sidecar exists to catch.
    let lock = state.join("localhost").join("state.lock.yaml");
    assert!(
        lock.exists(),
        "expected a machine lock at {}",
        lock.display()
    );
    assert!(
        lock.with_extension("yaml.b3").exists(),
        "expected a .b3 sidecar beside the lock"
    );
    let mut content = fs::read_to_string(&lock).unwrap();
    content.push_str("\n# tampered out of band\n");
    fs::write(&lock, content).unwrap();

    let cfg = write_config(dir, &["first", "second"]);
    let second_marker = dir.join("second.marker");
    assert!(!second_marker.exists());
    (cfg, state, second_marker)
}

#[test]
fn yes_must_not_disable_the_integrity_gate() {
    // THE REGRESSION. `--yes` is the only way to run apply non-interactively,
    // so this is what every CI apply did over tampered state: printed the
    // error, converged anyway, exited 0.
    let dir = tempfile::tempdir().unwrap();
    let (cfg, state, second_marker) = tampered_state(dir.path());

    let (out, ok) = apply(&cfg, &state, &["--yes"]);

    assert!(
        !ok,
        "apply --yes over a tampered lock exited 0. --yes is documented as \
         'skip confirmation prompt (CI mode)' and is mandatory for every \
         non-interactive apply, so this means the BLAKE3 gate is off for the \
         whole fleet.\noutput:\n{out}"
    );
    assert!(
        !second_marker.exists(),
        "apply executed a resource over state it had just reported as \
         tampered. A gate that prints ERROR and converges anyway is not a \
         gate.\noutput:\n{out}"
    );
}

#[test]
fn the_refusal_points_at_reseal_and_not_at_yes() {
    // An error that says "use --yes to override" teaches operators to reach for
    // the flag every CI job already passes, which is how the gate disappeared.
    // It must name the recovery instead.
    let dir = tempfile::tempdir().unwrap();
    let (cfg, state, _) = tampered_state(dir.path());

    let (out, ok) = apply(&cfg, &state, &["--yes"]);
    assert!(!ok, "apply must fail over a tampered lock:\n{out}");
    assert!(
        out.contains("reseal"),
        "the refusal must name the recovery command:\n{out}"
    );
    assert!(
        !out.contains("use --yes to override"),
        "the refusal must not point at --yes:\n{out}"
    );
}

#[test]
fn reseal_is_a_real_recovery_and_not_just_advice() {
    // The refusal sends the operator to `forjar reseal`, so that path is part
    // of this fix and is tested as such. An error message naming a command that
    // does not actually unblock the apply is a gate with no way out — the shape
    // that trains people to reach for --no-verify equivalents.
    let dir = tempfile::tempdir().unwrap();
    let (cfg, state, second_marker) = tampered_state(dir.path());

    let (out, ok) = apply(&cfg, &state, &["--yes"]);
    assert!(!ok, "apply must refuse first:\n{out}");

    let out = forjar()
        .args(["reseal", "--all", "--state-dir"])
        .arg(&state)
        .output()
        .expect("forjar reseal must run");
    assert!(
        out.status.success(),
        "reseal must succeed: {}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let (out, ok) = apply(&cfg, &state, &["--yes"]);
    assert!(ok, "apply must proceed after a deliberate reseal:\n{out}");
    assert!(
        second_marker.exists(),
        "the post-reseal apply must actually run:\n{out}"
    );
}

#[test]
fn no_apply_flag_lifts_the_gate() {
    // The fix is not "rename the override". Blessing tampered state is a
    // deliberate act with its own command (`reseal`), so no flag on `apply`
    // may wave it through — including the plausible-looking ones a hurried
    // operator would try.
    let dir = tempfile::tempdir().unwrap();
    let (cfg, state, second_marker) = tampered_state(dir.path());

    for extra in [
        vec!["--yes", "--force"],
        vec!["--yes", "--no-tripwire"],
        vec!["--yes", "--force-unlock"],
        vec!["--yes", "--refresh"],
        vec!["--yes", "--dry-run"],
    ] {
        let mut args = extra.clone();
        args.push("--no-color");
        let (out, ok) = apply(&cfg, &state, &args);
        assert!(!ok, "apply {extra:?} ran over a tampered lock:\n{out}");
        assert!(
            !second_marker.exists(),
            "apply {extra:?} executed a resource over tampered state:\n{out}"
        );
    }
}

#[test]
fn yes_help_does_not_advertise_itself_as_an_override() {
    // The help text is the reason the conflation was invisible: `--yes` said
    // "skip confirmation prompt", so nobody reading it knew it also switched
    // off tamper detection.
    let out = forjar()
        .args(["apply", "--help"])
        .output()
        .expect("forjar apply --help must run");
    let help = String::from_utf8_lossy(&out.stdout).to_string();
    let yes_line = help
        .lines()
        .find(|l| l.trim_start().starts_with("--yes"))
        .unwrap_or_else(|| panic!("apply --help must list --yes:\n{help}"));
    let lowered = yes_line.to_lowercase();
    assert!(
        !lowered.contains("integrity") && !lowered.contains("override"),
        "--yes must document a prompt, not a safety override: {yes_line}"
    );
}

#[test]
fn a_clean_state_dir_is_unaffected() {
    // The control. If the gate started refusing untampered state, the fix
    // would be worse than the defect — and this is what proves the two apply
    // runs above differ because of the TAMPER and not because of the flag.
    let dir = tempfile::tempdir().unwrap();
    let state = dir.path().join("state");
    fs::create_dir_all(&state).unwrap();

    let cfg = write_config(dir.path(), &["first"]);
    let (out, ok) = apply(&cfg, &state, &["--yes"]);
    assert!(ok, "first apply must converge:\n{out}");

    let cfg = write_config(dir.path(), &["first", "second"]);
    let (out, ok) = apply(&cfg, &state, &["--yes"]);
    assert!(ok, "second apply over intact state must converge:\n{out}");
    assert!(
        dir.path().join("second.marker").exists(),
        "the added resource must have run:\n{out}"
    );
}
