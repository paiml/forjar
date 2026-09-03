//! `lifecycle.ignore_drift: ["mode"]` must ignore the mode and nothing else.
//!
//! forjar#360, the half of #335 that could not ship with the refusal. The lock
//! recorded ONE opaque digest of the state query's stdout, so there was no
//! representation in which `mode` changed and `content` did not — a field list
//! had nothing to select over, and every narrowed form was a hard validation
//! error naming forjar#335.
//!
//! DRIVEN THROUGH THE REAL BINARY, because every surface in the chain has to
//! agree: `validate` must stop refusing, `apply` must record an observation
//! taken under the same mask the next `drift` will apply, and `drift` must then
//! be quiet about the mode while still loud about the bytes.
//!
//! # The anti-cheat, and why it is `resources_inspected`
//!
//! The obvious wrong fix is to widen `should_ignore_drift` back to "any entry
//! means skip the resource" — exactly the forjar#335 regression. Measured
//! against the shipped 1.24.0: applying this same file under
//! `ignore_drift: ["*"]` and then chmod-ing it reports
//! `drift_count 0, resources_inspected 0, resources_skipped 1,
//! skipped_by_reason {"lifecycle.ignore_drift": 1}`. A wholesale skip never
//! calls `census.inspected`, in the file pass or the state-query pass, so
//! asserting `resources_inspected >= 1` alongside `drift_count == 0` is
//! unsatisfiable by widening. Both halves are load-bearing; do not drop either.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

const FORJAR: &str = env!("CARGO_BIN_EXE_forjar");

struct Sandbox {
    dir: std::path::PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("forjar-360-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("sandbox");
        Self { dir }
    }

    fn managed(&self) -> std::path::PathBuf {
        self.dir.join("app.conf")
    }

    fn state_dir(&self) -> std::path::PathBuf {
        self.dir.join("state")
    }

    /// One file resource with a declared mode, carrying `ignore_drift`.
    ///
    /// The top-level `version`/`name` and the machine's `hostname`/`addr` are
    /// not decoration: without them the binary exits 3 at parse time, before
    /// anything under test is reached, and every arm goes red for the wrong
    /// reason.
    fn write_config(&self, ignore_drift: &[&str]) {
        let entries: String = ignore_drift
            .iter()
            .map(|f| format!("      - \"{f}\"\n"))
            .collect();
        let cfg = format!(
            "version: \"1.0\"\nname: per-field-ignore-drift\nmachines:\n  sandbox:\n\
             \x20   hostname: sandbox\n    addr: 127.0.0.1\nresources:\n  cfg:\n\
             \x20   type: file\n    machine: sandbox\n    path: {}\n    mode: \"0644\"\n\
             \x20   content: |\n      replica_count=3\n    lifecycle:\n      ignore_drift:\n{}",
            self.managed().display(),
            entries
        );
        fs::write(self.dir.join("forjar.yaml"), cfg).expect("config");
    }

    /// `validate` takes no `--state-dir`; passing one is a clap error, not a
    /// verdict, and would make both validation arms red for the wrong reason.
    fn validate(&self) -> (String, bool) {
        self.spawn(&["validate"], false)
    }

    fn run(&self, args: &[&str]) -> (String, bool) {
        self.spawn(args, true)
    }

    fn spawn(&self, args: &[&str], state_dir: bool) -> (String, bool) {
        let mut cmd = Command::new(FORJAR);
        cmd.args(args).arg("-f").arg(self.dir.join("forjar.yaml"));
        if state_dir {
            cmd.arg("--state-dir").arg(self.state_dir());
        }
        let out = cmd
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

    fn apply(&self, extra: &[&str]) -> String {
        let mut args = vec!["apply", "--yes"];
        args.extend_from_slice(extra);
        let (out, ok) = self.run(&args);
        assert!(ok, "apply failed:\n{out}");
        out
    }

    /// The machine-readable drift report. Text summaries have been confidently
    /// wrong here before; the JSON carries the census that keeps them honest.
    fn drift_json(&self) -> serde_json::Value {
        let (out, _) = self.run(&["drift", "--json"]);
        let start = out.find('{').unwrap_or_else(|| panic!("no JSON:\n{out}"));
        serde_json::from_str(&out[start..]).unwrap_or_else(|e| panic!("bad JSON ({e}):\n{out}"))
    }

    fn chmod(&self, mode: u32) {
        fs::set_permissions(self.managed(), fs::Permissions::from_mode(mode)).expect("chmod");
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn count(report: &serde_json::Value, key: &str) -> u64 {
    report[key]
        .as_u64()
        .unwrap_or_else(|| panic!("no {key} in {report}"))
}

/// ARM 1. A field forjar can actually suppress must validate.
///
/// Before: exit 3 with "per-field drift suppression is not implemented
/// (forjar#335)". The refusal was right while the observation was one digest;
/// it is over-rejection once the digest is taken under a mask.
#[test]
fn naming_one_real_field_validates() {
    let sb = Sandbox::new("validate");
    sb.write_config(&["mode"]);

    let (out, ok) = sb.validate();

    assert!(ok, "validate refused an implementable field list:\n{out}");
    assert!(
        !out.contains("335"),
        "the field is honoured now; it must not be flagged as unimplemented:\n{out}"
    );
}

/// ARM 2, THE ISSUE ITSELF. Change the mode on the host; drift must be quiet —
/// and must have LOOKED. Before: `drift_count 1, resources_inspected 1`,
/// `detail: "file state changed"`, with two opaque blake3 digests and no field
/// named. (`apply` was refused first, so this arm was double-red.)
#[test]
fn a_mode_change_is_not_drift_when_mode_is_ignored() {
    let sb = Sandbox::new("mode");
    sb.write_config(&["mode"]);
    sb.apply(&[]);

    sb.chmod(0o600);
    let report = sb.drift_json();

    assert_eq!(
        count(&report, "drift_count"),
        0,
        "a mode change was reported as drift under ignore_drift: [mode]:\n{report:#}"
    );
    assert!(
        count(&report, "resources_inspected") >= 1,
        "nothing was inspected — the resource was skipped wholesale, which is \
         the forjar#335 regression, not per-field suppression:\n{report:#}"
    );
}

/// ARM 3, THE THIRD WRITER. `apply --refresh` re-baselines the observed digest
/// through `cli::apply_variants::refreshed_live_hash`, which recomputes it from
/// the SAME state query. A fix that masks only `record_success` and
/// `check_nonfile_drift` writes an UNMASKED digest here, and the very next
/// `drift` reports false drift on the field the operator asked forjar to
/// ignore.
#[test]
fn refresh_does_not_rebaseline_an_unmasked_observation() {
    let sb = Sandbox::new("refresh");
    sb.write_config(&["mode"]);
    sb.apply(&[]);

    sb.chmod(0o600);
    sb.apply(&["--refresh"]);
    let report = sb.drift_json();

    assert_eq!(
        count(&report, "drift_count"),
        0,
        "--refresh re-baselined an unmasked observation:\n{report:#}"
    );
    assert!(
        count(&report, "resources_inspected") >= 1,
        "nothing was inspected after --refresh:\n{report:#}"
    );
}

/// ARM 4, THE OVER-SUPPRESSION GUARD. Green today via the content-hash
/// detector, and it must STAY green: the fix has to suppress mode and only
/// mode. Restore the mode first so the only difference is the bytes.
#[test]
fn a_content_change_still_drifts_when_only_mode_is_ignored() {
    let sb = Sandbox::new("content");
    sb.write_config(&["mode"]);
    sb.apply(&[]);

    sb.chmod(0o644);
    fs::write(sb.managed(), "replica_count=9999\n").expect("tamper");
    let report = sb.drift_json();

    assert!(
        count(&report, "drift_count") >= 1,
        "the bytes were changed under a resource that only ignores mode:\n{report:#}"
    );
    let findings = report["findings"].to_string();
    assert!(
        findings.contains("cfg"),
        "the finding must name the resource:\n{findings}"
    );
}

/// ARM 5, THE VOCABULARY. A field name forjar cannot suppress is still a hard
/// error naming the token — `ignore_drift` must not become a place where typos
/// go quiet now that some entries are honoured.
#[test]
fn a_field_forjar_cannot_suppress_is_still_refused() {
    let sb = Sandbox::new("vocab");
    sb.write_config(&["modes"]);

    let (out, ok) = sb.validate();

    assert!(!ok, "a misspelled field name was accepted:\n{out}");
    assert!(
        out.contains("modes"),
        "the message must name the token so the typo is findable:\n{out}"
    );
}
