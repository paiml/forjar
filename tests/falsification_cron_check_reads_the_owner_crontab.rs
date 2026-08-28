//! A cron job installed in root's crontab must READ as installed.
//!
//! forjar#348, measured on paiml's intel:
//!
//! ```text
//! $ crontab -l | grep -c ci-image-rebuild
//! 0                            <- what forjar's check looked at
//! $ sudo crontab -l | grep rebuild.sh
//! 30 3 * * * bash /home/noah/src/infra/machines/intel/sovereign-ci/rebuild.sh
//!                              <- where forjar's apply had put it
//! ```
//!
//! `cron::apply_script` carried a `SUDO=""` / `[ "$(id -u)" -ne 0 ] &&
//! SUDO="sudo"` preamble and wrote through `$SUDO crontab -u root -`.
//! `check_script` and `state_query_script` re-derived the command WITHOUT it.
//! `crontab -u <user>` refuses every non-root caller — cronie prints "must be
//! privileged to use -u" and exits 1, even for the caller's own username — and
//! the generators swallowed that with `2>/dev/null`, so `grep -qF` read an
//! empty stream and the verdict was `missing:`. The resource was correctly
//! installed and permanently unconvergeable, and its dependents were skipped:
//!
//! ```text
//! JIDOKA: intel/ci-image-rebuild failed - dependents will be skipped:
//!   apply exited 0 but the host does not report the declared state (check exit 1)
//! ```
//!
//! These tests EXECUTE the generated scripts against a fake `id`/`sudo`/
//! `crontab` that reproduces that host exactly, so the result does not depend
//! on who runs `cargo test`.

use forjar::core::types::{MachineTarget, Resource, ResourceType};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn tmpdir(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "forjar-348-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&p).unwrap();
    p
}

fn stub(bin: &Path, name: &str, body: &str) {
    fs::create_dir_all(bin).unwrap();
    let p = bin.join(name);
    fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
}

/// The host from the issue: forjar is a non-root SSH user, `sudo` works, and
/// `crontab -u` refuses anyone who did not come through it.
///
/// `sudo_works = false` is the other honest world: an unprivileged user on a
/// box with no passwordless sudo, who cannot observe the crontab AT ALL.
fn fake_host(tag: &str, sudo_works: bool) -> PathBuf {
    let dir = tmpdir(tag);
    let bin = dir.join("bin");

    stub(
        &bin,
        "id",
        "[ \"$1\" = \"-u\" ] && { echo 1000; exit 0; }\nexec /usr/bin/id \"$@\"",
    );

    if sudo_works {
        stub(
            &bin,
            "sudo",
            "[ \"$1\" = \"-n\" ] && shift\n[ $# -eq 0 ] && exit 0\nFORJAR_FAKE_ROOT=1 export FORJAR_FAKE_ROOT\nexec \"$@\"",
        );
    } else {
        stub(&bin, "sudo", "exit 1");
    }

    // Real cronie behaviour: `-u` is refused unless the caller is root.
    stub(
        &bin,
        "crontab",
        "if [ -z \"${FORJAR_FAKE_ROOT:-}\" ]; then\n\
         \x20 echo 'must be privileged to use -u' >&2\n\
         \x20 exit 1\n\
         fi\n\
         printf '%s\\n' '# forjar:ci-image-rebuild' '30 3 * * * bash /opt/rebuild.sh'\n\
         exit 0",
    );

    dir
}

fn run(script: &str, dir: &Path) -> Output {
    let bin = dir.join("bin");
    Command::new("sh")
        .arg("-c")
        .arg(script)
        .env(
            "PATH",
            format!("{}:{}", bin.display(), std::env::var("PATH").unwrap()),
        )
        .output()
        .expect("run generated script")
}

fn cron_resource(owner: Option<&str>) -> Resource {
    Resource {
        resource_type: ResourceType::Cron,
        machine: MachineTarget::Single("intel".to_string()),
        name: Some("ci-image-rebuild".to_string()),
        owner: owner.map(str::to_string),
        schedule: Some("30 3 * * *".to_string()),
        command: Some("bash /opt/rebuild.sh".to_string()),
        ..Default::default()
    }
}

/// THE DEFECT. The job IS in root's crontab, so the check must exit 0.
#[test]
fn a_job_installed_in_roots_crontab_checks_as_present() {
    let dir = fake_host("check", true);
    let r = cron_resource(None);
    let script = forjar::core::codegen::check_script(&r).expect("cron has a check script");
    let out = run(&script, &dir);

    assert_eq!(
        out.status.code(),
        Some(0),
        "the job is installed in root's crontab, but the check reports otherwise \
         — this is `apply exited 0 but the host does not report the declared \
         state`, forever.\nstdout: {}\nstderr: {}\nscript:\n{script}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );

    let _ = fs::remove_dir_all(&dir);
}

/// The drift half, which is the more expensive one: the observable feeds the
/// state hash, so an unprivileged read recorded `cron=MISSING:<name>` as the
/// OBSERVED state of a job that exists.
#[test]
fn the_state_query_observes_the_job_that_is_actually_there() {
    let dir = fake_host("query", true);
    let r = cron_resource(None);
    let script = forjar::resources::cron::state_query_script(&r);
    let out = run(&script, &dir);
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(
        stdout.contains("30 3 * * * bash /opt/rebuild.sh"),
        "the state query did not observe the installed job: {stdout}\nscript:\n{script}"
    );
    assert!(
        !stdout.contains("cron=MISSING:ci-image-rebuild"),
        "the observable records a job that exists as absent, so drift is wrong \
         in the same direction as the check: {stdout}"
    );

    let _ = fs::remove_dir_all(&dir);
}

/// THE SHARED DECISION POINT. apply, check and state_query must resolve the
/// same crontab AND the same privilege. They decided independently before,
/// which is exactly how they were able to disagree.
#[test]
fn apply_check_and_state_query_address_the_same_crontab() {
    let r = cron_resource(Some("deploy"));
    let apply = forjar::resources::cron::apply_script(&r);
    let check = forjar::resources::cron::check_script(&r);
    let query = forjar::resources::cron::state_query_script(&r);

    for (label, script) in [
        ("apply", &apply),
        ("check", &check),
        ("state_query", &query),
    ] {
        assert!(
            script.contains("$SUDO crontab -u 'deploy'"),
            "{label} does not read/write deploy's crontab with the privilege it \
             needs:\n{script}"
        );
    }
}

/// HONESTY WHEN IT CANNOT LOOK. On a host where the check has no way to read
/// the crontab, exit 1 asserts "the job is missing" about something it was
/// never allowed to observe. Exit 2 is SKIP — `cli::check` maps it to skip and
/// `output_verify` treats it as neither converged nor diverged.
#[test]
fn a_host_that_cannot_read_the_crontab_skips_instead_of_claiming_missing() {
    let dir = fake_host("nosudo", false);
    let r = cron_resource(None);
    let script = forjar::core::codegen::check_script(&r).expect("cron has a check script");
    let out = run(&script, &dir);

    assert_eq!(
        out.status.code(),
        Some(2),
        "a check that was refused permission must SKIP, not report the job \
         missing.\nstdout: {}\nstderr: {}\nscript:\n{script}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );

    let _ = fs::remove_dir_all(&dir);
}
