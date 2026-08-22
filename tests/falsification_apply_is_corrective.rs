//! Apply must be CORRECTIVE, not merely CREATIVE.
//!
//! THE DESIGN FLAW.
//!
//! A resource's apply script is allowed to guard its work behind a cheap
//! predicate so that re-applying is idempotent. The flaw is what that predicate
//! tests: if it asks "does something exist here?" instead of "is the DECLARED
//! state here?", then apply creates on a bare host and does NOTHING on a host
//! that is present-but-wrong. It exits 0 either way, so forjar reports
//! converged over a host it never corrected.
//!
//! MEASURED, 2026-08-19, on paiml/infra's intel and lambda-labs. The declared
//! mount source was changed from `//192.168.1.179/Personal-Drive` to
//! `//192.168.1.179/media` and applied:
//!
//!     intel:       1 converged, 0 unchanged, 0 failed
//!     lambda-labs: 1 converged, 0 unchanged, 0 failed
//!
//!     $ ssh intel findmnt -no SOURCE /mnt/unas
//!     //192.168.1.179/Personal-Drive          <-- still the OLD share
//!     $ ssh intel grep unas /etc/fstab
//!     //192.168.1.179/Personal-Drive ...      <-- fstab never updated either
//!
//! Both guards tested the TARGET PATH, never the source:
//!
//!     if ! mountpoint -q '/mnt/unas'; then mount ... ; fi
//!     if ! grep -q '/mnt/unas' /etc/fstab; then echo ... >> /etc/fstab; fi
//!
//! The second is the more damaging: /etc/fstab is written ONCE, at first apply,
//! and never corrected. Every later change to `source`, `fs_type` or `options`
//! is silently discarded — declaration and host diverge permanently while
//! forjar reports success forever.
//!
//! WHY BEHAVIOURAL AND NOT STATIC.
//!
//! A first attempt asserted that changing a declared field must change the
//! emitted check script. That instrument was wrong twice: `state_query` scripts
//! have invariant TEXT by design (they read live state, so their OUTPUT varies,
//! not their source), and it flagged `file.content` — which is correctly
//! observed via `cat | blake3sum`. A meta-test that reports working code as
//! broken teaches people to ignore it.
//!
//! So this EXECUTES the emitted apply against a host that is present-but-wrong,
//! the state a static test cannot express.
//!
//! Note what these replace: six tests across four files asserted the buggy
//! implementation's command text (`mountpoint -q '/mnt/lambda-raid'`,
//! `grep -q '/mnt/shared' /etc/fstab`). None caught the bug; all failed when it
//! was fixed. A test that pins generated text protects whatever was written
//! first.

use forjar::core::types::{MachineTarget, Resource, ResourceType};
use std::fs;
use std::process::Command;

fn mount_resource(source: &str) -> Resource {
    Resource {
        resource_type: ResourceType::Mount,
        machine: MachineTarget::Single("h".into()),
        path: Some("/mnt/unas".into()),
        source: Some(source.into()),
        fs_type: Some("cifs".into()),
        options: Some("rw,vers=3.1.1".into()),
        state: Some("mounted".into()),
        ..Default::default()
    }
}

/// Run an emitted apply script against a FAKE host.
///
/// `mountpoint`/`mount`/`mkdir`/`umount` are stubbed so the script cannot touch
/// the real machine, and /etc/fstab is redirected to a temp file.
/// `already_mounted` models the host being present-but-wrong.
fn run_apply_against_fake_host(
    script: &str,
    fstab: &std::path::Path,
    already_mounted: bool,
) -> String {
    let dir = tempfile::tempdir().unwrap();
    let bin = dir.path().join("bin");
    fs::create_dir_all(&bin).unwrap();

    let mp_exit = i32::from(!already_mounted);
    for (name, body) in [
        ("mountpoint", format!("#!/bin/sh\nexit {mp_exit}\n")),
        ("mount", "#!/bin/sh\nexit 0\n".to_string()),
        ("mkdir", "#!/bin/sh\nexit 0\n".to_string()),
        ("umount", "#!/bin/sh\nexit 0\n".to_string()),
    ] {
        let p = bin.join(name);
        fs::write(&p, body).unwrap();
        let mut perms = fs::metadata(&p).unwrap().permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o755);
        fs::set_permissions(&p, perms).unwrap();
    }

    let redirected = script.replace("/etc/fstab", fstab.to_str().unwrap());

    Command::new("bash")
        .arg("-c")
        .arg(&redirected)
        .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
        .output()
        .expect("bash must run");

    fs::read_to_string(fstab).unwrap_or_default()
}

#[test]
fn changing_the_source_rewrites_fstab() {
    // THE REGRESSION. fstab already declares this mountpoint with the OLD
    // source — the exact state of intel and lambda-labs on 2026-08-19.
    let dir = tempfile::tempdir().unwrap();
    let fstab = dir.path().join("fstab");
    fs::write(
        &fstab,
        "//192.168.1.179/Personal-Drive /mnt/unas cifs rw,vers=3.1.1 0 0\n",
    )
    .unwrap();

    let script = forjar::resources::mount::apply_script(&mount_resource("//192.168.1.179/media"));
    let after = run_apply_against_fake_host(&script, &fstab, true);

    assert!(
        after.contains("//192.168.1.179/media"),
        "apply must write the DECLARED source into fstab. The old guard \
         `grep -q <target> /etc/fstab` is satisfied by the stale line, so the \
         declaration is discarded and never re-applied.\nfstab after:\n{after}\n\
         script:\n{script}"
    );
    assert!(
        !after.contains("Personal-Drive"),
        "the stale entry must not survive — two entries for one mountpoint is \
         its own failure.\nfstab after:\n{after}"
    );
}

#[test]
fn a_wrong_source_already_mounted_is_remounted() {
    // Fixing fstab is not enough if the live mount keeps serving the old
    // filesystem until someone reboots.
    let script = forjar::resources::mount::apply_script(&mount_resource("//192.168.1.179/media"));
    assert!(
        script.contains("findmnt"),
        "apply must compare the MOUNTED source against the declared one, not \
         merely ask whether the path is a mountpoint. Emitted:\n{script}"
    );
}

#[test]
fn a_first_apply_on_a_bare_host_still_works() {
    // The corrective behaviour must not break the creative case.
    let dir = tempfile::tempdir().unwrap();
    let fstab = dir.path().join("fstab");
    fs::write(&fstab, "").unwrap();

    let script = forjar::resources::mount::apply_script(&mount_resource("//192.168.1.179/media"));
    let after = run_apply_against_fake_host(&script, &fstab, false);

    assert!(
        after.contains("//192.168.1.179/media") && after.contains("/mnt/unas"),
        "a bare host must still get its fstab entry.\nfstab after:\n{after}"
    );
    assert_eq!(
        after.matches("/mnt/unas").count(),
        1,
        "exactly one entry for the mountpoint.\nfstab after:\n{after}"
    );
}

#[test]
fn re_applying_an_already_correct_mount_changes_nothing() {
    // Idempotence is why the guards existed. Removing them naively would append
    // a duplicate fstab line on every apply.
    let dir = tempfile::tempdir().unwrap();
    let fstab = dir.path().join("fstab");
    fs::write(
        &fstab,
        "//192.168.1.179/media /mnt/unas cifs rw,vers=3.1.1 0 0\n",
    )
    .unwrap();

    let script = forjar::resources::mount::apply_script(&mount_resource("//192.168.1.179/media"));
    let after = run_apply_against_fake_host(&script, &fstab, true);

    assert_eq!(
        after.matches("/mnt/unas").count(),
        1,
        "re-applying a correct mount must not duplicate the fstab entry.\n\
         fstab after:\n{after}"
    );
}
