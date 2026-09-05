//! PMAT-159 — a release ships ONE version, and every artefact that states it
//! must state the same one.
//!
//! WHAT WAS OBSERVABLY WRONG, TWICE, IN THIS REPOSITORY.
//!
//! 1. **The lockfile lagged the manifest.** `Cargo.toml` was bumped and
//!    `Cargo.lock`'s own `[[package]] name = "forjar"` entry was not committed
//!    with it — *"the `Cargo.lock` version bump that every v1.4.x tag was
//!    missing"* (#131, `CHANGELOG.md` under [1.5.0]). `release.yml` then ran
//!    `cargo package`, cargo rewrote the lock, the tree went dirty and release
//!    creation was skipped. `ci.yml`'s `lockfile-preflight` job
//!    (`cargo package --locked --no-verify --workspace`) was added for exactly
//!    this, and it can only speak on a PR: nothing asserts the invariant from
//!    inside the test suite, where the pre-push quorum gate and the release
//!    protocol both look.
//!
//! 2. **A tag whose version nothing had read back.** `release.yml` compares the
//!    tag against `cargo metadata`, i.e. against the manifest — it never asks
//!    the BINARY what it calls itself. This fleet's standing rule is to verify
//!    the effective artefact rather than the declaration about it.
//!
//! WHY THESE ASSERTIONS. The oracle is the manifest TEXT, read at run time as
//! bytes, not `CARGO_PKG_VERSION` alone — a constant baked in at compile time
//! cannot disagree with itself, so a test that only compares it to another copy
//! of itself proves nothing. Every leg below compares an INDEPENDENTLY PRODUCED
//! artefact (the compiled binary's own `--version` output, the resolver's
//! lockfile entry, the hand-written changelog) against that text.
//!
//! HOW THEY WERE SHOWN TO DISCRIMINATE (v1.25.2):
//!   * bump `Cargo.toml` to 1.25.2 alone → `the_changelog_has_an_entry_for_this_version`
//!     goes RED ("CHANGELOG.md has no `## [1.25.2]` heading");
//!   * rewrite the lockfile's `forjar` entry to 1.25.1 and run the already-built
//!     test binary directly (so cargo cannot silently repair the lock first) →
//!     `the_lockfile_records_the_manifest_version` goes RED. That is the #131
//!     defect reproduced exactly.
//!
//! WHAT EACH LEG PINS, AND WHO ACTUALLY ENFORCES IT — because the second half of
//! that measurement is a disclosure, not a footnote. Under a bare `cargo test`,
//! cargo REBUILDS from the manifest on disk and REPAIRS a stale `Cargo.lock`
//! before any test body runs, so three of these four cannot go RED under the
//! command CI uses. They are kept, and this is what each is for:
//!
//!   * `the_compiled_version_is_the_manifest_version` and
//!     `the_built_binary_reports_the_manifest_version` pin *manifest text ==
//!     the artefact's own answer*. Cargo makes them agree by construction on the
//!     spot, so they discriminate only against an OUT-OF-BAND binary — the
//!     already-built test binary or an installed `forjar` run against a bumped
//!     manifest, which is exactly the "verify the effective artefact" case this
//!     fleet keeps meeting.
//!   * `the_lockfile_records_the_manifest_version` pins the #131 invariant, and
//!     the ENFORCER is cargo itself under **`--locked`**: with that flag cargo
//!     refuses at resolution and the tests never start, which is a build error
//!     rather than a review-able message. This test is the message-bearing
//!     witness for the same invariant — it names the defect, the cost and the
//!     fix — and it is the leg that goes red when the binary is run directly.
//!     Every invocation of this target is `--locked` for that reason:
//!     `ci.yml`'s `examples-validate` step says so in its own comment.
//!   * `the_changelog_has_an_entry_for_this_version` is the only leg that
//!     discriminates under a plain `cargo test`. Nothing in the toolchain
//!     writes `CHANGELOG.md`, so nothing can repair it underneath the
//!     assertion.
//!
//! Saying which is which is the point: a suite that lets a reader believe four
//! independent checks are running when one is would be the "reported a result it
//! did not measure" shape these files exist to refuse.

use std::path::{Path, PathBuf};
use std::process::Command;

/// A file in the crate root, read as bytes-to-text. `CARGO_MANIFEST_DIR` is the
/// `forjar` package root, which is also the workspace root and therefore where
/// `Cargo.lock` and `CHANGELOG.md` live.
fn repo_file(rel: &str) -> String {
    let p: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

/// The version as written in `[package]` of `Cargo.toml` — the one thing a
/// release bump edits by hand, and the oracle for every other leg here.
fn manifest_version() -> String {
    let manifest = repo_file("Cargo.toml");
    let mut in_package = false;
    for line in manifest.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            in_package = t == "[package]";
            continue;
        }
        if in_package {
            if let Some(rest) = t.strip_prefix("version") {
                let rest = rest.trim_start();
                if let Some(rest) = rest.strip_prefix('=') {
                    return rest.trim().trim_matches('"').to_string();
                }
            }
        }
    }
    panic!("Cargo.toml has no `version = \"...\"` under [package]");
}

/// The `version` of the `[[package]]` block whose `name` is exactly `forjar`.
/// `forjar-contracts` and `forjar-contracts-macros` are versioned separately
/// and must not be mistaken for it.
fn lockfile_version() -> String {
    let lock = repo_file("Cargo.lock");
    for block in lock.split("[[package]]") {
        let mut name = None;
        let mut version = None;
        for line in block.lines() {
            let t = line.trim();
            if let Some(v) = t.strip_prefix("name = ") {
                name = Some(v.trim_matches('"').to_string());
            } else if let Some(v) = t.strip_prefix("version = ") {
                version = Some(v.trim_matches('"').to_string());
            }
        }
        if name.as_deref() == Some("forjar") {
            return version.expect("the forjar [[package]] block carries no version");
        }
    }
    panic!("Cargo.lock has no `[[package]]` block named forjar");
}

/// The manifest text and the constant this test was compiled with agree.
///
/// This is the anchor: it says the artefact under test was built from the
/// manifest that is on disk right now. A binary compiled before the bump
/// reports the old version while the manifest reads the new one, which is the
/// "verify the effective artefact, not the declaration" failure this suite
/// exists to catch.
#[test]
fn the_compiled_version_is_the_manifest_version() {
    let manifest = manifest_version();
    assert_eq!(
        manifest,
        env!("CARGO_PKG_VERSION"),
        "Cargo.toml says {manifest} but this test was compiled against {} — \
         the build is stale; rebuild before trusting any other check here",
        env!("CARGO_PKG_VERSION")
    );
}

/// `forjar --version` prints exactly `forjar <manifest version>`.
///
/// The assertion is on the WHOLE rendering, not on a substring: a
/// `stdout.contains("forjar")` check (as in `tests/integration_smoke.rs`) is
/// satisfied by the binary's own name and would pass on any version at all.
#[test]
fn the_built_binary_reports_the_manifest_version() {
    let out = Command::new(env!("CARGO_BIN_EXE_forjar"))
        .arg("--version")
        .output()
        .expect("run forjar --version");
    assert!(
        out.status.success(),
        "forjar --version exited {:?}: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    let printed = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let expected = format!("forjar {}", manifest_version());
    assert_eq!(
        printed, expected,
        "the shipped binary does not describe this release: `forjar --version` \
         printed {printed:?}, the manifest says {expected:?}"
    );
}

/// `Cargo.lock`'s own `forjar` entry carries the manifest version.
///
/// The #131 defect: a bump committed without its lockfile hunk. `cargo` repairs
/// the lock silently on the next local build, so the divergence is invisible
/// until a consumer resolves with `--locked` — `release.yml`'s
/// `cargo package`, `ci.yml`'s `lockfile-preflight`, or a `cargo install
/// --locked` by a user.
#[test]
fn the_lockfile_records_the_manifest_version() {
    let manifest = manifest_version();
    let locked = lockfile_version();
    assert_eq!(
        locked, manifest,
        "Cargo.lock pins forjar {locked} while Cargo.toml says {manifest}. \
         Commit the lockfile hunk with the bump (#131): \
         `cargo build -p forjar` then `git add Cargo.lock`."
    );
}

/// `CHANGELOG.md` carries a dated heading for the version being released.
///
/// The file's shape is Keep a Changelog: `## [1.25.1] — 2026-09-04`. The date
/// is asserted too, because `## [Unreleased]` is a heading that describes no
/// release, and an undated one is the same thing with a number on it.
#[test]
fn the_changelog_has_an_entry_for_this_version() {
    let version = manifest_version();
    let changelog = repo_file("CHANGELOG.md");
    let head = format!("## [{version}]");
    let line = changelog
        .lines()
        .find(|l| l.starts_with(&head))
        .unwrap_or_else(|| {
            panic!(
                "CHANGELOG.md has no `{head}` heading — this release is undocumented. \
                 Existing headings: {}",
                changelog
                    .lines()
                    .filter(|l| l.starts_with("## ["))
                    .take(4)
                    .collect::<Vec<_>>()
                    .join(" | ")
            )
        });
    let tail = line[head.len()..]
        .trim_start()
        .trim_start_matches(['-', '\u{2014}'])
        .trim_start();
    let date: String = tail.chars().take(10).collect();
    let iso = date.len() == 10
        && date.chars().enumerate().all(|(i, c)| {
            if i == 4 || i == 7 {
                c == '-'
            } else {
                c.is_ascii_digit()
            }
        });
    assert!(
        iso,
        "the `{head}` heading carries no ISO date: {line:?} \
         (expected e.g. `{head} \u{2014} 2026-09-05`)"
    );
}
