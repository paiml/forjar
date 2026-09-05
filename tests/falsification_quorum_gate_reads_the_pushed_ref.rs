//! The quorum gate must judge the COMMIT BEING PUSHED, not the working tree.
//!
//! forjar#400. `scripts/quorum-gate.sh` takes the branch NAME from
//! `--remote-ref` — git's pre-push protocol hands it the real target, which
//! closed the `git checkout -b main && git push origin main:real-feature`
//! rename bypass — and then resolves EVERYTHING ELSE from the local checkout:
//! `git merge-base HEAD origin/main`, `git rev-parse HEAD`, `git diff <base>
//! HEAD`, `[ -f .quorum/<branch>.json ]`, `os.path.exists(test_file)`.
//!
//! The two halves disagree the moment they are allowed to. Measured, in a
//! synthetic repo against the pre-fix script:
//!
//!   * From a `branch-A` checkout, `--remote-ref refs/heads/branch-B` exits 1
//!     with `✗ QUORUM GATE: no quorum receipt at .quorum/branch-B.json`, while
//!     `git cat-file -e branch-B:.quorum/branch-B.json` says the receipt is
//!     right there in the commit being pushed. `git push origin branch-B` from
//!     any other branch is refused, for an enforced author, with a message
//!     naming a file that exists.
//!   * `PRINT_HASH=1` from the `branch-A` checkout printed
//!     `4d244e3669a28743…`; from `branch-B`, `4af530a587d2cc36…`. The receipt
//!     binds a hash of the WRONG DIFF, so the check that makes the whole gate
//!     more than theatre is computed against code nobody is pushing.
//!
//! And it fails open as readily as closed: the "receipt must be committed"
//! block used `git diff --quiet` / `git diff --cached --quiet`, both of which
//! exit 0 on an UNTRACKED path. A `.quorum/<branch>.json` written and never
//! `git add`ed carried a `waived.reason` straight past the gate — exit 0,
//! printing `(committed in … -- visible in the PR diff)` about a file `git
//! status` reports as `??`. That is the silent, unreviewable bypass the
//! script's own comments say cannot exist ("Bypass exists; silent bypass does
//! not"), available to exactly the enforced authors `QUORUM_SKIP` is refused
//! to.
//!
//! `.github/workflows/quorum.yml` checks out `pull_request.head.sha`, so in CI
//! HEAD *is* the pushed commit and none of this shows. It is the local hook —
//! byte-identical to the tracked one — that is wrong.
//!
//! Every fixture below pins `QUORUM_ACTOR` **and** the repo-local
//! `user.email`. That is load-bearing, not decoration: the fixture's own
//! `enforce.json` lists only `tester@example.com`, so a run that inherited the
//! machine's global email would fall to `advisory`, where `die` exits 0 — and
//! every assertion here would pass against the broken script.
//!
//! PMAT-159 gave that same gate a SECOND subject: its evidence pass recognised
//! no citation a RELEASE commit can make, so a release commit could not pass it
//! at all. That defect and its four fixtures are in the sibling file
//! `tests/falsification_quorum_anchors_release_shaped.rs`; they are not here
//! because their fixture block is ~200 lines and this file is against CB-040's
//! 500-line ceiling.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const ZERO_SHA: &str = "0000000000000000000000000000000000000000";
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
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

fn stdout_of(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

struct Run {
    code: i32,
    text: String,
}

/// Run the real gate script with the fixture as its working directory.
fn run_gate(repo: &Path, print_hash: bool, args: &[&str]) -> Run {
    let out = Command::new("bash")
        .arg(gate())
        .args(args)
        .current_dir(repo)
        .env("QUORUM_ACTOR", ACTOR)
        .env("PRINT_HASH", if print_hash { "1" } else { "0" })
        .env_remove("QUORUM_SKIP")
        .output()
        .expect("the gate script must run");
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    Run {
        code: out.status.code().unwrap_or(-1),
        text,
    }
}

fn write(repo: &Path, rel: &str, body: &str) {
    let p = repo.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, body).unwrap();
}

const WAIVED_RECEIPT: &str =
    r#"{"waived": {"reason": "fixture: exercise the gate's own resolution, not a quorum"}}"#;

/// main (with an enforce.json naming only the fixture actor), `origin/main`
/// pointed at it, then two divergent branches. `branch-B` carries a COMMITTED
/// waived receipt; the checkout is left on `branch-A`.
struct Fixture {
    _dir: tempfile::TempDir,
    repo: PathBuf,
    branch_b_tip: String,
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = dir.path().to_path_buf();
    git(&repo, &["init", "-q", "-b", "main"]);
    git(&repo, &["config", "user.email", ACTOR]);
    git(&repo, &["config", "user.name", "tester"]);
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

    git(&repo, &["checkout", "-q", "-b", "branch-B"]);
    write(&repo, "b.txt", "bbbb\n");
    write(&repo, ".quorum/branch-B.json", WAIVED_RECEIPT);
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-qm", "work on B"]);
    let branch_b_tip = stdout_of(&git(&repo, &["rev-parse", "HEAD"]));

    git(&repo, &["checkout", "-q", "main"]);
    git(&repo, &["checkout", "-q", "-b", "branch-A"]);
    write(&repo, "a.txt", "aaaa\n");
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-qm", "work on A"]);

    // The whole point: the checkout is NOT the branch being pushed.
    assert_eq!(
        stdout_of(&git(&repo, &["rev-parse", "--abbrev-ref", "HEAD"])),
        "branch-A"
    );
    Fixture {
        _dir: dir,
        repo,
        branch_b_tip,
    }
}

/// Pushing a branch you are not checked out on must find its receipt.
///
/// The receipt IS in the commit being pushed — the fixture asserts that with
/// `git cat-file -e` before it runs the gate, so a failure here cannot be
/// blamed on the fixture.
#[test]
fn a_receipt_committed_on_the_pushed_branch_is_found_from_another_checkout() {
    let f = fixture();
    let present = Command::new("git")
        .args(["cat-file", "-e", "branch-B:.quorum/branch-B.json"])
        .current_dir(&f.repo)
        .status()
        .expect("git cat-file must run");
    assert!(
        present.success(),
        "fixture is wrong: the receipt is not in branch-B's tree"
    );

    let r = run_gate(
        &f.repo,
        false,
        &[
            "--remote-ref",
            "refs/heads/branch-B",
            "--local-sha",
            &f.branch_b_tip,
        ],
    );
    assert!(
        !r.text.contains("no quorum receipt"),
        "the gate reported a missing receipt for a branch whose commit contains \
         one. It read `.quorum/branch-B.json` from the WORKING TREE (checked out \
         at branch-A) while taking the branch name from the pushed ref:\n{}",
        r.text
    );
    assert_eq!(
        r.code, 0,
        "the gate refused a push of branch-B from a branch-A checkout, for an \
         ENFORCED author, with the receipt committed on branch-B:\n{}",
        r.text
    );
}

/// The diff hash the receipt is bound to must be the pushed branch's diff.
///
/// This is the half a one-line "read the receipt with `git cat-file`" fix does
/// NOT reach: the waiver short-circuit runs BEFORE the hash binding, so the
/// assertion above would go green while the binding still described whatever
/// happened to be checked out.
#[test]
fn the_bound_diff_hash_is_the_pushed_branchs_diff_not_the_checkouts() {
    let f = fixture();

    git(&f.repo, &["checkout", "-q", "branch-B"]);
    let from_b = run_gate(&f.repo, true, &["--remote-ref", "refs/heads/branch-B"]);
    assert_eq!(from_b.code, 0, "PRINT_HASH run failed:\n{}", from_b.text);
    let honest = from_b.text.trim().to_string();
    assert!(!honest.is_empty(), "PRINT_HASH printed nothing");

    git(&f.repo, &["checkout", "-q", "branch-A"]);
    let from_a = run_gate(
        &f.repo,
        true,
        &[
            "--remote-ref",
            "refs/heads/branch-B",
            "--local-sha",
            &f.branch_b_tip,
        ],
    );
    assert_eq!(from_a.code, 0, "PRINT_HASH run failed:\n{}", from_a.text);
    assert_eq!(
        from_a.text.trim(),
        honest,
        "the hash the receipt must match differs by which branch happens to be \
         CHECKED OUT. It is computed from `git diff <merge-base> HEAD`, so the \
         binding — the one check that stops a receipt being recycled across \
         branches — describes code that is not being pushed"
    );
}

/// A branch DELETION carries an all-zero local sha and has no diff to refute.
///
/// The hook's own comment claimed the gate already skipped this. It did not:
/// `git merge-base 0000…0 origin/main` is `fatal: Not a valid commit name`,
/// exit 128, so the guard has to precede the merge-base, not merely the
/// receipt read.
#[test]
fn a_branch_deletion_is_not_asked_for_a_receipt() {
    let f = fixture();
    let r = run_gate(
        &f.repo,
        false,
        &[
            "--remote-ref",
            "refs/heads/branch-B",
            "--local-sha",
            ZERO_SHA,
        ],
    );
    assert_eq!(
        r.code, 0,
        "deleting a branch was refused by the quorum gate:\n{}",
        r.text
    );
    assert!(
        r.text.to_lowercase().contains("delet"),
        "the gate exited 0 on a deletion but said nothing about it — an early \
         exit that does not name its reason is the 'unmeasured is not a pass' \
         shape this script's own header rejects:\n{}",
        r.text
    );
}

/// An UNCOMMITTED receipt must never waive anything.
///
/// `git diff --quiet -- <path>` and `git diff --cached --quiet -- <path>` BOTH
/// exit 0 on an untracked path, so the "receipt must be committed" block was
/// silent for exactly the file that most needs it. With a `waived.reason` in a
/// file `git status` reports as `??`, the pre-fix gate exits 0 and prints
/// `(committed in .quorum/branch-A.json -- visible in the PR diff)`.
#[test]
fn an_untracked_receipt_cannot_waive_the_gate() {
    let f = fixture();
    write(&f.repo, ".quorum/branch-A.json", WAIVED_RECEIPT);
    let status = stdout_of(&git(
        &f.repo,
        &["status", "--porcelain", "--", ".quorum/branch-A.json"],
    ));
    assert!(
        status.starts_with("??"),
        "fixture is wrong: the receipt is not untracked (got {status:?})"
    );

    let tip = stdout_of(&git(&f.repo, &["rev-parse", "HEAD"]));
    let r = run_gate(
        &f.repo,
        false,
        &["--remote-ref", "refs/heads/branch-A", "--local-sha", &tip],
    );
    assert_ne!(
        r.code, 0,
        "a never-committed receipt WAIVED the gate for an enforced author. \
         Nothing about this bypass reaches a reviewer: it is not in the PR \
         diff, and the gate announces it as `(committed in … -- visible in the \
         PR diff)`. QUORUM_SKIP is refused to these authors precisely to stop \
         this:\n{}",
        r.text
    );
    assert!(
        !r.text.contains("WAIVED"),
        "the gate announced a waiver from an untracked file:\n{}",
        r.text
    );
}

/// Over-correction control: the fix must not break the ordinary case.
///
/// A developer standing on the branch they are pushing, with the receipt
/// committed, has to keep working — and so does a bare `./scripts/quorum-gate.sh`
/// with no arguments at all, which `Makefile`'s `quorum` target and
/// `.github/workflows/quorum.yml` both use.
#[test]
fn the_ordinary_same_branch_push_still_passes_with_and_without_the_flag() {
    let f = fixture();
    git(&f.repo, &["checkout", "-q", "branch-B"]);

    for args in [
        vec!["--remote-ref", "refs/heads/branch-B"],
        vec![
            "--remote-ref",
            "refs/heads/branch-B",
            "--local-sha",
            f.branch_b_tip.as_str(),
        ],
        vec![],
    ] {
        let r = run_gate(&f.repo, false, &args);
        assert_eq!(
            r.code, 0,
            "the gate refused the ordinary case with args {args:?}:\n{}",
            r.text
        );
        assert!(
            r.text.contains("WAIVED"),
            "args {args:?}: the gate exited 0 without reaching the committed \
             waiver, so it passed for some other reason:\n{}",
            r.text
        );
    }
}
