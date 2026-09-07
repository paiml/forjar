//! PMAT-201: gates A (a harness receipt per merged PR) and E (a quorum receipt
//! per merged PR) are MECHANICAL, and these are the falsifiers that say so.
//!
//! # Why this file exists
//!
//! The dogfood contract promises eight standing requirements enforced "from
//! Make, with no agent". Six of them were shell; A and E were prose in
//! `.claude/skills/forjar-dogfood/SKILL.md`, run by an agent that read GitHub
//! and decided. An agent that decides is not a gate: it cannot be shown to go
//! red, it does not run in `make dogfood-release`, and its verdict is a
//! paragraph rather than an exit code. The PMAT-163 merge review refuted the
//! branch on exactly that.
//!
//! So A and E are now `scripts/dogfood/harness.sh` and
//! `scripts/dogfood/quorum.sh`, sharing the release window in
//! `scripts/dogfood/lib/window.sh`.
//!
//! # What a green run here proves
//!
//! That both scripts exist, and for each of them:
//!
//! 1. A `gh` that cannot answer is a FAIL carrying the word UNMEASURED — never
//!    a PASS over "no PRs found". The set of PRs merged since the last tag is a
//!    fact only GitHub holds, and an unenumerable window is not an empty one.
//! 2. A page that fills `--limit` is UNMEASURED, because a truncated window
//!    would let the gate check a subset while reporting the whole.
//! 3. Each defect the gate exists to catch — a merged PR with no ticket id, a
//!    missing harness receipt, a receipt that was truncated (no END marker),
//!    a receipt with two verdicts, a missing quorum receipt, a quorum below the
//!    lane floor, a quorum with a waiver key — produces a non-zero exit AND a
//!    `GATE <letter> FAIL` line naming the offender. An exit code with no
//!    verdict line is a death, not a judgement, and is asserted against.
//! 4. Both gates CAN pass. Without that half, every assertion above would be
//!    satisfied by a script that only ever fails.
//!
//! # Why these run the real scripts
//!
//! The subject is a shell script; a Rust re-implementation of its logic would
//! stay green over a script that no longer exists. Each case builds a small
//! repository shaped the way the gates read one — a `v*` tag, a commit after it
//! standing in for the squash merge, the receipts committed in that commit —
//! copies the REAL scripts into it, and runs them. `gh` is injected through
//! `$GH`, exactly as `scripts/dogfood/release-check.sh` names the tool it
//! requires: `GH=false` is "gh cannot answer", a stub is "gh answers this".
//! Nothing here touches the network or the real repository.

#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The tag that bounds the window from below.
const PREV_TAG: &str = "v0.0.1";
/// The PR the stubbed `gh` reports as merged into this window.
const PR: u32 = 77;
/// The PR's head branch. It carries the ticket id (so gate A's first source —
/// the branch name — is exercised) and a `/` (so the slug's `/` -> `-` rewrite
/// that names the quorum receipt is exercised too).
const HEAD_REF: &str = "PMAT-999/harness-and-quorum";
/// The ticket id gate A must read out of [`HEAD_REF`].
const TICKET: &str = "PMAT-999";
/// The `--limit` both gates ask GitHub for, mirrored from the scripts.
const PR_PAGE_LIMIT: usize = 200;

fn slug() -> String {
    HEAD_REF.replace('/', "-")
}

fn impl_receipt_path() -> String {
    format!("docs/audits/impl-{TICKET}-receipt.md")
}

fn quorum_receipt_path() -> String {
    format!(".quorum/{}.json", slug())
}

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

/// A stub `gh` that answers every invocation with `json`, so "GitHub said this"
/// can be varied without varying anything else.
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

/// A harness receipt shaped the way `paiml-implement` writes one: one
/// `Verdict:` line, and the END marker as the very last line.
fn good_impl_receipt() -> String {
    format!("# impl {TICKET}\n\nVerdict: DONE\n\nEvidence: the suite is green.\n\nIMPL-{TICKET}-RECEIPT-END\n")
}

/// The same receipt with its last line lost — the shape a truncated write
/// leaves behind, which is exactly what the END marker exists to expose.
fn truncated_impl_receipt() -> String {
    format!("# impl {TICKET}\n\nVerdict: DONE\n\nEvidence: the suite is gr")
}

/// Two verdicts is no verdict: a reader cannot tell which one the run reached.
fn two_verdict_impl_receipt() -> String {
    format!(
        "# impl {TICKET}\n\nVerdict: DONE\n\nverdict: NOT-MEASURED\n\nIMPL-{TICKET}-RECEIPT-END\n"
    )
}

/// A quorum receipt at the floor the gate enforces: 3 lanes, 3 judges, 3
/// refuters per claim, at least one claim actually refuted, and evidence.
fn good_quorum_receipt() -> &'static str {
    r#"{"quorum": {"lanes": ["a", "b", "c"], "judges": 3, "refuters_per_claim": 3,
        "claims_confirmed": 12, "claims_refuted": 2},
        "evidence": {"files": ["docs/audits/claims.md"]}}"#
}

/// Below the lane floor: 2 lanes, not 3.
fn thin_quorum_receipt() -> &'static str {
    r#"{"quorum": {"lanes": ["a", "b"], "judges": 3, "refuters_per_claim": 3,
        "claims_confirmed": 12, "claims_refuted": 2},
        "evidence": {"files": ["docs/audits/claims.md"]}}"#
}

/// A quorum that was waived rather than run. The key is nested, because a
/// check that only reads the top level is a check that is trivially evaded.
fn waived_quorum_receipt() -> &'static str {
    r#"{"quorum": {"lanes": ["a", "b", "c"], "judges": 3, "refuters_per_claim": 3,
        "claims_confirmed": 12, "claims_refuted": 2,
        "waiver": {"reason": "no credits left"}},
        "evidence": {"files": ["docs/audits/claims.md"]}}"#
}

struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
    /// The commit that landed after [`PREV_TAG`] — the squash merge the stubbed
    /// `gh` claims for PR #77, and an ancestor of HEAD.
    head: String,
}

impl Fixture {
    /// `gh` answering with the one merged PR, on [`HEAD_REF`].
    fn gh_reporting_the_pr(&self) -> String {
        self.gh_reporting(HEAD_REF, "a title with no id", "a body with no id")
    }

    /// `gh` answering with one merged PR whose branch, title and body are the
    /// caller's — the three places gate A looks for a ticket id, in order.
    fn gh_reporting(&self, head_ref: &str, title: &str, body: &str) -> String {
        stub_gh(
            self.root.parent().expect("tempdir"),
            "gh-one-pr",
            &format!(
                r#"[{{"number":{PR},"mergedAt":"2026-09-05T00:00:00Z","mergeCommit":{{"oid":"{}"}},"headRefName":"{head_ref}","title":"{title}","body":"{body}"}}]"#,
                self.head
            ),
        )
    }

    /// `gh` answering with exactly `--limit` PRs: the page is full, so the
    /// window may be truncated and neither gate can know the set it checks.
    fn gh_filling_the_page(&self) -> String {
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

struct Run {
    code: i32,
    text: String,
}

impl Run {
    fn assert_not_green(&self, letter: &str, why: &str) {
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

    fn assert_green(&self, letter: &str, why: &str) {
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

    fn assert_says(&self, needle: &str) {
        assert!(
            self.text.contains(needle),
            "the verdict does not name {needle:?}, so a reader cannot act on it:\n{}",
            self.text
        );
    }
}

fn run(fx: &Fixture, script: &str, gh: &str) -> Run {
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
fn fixture(impl_receipt: Option<&str>, quorum_receipt: Option<&str>) -> Fixture {
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
    ] {
        let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel);
        let body = std::fs::read_to_string(&src)
            .unwrap_or_else(|e| panic!("the gate under test must exist at {}: {e}", src.display()));
        write(&root, rel, &body);
    }
    write(&root, "seed.txt", "the release before this window\n");
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

    Fixture {
        _dir: dir,
        root,
        head,
    }
}

// ------------------------------------------------------------------ gate A

#[test]
fn gate_a_a_gh_that_cannot_answer_is_unmeasured_and_red() {
    let fx = fixture(Some(&good_impl_receipt()), Some(good_quorum_receipt()));
    // `false` is a tool that runs and refuses to answer — the shape of a
    // missing, unauthenticated or rate-limited gh.
    let r = run(&fx, "harness.sh", "false");
    r.assert_not_green(
        "A",
        "gh could not enumerate the merged PRs, so the window is UNMEASURED and \
         UNMEASURED is a failure, never a pass over an empty set",
    );
    r.assert_says("UNMEASURED");
}

#[test]
fn gate_a_a_full_page_is_unmeasured_and_red() {
    let fx = fixture(Some(&good_impl_receipt()), Some(good_quorum_receipt()));
    let gh = fx.gh_filling_the_page();
    let r = run(&fx, "harness.sh", &gh);
    r.assert_not_green(
        "A",
        "GitHub filled the --limit, so the window may be truncated and the gate \
         would check a subset while reporting the whole",
    );
    r.assert_says("UNMEASURED");
}

#[test]
fn gate_a_a_merged_pr_with_no_ticket_id_is_named_and_red() {
    let fx = fixture(Some(&good_impl_receipt()), Some(good_quorum_receipt()));
    let gh = fx.gh_reporting("fix/no-ticket-anywhere", "a title", "a body");
    let r = run(&fx, "harness.sh", &gh);
    r.assert_not_green(
        "A",
        "a merged PR that names no ticket in its branch, title or body has no \
         address at which a harness receipt could be looked for",
    );
    r.assert_says(&format!("#{PR}"));
}

#[test]
fn gate_a_a_missing_harness_receipt_is_named_and_red() {
    let fx = fixture(None, Some(good_quorum_receipt()));
    let r = run(&fx, "harness.sh", &fx.gh_reporting_the_pr());
    r.assert_not_green(
        "A",
        "PR #77 is in this window with no docs/audits/impl-PMAT-999-receipt.md, \
         so nothing says it was implemented under the harness",
    );
    r.assert_says(&impl_receipt_path());
}

#[test]
fn gate_a_a_receipt_without_the_end_marker_is_red() {
    let fx = fixture(Some(&truncated_impl_receipt()), Some(good_quorum_receipt()));
    let r = run(&fx, "harness.sh", &fx.gh_reporting_the_pr());
    r.assert_not_green(
        "A",
        "the END marker is what makes truncation detectable rather than silent; \
         a receipt without it may be missing the half that mattered",
    );
    r.assert_says(&format!("IMPL-{TICKET}-RECEIPT-END"));
}

#[test]
fn gate_a_a_receipt_with_two_verdicts_is_red() {
    let fx = fixture(
        Some(&two_verdict_impl_receipt()),
        Some(good_quorum_receipt()),
    );
    let r = run(&fx, "harness.sh", &fx.gh_reporting_the_pr());
    r.assert_not_green(
        "A",
        "two verdict lines is no verdict: a reader cannot tell which one the \
         run reached",
    );
    r.assert_says("verdict");
}

#[test]
fn gate_a_a_pr_with_its_harness_receipt_passes() {
    let fx = fixture(Some(&good_impl_receipt()), Some(good_quorum_receipt()));
    let r = run(&fx, "harness.sh", &fx.gh_reporting_the_pr());
    r.assert_green(
        "A",
        "a window whose one PR carries a well-formed harness receipt must pass, \
         or every failure above proves nothing about the gate",
    );
    // The count is asserted because "0 of 0 PRs carry a receipt" is the vacuous
    // pass this whole file refuses: a green line here must say ONE.
    r.assert_says("1 of 1");
    r.assert_says(&format!("GATE A #{PR} {TICKET} {}", impl_receipt_path()));
}

// ------------------------------------------------------------------ gate E

#[test]
fn gate_e_a_gh_that_cannot_answer_is_unmeasured_and_red() {
    let fx = fixture(Some(&good_impl_receipt()), Some(good_quorum_receipt()));
    let r = run(&fx, "quorum.sh", "false");
    r.assert_not_green(
        "E",
        "gh could not enumerate the merged PRs, so the window is UNMEASURED and \
         UNMEASURED is a failure",
    );
    r.assert_says("UNMEASURED");
}

#[test]
fn gate_e_a_missing_quorum_receipt_is_named_and_red() {
    let fx = fixture(Some(&good_impl_receipt()), None);
    let r = run(&fx, "quorum.sh", &fx.gh_reporting_the_pr());
    r.assert_not_green(
        "E",
        "PR #77 merged with no committed .quorum/<slug>.json, so its claims were \
         never refuted by anything this gate can read",
    );
    r.assert_says(&quorum_receipt_path());
}

#[test]
fn gate_e_a_thin_quorum_is_red() {
    let fx = fixture(Some(&good_impl_receipt()), Some(thin_quorum_receipt()));
    let r = run(&fx, "quorum.sh", &fx.gh_reporting_the_pr());
    r.assert_not_green(
        "E",
        "2 lanes is below the floor of 3 that scripts/quorum-gate.sh enforces at \
         push time, so the receipt never cleared the bar it claims to record",
    );
    r.assert_says("lanes");
}

#[test]
fn gate_e_a_waiver_anywhere_in_the_receipt_is_red() {
    let fx = fixture(Some(&good_impl_receipt()), Some(waived_quorum_receipt()));
    let r = run(&fx, "quorum.sh", &fx.gh_reporting_the_pr());
    r.assert_not_green(
        "E",
        "a waiver is an unrefuted claim wearing a receipt's clothes, and nesting \
         it one level deeper must not buy it a pass",
    );
    r.assert_says("waiv");
}

#[test]
fn gate_e_a_pr_with_its_quorum_receipt_passes() {
    let fx = fixture(Some(&good_impl_receipt()), Some(good_quorum_receipt()));
    let r = run(&fx, "quorum.sh", &fx.gh_reporting_the_pr());
    r.assert_green(
        "E",
        "a window whose one PR carries a receipt at the floor must pass, or \
         every failure above proves nothing about the gate",
    );
    r.assert_says("1 of 1");
    r.assert_says(&format!("GATE E #{PR} {}", quorum_receipt_path()));
}
