//! forjar#385: `drift` refused to run at all when the state dir was absent.
//!
//! #380 taught `forjar drift` to execute a `type: task` completion_check and to
//! print its denominator. That immediately exposed the next layer: paiml/infra's
//! nightly drift lane had never measured anything, because `state/` is
//! gitignored there and so is absent from every CI checkout.
//!
//! ```text
//! FAIL gx10         forjar drift exited 1: error: cannot read state dir .../infra/state
//! drift-tripwire: 0 of 2 requested machine(s) measured
//! FAIL: no machine was measured — this run measured NOTHING
//! ```
//!
//! #380's own reasoning says why that refusal is wrong: for a `type: task` the
//! observable is an ASSERTION, not a baseline — "a completion_check that fails
//! right now is drift whether or not anything was ever written down about it".
//! A run with no lock can still execute every task check and give a TRUE answer
//! about the host. It cannot report hash-drift for File/Image resources, which
//! is a SMALLER answer, not an invalid one — and the census is what says so.
//!
//! THE LINE THIS FILE DEFENDS. `absent` and `unreadable` are different faults:
//! "never applied from here" is routine, "present and I cannot read it" is a
//! broken host. Collapsing the second into the first would be the same defect
//! in a new place, so both directions are pinned here.
//!
//! DRIVEN THROUGH THE REAL BINARY — the exit status and the census are what the
//! CI lane reads, so what the lane sees is the thing under test.

use std::fs;
use std::process::Command;

const FORJAR: &str = env!("CARGO_BIN_EXE_forjar");

struct Sandbox {
    dir: std::path::PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("forjar-385-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("sandbox");
        Self { dir }
    }

    /// The file the guard asserts. Deleting it is the violation.
    fn marker(&self) -> std::path::PathBuf {
        self.dir.join("runner.registered")
    }

    /// The state dir this sandbox names but never creates — the CI-checkout
    /// shape. Nothing here ever runs `apply`, which is the whole point.
    fn state(&self) -> std::path::PathBuf {
        self.dir.join("state")
    }

    /// One report-only guard — `command` exits 1 by design, which is what a
    /// fleet writes when the remedy is a human's job — plus a managed file,
    /// whose drift genuinely DOES need a baseline. The pair is deliberate: the
    /// census has to distinguish what it measured from what it could not.
    fn write_config(&self) {
        let cfg = format!(
            "version: \"1.0\"\nname: drift-385\nmachines:\n  sandbox:\n\
             \x20   hostname: sandbox\n    addr: 127.0.0.1\nresources:\n  runner-registered:\n\
             \x20   type: task\n    machine: sandbox\n\
             \x20   command: |\n      echo 'the runner is not registered' >&2\n      exit 1\n\
             \x20   completion_check: \"[ -f '{marker}' ]\"\n\
             \x20 fleet-hosts:\n    type: file\n    machine: sandbox\n\
             \x20   path: \"{hosts}\"\n    content: \"10.42.0.15 gx10\\n\"\n",
            marker = self.marker().display(),
            hosts = self.dir.join("hosts").display(),
        );
        fs::write(self.dir.join("forjar.yaml"), cfg).expect("config");
    }

    fn run(&self, args: &[&str]) -> (String, bool) {
        self.run_with_config(args, &self.dir.join("forjar.yaml"))
    }

    fn run_with_config(&self, args: &[&str], config: &std::path::Path) -> (String, bool) {
        let out = Command::new(FORJAR)
            .args(args)
            .arg("-f")
            .arg(config)
            .arg("--state-dir")
            .arg(self.state())
            .current_dir(&self.dir)
            .output()
            .expect("run forjar");
        (
            format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            ),
            out.status.success(),
        )
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        // Restore any mode this test removed, or the sandbox outlives the run.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(self.state(), fs::Permissions::from_mode(0o755));
        }
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// THE FALSIFICATION. No state dir was ever created, the guard's assertion is
/// false on this host, and `--tripwire` must say so instead of refusing to look.
#[test]
fn an_absent_state_dir_still_fails_on_a_violated_guard() {
    let sb = Sandbox::new("violated");
    sb.write_config();
    assert!(!sb.state().exists(), "the state dir must not exist");

    let (out, ok) = sb.run(&["drift", "-m", "sandbox", "--tripwire"]);

    assert!(
        !out.contains("cannot read state dir"),
        "an absent state dir is 'never applied from here', not a fatal error:\n{out}"
    );
    assert!(
        !ok,
        "drift exited 0 over a guard whose completion_check fails:\n{out}"
    );
    assert!(
        out.contains("runner-registered") && out.contains("completion_check"),
        "the finding must name the resource AND the assertion that failed:\n{out}"
    );
}

/// THE CONTROL. The same run with the assertion satisfied must stay green — a
/// detector that reports drift unconditionally is worth no more than one that
/// never reports it.
#[test]
fn an_absent_state_dir_with_a_satisfied_guard_is_green() {
    let sb = Sandbox::new("satisfied");
    sb.write_config();
    fs::write(sb.marker(), "registered").expect("marker");

    let (out, ok) = sb.run(&["drift", "-m", "sandbox", "--tripwire"]);

    assert!(ok, "a satisfied guard must not report drift:\n{out}");
    assert!(out.contains("No drift detected."), "{out}");
}

/// THE CENSUS. A smaller answer is only honest if it says what it left out. The
/// task was measured; the file needs a baseline that does not exist, and the
/// run must name the missing lock as the reason rather than fold it into a
/// clean bill of health.
#[test]
fn the_census_names_the_missing_lock() {
    let sb = Sandbox::new("census");
    sb.write_config();
    fs::write(sb.marker(), "registered").expect("marker");

    let (out, ok) = sb.run(&["drift", "-m", "sandbox"]);

    assert!(ok, "{out}");
    assert!(
        out.contains("inspected 1 of 2 resource(s) in scope: task 1"),
        "the run must state what it DID measure:\n{out}"
    );
    assert!(
        out.contains("no lock"),
        "the skipped population must be attributed to the missing lock:\n{out}"
    );
    assert!(
        out.contains("1 resource(s) inspected, 1 not inspected."),
        "the closing verdict must carry the denominator:\n{out}"
    );
}

/// THE MACHINE-READABLE CENSUS. paiml/infra's drift lane parses `--json`; a
/// consumer that cannot see the skipped population is as blind as the human
/// output #380 fixed.
#[test]
fn the_json_report_attributes_the_skips_to_the_missing_lock() {
    let sb = Sandbox::new("json");
    sb.write_config();
    fs::write(sb.marker(), "registered").expect("marker");

    let (out, ok) = sb.run(&["drift", "-m", "sandbox", "--json"]);

    assert!(ok, "{out}");
    let report: serde_json::Value = serde_json::from_str(&out)
        .unwrap_or_else(|e| panic!("drift --json emitted invalid JSON ({e}):\n{out}"));
    assert_eq!(report["machines_checked"], 1, "{out}");
    assert_eq!(report["drift_count"], 0, "{out}");
    assert_eq!(report["resources_inspected"], 1, "{out}");
    assert_eq!(report["resources_skipped"], 1, "{out}");
    assert_eq!(report["census"][0]["inspected_by_type"]["task"], 1, "{out}");
    let reasons = &report["census"][0]["skipped_by_reason"];
    let named = reasons
        .as_object()
        .map(|o| o.keys().any(|k| k.contains("no lock")))
        .unwrap_or(false);
    assert!(
        named,
        "the skip reason must name the missing lock: {reasons}"
    );
}

/// THE OPT-OUT IS STILL NOT A SILENCER. `--no-task-checks` may decline the only
/// work a lockless run can do; it may not report that as coverage.
#[test]
fn no_task_checks_over_an_absent_state_dir_inspects_nothing_and_admits_it() {
    let sb = Sandbox::new("optout");
    sb.write_config();

    let (out, ok) = sb.run(&["drift", "-m", "sandbox", "--tripwire", "--no-task-checks"]);

    assert!(ok, "--no-task-checks must not execute the check:\n{out}");
    assert!(
        out.contains("inspected 0 of 2"),
        "the run must admit it inspected nothing:\n{out}"
    );
    assert!(
        out.contains("--no-task-checks"),
        "the skipped population must name the flag that skipped it:\n{out}"
    );
}

/// THE LINE. A state dir that is PRESENT and cannot be read is a broken host,
/// not a fresh checkout, and must stay fatal.
///
/// Both branches assert real behaviour rather than skipping: root can read a
/// `0o000` directory, in which case forjar sees a present-and-empty state dir
/// and the honest result is the ordinary no-lock run. The privilege-independent
/// half of this contract is `a_state_dir_that_is_not_a_directory_is_still_fatal`.
#[cfg(unix)]
#[test]
fn an_unreadable_state_dir_is_still_fatal() {
    use std::os::unix::fs::PermissionsExt;
    let sb = Sandbox::new("unreadable");
    sb.write_config();
    fs::write(sb.marker(), "registered").expect("marker");
    fs::create_dir_all(sb.state()).expect("state dir");
    fs::set_permissions(sb.state(), fs::Permissions::from_mode(0o000)).expect("chmod");

    let readable_anyway = fs::read_dir(sb.state()).is_ok();
    let (out, ok) = sb.run(&["drift", "-m", "sandbox"]);

    if readable_anyway {
        assert!(
            ok,
            "a readable (root) state dir must scan, not fail:\n{out}"
        );
    } else {
        assert!(!ok, "an unreadable state dir must stay fatal:\n{out}");
        assert!(
            out.contains("cannot read state dir"),
            "the fatal error must still name the unreadable dir:\n{out}"
        );
    }
}

/// The same line, without depending on the effective uid: a state path that is
/// a regular file is present and unusable for anyone.
#[test]
fn a_state_dir_that_is_not_a_directory_is_still_fatal() {
    let sb = Sandbox::new("notadir");
    sb.write_config();
    fs::write(sb.marker(), "registered").expect("marker");
    fs::write(sb.state(), "not a directory").expect("state file");

    let (out, ok) = sb.run(&["drift", "-m", "sandbox"]);

    assert!(
        !ok,
        "a state path that is a FILE is present and unusable — that is not \
         'never applied from here':\n{out}"
    );
    assert!(out.contains("cannot read state dir"), "{out}");
}

/// NO LOCK AND NO CONFIG IS NOT A CLEAN BILL OF HEALTH. With neither a baseline
/// nor a declaration there is nothing to assert against the host, and a green
/// exit over zero information is the defect this whole file descends from.
#[test]
fn an_absent_state_dir_and_no_config_measures_nothing_and_says_so() {
    let sb = Sandbox::new("nothing");
    let missing = sb.dir.join("no-such-forjar.yaml");

    let (out, ok) = sb.run_with_config(&["drift"], &missing);

    assert!(
        !ok,
        "no lock and no config means nothing was measured; that must not \
         exit 0:\n{out}"
    );
    assert!(
        !out.contains("No drift detected."),
        "a run with nothing to look at must not print a verdict:\n{out}"
    );
}

/// A MACHINE YOU NAMED AND COULD NOT CHECK IS STILL NOT A PASS. The locked path
/// already refuses an unknown `-m`; losing that on the lockless path would
/// restore the false green a typo used to buy.
#[test]
fn an_absent_state_dir_still_rejects_an_unknown_machine() {
    let sb = Sandbox::new("unknown");
    sb.write_config();

    let (out, ok) = sb.run(&["drift", "-m", "sandbo", "--tripwire"]);

    assert!(!ok, "an unknown machine must not report clean:\n{out}");
    assert!(
        out.contains("sandbo") && out.contains("sandbox"),
        "the error must name what was asked for and what exists:\n{out}"
    );
}

/// THE PREVIEW MUST AGREE WITH THE RUN. `--dry-run` exists to say what a real
/// run would check; a preview that dies where the run succeeds is a worse
/// answer than no preview.
#[test]
fn the_dry_run_preview_survives_an_absent_state_dir() {
    let sb = Sandbox::new("dryrun");
    sb.write_config();

    let (out, ok) = sb.run(&["drift", "-m", "sandbox", "--dry-run"]);

    assert!(
        ok,
        "the preview must not die on an absent state dir:\n{out}"
    );
    assert!(
        out.contains("runner-registered"),
        "the preview must name the guard the real run would execute:\n{out}"
    );
}
