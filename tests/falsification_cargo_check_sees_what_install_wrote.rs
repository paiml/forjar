//! forjar#489 (PMAT-213): the cargo CHECK must repair the same PATH the cargo
//! INSTALL repairs.
//!
//! # The defect
//!
//! `apply_cargo_present` bootstraps rustup with `--no-modify-path` and then
//! repairs its own environment:
//!
//! ```sh
//! export PATH="$HOME/.cargo/bin:$PATH"
//! ```
//!
//! The check (`package_check.rs`, cargo arm) and the drift observable
//! (`package/cargo.rs::state_query`) did not. Both run `cargo install --list`
//! and `command -v <bin>` with whatever PATH a NON-INTERACTIVE shell has, and
//! forjar itself creates the host where that PATH lacks cargo: Ubuntu's stock
//! `~/.bashrc` returns at line 8 when not interactive, and rustup appends
//! `. "$HOME/.cargo/env"` at the BOTTOM of it, so the line never runs.
//!
//! Measured by the operator on the box named `yoga` against 1.25.2, on a
//! freshly reimaged disk: `~/.cargo/bin/rg` present and executable,
//! `rg --version` -> `ripgrep 15.1.0`, `~/.cargo/.crates.toml` valid and
//! listing it, `cargo install --list` listing it — and forjar reporting
//! `missing:ripgrep`, forever. Likewise `bat` and `fd-find`. The second
//! symptom shares the root cause: the install action's own
//! `command -v cargo ||` guard keeps missing, so every apply re-bootstraps a
//! toolchain that is already installed.
//!
//! # Why these tests EXECUTE the script
//!
//! The text assertions below are ordering claims and cannot be satisfied by a
//! comment (comment lines are stripped before the search, and the needle is
//! anchored to a `PATH=` assignment). But a text match still only proves the
//! bytes are there. `check_reports_installed_when_cargo_is_off_the_path` and
//! its drift twin RUN the emitted shell against a stub cargo that is reachable
//! only through `$CARGO_HOME/bin` — the exact shape of the yoga host. Those
//! two fail on any fix that is spelled right and wired wrong.
//!
//! This repository has been bitten by the text-match-hits-its-own-comment
//! failure before: RULE 8 in `tests/falsification_release_workflow_shape.rs`
//! searched a whole job's text for a flag that its own comment also named.

use forjar::core::types::{MachineTarget, Resource, ResourceType};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

fn cargo_pkg(packages: &[&str]) -> Resource {
    Resource {
        resource_type: ResourceType::Package,
        machine: MachineTarget::Single("local".to_string()),
        provider: Some("cargo".to_string()),
        packages: packages.iter().map(|s| (*s).to_string()).collect(),
        ..Default::default()
    }
}

fn check(packages: &[&str]) -> String {
    forjar::resources::package_check::check_script(&cargo_pkg(packages))
}

fn drift(packages: &[&str]) -> String {
    forjar::resources::package::state_query_script(&cargo_pkg(packages))
}

fn install(packages: &[&str]) -> String {
    forjar::resources::package::apply_script(&cargo_pkg(packages))
}

// --------------------------------------------------------------------------
// Comment-blind, command-anchored search.
// --------------------------------------------------------------------------

/// The script with every whole-line `#` comment removed.
///
/// A fix that lives in a comment is not a fix. Every text assertion in this
/// file searches THIS, never the raw script.
fn without_comments(script: &str) -> String {
    script
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

/// True if `line` is an actual PATH repair COMMAND — it assigns `PATH` and the
/// value it assigns names cargo's bin directory.
///
/// Anchored to the assignment, so prose that merely mentions `PATH` and
/// `~/.cargo/bin` cannot satisfy it.
fn is_path_repair(line: &str) -> bool {
    let t = line.trim();
    if t.starts_with('#') {
        return false;
    }
    let assigns_path = t.contains("PATH=\"") || t.contains("PATH='");
    let names_cargo_bin =
        t.contains("${CARGO_HOME:-$HOME/.cargo}/bin") || t.contains("$HOME/.cargo/bin");
    assigns_path && names_cargo_bin
}

/// Byte offset of the first PATH-repair command, comments already gone.
fn repair_at(script: &str) -> Option<usize> {
    let stripped = without_comments(script);
    let mut off = 0usize;
    for line in stripped.lines() {
        if is_path_repair(line) {
            return Some(off);
        }
        off += line.len() + 1;
    }
    None
}

/// Byte offset of the first occurrence of `needle`, comments already gone.
fn cmd_at(script: &str, needle: &str) -> Option<usize> {
    without_comments(script).find(needle)
}

fn assert_repair_precedes(script: &str, needle: &str, what: &str) {
    let repair = repair_at(script).unwrap_or_else(|| {
        panic!("{what}: no PATH repair command at all.\n--- script ---\n{script}")
    });
    let use_site = match cmd_at(script, needle) {
        Some(i) => i,
        None => return, // the script does not use it; nothing to order against
    };
    assert!(
        repair < use_site,
        "{what}: the PATH repair is at {repair} but `{needle}` is used at \
         {use_site} — the repair must come FIRST or it repairs nothing.\n\
         --- script ---\n{script}"
    );
}

// --------------------------------------------------------------------------
// 1. The check repairs PATH before it asks cargo anything.
// --------------------------------------------------------------------------

#[test]
fn check_script_repairs_path_before_it_asks_cargo() {
    let s = check(&["ripgrep"]);
    assert_repair_precedes(&s, "cargo install --list", "cargo check_script");
    assert_repair_precedes(&s, "command -v", "cargo check_script");
}

#[test]
fn drift_observable_repairs_path_before_it_asks_cargo() {
    let s = drift(&["ripgrep"]);
    assert_repair_precedes(&s, "cargo install --list", "cargo state_query");
    assert_repair_precedes(&s, "command -v", "cargo state_query");
}

/// The install action's repair is what the other two were missing. Deleting it
/// would "harmonise" the three sites in the wrong direction.
#[test]
fn install_action_still_repairs_path() {
    let s = install(&["ripgrep"]);
    assert!(
        repair_at(&s).is_some(),
        "the install action lost its PATH repair — that is the whole reason \
         the check could be fixed by copying it.\n--- script ---\n{s}"
    );
}

/// forjar#489, second symptom: `command -v cargo ||` re-ran rustup-init on
/// EVERY apply, because cargo was installed and off PATH. The repair has to
/// precede the bootstrap guard for the guard to see the toolchain it already
/// installed.
#[test]
fn install_action_repairs_path_before_its_bootstrap_guard() {
    let s = install(&["ripgrep"]);
    assert_repair_precedes(&s, "command -v cargo", "cargo install action");
    assert_repair_precedes(&s, "rustup", "cargo install action");
}

/// The repair must survive comment-stripping in all three scripts — i.e. it
/// must be code. This is the mutation this file exists to catch: turning the
/// fix into a `# export PATH=...` note passes a naive `contains`.
#[test]
fn every_repair_is_code_and_not_a_comment() {
    for (what, s) in [
        ("check_script", check(&["ripgrep"])),
        ("state_query", drift(&["ripgrep"])),
        ("apply_script", install(&["ripgrep"])),
    ] {
        let stripped = without_comments(&s);
        assert!(
            stripped.lines().any(is_path_repair),
            "{what}: the PATH repair does not survive comment-stripping, so it \
             is prose, not a command.\n--- script ---\n{s}"
        );
    }
}

// --------------------------------------------------------------------------
// 2. The yoga host, reproduced: cargo reachable ONLY via $CARGO_HOME/bin.
// --------------------------------------------------------------------------

struct Host {
    _root: PathBuf,
    home: PathBuf,
    cargo_home: PathBuf,
}

fn exe(path: &Path, body: &str) {
    let mut f = std::fs::File::create(path).unwrap();
    f.write_all(body.as_bytes()).unwrap();
    drop(f);
    let mut perms = std::fs::metadata(path).unwrap().permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o755);
    std::fs::set_permissions(path, perms).unwrap();
}

/// A host where `ripgrep` really is installed: `$CARGO_HOME/bin/cargo` lists
/// it, `$CARGO_HOME/bin/rg` runs and reports its version — and NOTHING is on
/// the non-interactive PATH. `$HOME` is a different, empty directory, so a fix
/// that hard-codes `$HOME/.cargo` without honouring `CARGO_HOME` fails here
/// too.
fn yoga(name: &str) -> Host {
    let root = std::env::temp_dir().join(format!("fj-489-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let home = root.join("home");
    let cargo_home = root.join("cargo-home");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(cargo_home.join("bin")).unwrap();

    exe(
        &cargo_home.join("bin/cargo"),
        "#!/bin/sh\n\
         if [ \"$1\" = install ] && [ \"$2\" = --list ]; then\n\
           printf 'ripgrep v15.1.0:\\n    rg\\n'\n\
           exit 0\n\
         fi\n\
         exit 1\n",
    );
    exe(
        &cargo_home.join("bin/rg"),
        "#!/bin/sh\nprintf 'ripgrep 15.1.0\\n'\n",
    );

    Host {
        _root: root,
        home,
        cargo_home,
    }
}

fn run_on(host: &Host, script: &str) -> std::process::Output {
    Command::new("/bin/sh")
        .arg("-c")
        .arg(script)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", &host.home)
        .env("CARGO_HOME", &host.cargo_home)
        .output()
        .expect("sh")
}

#[test]
fn check_reports_installed_when_cargo_is_off_the_noninteractive_path() {
    let host = yoga("check");
    let out = run_on(&host, &check(&["ripgrep"]));
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(
        stdout.contains("installed:ripgrep"),
        "forjar reported a crate it installed as missing, because cargo was \
         not on the non-interactive PATH (forjar#489).\n\
         stdout: {stdout}\nstderr: {stderr}\n--- script ---\n{}",
        check(&["ripgrep"])
    );
    assert_eq!(
        out.status.code(),
        Some(0),
        "the check must EXIT converged, not merely print the marker.\n\
         stdout: {stdout}\nstderr: {stderr}"
    );
}

#[test]
fn drift_observable_sees_the_crate_when_cargo_is_off_the_noninteractive_path() {
    let host = yoga("drift");
    let out = run_on(&host, &drift(&["ripgrep"]));
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(
        stdout.contains("ripgrep=installed"),
        "the drift observable reported MISSING for an installed crate, so \
         every apply looks like drift (forjar#489).\nstdout: {stdout}"
    );
    assert!(
        !stdout.contains("GONE"),
        "the binary resolved through PATH failed even with the repair in \
         place.\nstdout: {stdout}"
    );
}

/// The repair must not depend on `$HOME` when `CARGO_HOME` is set — the fleet
/// runs cargo out of a shared `CARGO_HOME` on several boxes, and the
/// surrounding code (`_CARGO_BIN`, `_CRATES_TOML`) already honours it.
#[test]
fn the_repair_honours_cargo_home() {
    for s in [check(&["ripgrep"]), drift(&["ripgrep"])] {
        let line = without_comments(&s)
            .lines()
            .find(|l| is_path_repair(l))
            .map(str::to_string)
            .expect("a PATH repair");
        assert!(
            line.contains("CARGO_HOME"),
            "the repair hard-codes $HOME/.cargo and ignores CARGO_HOME: {line}"
        );
    }
}

/// A prelude that breaks the script it protects is worse than no prelude.
/// `set -u` is in force in several of these scripts, and `forjar apply` runs
/// every generated script through its own I8 gate.
#[test]
fn the_repaired_scripts_are_still_safe_and_lintable() {
    for (what, s) in [
        ("check_script", check(&["ripgrep", "bat"])),
        ("state_query", drift(&["ripgrep", "bat"])),
    ] {
        let host = yoga("safe");
        let out = run_on(&host, &format!("set -eu\n{s}"));
        assert!(
            !String::from_utf8_lossy(&out.stderr).contains("unbound variable"),
            "{what} breaks under `set -u`: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}
