//! PMAT-178 / PMAT-180: what `scripts/dogfood/release-check.sh` counts as the
//! set of PRs in a release, and what it counts as the tag.
//!
//! # The two defects these pin
//!
//! **The PR window (PMAT-178).** Arm 5 enumerated the release's PRs with
//! `git log --merges`. This repository squash-merges, and a squash merge leaves
//! one ordinary commit with no second parent — so on a squash-merged release
//! the enumeration returned NOTHING, the receipt loop under it ran zero times,
//! and the arm reported PASS over an empty set. That is the exact vacuity the
//! whole dogfood contract exists to refuse: a loop over an empty directory is
//! indistinguishable, in CI output, from a check that ran.
//!
//! **The tag (PMAT-180).** Arms 1–4 decided PENDING from the LOCAL tag list.
//! `git tag` lists what this checkout happens to have fetched, so a version
//! that was tagged and pushed but never released read as "not cut yet" —
//! PENDING — which is the one state that must never cover a broken release.
//! The remote is the authority: PENDING is available only when origin has no
//! such tag.
//!
//! # Why these run the real script
//!
//! The gate's subject is a shell script, and a test that re-implements its
//! logic in Rust would pass over a script that no longer exists. So each case
//! builds a small repository shaped the way the script reads one — a
//! `Cargo.toml` version, two tags' worth of history, an `origin` — copies the
//! REAL `scripts/dogfood/release-check.sh` into it, and runs it. `gh` is
//! injected through `$GH`, which is how the script names the tool it requires:
//! `GH=false` is "gh cannot answer", and a two-line stub is "gh answers this".
//! Nothing here touches the network or the real repository.

#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The fixture's own version, and therefore the tag the script looks for.
const VERSION: &str = "0.0.2";
const TAG: &str = "v0.0.2";
/// The tag the script must find as the previous one, bounding the PR window.
const PREV_TAG: &str = "v0.0.1";
/// The PR number the stubbed `gh` reports as merged into this release.
const PR: u32 = 77;

fn git(cwd: &Path, args: &[&str]) -> Output {
    let out = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("GIT_AUTHOR_NAME", "tester")
        .env("GIT_AUTHOR_EMAIL", "tester@example.com")
        .env("GIT_COMMITTER_NAME", "tester")
        .env("GIT_COMMITTER_EMAIL", "tester@example.com")
        .output()
        .expect("git must run");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

fn stdout_of(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn write(root: &Path, rel: &str, body: &str) {
    let p = root.join(rel);
    std::fs::create_dir_all(p.parent().expect("a parent")).expect("mkdir");
    std::fs::write(p, body).expect("write");
}

/// A stub `gh` that answers every invocation with `json`, so that "GitHub said
/// this" can be varied without varying anything else.
fn stub_gh(dir: &Path, name: &str, json: &str) -> String {
    let p = dir.join(name);
    std::fs::write(
        &p,
        format!("#!/usr/bin/env bash\ncat <<'FIXTURE_JSON'\n{json}\nFIXTURE_JSON\n"),
    )
    .expect("write stub");
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    p.to_string_lossy().into_owned()
}

struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
    /// The commit that landed after `PREV_TAG` — the merge commit the stubbed
    /// `gh` claims for PR #77, and an ancestor of HEAD.
    head: String,
}

impl Fixture {
    /// `gh` answering with one merged PR whose merge commit is in this release.
    fn gh_reporting_the_pr(&self) -> String {
        stub_gh(
            self.root.parent().expect("tempdir"),
            "gh-one-pr",
            &format!(
                r#"[{{"number":{PR},"mergedAt":"2026-09-05T00:00:00Z","mergeCommit":{{"oid":"{}"}}}}]"#,
                self.head
            ),
        )
    }

    fn with_receipt_for(&self, pr: u32) {
        write(
            &self.root,
            &format!("docs/audits/quorum-{pr}.md"),
            "# quorum receipt (fixture)\n",
        );
    }
}

struct Run {
    code: i32,
    text: String,
}

impl Run {
    fn assert_not_green(&self, why: &str) {
        assert_ne!(self.code, 0, "{why}; the gate exited 0:\n{}", self.text);
        assert!(
            !self.text.contains("GATE R PASS"),
            "{why}; the gate printed a PASS line:\n{}",
            self.text
        );
        assert!(
            self.text.contains("GATE R FAIL"),
            "{why}; the gate exited {} without a GATE R FAIL line, which is a \
             death rather than a verdict:\n{}",
            self.code,
            self.text
        );
    }

    fn assert_says(&self, needle: &str) {
        assert!(
            self.text.contains(needle),
            "the verdict does not name {needle:?}, so a reader cannot act on it:\n{}",
            self.text
        );
    }
}

fn run(fx: &Fixture, gh: &str) -> Run {
    let out = Command::new("bash")
        .arg(fx.root.join("scripts/dogfood/release-check.sh"))
        .current_dir(&fx.root)
        .env("GH", gh)
        .output()
        .expect("bash must run");
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    Run {
        code: out.status.code().unwrap_or(-1),
        text,
    }
}

/// A repository shaped the way the gate reads one: a `Cargo.toml` at
/// [`VERSION`], a previous tag, a commit that landed after it, the crux
/// document arm 6 wants, and an `origin` that is a bare clone.
///
/// `tag_on_origin` puts [`TAG`] on origin and NOT in the checkout, which is the
/// state PMAT-180 is about: a released version this clone has not fetched.
fn fixture(tag_on_origin: bool) -> Fixture {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("repo");
    std::fs::create_dir_all(&root).expect("mkdir repo");

    // The ambient pmat pre-commit hook must not run here: it formats and
    // analyses a crate that does not exist in this fixture, and its failure
    // would be reported as this test's.
    let nohooks = dir.path().join("nohooks");
    std::fs::create_dir_all(&nohooks).expect("mkdir nohooks");

    git(&root, &["init", "-q", "-b", "main"]);
    git(
        &root,
        &["config", "core.hooksPath", &nohooks.to_string_lossy()],
    );
    git(&root, &["config", "user.email", "tester@example.com"]);
    git(&root, &["config", "user.name", "tester"]);
    git(&root, &["config", "commit.gpgsign", "false"]);

    let script = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts/dogfood/release-check.sh"),
    )
    .expect("the gate under test must exist");
    write(&root, "scripts/dogfood/release-check.sh", &script);
    write(
        &root,
        "Cargo.toml",
        &format!("[package]\nname = \"fixture\"\nversion = \"{VERSION}\"\n"),
    );
    write(
        &root,
        &format!("docs/audits/crux-{VERSION}.md"),
        "# crux (fixture)\n",
    );
    git(
        &root,
        &[
            "add",
            "scripts/dogfood/release-check.sh",
            "Cargo.toml",
            &format!("docs/audits/crux-{VERSION}.md"),
        ],
    );
    git(&root, &["commit", "-qm", "the previous release"]);
    git(&root, &["tag", PREV_TAG]);

    write(&root, "shipped.txt", "work that reached this release\n");
    git(&root, &["add", "shipped.txt"]);
    git(&root, &["commit", "-qm", "squash-merged work (#77)"]);
    let head = stdout_of(&git(&root, &["rev-parse", "HEAD"]));

    if tag_on_origin {
        git(&root, &["tag", TAG]);
    }
    let origin = dir.path().join("origin.git");
    git(
        dir.path(),
        &[
            "clone",
            "-q",
            "--bare",
            &root.to_string_lossy(),
            &origin.to_string_lossy(),
        ],
    );
    git(
        &root,
        &["remote", "add", "origin", &origin.to_string_lossy()],
    );
    if tag_on_origin {
        // Cloned to origin, then dropped here: the tag exists where it is
        // served from and not where the gate runs.
        git(&root, &["tag", "-d", TAG]);
    }

    Fixture {
        _dir: dir,
        root,
        head,
    }
}

#[test]
fn a_gh_that_cannot_answer_is_a_fail_and_never_a_pass() {
    let fx = fixture(false);
    fx.with_receipt_for(PR);
    // `false` is a tool that runs and refuses to answer — the shape of a
    // missing, unauthenticated or rate-limited gh.
    let r = run(&fx, "false");
    r.assert_not_green(
        "gh could not enumerate the release's PRs, so the window is UNMEASURED \
         and UNMEASURED is a failure",
    );
    r.assert_says("pr list");
}

#[test]
fn an_empty_pr_set_over_commits_that_landed_is_red() {
    let fx = fixture(false);
    // GitHub reports no merged PR in the window while a commit sits between
    // the previous tag and HEAD: either the enumeration is broken (the
    // `git log --merges` defect) or work reached the release without review.
    let gh = stub_gh(fx.root.parent().expect("tempdir"), "gh-empty", "[]");
    let r = run(&fx, &gh);
    r.assert_not_green(
        "an empty PR set over a non-empty commit range is the vacuity this arm \
         used to pass on",
    );
    r.assert_says(PREV_TAG);
}

#[test]
fn a_pr_with_its_receipt_passes_before_the_tag_is_cut() {
    let fx = fixture(false);
    fx.with_receipt_for(PR);
    let r = run(&fx, &fx.gh_reporting_the_pr());
    assert_eq!(
        r.code, 0,
        "a release whose one PR has its receipt must pass:\n{}",
        r.text
    );
    // The anti-vacuity half of the case above: the arm CAN pass, so its
    // failures are about the PR set and not about the fixture. The count is
    // asserted because "0 PR(s) all have receipts" is the vacuous pass this
    // whole file exists to refuse — a green line here must say ONE.
    assert!(r.text.contains("GATE R PASS"), "no PASS line:\n{}", r.text);
    r.assert_says("1 PR(s)");
    r.assert_says("PENDING");
}

#[test]
fn a_pr_without_its_receipt_is_named_and_red() {
    let fx = fixture(false);
    let r = run(&fx, &fx.gh_reporting_the_pr());
    r.assert_not_green("PR #77 is in this release with no quorum receipt");
    r.assert_says(&format!("docs/audits/quorum-{PR}.md"));
}

#[test]
fn a_tag_that_exists_on_the_remote_is_never_pending() {
    let fx = fixture(true);
    fx.with_receipt_for(PR);
    let r = run(&fx, &fx.gh_reporting_the_pr());
    r.assert_not_green(
        "origin serves the release tag, so the release has happened and no arm \
         about it may be reported as not-yet-happened",
    );
    r.assert_says(TAG);
    assert!(
        !r.text.contains("PENDING"),
        "the gate called a released version PENDING:\n{}",
        r.text
    );
}
