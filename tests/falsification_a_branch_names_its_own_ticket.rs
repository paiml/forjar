//! PMAT-535: the branch name is load-bearing, and nothing checked it.
//!
//! # What it decides
//!
//! Three gates read one unvalidated string:
//!
//! | gate | resolves from the branch name |
//! |---|---|
//! | A | `docs/audits/impl-<ticket>-receipt.md`, the receipt path |
//! | E | `.quorum/<slug>.json`, this receipt's own filename |
//! | T | the RELEASE WINDOW a merged PR belongs to |
//!
//! # What went wrong
//!
//! PR #532 was pushed from `PMAT-520-book-v1.29.0` while every commit's
//! trailer, its title and its receipt said **PMAT-531**. Every window arm
//! therefore credited PMAT-520 — which had already SHIPPED in 1.29.0 — to the
//! v1.30.0 window, and PMAT-531, the ticket that actually owned the work, was
//! invisible to all of them.
//!
//! Nobody added that label by hand. Gate T demanded it, and gate T was doing
//! exactly what it was told.
//!
//! # The rule, and what it is not
//!
//! If the branch names a ticket, **some commit being pushed must claim it**.
//!
//! Not "every trailer equals the branch's id": a branch legitimately carries
//! commits for more than one ticket, which this repository does on every
//! release. What it must not do is name one that none of them claims.
//!
//! At push time, because after the push the name is in three gates' arithmetic
//! and a merge commit cannot be renamed.

#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const ACTOR: &str = "tester@example.com";

fn gate() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/quorum-gate.sh")
}

fn git(repo: &Path, args: &[&str]) -> Output {
    let out = Command::new("git")
        .args(args)
        .current_dir(repo)
        .env("GIT_AUTHOR_NAME", "tester")
        .env("GIT_AUTHOR_EMAIL", ACTOR)
        .env("GIT_COMMITTER_NAME", "tester")
        .env("GIT_COMMITTER_EMAIL", ACTOR)
        .output()
        .expect("git must run");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

fn write(repo: &Path, rel: &str, body: &str) {
    let p = repo.join(rel);
    std::fs::create_dir_all(p.parent().expect("a parent")).expect("mkdir");
    std::fs::write(p, body).expect("write");
}

/// A repository on `branch`, with one commit carrying each of `tickets`.
fn fixture(branch: &str, tickets: &[&str]) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = dir.path().to_path_buf();
    git(&repo, &["init", "-q", "-b", "main"]);
    git(&repo, &["config", "commit.gpgsign", "false"]);
    write(&repo, "README.md", "fixture\n");
    write(
        &repo,
        ".quorum/enforce.json",
        &format!("{{\"enforced_for\": [\"{ACTOR}\"]}}\n"),
    );
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-qm", "base"]);
    git(&repo, &["update-ref", "refs/remotes/origin/main", "HEAD"]);

    git(&repo, &["checkout", "-q", "-b", branch]);
    for (i, t) in tickets.iter().enumerate() {
        write(&repo, &format!("work-{i}.txt"), "work\n");
        git(&repo, &["add", "-A"]);
        // The trailer block is one paragraph at the END, which is the only
        // place git's own parser looks — and the shape this repository's
        // commits did NOT have until this ticket measured it.
        git(
            &repo,
            &[
                "commit",
                "-qm",
                &format!("work for {t}\n\nPmat-Ticket: {t}\nCo-Authored-By: t <t@t>"),
            ],
        );
    }
    (dir, repo)
}

fn run(repo: &Path) -> (i32, String) {
    let out = Command::new("bash")
        .arg(gate())
        .current_dir(repo)
        .env("QUORUM_ACTOR", ACTOR)
        .env_remove("QUORUM_SKIP")
        .output()
        .expect("the gate must run");
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    (out.status.code().unwrap_or(-1), text)
}

/// The exact shape PR #532 was in: a branch named for a ticket no commit
/// claims.
#[test]
fn a_branch_named_for_a_ticket_no_commit_claims_is_named_and_red() {
    let (_d, repo) = fixture("PMAT-520-book-v1.29.0", &["PMAT-531"]);
    let (code, text) = run(&repo);
    assert_ne!(code, 0, "the gate accepted a misnamed branch:\n{text}");
    assert!(
        text.contains("names PMAT-520") && text.contains("no commit being pushed claims it"),
        "the refusal does not say which ticket the branch names and that nothing \
         claims it:\n{text}"
    );
    // It must say WHY it matters, or the next person renames the trailer
    // instead of the branch and moves the defect rather than fixing it.
    assert!(
        text.contains("RELEASE WINDOW"),
        "the refusal does not say what the branch name decides:\n{text}"
    );
    // And it must list what IS claimed, so the fix is obvious.
    assert!(
        text.contains("PMAT-531"),
        "the refusal does not name the ticket the commits actually claim:\n{text}"
    );
}

/// A branch that names its own ticket passes this arm.
///
/// The gate goes on to refuse it for having no quorum receipt, which is a
/// different arm and is asserted by name — a test that only checked "exit != 0"
/// would pass over a branch check that refused everything.
#[test]
fn a_branch_that_names_its_own_ticket_passes_this_arm() {
    let (_d, repo) = fixture("PMAT-531-book-v1.29.0", &["PMAT-531"]);
    let (_code, text) = run(&repo);
    assert!(
        !text.contains("no commit being pushed claims it"),
        "a branch naming its own ticket was refused by the branch arm:\n{text}"
    );
    assert!(
        text.contains("no quorum receipt"),
        "the gate did not reach the receipt arm, so this case is not measuring \
         what it claims:\n{text}"
    );
}

/// A branch carrying SEVERAL tickets passes as long as it names one of them.
///
/// This repository does that on every release: a cut branch carries the cut's
/// ticket and the tickets whose status it corrects. A rule demanding every
/// trailer match the branch would refuse the work it exists to protect.
#[test]
fn a_branch_carrying_several_tickets_passes_if_it_names_one() {
    let (_d, repo) = fixture(
        "PMAT-531-the-booking",
        &["PMAT-537", "PMAT-531", "PMAT-240"],
    );
    let (_code, text) = run(&repo);
    assert!(
        !text.contains("no commit being pushed claims it"),
        "a branch naming one of the tickets it carries was refused:\n{text}"
    );
}

/// A branch with no ticket id in its name is not this arm's business.
///
/// `fix/ci-lint` and `chore/deps-update` are the naming this repository's own
/// guidelines suggest for work that has no ticket. Refusing them would be this
/// arm inventing a requirement nobody asked for.
#[test]
fn a_branch_with_no_ticket_id_is_not_refused_by_this_arm() {
    let (_d, repo) = fixture("fix/ci-lint", &["PMAT-531"]);
    let (_code, text) = run(&repo);
    assert!(
        !text.contains("no commit being pushed claims it"),
        "a branch with no ticket id in its name was refused:\n{text}"
    );
}

/// The arm reads the trailer the way pmat does, not the way git does.
///
/// `git log --format='%(trailers:key=…)'` reads trailers from the LAST
/// PARAGRAPH only, so a message written with several `-m` flags — each of which
/// becomes its own paragraph — has a `Pmat-Ticket:` line git does not consider
/// a trailer. Measured: every commit in this session until this ticket was in
/// that shape. pmat's CB-2113 and this repository's commit-msg hook both match
/// the LINE wherever it appears, and a gate disagreeing with them about what a
/// trailer is would refuse commits they accept.
#[test]
fn a_pmat_ticket_line_git_would_not_call_a_trailer_still_counts() {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = dir.path().to_path_buf();
    git(&repo, &["init", "-q", "-b", "main"]);
    git(&repo, &["config", "commit.gpgsign", "false"]);
    write(&repo, "README.md", "fixture\n");
    write(
        &repo,
        ".quorum/enforce.json",
        &format!("{{\"enforced_for\": [\"{ACTOR}\"]}}\n"),
    );
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-qm", "base"]);
    git(&repo, &["update-ref", "refs/remotes/origin/main", "HEAD"]);

    git(&repo, &["checkout", "-q", "-b", "PMAT-531-work"]);
    write(&repo, "w.txt", "w\n");
    git(&repo, &["add", "-A"]);
    // Pmat-Ticket in its OWN paragraph, two above the last — exactly what
    // `git commit -m A -m B -m C` produces, and exactly what git's trailer
    // parser returns nothing for.
    git(
        &repo,
        &[
            "commit",
            "-qm",
            "work\n\nPmat-Ticket: PMAT-531\n\nClaude-Session: x\n\nCo-Authored-By: t <t@t>",
        ],
    );
    let by_git = String::from_utf8_lossy(
        &git(
            &repo,
            &[
                "log",
                "-1",
                "--format=%(trailers:key=Pmat-Ticket,valueonly=true)",
            ],
        )
        .stdout,
    )
    .trim()
    .to_string();
    assert!(
        by_git.is_empty(),
        "git now reads this shape as a trailer, so this case no longer proves \
         the arm must read it differently — it returned {by_git:?}"
    );

    let (_code, text) = run(&repo);
    assert!(
        !text.contains("no commit being pushed claims it"),
        "the arm missed a Pmat-Ticket line git would not call a trailer, so it \
         disagrees with pmat's CB-2113 and the commit-msg hook about what one \
         is:\n{text}"
    );
}

/// A branch whose commits claim NOTHING is a different gate's finding.
///
/// The commit-msg hook refuses a message with no `Pmat-Ticket`, and pmat's
/// CB-2113 refuses it again. An arm that also refused it would be reporting
/// someone else's finding in its own words, and it broke a pre-existing fixture
/// that builds its commits without hooks.
///
/// The defect THIS arm is about had a trailer. It was the wrong one.
#[test]
fn a_branch_whose_commits_claim_nothing_is_not_this_arms_finding() {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = dir.path().to_path_buf();
    git(&repo, &["init", "-q", "-b", "main"]);
    git(&repo, &["config", "commit.gpgsign", "false"]);
    write(&repo, "README.md", "fixture\n");
    write(
        &repo,
        ".quorum/enforce.json",
        &format!("{{\"enforced_for\": [\"{ACTOR}\"]}}\n"),
    );
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-qm", "base"]);
    git(&repo, &["update-ref", "refs/remotes/origin/main", "HEAD"]);

    git(&repo, &["checkout", "-q", "-b", "PMAT-999-claims-nothing"]);
    write(&repo, "w.txt", "w\n");
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-qm", "no trailer at all"]);

    let (_code, text) = run(&repo);
    assert!(
        !text.contains("no commit being pushed claims it"),
        "this arm refused a branch whose commits carry no Pmat-Ticket at all, \
         which is the commit-msg hook's finding and CB-2113's, not this one's:\n{text}"
    );
}
