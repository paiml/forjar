//! Unit tests for the atomic binary-install shell helpers.
//!
//! These run the emitted shell. The behavioural falsification — a genuinely
//! RUNNING binary and a genuinely dangling symlink, through the real
//! generators — lives in `tests/falsification_replace_running_binary.rs`.

use super::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

fn sandbox(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("forjar-shell-install-{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("sandbox");
    dir
}

fn sh(script: &str) -> std::process::Output {
    Command::new("sh")
        .arg("-c")
        .arg(script)
        .output()
        .expect("run shell")
}

fn write_exec(path: &Path, body: &str) {
    fs::write(path, body).expect("write");
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("chmod");
}

#[test]
fn the_function_name_matches_what_is_emitted() {
    assert!(atomic_install_fn().contains(&format!("{ATOMIC_INSTALL_FN}() {{")));
}

#[test]
fn install_bin_lands_the_bytes_and_makes_them_executable() {
    let dir = sandbox("basic");
    let src = dir.join("src");
    write_exec(&src, "#!/bin/sh\necho hello\n");
    fs::create_dir_all(dir.join("sub")).expect("dest dir");
    let dst = dir.join("sub/dst");

    let out = sh(&format!(
        "set -eu\n{}\n_fj_install_bin '{}' '{}'\n'{}'",
        atomic_install_fn(),
        src.display(),
        dst.display(),
        dst.display()
    ));
    assert!(
        out.status.success(),
        "install failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "hello");
    let mode = fs::metadata(&dst).expect("dst").permissions().mode() & 0o777;
    assert_eq!(mode, 0o755, "destination is not executable");
}

/// The helper deliberately does NOT create the destination directory — that
/// `mkdir -p "$dir"` tripped bashrs SEC010 (path traversal) in the shipped
/// `install.sh`, and every caller already creates the directory. A missing
/// directory must therefore FAIL, loudly, rather than be papered over.
#[test]
fn a_missing_destination_directory_is_a_failure_not_a_silent_pass() {
    let dir = sandbox("nodir");
    let src = dir.join("src");
    write_exec(&src, "#!/bin/sh\n:\n");
    let dst = dir.join("absent/dst");
    let out = sh(&format!(
        "set -eu\n{}\nif _fj_install_bin '{}' '{}'; then echo UNEXPECTED_OK; else echo REPORTED_FAILURE; fi",
        atomic_install_fn(),
        src.display(),
        dst.display()
    ));
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("REPORTED_FAILURE"),
        "a missing destination directory must fail: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(!dst.exists());
}

/// The staging file must be a SIBLING of the destination, or the final `mv`
/// can cross a filesystem boundary and degrade from `rename(2)` into
/// copy-then-unlink — which is neither atomic nor ETXTBSY-safe, and would
/// reintroduce the defect on any host where /tmp is a separate mount.
#[test]
fn staging_happens_in_the_destination_directory() {
    assert!(
        atomic_install_fn().contains(r#"_fji_tmp="$_fji_dir/.forjar-install.$$""#),
        "staging file must be a sibling of the destination"
    );
    assert!(
        !atomic_install_fn().contains("/tmp"),
        "staging must not be placed on a possibly-different filesystem"
    );
}

/// No `cp` may target the destination path itself: an in-place open is the
/// whole defect. Only the sibling temp may be opened for writing.
#[test]
fn nothing_opens_the_destination_in_place() {
    let f = atomic_install_fn();
    assert!(
        !f.contains(r#"cp -f "$_fji_src" "$_fji_dst""#),
        "the destination must never be a copy target"
    );
    assert!(
        f.contains(r#"mv -f "$_fji_tmp" "$_fji_dst""#),
        "the destination must be reached by rename(2)"
    );
}

#[test]
fn a_failed_copy_leaves_no_staging_litter_and_reports_failure() {
    let dir = sandbox("failure");
    let dst = dir.join("dst");
    let out = sh(&format!(
        "set -eu\n{}\nif _fj_install_bin '{}/does-not-exist' '{}'; then echo UNEXPECTED_OK; else echo REPORTED_FAILURE; fi",
        atomic_install_fn(),
        dir.display(),
        dst.display()
    ));
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("REPORTED_FAILURE"),
        "a missing source must fail the function: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(!dst.exists(), "a failed install must not create the target");
    let litter: Vec<_> = fs::read_dir(&dir)
        .expect("readdir")
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with(".forjar-install")
        })
        .collect();
    assert!(litter.is_empty(), "staging file left behind: {litter:?}");
}

#[test]
fn install_bins_lands_every_binary_in_the_staging_dir() {
    let dir = sandbox("many");
    let staging = dir.join("staging/bin");
    fs::create_dir_all(&staging).expect("staging");
    write_exec(&staging.join("alpha"), "#!/bin/sh\necho alpha\n");
    write_exec(&staging.join("beta"), "#!/bin/sh\necho beta\n");
    let dest = dir.join("dest");
    fs::create_dir_all(&dest).expect("dest");

    let out = sh(&format!(
        "set -eu\n{}\n_fj_install_bins '{}' '{}'\n'{}/alpha'\n'{}/beta'",
        atomic_install_dir_fn(),
        staging.display(),
        dest.display(),
        dest.display(),
        dest.display()
    ));
    assert!(
        out.status.success(),
        "install_bins failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("alpha") && stdout.contains("beta"),
        "{stdout}"
    );
}

/// A crate whose staging `bin/` is empty must not be reported as installed by
/// the helper. (The provider has its own louder check; this one guards the
/// loop from silently succeeding on nothing.)
#[test]
fn install_bins_over_an_empty_staging_dir_installs_nothing() {
    let dir = sandbox("empty");
    let staging = dir.join("staging/bin");
    fs::create_dir_all(&staging).expect("staging");
    let dest = dir.join("dest");
    fs::create_dir_all(&dest).expect("dest");

    let out = sh(&format!(
        "set -eu\n{}\n_fj_install_bins '{}' '{}'",
        atomic_install_dir_fn(),
        staging.display(),
        dest.display()
    ));
    assert!(out.status.success(), "empty staging must not be an error");
    assert_eq!(
        fs::read_dir(&dest).expect("readdir").count(),
        0,
        "nothing should have been installed"
    );
}

/// An empty runner must fall back to `command`, not to the empty string.
/// The prefix is a QUOTED command name here (`"$_fji_run" cp ...`), which is
/// what keeps the emitted shell free of SC2086/SC2183; an empty value would
/// try to execute `""` and fail every unprivileged install.
#[test]
fn an_empty_runner_prefix_contributes_no_argument() {
    let dir = sandbox("runner");
    let src = dir.join("src");
    write_exec(&src, "#!/bin/sh\necho ok\n");
    let dst = dir.join("dst");
    let out = sh(&format!(
        "set -eu\n{}\n_fj_install_bin '{}' '{}' ''\n'{}'",
        atomic_install_fn(),
        src.display(),
        dst.display(),
        dst.display()
    ));
    assert!(
        out.status.success(),
        "explicit empty runner broke the call: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "ok");
}
