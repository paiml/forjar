//! GH-257: the cargo package check must ask cargo, not the PATH.
//!
//! The check was `command -v <crate_name>`, wrong in two independent ways.
//!
//! **A crate's name is not its binary's name.** `kani-verifier` installs
//! `cargo-kani` and `kani`, so `command -v kani-verifier` can never succeed.
//! Observed on intel: `forjar check -r kani-verifier` FAILED while
//! `cargo-kani --version` printed `cargo-kani 0.67.0`.
//!
//! **`command -v` tests existence, not function.** It succeeds for any path
//! that exists and carries the executable bit. Also observed on intel:
//! `~/.cargo/bin/rustup` was gone while `~/.cargo/bin/cargo` remained as a
//! symlink *to it* — a dangling link that satisfied every existence-check and
//! could not run a single command.
//!
//! Those two produced opposite errors at once (check said missing, apply said
//! unchanged), which is how a CI host rotted with nothing noticing.
//!
//! These tests EXECUTE the emitted script against a stub `cargo`. The test this
//! replaces asserted `script.contains("command -v 'batuta'")` — a text match
//! that passes on a script checking entirely the wrong thing, which is exactly
//! how the defect survived having a test.

use forjar::core::types::{MachineTarget, Resource, ResourceType};
use std::io::Write;
use std::process::Command;

fn cargo_pkg(packages: &[&str], version: Option<&str>) -> Resource {
    Resource {
        resource_type: ResourceType::Package,
        machine: MachineTarget::Single("local".to_string()),
        provider: Some("cargo".to_string()),
        packages: packages.iter().map(|s| (*s).to_string()).collect(),
        version: version.map(str::to_string),
        ..Default::default()
    }
}

/// A stub `cargo` whose `install --list` prints the given inventory.
///
/// Real `cargo install --list` output, so the parsing under test faces the
/// real shape rather than a convenient one.
fn stub_cargo(dir: &std::path::Path, listing: &str) {
    let bin = dir.join("cargo");
    let mut f = std::fs::File::create(&bin).unwrap();
    writeln!(f, "#!/usr/bin/env bash").unwrap();
    writeln!(f, "if [ \"$1\" = install ] && [ \"$2\" = --list ]; then").unwrap();
    writeln!(f, "cat <<'LISTING'\n{listing}\nLISTING").unwrap();
    writeln!(f, "  exit 0").unwrap();
    writeln!(f, "fi").unwrap();
    writeln!(f, "exit 1").unwrap();
    drop(f);
    let mut perms = std::fs::metadata(&bin).unwrap().permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o755);
    std::fs::set_permissions(&bin, perms).unwrap();
}

/// Run the emitted check script with only the stub dir on PATH.
fn run_check(script: &str, stub_dir: &std::path::Path) -> bool {
    Command::new("bash")
        .arg("-c")
        .arg(script)
        .env("PATH", format!("{}:/usr/bin:/bin", stub_dir.display()))
        .output()
        .expect("bash must run")
        .status
        .success()
}

const KANI_LISTING: &str = "kani-verifier v0.67.0:\n    cargo-kani\n    kani";

#[test]
fn a_crate_whose_binary_has_a_different_name_is_found() {
    // THE REGRESSION. `kani-verifier` installs `cargo-kani` and `kani`, never a
    // binary of its own name, so the old `command -v kani-verifier` reported
    // missing on a host where it was installed and working.
    let dir = tempfile::tempdir().unwrap();
    stub_cargo(dir.path(), KANI_LISTING);

    let script =
        forjar::resources::package_check::check_script(&cargo_pkg(&["kani-verifier"], None));
    assert!(
        run_check(&script, dir.path()),
        "kani-verifier is installed per cargo's own record; the check must \
         agree.\nscript:\n{script}"
    );
}

#[test]
fn a_crate_that_is_not_installed_is_reported_missing() {
    // The gate must still be able to fail, or it is worse than none.
    let dir = tempfile::tempdir().unwrap();
    stub_cargo(dir.path(), KANI_LISTING);

    let script =
        forjar::resources::package_check::check_script(&cargo_pkg(&["not-installed"], None));
    assert!(
        !run_check(&script, dir.path()),
        "a crate absent from cargo's record must report missing"
    );
}

#[test]
fn a_prefix_of_an_installed_crate_is_not_a_match() {
    // `pmat` must not be satisfied by `pmat-extra`. Anchoring is the whole
    // reason the needle carries `^` and the ` v` before the version.
    let dir = tempfile::tempdir().unwrap();
    stub_cargo(dir.path(), "pmat-extra v1.0.0:\n    pmat-extra");

    let script = forjar::resources::package_check::check_script(&cargo_pkg(&["pmat"], None));
    assert!(
        !run_check(&script, dir.path()),
        "`pmat` must not be satisfied by `pmat-extra`"
    );
}

#[test]
fn a_pinned_version_must_match_the_installed_one() {
    // A pin that is never verified is a comment. With `version:` set, the
    // check has to compare it, or a stale build silently satisfies a bump.
    let dir = tempfile::tempdir().unwrap();
    stub_cargo(dir.path(), "forjar v1.13.1:\n    forjar");

    let matching =
        forjar::resources::package_check::check_script(&cargo_pkg(&["forjar"], Some("1.13.1")));
    assert!(
        run_check(&matching, dir.path()),
        "the installed version matches the pin; the check must pass"
    );

    let bumped =
        forjar::resources::package_check::check_script(&cargo_pkg(&["forjar"], Some("1.14.0")));
    assert!(
        !run_check(&bumped, dir.path()),
        "1.13.1 is installed but 1.14.0 is pinned — the check must report missing"
    );
}

#[test]
fn the_check_does_not_depend_on_path_lookup() {
    // The original failure mode: a binary present on a LOGIN shell's PATH and
    // absent from the runner service's. Asking cargo removes PATH from the
    // question entirely. Here the crate is installed per cargo but no binary of
    // any name exists on PATH — the old check would say missing.
    let dir = tempfile::tempdir().unwrap();
    stub_cargo(dir.path(), KANI_LISTING);

    let script =
        forjar::resources::package_check::check_script(&cargo_pkg(&["kani-verifier"], None));
    assert!(
        run_check(&script, dir.path()),
        "installation is a fact about cargo's record, not about PATH"
    );
}

#[test]
fn feature_syntax_is_stripped_before_matching() {
    // `copia[cli]` is the crate `copia` with a feature; cargo lists it as
    // `copia`, so the brackets must not reach the needle.
    let dir = tempfile::tempdir().unwrap();
    stub_cargo(dir.path(), "copia v0.2.0:\n    copia");

    let script = forjar::resources::package_check::check_script(&cargo_pkg(&["copia[cli]"], None));
    assert!(
        run_check(&script, dir.path()),
        "the feature suffix must be stripped before matching cargo's record"
    );
}
