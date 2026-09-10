//! Shared fixture for the gate A / gate E falsification tests
//! (`tests/falsification_dogfood_harness_and_quorum.rs`): a temp git repo with
//! a previous tag and one squash-merged PR, a stubbed `gh`, and receipts of
//! every shape the gates must accept or refuse. Kept out of the test file so
//! the file-size gate holds; nothing here asserts anything.
#![allow(dead_code)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The tag that bounds the window from below.
pub(crate) const PREV_TAG: &str = "v0.0.1";
/// The PR the stubbed `gh` reports as merged into this window.
pub(crate) const PR: u32 = 77;
/// The PR's head branch. It carries the ticket id (so gate A's first source —
/// the branch name — is exercised) and a `/` (so the slug's `/` -> `-` rewrite
/// that names the quorum receipt is exercised too).
pub(crate) const HEAD_REF: &str = "PMAT-999/harness-and-quorum";
/// The ticket id gate A must read out of [`HEAD_REF`].
pub(crate) const TICKET: &str = "PMAT-999";
/// The `--limit` both gates ask GitHub for, mirrored from the scripts.
pub(crate) const PR_PAGE_LIMIT: usize = 200;

pub(crate) fn slug() -> String {
    HEAD_REF.replace('/', "-")
}

pub(crate) fn impl_receipt_path() -> String {
    format!("docs/audits/impl-{TICKET}-receipt.md")
}

pub(crate) fn quorum_receipt_path() -> String {
    format!(".quorum/{}.json", slug())
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

/// A stub `gh` that answers every invocation with `json`, so "GitHub said this"
/// can be varied without varying anything else.
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

/// A harness receipt shaped the way `paiml-implement` writes one: one
/// `Verdict:` line, and the END marker as the very last line.
pub(crate) fn good_impl_receipt() -> String {
    format!("# impl {TICKET}\n\nVerdict: DONE\n\nEvidence: the suite is green.\n\nIMPL-{TICKET}-RECEIPT-END\n")
}

/// The same receipt with its last line lost — the shape a truncated write
/// leaves behind, which is exactly what the END marker exists to expose.
pub(crate) fn truncated_impl_receipt() -> String {
    format!("# impl {TICKET}\n\nVerdict: DONE\n\nEvidence: the suite is gr")
}

/// Two verdicts is no verdict: a reader cannot tell which one the run reached.
pub(crate) fn two_verdict_impl_receipt() -> String {
    format!(
        "# impl {TICKET}\n\nVerdict: DONE\n\nverdict: NOT-MEASURED\n\nIMPL-{TICKET}-RECEIPT-END\n"
    )
}

/// A quorum receipt at the floor the gate enforces: 3 lanes, 3 judges, 3
/// refuters per claim, at least one claim actually refuted, and evidence.
pub(crate) fn good_quorum_receipt() -> &'static str {
    r#"{"quorum": {"lanes": ["a", "b", "c"], "judges": 3, "refuters_per_claim": 3,
        "claims_confirmed": 12, "claims_refuted": 2},
        "evidence": {"files": ["docs/audits/claims.md"]}}"#
}

/// Below the lane floor: 2 lanes, not 3.
pub(crate) fn thin_quorum_receipt() -> &'static str {
    r#"{"quorum": {"lanes": ["a", "b"], "judges": 3, "refuters_per_claim": 3,
        "claims_confirmed": 12, "claims_refuted": 2},
        "evidence": {"files": ["docs/audits/claims.md"]}}"#
}

/// A quorum that was waived rather than run. The key is nested, because a
/// check that only reads the top level is a check that is trivially evaded.
pub(crate) fn waived_quorum_receipt() -> &'static str {
    r#"{"quorum": {"lanes": ["a", "b", "c"], "judges": 3, "refuters_per_claim": 3,
        "claims_confirmed": 12, "claims_refuted": 2,
        "waiver": {"reason": "no credits left"}},
        "evidence": {"files": ["docs/audits/claims.md"]}}"#
}

/// A registry in the shape `docs/roadmaps/roadmap.yaml` has: one `- id:`
/// block per row, each with a `labels:` list (empty ones are `labels: []`,
/// the way `pmat work add` writes them).
pub(crate) fn roadmap(rows: &[(&str, &[&str])]) -> String {
    let mut text = String::from("roadmap_version: '1.0'\nroadmap:\n");
    for (id, labels) in rows {
        text.push_str(&format!(
            "- id: {id}\n  title: fixture row\n  status: planned\n"
        ));
        if labels.is_empty() {
            text.push_str("  labels: []\n");
        } else {
            text.push_str("  labels:\n");
            for l in *labels {
                text.push_str(&format!("  - {l}\n"));
            }
        }
        text.push_str("  notes: null\n");
    }
    text
}

pub(crate) struct Fixture {
    pub(crate) _dir: tempfile::TempDir,
    pub(crate) root: PathBuf,
    /// The commit that landed after [`PREV_TAG`] — the squash merge the stubbed
    /// `gh` claims for PR #77, and an ancestor of HEAD.
    pub(crate) head: String,
    /// The commit [`PREV_TAG`] points at — the previous release's own squash
    /// merge, which GitHub also returns for `merged:>=<tag date>` because it
    /// landed at the very second the tag was cut.
    pub(crate) tagged: String,
}

impl Fixture {
    /// Declare, on a new row `owner` committed at HEAD with its own good
    /// harness receipt, that the id `stray` (which is no row) means `owner` —
    /// the `alias:<id>` label a misnamed branch resolves through.
    pub(crate) fn declare_alias(&self, stray: &str, owner: &str) {
        self.declare_alias_on(stray, &[owner]);
    }

    /// The same declaration on EVERY row in `owners` — two owners is the
    /// shape the resolver must refuse: a declaration naming two owners names
    /// none.
    pub(crate) fn declare_alias_on(&self, stray: &str, owners: &[&str]) {
        let alias = format!("alias:{stray}");
        let labels: [&str; 1] = [alias.as_str()];
        let mut rows: Vec<(&str, &[&str])> = vec![(TICKET, &[])];
        for owner in owners {
            rows.push((owner, &labels));
        }
        write(&self.root, "docs/roadmaps/roadmap.yaml", &roadmap(&rows));
        for owner in owners {
            write(
                &self.root,
                &format!("docs/audits/impl-{owner}-receipt.md"),
                &good_impl_receipt().replace(TICKET, owner),
            );
        }
        git(&self.root, &["add", "-A"]);
        git(
            &self.root,
            &["commit", "-qm", "the owner declares the misnomer"],
        );
    }

    /// `gh` answering with two merged PRs: #76, the previous release's own PR
    /// whose merge commit IS the tagged commit, and #77, the work in this
    /// window. Only #77 is this window's.
    pub(crate) fn gh_reporting_the_pr_and_the_previous_release(&self) -> String {
        stub_gh(
            self.root.parent().expect("tempdir"),
            "gh-two-prs",
            &format!(
                r#"[{{"number":76,"mergedAt":"2026-09-04T00:00:00Z","mergeCommit":{{"oid":"{}"}},"headRefName":"release/v0.0.1","title":"release: v0.0.1","body":""}},{{"number":{PR},"mergedAt":"2026-09-05T00:00:00Z","mergeCommit":{{"oid":"{}"}},"headRefName":"{HEAD_REF}","title":"a title with no id","body":"a body with no id"}}]"#,
                self.tagged, self.head
            ),
        )
    }

    /// `gh` answering with the one merged PR, on [`HEAD_REF`].
    pub(crate) fn gh_reporting_the_pr(&self) -> String {
        self.gh_reporting(HEAD_REF, "a title with no id", "a body with no id")
    }

    /// `gh` answering with one merged PR whose branch, title and body are the
    /// caller's — the three places gate A looks for a ticket id, in order.
    pub(crate) fn gh_reporting(&self, head_ref: &str, title: &str, body: &str) -> String {
        stub_gh(
            self.root.parent().expect("tempdir"),
            "gh-one-pr",
            &format!(
                r#"[{{"number":{PR},"mergedAt":"2026-09-05T00:00:00Z","mergeCommit":{{"oid":"{}"}},"headRefName":"{head_ref}","title":"{title}","body":"{body}"}}]"#,
                self.head
            ),
        )
    }

    /// `gh` answering that nothing was merged — while the fixture's HEAD holds
    /// one commit after the tag, so the window is not empty, only unclaimed.
    pub(crate) fn gh_reporting_no_prs(&self) -> String {
        stub_gh(self.root.parent().expect("tempdir"), "gh-no-prs", "[]")
    }

    /// `gh` answering with exactly `--limit` PRs: the page is full, so the
    /// window may be truncated and neither gate can know the set it checks.
    pub(crate) fn gh_filling_the_page(&self) -> String {
        let rows: Vec<String> = (0..PR_PAGE_LIMIT)
            .map(|i| {
                format!(
                    r#"{{"number":{},"mergedAt":"2026-09-05T00:00:00Z","mergeCommit":{{"oid":"{}"}},"headRefName":"{HEAD_REF}","title":"t","body":"b"}}"#,
                    i + 1,
                    self.head
                )
            })
            .collect();
        stub_gh(
            self.root.parent().expect("tempdir"),
            "gh-full-page",
            &format!("[{}]", rows.join(",")),
        )
    }
}

pub(crate) struct Run {
    pub(crate) code: i32,
    pub(crate) text: String,
}

impl Run {
    pub(crate) fn assert_not_green(&self, letter: &str, why: &str) {
        assert_ne!(self.code, 0, "{why}; the gate exited 0:\n{}", self.text);
        assert!(
            !self.text.contains(&format!("GATE {letter} PASS")),
            "{why}; the gate printed a PASS line:\n{}",
            self.text
        );
        assert!(
            self.text.contains(&format!("GATE {letter} FAIL")),
            "{why}; the gate exited {} without a GATE {letter} FAIL line, which \
             is a death rather than a verdict:\n{}",
            self.code,
            self.text
        );
    }

    pub(crate) fn assert_green(&self, letter: &str, why: &str) {
        assert_eq!(
            self.code, 0,
            "{why}; the gate exited {}:\n{}",
            self.code, self.text
        );
        assert!(
            self.text.contains(&format!("GATE {letter} PASS")),
            "{why}; the gate exited 0 without a GATE {letter} PASS line:\n{}",
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

pub(crate) fn run(fx: &Fixture, script: &str, gh: &str) -> Run {
    let out = Command::new("bash")
        .arg(fx.root.join(format!("scripts/dogfood/{script}")))
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

/// A repository shaped the way both gates read one: a `v*` tag, then one
/// commit standing in for PR #77's squash merge, carrying whichever receipts
/// the case wants. Both gates read the receipts AT HEAD, so they are committed
/// here rather than merely written.
pub(crate) fn fixture(impl_receipt: Option<&str>, quorum_receipt: Option<&str>) -> Fixture {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("repo");
    std::fs::create_dir_all(&root).expect("mkdir repo");

    // The ambient pmat pre-commit hook must not run here: it analyses a crate
    // that does not exist in this fixture, and its failure would be reported as
    // this test's.
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
        "scripts/dogfood/harness.sh",
        "scripts/dogfood/quorum.sh",
        "scripts/dogfood/lib/window.sh",
        "scripts/dogfood/lib/receipt.sh",
    ] {
        let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
        let body = std::fs::read_to_string(&src)
            .unwrap_or_else(|e| panic!("the gate under test must exist at {}: {e}", src.display()));
        write(&root, rel, &body);
    }
    write(&root, "seed.txt", "the release before this window\n");
    // The ticket registry gate A resolves every PR's id against (PMAT-225):
    // the fixture ticket is a row, and nothing else is.
    write(
        &root,
        "docs/roadmaps/roadmap.yaml",
        &roadmap(&[(TICKET, &[])]),
    );
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-qm", "the previous release"]);
    git(&root, &["tag", PREV_TAG]);

    write(&root, "shipped.txt", "work that landed in this window\n");
    if let Some(body) = impl_receipt {
        write(&root, &impl_receipt_path(), body);
    }
    if let Some(body) = quorum_receipt {
        write(&root, &quorum_receipt_path(), body);
    }
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-qm", "squash-merged work (#77)"]);
    let head = stdout_of(&git(&root, &["rev-parse", "HEAD"]));
    let tagged = stdout_of(&git(
        &root,
        &["rev-parse", &format!("{PREV_TAG}^{{commit}}")],
    ));

    Fixture {
        _dir: dir,
        root,
        head,
        tagged,
    }
}
