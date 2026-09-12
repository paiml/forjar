//! Shared fixture for the gate R tests (`scripts/dogfood/release-check.sh`).
//!
//! A repository shaped the way the gate reads one, with a stubbed `gh`. Kept
//! out of the test files so the file-size gate holds; nothing here asserts
//! anything.
//!
//! The gate's subject is a shell script, and a test that re-implemented its
//! logic in Rust would pass over a script that no longer exists. So each case
//! builds the repository, copies the REAL script into it, and runs it. `gh` is
//! injected through `$GH`, which is how the script names the tool it requires:
//! `GH=false` is "gh cannot answer", and a stub is "gh answers this".
//! Nothing here touches the network or the real repository.

#![allow(dead_code)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The fixture's own version, and therefore the tag the script looks for.
pub(crate) const VERSION: &str = "0.0.2";
pub(crate) const TAG: &str = "v0.0.2";
/// The tag the script must find as the previous one, bounding the PR window.
pub(crate) const PREV_TAG: &str = "v0.0.1";
/// [`PREV_TAG`]'s version, which is where `Cargo.toml` sits after a finished
/// release: the cut bumps it, the tag is made, and nothing bumps it again
/// until the next cut starts.
pub(crate) const PREV_VERSION: &str = "0.0.1";
/// One tag further back, so the published fixture's window has a lower bound.
pub(crate) const PREV_PREV_TAG: &str = "v0.0.0";
/// The PR number the stubbed `gh` reports as merged into this release.
pub(crate) const PR: u32 = 77;
/// The PR's head branch, chosen with a `/` so the slug's `/` -> `-` rewrite
/// (the exact one `scripts/quorum-gate.sh` applies) is actually exercised.
pub(crate) const HEAD_REF: &str = "feat/pr-77";

pub(crate) fn slug() -> String {
    HEAD_REF.replace('/', "-")
}

pub(crate) fn git(cwd: &Path, args: &[&str]) -> Output {
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

pub(crate) fn stdout_of(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

pub(crate) fn write(root: &Path, rel: &str, body: &str) {
    let p = root.join(rel);
    std::fs::create_dir_all(p.parent().expect("a parent")).expect("mkdir");
    std::fs::write(p, body).expect("write");
}

/// A stub `gh` that answers every invocation with `json`, so that "GitHub said
/// this" can be varied without varying anything else.
pub(crate) fn stub_gh(dir: &Path, name: &str, json: &str) -> String {
    let p = dir.join(name);
    std::fs::write(
        &p,
        format!("#!/usr/bin/env bash\ncat <<'FIXTURE_JSON'\n{json}\nFIXTURE_JSON\n"),
    )
    .expect("write stub");
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    p.to_string_lossy().into_owned()
}

/// A receipt shaped like a real one — exactly the floor lib/receipt.sh reads:
/// 3 lanes, 3 judges, 3 refuters per claim, one refuted claim, one evidence file.
pub(crate) fn good_receipt() -> &'static str {
    r#"{"quorum": {"lanes": ["lane-a", "lane-b", "lane-c"], "judges": 3, "refuters_per_claim": 3, "claims_refuted": 1}, "evidence": {"files": [{"path": "x"}]}}"#
}

/// A receipt whose waiver hides under `quorum` as an `override` — invisible to
/// a top-level check, refused by the shared predicate.
pub(crate) fn nested_override_receipt() -> &'static str {
    r#"{"quorum": {"lanes": ["lane-a", "lane-b", "lane-c"], "judges": 3, "refuters_per_claim": 3, "claims_refuted": 1, "override": {"by": "author"}}, "evidence": {"files": [{"path": "x"}]}}"#
}

/// A receipt at every floor but one: no claim was refuted, so no one hunted.
pub(crate) fn unhunted_receipt() -> &'static str {
    r#"{"quorum": {"lanes": ["lane-a", "lane-b", "lane-c"], "judges": 3, "refuters_per_claim": 3, "claims_refuted": 0}, "evidence": {"files": [{"path": "x"}]}}"#
}

/// A receipt that waived the quorum instead of running one.
pub(crate) fn waived_receipt() -> &'static str {
    r#"{"quorum": {"lanes": ["lane-a", "lane-b", "lane-c"], "judges": 3, "refuters_per_claim": 3, "claims_refuted": 1}, "evidence": {"files": [{"path": "x"}]}, "waived": {"reason": "no credits"}}"#
}

/// A receipt below the lane floor — 2 lanes, not 3.
pub(crate) fn thin_receipt() -> &'static str {
    r#"{"quorum": {"lanes": ["lane-a", "lane-b"], "judges": 3, "refuters_per_claim": 3, "claims_refuted": 1}, "evidence": {"files": [{"path": "x"}]}}"#
}

pub(crate) struct Fixture {
    pub(crate) _dir: tempfile::TempDir,
    pub(crate) root: PathBuf,
    /// The commit that landed after `PREV_TAG` — the merge commit the stubbed
    /// `gh` claims for PR #77, and an ancestor of HEAD.
    pub(crate) head: String,
}

impl Fixture {
    /// `gh` answering with one merged PR whose merge commit is in this
    /// release, on the branch [`HEAD_REF`].
    pub(crate) fn gh_reporting_the_pr(&self) -> String {
        stub_gh(
            self.root.parent().expect("tempdir"),
            "gh-one-pr",
            &format!(
                r#"[{{"number":{PR},"mergedAt":"2026-09-05T00:00:00Z","mergeCommit":{{"oid":"{}"}},"headRefName":"{HEAD_REF}"}}]"#,
                self.head
            ),
        )
    }
}

pub(crate) struct Run {
    pub(crate) code: i32,
    pub(crate) text: String,
}

impl Run {
    pub(crate) fn assert_not_green(&self, why: &str) {
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

    pub(crate) fn assert_says(&self, needle: &str) {
        assert!(
            self.text.contains(needle),
            "the verdict does not name {needle:?}, so a reader cannot act on it:\n{}",
            self.text
        );
    }
}

pub(crate) fn run(fx: &Fixture, gh: &str) -> Run {
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
/// [`VERSION`], a previous tag, a commit that landed after it carrying PR
/// #77's own `.quorum/<slug>.json` (or none, per `receipt`), the crux
/// document and `CHANGELOG.md` Arm 6 wants once the version differs from the
/// last cut release (it does here: [`VERSION`] vs [`PREV_TAG`]), and an
/// `origin` that is a bare clone.
///
/// `tag_on_origin` puts [`TAG`] on origin and NOT in the checkout, which is
/// the state PMAT-180 is about: a released version this clone has not
/// fetched. `receipt`, if given, is committed alongside the squash-merged
/// work — exactly how a real squash merge lands a branch's own committed
/// receipt in the same tree.
pub(crate) fn fixture(tag_on_origin: bool, receipt: Option<&str>) -> Fixture {
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
    let receipt_lib = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts/dogfood/lib/receipt.sh"),
    )
    .expect(
        "scripts/dogfood/lib/receipt.sh must exist — Arm 5 sources the shared receipt predicate",
    );
    write(&root, "scripts/dogfood/lib/receipt.sh", &receipt_lib);
    let crux_script = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts/dogfood/crux-reconcile.sh"),
    )
    .expect("scripts/dogfood/crux-reconcile.sh must exist — Arm 6 calls it directly");
    write(&root, "scripts/dogfood/crux-reconcile.sh", &crux_script);
    write(
        &root,
        "Cargo.toml",
        &format!("[package]\nname = \"fixture\"\nversion = \"{VERSION}\"\n"),
    );
    // The row and bullet Arm 6 -> crux-reconcile.sh need once VERSION differs
    // from the previous tag's version (it does: 0.0.2 vs 0.0.1), naming 3
    // distinct surveyed systems so the row is not "thin".
    write(
        &root,
        "CHANGELOG.md",
        "## [Unreleased]\n\n**Fixture behaviour bullet.** Exercises the crux reconciliation for this fixture.\n",
    );
    write(
        &root,
        &format!("docs/audits/crux-{VERSION}.md"),
        "# crux (fixture)\n\n| Fixture behaviour bullet. | Ansible, Terraform, Nix | matches |\n",
    );
    git(
        &root,
        &[
            "add",
            "scripts/dogfood/release-check.sh",
            "scripts/dogfood/crux-reconcile.sh",
            "Cargo.toml",
            "CHANGELOG.md",
            &format!("docs/audits/crux-{VERSION}.md"),
        ],
    );
    git(&root, &["commit", "-qm", "the previous release"]);
    git(&root, &["tag", PREV_TAG]);

    write(&root, "shipped.txt", "work that reached this release\n");
    git(&root, &["add", "shipped.txt"]);
    if let Some(body) = receipt {
        write(&root, &format!(".quorum/{}.json", slug()), body);
        git(&root, &["add", &format!(".quorum/{}.json", slug())]);
    }
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

/// A repository in the state a SUCCESSFUL RELEASE leaves behind (PMAT-234).
///
/// Everything the pre-tag fixture builds, plus: the tag in BOTH the checkout
/// and origin, and `Cargo.toml` back at that tag's version — which is exactly
/// what `release-goal.sh cut` and the release PR leave on main. Arm 6 then
/// notes that no crux document is owed because no cut is in flight, and that
/// note is the one the flat `pending` string turned into the words `pre-tag`
/// and `PENDING until the tag is cut` about a release that had shipped.
///
/// Arms 3 and 4 shell out to `cargo search` and `curl` by name, so this puts a
/// stub of each FIRST on `PATH` — the same way the script resolves them. That
/// is the honest injection point: there is no environment variable for either,
/// and a test that skipped those arms would not be exercising the state this
/// case is about.
pub(crate) fn published_fixture() -> Fixture {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("repo");
    std::fs::create_dir_all(&root).expect("mkdir repo");
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

    for rel in [
        "scripts/dogfood/release-check.sh",
        "scripts/dogfood/lib/receipt.sh",
        "scripts/dogfood/crux-reconcile.sh",
    ] {
        let body = std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel))
            .unwrap_or_else(|e| panic!("the gate under test must exist at {rel}: {e}"));
        write(&root, rel, &body);
    }
    // PREV_TAG's release, then the work, then the tag this fixture is about.
    // Cargo.toml never leaves PREV_VERSION, because that is the state a
    // finished release leaves: the version equals the newest tag's.
    write(
        &root,
        "Cargo.toml",
        &format!("[package]\nname = \"fixture\"\nversion = \"{PREV_VERSION}\"\n"),
    );
    write(&root, "CHANGELOG.md", "## [Unreleased]\n\n");
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-qm", "the release before this one"]);
    git(&root, &["tag", PREV_PREV_TAG]);

    write(&root, "shipped.txt", "work that reached this release\n");
    write(&root, &format!(".quorum/{}.json", slug()), good_receipt());
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-qm", "squash-merged work (#77)"]);
    let head = stdout_of(&git(&root, &["rev-parse", "HEAD"]));
    git(&root, &["tag", PREV_TAG]);

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
    git(&root, &["fetch", "-q", "origin"]);

    Fixture {
        _dir: dir,
        root,
        head,
    }
}

/// A `gh` that answers BOTH questions the published state asks it: the release
/// object for `gh release view`, and the PR window for everything else. The
/// pre-tag fixture's stub answers every invocation with the PR array, which
/// `jq -r .isPrerelease` cannot index — a stub that answers the wrong question
/// is a fixture defect, not a gate finding.
pub(crate) fn stub_gh_published(dir: &Path, head: &str, draft: &str) -> String {
    stub_gh_published_as(dir, head, draft, "false", PREV_TAG)
}

/// The same stub with the two fields PMAT-534's arm reads under the caller's
/// control: whether the release is a prerelease, and what
/// `gh api repos/<r>/releases/latest --jq .tag_name` answers.
///
/// The `api` branch must come FIRST: `gh api …` and `gh release view …` are
/// told apart by `$1`, and a stub that answered the PR list for `api` would
/// make the gate read a JSON array where it wants a tag name.
pub(crate) fn stub_gh_published_as(
    dir: &Path,
    head: &str,
    draft: &str,
    prerelease: &str,
    latest: &str,
) -> String {
    let p = dir.join(format!("gh-published-{prerelease}-{latest}"));
    let body = format!(
        r#"{{"number":{PR},"mergedAt":"2026-09-05T00:00:00Z","mergeCommit":{{"oid":"{head}"}},"headRefName":"{HEAD_REF}"}}"#
    );
    let release =
        format!(r#"{{"tagName":"{PREV_TAG}","isPrerelease":{prerelease},"isDraft":{draft}}}"#);
    let script = format!(
        "#!/usr/bin/env bash\n         if [ \"${{1:-}}\" = api ]; then\n  printf '%s\\n' '{latest}'\n  exit 0\nfi\n         if [ \"${{1:-}}\" = release ]; then\n  printf '%s' '{release}'\n  exit 0\nfi\n         printf '%s' '[{body}]'\n"
    );
    std::fs::write(&p, script).expect("write stub");
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    p.to_string_lossy().into_owned()
}

/// A `gh` whose `api` call FAILS — the shape of a token without the scope, a
/// rate limit, or a repository with no published release at all.
pub(crate) fn stub_gh_published_api_broken(dir: &Path, head: &str) -> String {
    let p = dir.join("gh-published-api-broken");
    let body = format!(
        r#"{{"number":{PR},"mergedAt":"2026-09-05T00:00:00Z","mergeCommit":{{"oid":"{head}"}},"headRefName":"{HEAD_REF}"}}"#
    );
    let release = format!(r#"{{"tagName":"{PREV_TAG}","isPrerelease":false,"isDraft":false}}"#);
    let script = format!(
        "#!/usr/bin/env bash\n         if [ \"${{1:-}}\" = api ]; then\n  echo 'HTTP 404: Not Found' >&2\n  exit 1\nfi\n         if [ \"${{1:-}}\" = release ]; then\n  printf '%s' '{release}'\n  exit 0\nfi\n         printf '%s' '[{body}]'\n"
    );
    std::fs::write(&p, script).expect("write stub");
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    p.to_string_lossy().into_owned()
}

/// `cargo` and `curl` answering the way a published release makes them answer.
/// Returned as a directory to put FIRST on `PATH`.
pub(crate) fn stub_release_tools(dir: &Path, crate_version: &str, doc_status: &str) -> PathBuf {
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).expect("mkdir bin");
    // `cargo search` prints `name = "x.y.z"    # description`; the script takes
    // field 3 and strips the quotes.
    std::fs::write(
        bin.join("cargo"),
        format!("#!/usr/bin/env bash\nprintf 'forjar = \"{crate_version}\"    # fixture\\n'\n"),
    )
    .expect("write cargo stub");
    std::fs::write(
        bin.join("curl"),
        format!("#!/usr/bin/env bash\nprintf '{{\"doc_status\": {doc_status}}}'\n"),
    )
    .expect("write curl stub");
    for f in ["cargo", "curl"] {
        std::fs::set_permissions(bin.join(f), std::fs::Permissions::from_mode(0o755))
            .expect("chmod");
    }
    bin
}

/// Run the gate with `gh`, `cargo` and `curl` all answering.
pub(crate) fn run_published(fx: &Fixture, gh: &str, bin: &Path) -> Run {
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let out = Command::new("bash")
        .arg(fx.root.join("scripts/dogfood/release-check.sh"))
        .current_dir(&fx.root)
        .env("GH", gh)
        .env("PATH", path)
        .output()
        .expect("bash must run");
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    Run {
        code: out.status.code().unwrap_or(-1),
        text,
    }
}

impl Run {
    pub(crate) fn assert_green(&self, why: &str) {
        assert_eq!(
            self.code, 0,
            "{why}; the gate exited {}:\n{}",
            self.code, self.text
        );
        assert!(
            self.text.contains("GATE R PASS"),
            "{why}; exit 0 without a GATE R PASS line:\n{}",
            self.text
        );
    }

    pub(crate) fn assert_never_says(&self, needle: &str, why: &str) {
        assert!(
            !self.text.contains(needle),
            "{why}; the verdict says {needle:?}:\n{}",
            self.text
        );
    }
}
