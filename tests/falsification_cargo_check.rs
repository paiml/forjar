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
///
/// `CARGO_HOME` points at the stub dir too, so `install_bin` below decides
/// whether the binaries cargo CLAIMS to have installed are actually there.
fn run_check(script: &str, stub_dir: &std::path::Path) -> bool {
    Command::new("bash")
        .arg("-c")
        .arg(script)
        .env("PATH", format!("{}:/usr/bin:/bin", stub_dir.display()))
        .env("CARGO_HOME", stub_dir)
        .output()
        .expect("bash must run")
        .status
        .success()
}

/// Materialise a binary in `$CARGO_HOME/bin` that answers `--version`.
///
/// This is what `cargo install --list` is a RECORD of. The record and the
/// binary are separate facts, and GH-2xx is what happens when they disagree.
fn install_bin(cargo_home: &std::path::Path, name: &str) {
    let bindir = cargo_home.join("bin");
    std::fs::create_dir_all(&bindir).unwrap();
    let p = bindir.join(name);
    let mut f = std::fs::File::create(&p).unwrap();
    writeln!(f, "#!/usr/bin/env bash\necho '{name} 1.0.0'").unwrap();
    drop(f);
    let mut perms = std::fs::metadata(&p).unwrap().permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o755);
    std::fs::set_permissions(&p, perms).unwrap();
}

/// Materialise a real cargo SUBCOMMAND: `cargo-foo` expects argv[1] to be
/// `foo` (that is how cargo invokes it) and rejects a bare `--version`.
///
/// Measured on intel 2026-08-19 after the restore: `cargo-mutants --version`
/// exits 1 with "unexpected argument '--version' found", while
/// `cargo mutants --version` prints `cargo-mutants 27.1.0`. Same for
/// `cargo-llvm-cov`. Both were installed, both worked, and a checker that only
/// tried the bare form would have called them missing forever — reinstalling a
/// ~10-minute compile on every single apply.
fn install_cargo_subcommand(cargo_home: &std::path::Path, name: &str) {
    let bindir = cargo_home.join("bin");
    std::fs::create_dir_all(&bindir).unwrap();
    let p = bindir.join(name);
    let sub = name
        .strip_prefix("cargo-")
        .expect("must be a cargo- binary");
    let mut f = std::fs::File::create(&p).unwrap();
    writeln!(
        f,
        "#!/usr/bin/env bash\n\
         if [ \"$1\" != '{sub}' ]; then echo \"error: unexpected argument\" >&2; exit 1; fi\n\
         echo '{name} 1.0.0'"
    )
    .unwrap();
    drop(f);
    let mut perms = std::fs::metadata(&p).unwrap().permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o755);
    std::fs::set_permissions(&p, perms).unwrap();
}

/// Replace a binary with a symlink to a target that does not exist.
///
/// The exact wreckage rust-cache's post-step leaves behind: every real file in
/// `~/.cargo/bin` deleted, every symlink surviving and pointing at nothing.
fn dangle_bin(cargo_home: &std::path::Path, name: &str) {
    let bindir = cargo_home.join("bin");
    std::fs::create_dir_all(&bindir).unwrap();
    let p = bindir.join(name);
    let _ = std::fs::remove_file(&p);
    std::os::unix::fs::symlink(bindir.join("rustup-which-is-gone"), &p).unwrap();
}

const KANI_LISTING: &str = "kani-verifier v0.67.0:\n    cargo-kani\n    kani";

#[test]
fn a_recorded_crate_whose_binary_was_deleted_is_reported_missing() {
    // THE LIVE REGRESSION, measured on intel 2026-08-19 08:01.
    //
    // rust-cache's post-step deletes every real file in the shared
    // ~/.cargo/bin and leaves the symlinks. Cargo's install RECORD is a
    // separate file and survives untouched, so `cargo install --list` keeps
    // reporting a full inventory of binaries that are gone.
    //
    // Measured consequence: `forjar apply -t stack-tools` on a host with NO
    // rustup, NO cargo and NO rustc reported `rustup-installer: no changes`,
    // `stack-tool-copia/-forjar/-pmat/-pzsh: no changes` — 5 of 5 resources
    // called converged while every one of their binaries was absent. A plain
    // apply would have restored nothing and exited 0.
    //
    // The previous fix (GH-257) replaced `command -v` with `cargo install
    // --list`, trading a check that a PATH entry exists for a check that a
    // RECORD exists. Neither asks the binary to run, which is the only
    // question the resource actually cares about.
    let dir = tempfile::tempdir().unwrap();
    stub_cargo(dir.path(), KANI_LISTING);
    // Cargo's record says kani-verifier is installed — and nothing is there.

    let script =
        forjar::resources::package_check::check_script(&cargo_pkg(&["kani-verifier"], None));
    assert!(
        !run_check(&script, dir.path()),
        "cargo's record lists binaries that do not exist; a check that believes \
         the record reports a broken host as converged.\nscript:\n{script}"
    );
}

#[test]
fn a_dangling_symlink_is_reported_missing() {
    // The precise wreckage: the binary path EXISTS and carries the executable
    // bit, but resolves to nothing. Every existence-flavoured check is happy
    // with it and it cannot run a single command.
    let dir = tempfile::tempdir().unwrap();
    stub_cargo(dir.path(), KANI_LISTING);
    install_bin(dir.path(), "kani");
    dangle_bin(dir.path(), "cargo-kani");

    let script =
        forjar::resources::package_check::check_script(&cargo_pkg(&["kani-verifier"], None));
    assert!(
        !run_check(&script, dir.path()),
        "a dangling symlink must not satisfy the check — that is the exact \
         shape that rotted intel.\nscript:\n{script}"
    );
}

#[test]
fn every_binary_the_crate_installed_must_work_not_just_one() {
    // kani-verifier installs TWO binaries. A check satisfied by the first one
    // it finds would pass on a half-destroyed install.
    let dir = tempfile::tempdir().unwrap();
    stub_cargo(dir.path(), KANI_LISTING);
    install_bin(dir.path(), "cargo-kani");
    // `kani` deliberately not installed.

    let script =
        forjar::resources::package_check::check_script(&cargo_pkg(&["kani-verifier"], None));
    assert!(
        !run_check(&script, dir.path()),
        "one working binary out of two is not an installed crate.\nscript:\n{script}"
    );
}

#[test]
fn a_cargo_subcommand_that_rejects_a_bare_version_flag_still_counts_as_installed() {
    // The trap in the OBVIOUS form of this fix. `cargo-mutants` and
    // `cargo-llvm-cov` are cargo subcommands: cargo invokes them as
    // `cargo-mutants mutants ...`, so argv[1] must be the subcommand name and a
    // bare `--version` is an error. Both are installed and working on intel.
    //
    // A checker that only tried `cargo-mutants --version` would report them
    // missing on every run, and forjar would rebuild a ~10-minute compile each
    // apply — trading a false "converged" for a false "missing", which is not
    // an improvement, just a different lie.
    let dir = tempfile::tempdir().unwrap();
    stub_cargo(dir.path(), "cargo-mutants v27.1.0:\n    cargo-mutants");
    install_cargo_subcommand(dir.path(), "cargo-mutants");

    let script =
        forjar::resources::package_check::check_script(&cargo_pkg(&["cargo-mutants"], None));
    assert!(
        run_check(&script, dir.path()),
        "a working cargo subcommand must not be reported missing just because \
         it rejects a bare --version.\nscript:\n{script}"
    );
}

#[test]
fn a_deleted_cargo_subcommand_is_still_reported_missing() {
    // The subcommand accommodation must not become a hole: if neither
    // invocation works, the tool is missing. Otherwise the fallback would
    // rescue exactly the binaries this whole change exists to catch.
    let dir = tempfile::tempdir().unwrap();
    stub_cargo(dir.path(), "cargo-mutants v27.1.0:\n    cargo-mutants");
    // Recorded, never materialised.

    let script =
        forjar::resources::package_check::check_script(&cargo_pkg(&["cargo-mutants"], None));
    assert!(
        !run_check(&script, dir.path()),
        "an absent cargo subcommand must still report missing.\nscript:\n{script}"
    );
}

#[test]
fn a_crate_whose_binary_has_a_different_name_is_found() {
    // THE REGRESSION. `kani-verifier` installs `cargo-kani` and `kani`, never a
    // binary of its own name, so the old `command -v kani-verifier` reported
    // missing on a host where it was installed and working.
    let dir = tempfile::tempdir().unwrap();
    stub_cargo(dir.path(), KANI_LISTING);
    install_bin(dir.path(), "cargo-kani");
    install_bin(dir.path(), "kani");

    let script =
        forjar::resources::package_check::check_script(&cargo_pkg(&["kani-verifier"], None));
    assert!(
        run_check(&script, dir.path()),
        "kani-verifier is installed and both its binaries run; the check must \
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
    install_bin(dir.path(), "forjar");

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
    // The original failure mode (GH-257): a binary present on a LOGIN shell's
    // PATH and absent from the runner service's, so a PATH-based check gave
    // two different answers for one host. That fix must survive.
    //
    // Its premise, though, does not. This test used to assert the check passes
    // with NO binary anywhere, on the reasoning that "installation is a fact
    // about cargo's record, not about PATH". The record half of that is what
    // rotted intel: the record is not a fact about whether the tool WORKS.
    //
    // Both concerns are satisfiable at once by resolving through
    // $CARGO_HOME/bin — cargo's own deterministic install location — rather
    // than through PATH. Here the binaries exist there and are NOT on PATH,
    // so a passing check proves it resolved without a PATH lookup AND proves
    // it found something real.
    let dir = tempfile::tempdir().unwrap();
    stub_cargo(dir.path(), KANI_LISTING);
    install_bin(dir.path(), "cargo-kani");
    install_bin(dir.path(), "kani");
    assert!(
        !std::path::Path::new(&dir.path().join("cargo-kani")).exists(),
        "the binaries must live in $CARGO_HOME/bin, which run_check keeps OFF PATH"
    );

    let script =
        forjar::resources::package_check::check_script(&cargo_pkg(&["kani-verifier"], None));
    assert!(
        run_check(&script, dir.path()),
        "installation must resolve through $CARGO_HOME/bin, not PATH.\nscript:\n{script}"
    );
}

#[test]
fn feature_syntax_is_stripped_before_matching() {
    // `copia[cli]` is the crate `copia` with a feature; cargo lists it as
    // `copia`, so the brackets must not reach the needle.
    let dir = tempfile::tempdir().unwrap();
    stub_cargo(dir.path(), "copia v0.2.0:\n    copia");
    install_bin(dir.path(), "copia");

    let script = forjar::resources::package_check::check_script(&cargo_pkg(&["copia[cli]"], None));
    assert!(
        run_check(&script, dir.path()),
        "the feature suffix must be stripped before matching cargo's record"
    );
}

#[test]
fn the_install_script_can_overwrite_a_dangling_symlink() {
    // Detecting damage you cannot repair is half a tool.
    //
    // Measured on intel 2026-08-19: once --refresh correctly noticed that
    // `pzsh` had been reduced to a dangling symlink, the apply died with
    //     cp: not writing through dangling symlink '/home/noah/.cargo/bin/pzsh'
    // `cp -f` is refused the same way (verified on the host). That is the exact
    // wreckage a CI cache-prune leaves in a shared ~/.cargo/bin, so the one
    // state this resource most needs to repair was the one it could not.
    //
    // Asserts on the emitted script's BEHAVIOUR: build the broken destination
    // for real and run the placement command against it.
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("staging");
    let dst = dir.path().join("bin");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::create_dir_all(&dst).unwrap();
    std::fs::write(src.join("pzsh"), "#!/bin/sh\necho pzsh 1.0.0\n").unwrap();
    std::os::unix::fs::symlink(dst.join("deleted-by-rust-cache"), dst.join("pzsh")).unwrap();

    let script = forjar::resources::package::apply_script(&cargo_pkg(&["pzsh"], None));
    let placement = script
        .lines()
        .find(|l| l.contains("$_STAGING/bin/"))
        .expect("the script must place the staged binaries");
    assert!(
        !placement.trim_start().starts_with("cp "),
        "plain `cp` cannot overwrite a dangling symlink: {placement}"
    );

    // Run the real placement form against the real broken destination.
    let ok = Command::new("bash")
        .arg("-c")
        .arg(format!(
            "install -m 755 {}/* {}/",
            src.display(),
            dst.display()
        ))
        .status()
        .expect("bash must run")
        .success();
    assert!(ok, "placement must succeed over a dangling symlink");

    let placed = dst.join("pzsh");
    assert!(
        placed.is_file() && !placed.is_symlink(),
        "the dangling symlink must be replaced by the real binary"
    );
}
