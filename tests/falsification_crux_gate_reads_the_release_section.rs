//! PMAT-165: gate H must not go blind at the moment of the cut.
//!
//! # Why this test exists
//!
//! `scripts/dogfood/crux-reconcile.sh` reads the behaviour paragraphs of
//! `CHANGELOG.md`'s `[Unreleased]` section and demands a CRUX row for each. The
//! release cut renames that section to `[<version>]` BEFORE the tag exists, so
//! between the cut commit and the tag the paragraphs live under the version
//! heading and `[Unreleased]` is empty. Measured on this branch: with the
//! section rule absent, the gate said
//!
//! ```text
//! GATE H FAIL no behaviour bullet under [Unreleased] in CHANGELOG.md
//! ```
//!
//! which is the gate reporting a release with no behaviour changes at the one
//! moment it is most supposed to be reading them. A gate that goes quiet
//! exactly when the release happens is worse than no gate: it is a gate that
//! passes the release it was written to stop.
//!
//! The rule: read `[Unreleased]` when it opens at least one bold paragraph,
//! otherwise read `[<version from Cargo.toml>]`. After the tag the PENDING arm
//! takes over, because the version then equals the newest reachable tag.

#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn write(root: &Path, rel: &str, body: &str) {
    let p = root.join(rel);
    std::fs::create_dir_all(p.parent().expect("parent")).expect("mkdir");
    std::fs::write(p, body).expect("write");
}

/// A tree holding only what gate H reads: the script, a Cargo.toml with a
/// version, a CHANGELOG and a crux document.
fn fixture(changelog: &str, crux: &str, version: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    for rel in ["scripts/dogfood/crux-reconcile.sh"] {
        let body = std::fs::read_to_string(repo().join(rel)).expect("the gate under test");
        write(root, rel, &body);
    }
    write(
        root,
        "Cargo.toml",
        &format!("[package]\nname = \"forjar\"\nversion = \"{version}\"\n"),
    );
    write(root, "CHANGELOG.md", changelog);
    write(root, &format!("docs/audits/crux-{version}.md"), crux);
    // A git repo with ONE commit and no v* tag: `git tag --merged HEAD` needs a
    // resolvable HEAD, and the absence of a v* tag keeps this test on the arm
    // under test rather than on the post-tag PENDING arm.
    let git = |args: &[&str]| {
        Command::new("git")
            .args(args)
            .current_dir(root)
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@example.com")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@example.com")
            .status()
            .expect("git");
    };
    git(&["init", "-q", "-b", "main"]);
    git(&["config", "commit.gpgsign", "false"]);
    git(&["config", "core.hooksPath", "/nonexistent"]);
    git(&["add", "-A"]);
    git(&["commit", "-qm", "fixture"]);
    dir
}

fn run(dir: &tempfile::TempDir) -> (i32, String) {
    let out = Command::new("bash")
        .arg("scripts/dogfood/crux-reconcile.sh")
        .current_dir(dir.path())
        .output()
        .expect("run the gate");
    let text =
        String::from_utf8_lossy(&out.stdout).to_string() + &String::from_utf8_lossy(&out.stderr);
    (out.status.code().unwrap_or(-1), text)
}

/// One row per key, naming three surveyed systems in the SAME row: the gate
/// takes the first row containing the key and counts the systems in it, so
/// three one-system rows do not satisfy a three-system floor.
const CRUX: &str = "# CRUX\n\n| Behaviour | System | How [X] | forjar [V] | Delta | Disposition |\n|---|---|---|---|---|---|\n| the thing changed for everyone | Ansible, Terraform, Nix | a [X] | b [V] | c | reject(x) |\n";

/// The cut: `[Unreleased]` is empty and the paragraphs are under `[1.26.0]`.
#[test]
fn at_the_cut_the_gate_reads_the_version_section() {
    let changelog = "# Changelog\n\n## [Unreleased]\n\n## [1.26.0] - 2026-09-07\n\n**the thing changed for everyone**\n\nBody.\n";
    let fx = fixture(changelog, CRUX, "1.26.0");
    let (code, text) = run(&fx);
    assert_eq!(code, 0, "gate H went red at the cut: {text}");
    assert!(
        text.contains("under [1.26.0]"),
        "it did not say which section it read: {text}"
    );
}

/// Before the cut, `[Unreleased]` still wins even though a version section
/// exists — otherwise a paragraph added after the cut would go unreconciled.
#[test]
fn before_the_cut_unreleased_still_wins() {
    let changelog = "# Changelog\n\n## [Unreleased]\n\n**a paragraph nobody reconciled**\n\n## [1.26.0] - 2026-09-07\n\n**the thing changed for everyone**\n";
    let fx = fixture(changelog, CRUX, "1.26.0");
    let (code, text) = run(&fx);
    assert_ne!(
        code, 0,
        "an unreconciled [Unreleased] paragraph was not caught: {text}"
    );
    assert!(
        text.contains("under [Unreleased]"),
        "it read the wrong section: {text}"
    );
}

/// The anti-vacuity arm: a release whose section has no behaviour paragraph at
/// all is still a failure, in either section. A gate that reads an empty
/// section as "nothing to check" is the shape this whole file exists to refuse.
#[test]
fn a_release_with_no_behaviour_paragraph_anywhere_is_red() {
    let changelog = "# Changelog\n\n## [Unreleased]\n\n## [1.26.0] - 2026-09-07\n\nA sentence with no bold lead-in.\n";
    let fx = fixture(changelog, CRUX, "1.26.0");
    let (code, text) = run(&fx);
    assert_ne!(code, 0, "an empty release section passed: {text}");
    assert!(text.contains("no behaviour bullet"), "wrong reason: {text}");
}

/// And the row still has to be there: the section rule must not become a way
/// to pass by renaming.
#[test]
fn the_row_is_still_required_under_the_version_section() {
    let changelog = "# Changelog\n\n## [Unreleased]\n\n## [1.26.0] - 2026-09-07\n\n**something else entirely unreconciled**\n";
    let fx = fixture(changelog, CRUX, "1.26.0");
    let (code, text) = run(&fx);
    assert_ne!(
        code, 0,
        "an unreconciled paragraph passed under the version section: {text}"
    );
    assert!(text.contains("have no row"), "wrong reason: {text}");
}
