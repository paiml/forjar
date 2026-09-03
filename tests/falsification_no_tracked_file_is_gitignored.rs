//! No file this repository TRACKS may also be matched by `.gitignore`.
//!
//! forjar#401. Fourteen tracked paths — 3,759,246 bytes at `f0cbf635` — were
//! matched by an ignore rule. `.gitignore:47` was `**/.pmat/` and `.gitignore:34`
//! was `.pv/`, while the index carried `.pmat/baseline.json` (1,220,063 bytes),
//! `docs/book/.pmat/context.db` (2,482,176), `.pmat/file-health-baseline.json`,
//! `.pmat/metrics/dependencies.json`, `.pmat/project.toml`,
//! `docs/book/.pmat/context.idx/manifest.json`, `forjar/.pv/lint-previous.json`
//! and six `contracts/.pv/cache/lint/*.json`.
//!
//! # Why "it still works" is not a defence
//!
//! gitignore never affects a file already in the index, so the ratchets kept
//! reading their baselines and the state was defended in `.gitignore`'s own
//! comments as deliberate. What it actually bought:
//!
//!   * `.git/hooks/post-commit` regenerates `.pmat/baseline.json` after EVERY
//!     commit and re-stages it (`git add "${BASELINE_PATH}" 2>/dev/null ||
//!     true`). The add prints "The following paths are ignored" and returns 1 —
//!     and STAGES THE FILE ANYWAY, which is why the churn never stopped. The
//!     exit code is cosmetic; `|| true` was swallowing a warning, not a failure.
//!   * Two `pmat tdg baseline create` runs over the same tree differ in exactly
//!     one line: `created_at`. MEASURED, 1,196,909 bytes each, `diff | wc -l`
//!     -> 4, and nothing outside the timestamp. So every branch rewrites line 3
//!     of a 1.2 MB file and any two-branch merge conflicts there. Two `.quorum`
//!     receipts in this tree name that conflict as the reason a re-anchor was
//!     needed.
//!   * `git checkout <branch>` refuses across the churn, and 75 revisions of
//!     `.pmat/baseline.json` sit in the history.
//!
//! # Why `--no-index` is load-bearing
//!
//! `git check-ignore` WITHOUT `--no-index` consults the index first and reports
//! a tracked path as not-ignored — by design, and it hides this defect
//! completely. `--no-index` asks the question the rules actually answer.
//!
//! # Why not `-v`, and why there are no `!` negations
//!
//! `git check-ignore -v` prints a record for NEGATION matches too and exits 0
//! for them, so a `-v` pipeline asserting "no output" is unsatisfiable while any
//! file is deliberately re-included with `!`. MEASURED: the `-v` form emitted 4
//! lines where this form emits 0. A gate that can never pass is worse than no
//! gate.
//!
//! The negation question turned out to be moot. Keeping the two load-bearing
//! files tracked under `.pmat/` and re-including them was the first fix, and it
//! survived exactly one commit: pmat writes its own `.pmat/.gitignore`
//! containing `*`, a .gitignore in a deeper directory outranks every pattern in
//! a shallower one, and `git check-ignore --no-index -v .pmat/cb200-baseline.json`
//! then answered `.pmat/.gitignore:10:*` with the root negation doing nothing.
//! That file is untracked, so it exists only where pmat has run — the rule was
//! about to differ between CI and every developer. The CB-200 ceiling moved to
//! `scripts/ratchets/cb200-baseline.json` instead, and nothing under `.pmat/`
//! is tracked at all.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn is_a_git_checkout(dir: &Path) -> bool {
    Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(dir)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Every path in the index that an ignore rule also matches.
///
/// NUL-delimited end to end: a path containing a space or a quote is quoted by
/// git's default output, and a checker that silently skips such a path would be
/// green for the wrong reason.
fn tracked_and_ignored(dir: &Path) -> Vec<String> {
    let listed = Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(dir)
        .output()
        .expect("git ls-files must run");
    assert!(
        listed.status.success(),
        "git ls-files failed in {}: {}",
        dir.display(),
        String::from_utf8_lossy(&listed.stderr)
    );
    assert!(
        !listed.stdout.is_empty(),
        "git ls-files listed NOTHING in {} — the checker has no denominator and \
         its verdict would be meaningless",
        dir.display()
    );

    let mut child = Command::new("git")
        .args(["check-ignore", "--no-index", "--stdin", "-z"])
        .current_dir(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("git check-ignore must spawn");
    child
        .stdin
        .take()
        .expect("stdin is piped")
        .write_all(&listed.stdout)
        .expect("feeding the path list must succeed");
    let out = child
        .wait_with_output()
        .expect("git check-ignore must finish");

    // 0 = at least one path is ignored, 1 = none are. Anything else is an error
    // and must not be read as "clean".
    let code = out.status.code().unwrap_or(-1);
    assert!(
        code == 0 || code == 1,
        "git check-ignore exited {code}: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    out.stdout
        .split(|b| *b == 0)
        .filter(|p| !p.is_empty())
        .map(|p| String::from_utf8_lossy(p).into_owned())
        .collect()
}

#[test]
fn no_tracked_file_in_this_repo_is_matched_by_gitignore() {
    let root = repo_root();
    if !is_a_git_checkout(&root) {
        println!("SKIP: {} is not a git checkout", root.display());
        return;
    }
    let offenders = tracked_and_ignored(&root);
    println!(
        "checked every tracked path in {}; {} also matched by .gitignore",
        root.display(),
        offenders.len()
    );
    assert!(
        offenders.is_empty(),
        "{} tracked file(s) are also matched by .gitignore: {:#?}\n\n\
         gitignore does not un-track them, so nothing visibly breaks — which is \
         exactly why this survived. What it costs is real: the post-commit hook \
         regenerates and re-stages `.pmat/baseline.json` after every commit (the \
         `git add` warns, exits 1 and stages it anyway), two runs of \
         `pmat tdg baseline create` over an unchanged tree differ only at \
         `created_at`, so line 3 of a 1.2 MB file is rewritten per branch and \
         any two-branch merge conflicts there. Untrack the derived cache, or — \
         if the repo genuinely owns the file — move it out from under whatever \
         rule claims it, so the rules say what the index says. An `!` negation \
         is NOT a general answer: it cannot reach past a .gitignore in a deeper \
         directory, and pmat writes one containing `*` (#401).",
        offenders.len(),
        offenders
    );
}

/// The other half of the fix: the CB-200 floor must still be a file git keeps.
///
/// Emptying the index satisfies the invariant above all by itself, so on its
/// own that assertion would be equally happy with the ratchet's baseline gone —
/// and `scripts/cb200-ratchet.sh` hard-exits 1 when it is absent, which turns
/// the "UNMEASURED is a failure" ratchet into a permanent failure instead.
///
/// This reads the path out of the script rather than restating it, so moving
/// the file without moving the reader fails here.
///
/// It also pins the constraint that forced the move. The obvious fix was to
/// keep `.pmat/cb200-baseline.json` tracked and re-include it with
/// `!/.pmat/cb200-baseline.json`. That does not work: pmat writes its own
/// `.pmat/.gitignore` containing `*`, and a .gitignore in a deeper directory
/// outranks every pattern in a shallower one — MEASURED, `git check-ignore
/// --no-index -v .pmat/cb200-baseline.json` answered `.pmat/.gitignore:10:*`
/// with the negation sitting in the root file doing nothing. That nested file
/// is itself untracked, so the rule differs between a fresh CI checkout and any
/// machine where pmat has run.
#[test]
fn the_cb200_ratchets_baseline_is_tracked_and_not_ignored() {
    let root = repo_root();
    let script = root.join("scripts/cb200-ratchet.sh");
    let text = std::fs::read_to_string(&script).expect("cb200-ratchet.sh must be readable");
    let base = text
        .lines()
        .find_map(|l| l.strip_prefix("BASE="))
        .map(|v| v.trim().trim_matches('"').to_string())
        .expect("scripts/cb200-ratchet.sh must set BASE=");
    println!("cb200-ratchet.sh reads its ceiling from {base}");

    assert!(
        root.join(&base).is_file(),
        "`{base}` is the ceiling scripts/cb200-ratchet.sh reads, and it is not \
         on disk. The script hard-exits 1 without it, so the ratchet is not \
         merely unmeasured — it is permanently red"
    );

    let tracked = Command::new("git")
        .args(["ls-files", "--error-unmatch", "--", &base])
        .current_dir(&root)
        .output()
        .expect("git ls-files must run");
    assert!(
        tracked.status.success(),
        "`{base}` is not tracked. A ratchet floor that every clone regenerates \
         locally is not a shared floor — it is whatever that tree happened to \
         measure, which is the 'UNMEASURED is a pass' shape the script's own \
         header rejects"
    );

    assert!(
        tracked_and_ignored(&root).iter().all(|p| p != &base),
        "`{base}` is tracked AND ignored. Under `.pmat/` that was unfixable: \
         pmat writes `.pmat/.gitignore` with `*` in it, a deeper .gitignore \
         outranks the root one, and no `!` negation at the root can reach past \
         it. The repo's own ratchet floor has to live somewhere the repo owns"
    );
}

/// Poka-yoke: prove the checker can go RED, and that `--no-index` is what makes
/// it able to.
///
/// Without this, a refactor that pointed the checker at the wrong directory, or
/// dropped `--no-index`, would leave the assertion above green over an empty or
/// blind set — which is the shape of the defect it is guarding against.
#[test]
fn the_checker_sees_a_tracked_and_ignored_file_only_with_no_index() {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = dir.path();
    let git = |args: &[&str]| {
        let out = Command::new("git")
            .args(args)
            .current_dir(repo)
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@t")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@t")
            .output()
            .expect("git must run");
        assert!(
            out.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    };
    git(&["init", "-q", "-b", "main"]);
    std::fs::create_dir_all(repo.join("cache")).unwrap();
    std::fs::write(repo.join("cache/derived.json"), "{}\n").unwrap();
    std::fs::write(repo.join("keep.txt"), "hello\n").unwrap();
    std::fs::write(repo.join(".gitignore"), "**/cache/\n").unwrap();
    // -f, because the point of the fixture is a file that is tracked DESPITE an
    // ignore rule — which is the state this repository was in.
    git(&["add", "-f", "cache/derived.json"]);
    git(&["add", ".gitignore", "keep.txt"]);
    git(&["commit", "-qm", "fixture"]);

    assert_eq!(
        tracked_and_ignored(repo),
        vec!["cache/derived.json".to_string()],
        "the checker did not report a file that is BOTH tracked and ignored, so \
         its green verdict on this repository proves nothing"
    );

    // The same question WITHOUT --no-index: git consults the index, sees the
    // path is tracked, and answers "not ignored". This is the blind spot that
    // let #401 sit in the tree, so it is pinned rather than trusted.
    let blind = Command::new("git")
        .args(["check-ignore", "cache/derived.json"])
        .current_dir(repo)
        .output()
        .expect("git check-ignore must run");
    assert_eq!(
        blind.status.code(),
        Some(1),
        "`git check-ignore` without --no-index reported the tracked path as \
         ignored; the --no-index flag would then not be load-bearing and this \
         suite's central claim would be wrong"
    );
}
