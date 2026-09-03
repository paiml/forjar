//! forjar#362: cron's observable never contained the job.
//!
//! # Why these tests execute the script
//!
//! `tests_cron.rs` and `tests_cron_b.rs` assert on the generated TEXT, and that
//! is exactly how this shipped: the text was always plausible.
//! `test_fj033_apply_cmd_tag_idempotency` was named "(prevents duplication)"
//! over a script that duplicates, and `test_fj033_apply_preserves_existing_entries`
//! carried the message "must remove old entry before re-adding" over a filter
//! that does not remove it. Both passed for years.
//!
//! These tests run the real generated scripts against a hermetic fake
//! `crontab` and assert on the BYTES in the crontab store and on the stdout the
//! digest is taken over. A generator can only pass by actually being right.
//!
//! # Harness notes, each one paid for
//!
//! - Spawn `bash`, never `sh`. `apply_script` opens with `set -euo pipefail`;
//!   dash aborts at line 1 with "Illegal option -o pipefail" and writes
//!   nothing, which makes every write-side test red for the wrong reason and
//!   red after the fix too. `transport::local` spawns bash for this reason.
//! - A fake `id` pins `id -u` to a non-root value. Under `set -e` the SUDO
//!   preamble's `[ "$(id -u)" -ne 0 ] && SUDO="sudo"` is a failing AND-list
//!   when the caller IS root, so a root test runner would abort the script
//!   before it touched the store.
//! - Resource names in a case must have no prefix-sibling in the same store
//!   unless the case is ABOUT the collision: `grep -A1` is unanchored, so a
//!   sibling named `backup-db` widens `backup`'s window onto a second block and
//!   the blindness accidentally disappears. That false green is what makes
//!   forjar#362 look already-fixed on a re-test.

use super::cron::*;
use crate::core::types::{MachineTarget, Resource, ResourceType};
use std::path::{Path, PathBuf};

const CMD: &str = "/usr/local/bin/backup.sh";

fn cron(name: &str, schedule: &str, command: &str) -> Resource {
    Resource {
        resource_type: ResourceType::Cron,
        machine: MachineTarget::Single("m1".to_string()),
        owner: Some("root".to_string()),
        name: Some(name.to_string()),
        schedule: Some(schedule.to_string()),
        command: Some(command.to_string()),
        ..Default::default()
    }
}

fn absent(name: &str) -> Resource {
    let mut r = cron(name, "0 3 * * *", CMD);
    r.state = Some("absent".to_string());
    r
}

fn write_exe(path: &Path, body: &str) {
    std::fs::write(path, body).expect("write fake binary");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    }
}

/// A hermetic crontab: fake `crontab`, `sudo` and `id` on PATH over one file.
struct FakeCrontab {
    dir: PathBuf,
}

impl FakeCrontab {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir()
            .join(format!("forjar-cron-exec-{}-{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("bin")).expect("create fake bin dir");
        let fx = FakeCrontab { dir };
        write_exe(&fx.bin().join("crontab"), FAKE_CRONTAB);
        write_exe(&fx.bin().join("sudo"), FAKE_SUDO);
        write_exe(&fx.bin().join("id"), FAKE_ID);
        fx
    }

    fn bin(&self) -> PathBuf {
        self.dir.join("bin")
    }

    fn store(&self) -> PathBuf {
        self.dir.join("crontab.store")
    }

    fn seed(&self, text: &str) {
        std::fs::write(self.store(), text).expect("seed crontab store");
    }

    /// The crontab as the fake `crontab -l` would print it.
    fn read(&self) -> String {
        std::fs::read_to_string(self.store()).unwrap_or_default()
    }

    fn run(&self, script: &str) -> std::process::Output {
        let path = format!(
            "{}:{}",
            self.bin().display(),
            std::env::var("PATH").unwrap_or_default()
        );
        std::process::Command::new("bash")
            .arg("-c")
            .arg(script)
            .env("PATH", path)
            .env("FORJAR_TEST_CRON_STORE", self.store())
            .output()
            .expect("bash is available")
    }

    fn apply(&self, r: &Resource) {
        let out = self.run(&apply_script(r));
        assert!(
            out.status.success(),
            "apply script failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    /// The exact stdout `resource_ops` hashes into `observed`.
    fn observation(&self, r: &Resource) -> String {
        let out = self.run(&state_query_script(r));
        String::from_utf8_lossy(&out.stdout).to_string()
    }
}

impl Drop for FakeCrontab {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

const FAKE_CRONTAB: &str = r#"#!/usr/bin/env bash
store="$FORJAR_TEST_CRON_STORE"
mode=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    -u) shift 2 ;;
    -l) mode=list; shift ;;
    -)  mode=write; shift ;;
    *)  shift ;;
  esac
done
case "$mode" in
  list) [ -f "$store" ] || exit 1; cat "$store" ;;
  write) cat > "$store" ;;
esac
"#;

const FAKE_SUDO: &str = r#"#!/usr/bin/env bash
[ "$1" = "-n" ] && shift
exec "$@"
"#;

const FAKE_ID: &str = r#"#!/usr/bin/env bash
echo 1000
"#;

// ── The observable ──────────────────────────────────────────────

/// forjar#362 as filed: tamper the schedule on the host, the digest must move.
#[test]
fn fj362_the_observation_changes_when_the_schedule_is_tampered() {
    let fx = FakeCrontab::new("t1");
    let r = cron("backup", "0 3 * * *", CMD);
    fx.apply(&r);
    let before = fx.observation(&r);
    fx.seed(&fx.read().replace("0 3 * * *", "0 4 * * *"));
    let after = fx.observation(&r);
    assert!(
        before.contains("0 3 * * *"),
        "the observation must contain the job it claims to watch: {before:?}"
    );
    assert_ne!(
        before, after,
        "a schedule tamper left the observation byte-identical: {before:?}"
    );
}

/// The same, for the command — the half an operator cares about most.
#[test]
fn fj362_the_observation_changes_when_the_command_is_tampered() {
    let fx = FakeCrontab::new("t1b");
    let r = cron("backup", "0 3 * * *", CMD);
    fx.apply(&r);
    let before = fx.observation(&r);
    fx.seed(&fx.read().replace(CMD, "/usr/bin/curl-evil"));
    assert_ne!(
        before,
        fx.observation(&r),
        "a command swap left the observation byte-identical: {before:?}"
    );
}

/// Specificity guard: deleting the marker already fires today, and must keep
/// firing. Without this the fix could be "hash something else entirely".
#[test]
fn fj362_the_observation_still_changes_when_the_marker_is_deleted() {
    let fx = FakeCrontab::new("t1c");
    let r = cron("backup", "0 3 * * *", CMD);
    fx.apply(&r);
    let before = fx.observation(&r);
    fx.seed(&fx.read().replace("# forjar:backup\n", ""));
    assert_ne!(before, fx.observation(&r));
}

// ── The write side ──────────────────────────────────────────────

/// The reachable everyday harm: edit the schedule in the CONFIG, apply, and
/// the old job stays scheduled outside the marker block forever while drift,
/// check and plan all report converged.
#[test]
fn fj362_a_config_schedule_edit_unschedules_the_old_job() {
    let fx = FakeCrontab::new("t2");
    fx.apply(&cron("backup", "0 3 * * *", CMD));
    fx.apply(&cron("backup", "0 4 * * *", CMD));
    let store = fx.read();
    assert_eq!(
        store.matches(CMD).count(),
        1,
        "the old schedule is still installed: {store:?}"
    );
    assert!(!store.contains("0 3 * * *"), "{store:?}");
}

/// Repairing a host-tampered job must remove the tampered line.
///
/// This is what forbids the `grep -A2`-only fix the issue proposes: widening
/// the window alone makes the tamper visible ONCE, then the repair apply leaves
/// the tampered entry above the reinstalled block and the digest returns to the
/// pristine value — permanent blindness traded for fire-once-then-lie.
#[test]
fn fj362_repairing_a_tampered_job_removes_the_tampered_line() {
    let fx = FakeCrontab::new("t3");
    let r = cron("backup", "0 3 * * *", CMD);
    fx.apply(&r);
    fx.seed(&fx.read().replace("0 3 * * *", "0 4 * * *"));
    fx.apply(&r);
    let store = fx.read();
    assert_eq!(store.matches(CMD).count(), 1, "{store:?}");
    assert!(!store.contains("0 4 * * *"), "{store:?}");
}

/// The apply script itself must be idempotent. A persistent state dir hides
/// this at the product level (the planner no-ops), so the exec test is the only
/// place it is visible — and a fresh `--state-dir`, the CI-checkout shape,
/// re-plans every run.
#[test]
fn fj362_three_applies_install_one_job() {
    let fx = FakeCrontab::new("t4");
    let r = cron("backup", "0 3 * * *", CMD);
    fx.apply(&r);
    fx.apply(&r);
    fx.apply(&r);
    let store = fx.read();
    assert_eq!(store.matches(CMD).count(), 1, "{store:?}");
}

/// `state: absent` must actually unschedule the job, and must not eat a
/// prefix-sibling's markers or an unrelated line on the way past.
#[test]
fn fj362_absent_removes_the_job_and_spares_everything_else() {
    let fx = FakeCrontab::new("t5");
    fx.seed(concat!(
        "MAILTO=me\n",
        "# forjar:backup-db\n",
        "# forjar-cmd:backup-db\n",
        "0 5 * * * /usr/local/bin/dump-db.sh\n",
        "# forjar:backup\n",
        "# forjar-cmd:backup\n",
        "0 3 * * * /usr/local/bin/backup.sh\n",
    ));
    fx.apply(&absent("backup"));
    let store = fx.read();
    assert!(!store.contains(CMD), "the job survived absent: {store:?}");
    assert!(store.contains("# forjar:backup-db"), "{store:?}");
    assert!(store.contains("/usr/local/bin/dump-db.sh"), "{store:?}");
    assert!(store.contains("MAILTO=me"), "{store:?}");
}

/// forjar lints every script it generates with bashrs before executing it
/// (`transport::validate_before_exec` -> `purifier::validate_script`) and
/// refuses to run one with an Error-severity finding. A rewritten script that
/// trips that gate is unrunnable, so lint the raw text here — stricter than the
/// real call site, which lints after `strip_data_payloads`.
#[test]
fn fj362_every_generated_cron_script_survives_forjars_own_bashrs_gate() {
    let present = cron("backup", "0 3 * * *", CMD);
    let scripts = [
        check_script(&present),
        apply_script(&present),
        apply_script(&absent("backup")),
        state_query_script(&present),
    ];
    for script in scripts {
        crate::core::purifier::validate_script(&script)
            .unwrap_or_else(|e| panic!("bashrs refused a generated cron script: {e}\n{script}"));
    }
}

// ── The prefix collision ────────────────────────────────────────

/// `check` for `backup` against a crontab holding only `backup-db` must report
/// missing. The unanchored `grep -qF` says the job is installed.
#[test]
fn fj362_check_does_not_mistake_a_prefix_sibling_for_the_job() {
    let fx = FakeCrontab::new("t6");
    fx.seed("# forjar:backup-db\n# forjar-cmd:backup-db\n0 5 * * * /usr/local/bin/dump-db.sh\n");
    let out = fx.run(&check_script(&cron("backup", "0 3 * * *", CMD)));
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(
        stdout.contains("missing:backup"),
        "check claimed a prefix-sibling was the job: {stdout:?}"
    );
    assert!(!out.status.success(), "exit code disagreed with the marker");
}

/// Applying `backup` must not delete a prefix-sibling's markers, which would
/// orphan its still-scheduled job and flip its own check to missing.
#[test]
fn fj362_applying_a_job_spares_a_prefix_siblings_markers() {
    let fx = FakeCrontab::new("t7");
    fx.seed("# forjar:backup-db\n# forjar-cmd:backup-db\n0 5 * * * /usr/local/bin/dump-db.sh\n");
    fx.apply(&cron("backup", "0 3 * * *", CMD));
    let store = fx.read();
    assert!(store.contains("# forjar:backup-db"), "{store:?}");
    assert!(store.contains("# forjar-cmd:backup-db"), "{store:?}");
    assert!(store.contains("/usr/local/bin/dump-db.sh"), "{store:?}");
}
