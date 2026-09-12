//! PMAT-240: the SIGPIPE class, over every script rather than three of them.
//!
//! # What the class is
//!
//! A pipeline whose right-hand side can exit before its left-hand side
//! finishes — `grep -q`, `grep -m`, `head`, `jq -e` — makes the left side take
//! SIGPIPE. Under `set -o pipefail` the pipeline returns **141**, so a gate
//! that reads its own output reports UNMEASURED at random.
//!
//! Gate T did exactly that on roughly one run in three before PMAT-239 fixed
//! three instances. PMAT-239's own rule then covered **three files**, which is
//! the shape of fix that says nothing about the rest of the tree: eighteen more
//! sites were in the census it committed, and thirteen were still there when
//! this ticket opened.
//!
//! # Why the rule is worth more than the fixes
//!
//! This session hit the class **twice more** while closing it. Gate T's new T9
//! arm shipped with `printf | awk '… exit'` and died 141 on its first run, in
//! the file that already carried PMAT-239's rule. And a proof log written with
//! `tee "$LOG" | head -14` landed as 9 lines instead of 459, which three review
//! lanes refused a receipt over.
//!
//! Both were written by someone who had just read the rule. A rule that covers
//! three files is a rule you can walk out of.
//!
//! # What this refuses
//!
//! Every `.sh` under `scripts/` that sets `pipefail`. A comment is not code, a
//! `||` is not a pipeline, and a pipeline whose exit status is explicitly
//! accepted is fine — but that has to be written down, not inferred, so the
//! only escape is a trailing `# sigpipe-ok: <reason>` on the line.

#![cfg(unix)]

use std::path::{Path, PathBuf};

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Every tracked `.sh` under `scripts/`, recursively.
fn scripts(root: &Path) -> Vec<String> {
    let out = std::process::Command::new("git")
        .args(["ls-files", "scripts/*.sh", "scripts/**/*.sh"])
        .current_dir(root)
        .output()
        .expect("git must run");
    assert!(out.status.success(), "git ls-files failed");
    let v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_string)
        .collect();
    // A census that enumerated nothing would pass silently, which is the
    // vacuity this whole file is about.
    assert!(
        v.len() >= 10,
        "git ls-files found {} scripts under scripts/, which is too few to be \
         the real set — the rule would pass over an empty census",
        v.len()
    );
    v
}

/// The commands that can leave a pipeline before its left side is done.
const EARLY: [&str; 4] = ["grep -q", "grep -m", "head", "jq -e"];

/// The offending pipelines in one script's text, as `line:code`.
///
/// Split out so the walk over the tree stays a loop and this stays a
/// predicate — the first version was one function and the complexity gate
/// refused it at 38 against a limit of 25, which is the same "one function
/// doing three jobs" the gate exists to catch.
fn offending_pipelines(rel: &str, text: &str) -> Vec<String> {
    // EVERY script, not only those that set pipefail themselves.
    //
    // The first version skipped a file whose text lacked the word, and that
    // skipped `scripts/dogfood/lib/releases.sh` — a LIBRARY, sourced by gates
    // that do set it, so it inherits pipefail and dies exactly the same way.
    // Two of the twelve sites this ticket fixed were in it, and the rule would
    // not have caught either.
    //
    // And pipefail is not the whole hazard anyway. Without it the left side
    // still takes SIGPIPE and its output is still TRUNCATED — silently. That
    // is how a 459-line proof log in this very session landed as 9 lines and
    // three review lanes refused a receipt describing it.
    text.lines()
        .enumerate()
        .filter_map(|(n, line)| {
            let t = line.trim_start();
            // A comment is not code, and an exception that was WRITTEN DOWN is
            // an exception. One nobody wrote down is the same as no rule.
            if t.starts_with('#') || line.contains("# sigpipe-ok:") {
                return None;
            }
            // A TRAILING COMMENT IS NOT CODE EITHER. The first version of this
            // rule fired on a comment describing the pipeline that had just
            // been REMOVED, and a rule that reads prose would push people to
            // stop explaining their fixes.
            let code = t.split_once(" # ").map_or(t, |(before, _)| before);
            // EVERY stage, not just the one after the first pipe. `printf |
            // sed | head -1` has a safe middle and a fatal end, and a rule
            // that looked only at the first `|` passed over it — measured:
            // two of the twelve sites this ticket fixed, in the same file.
            let mut stages = code.split('|');
            stages.next()?; // the leftmost command is never the early exit
            let fatal = stages.any(|stage| {
                // `||` is a shell operator, not a pipeline: it shows up here
                // as an EMPTY stage between two splits.
                if stage.is_empty() {
                    return false;
                }
                let stage = stage.trim_start();
                EARLY.iter().any(|e| stage.starts_with(e))
            });
            fatal.then(|| format!("{rel}:{}  {}", n + 1, line.trim()))
        })
        .collect()
}

#[test]
fn no_script_under_scripts_pipes_into_a_command_that_can_exit_early() {
    let root = repo();
    let offenders: Vec<String> = scripts(&root)
        .into_iter()
        .flat_map(|rel| {
            let text = std::fs::read_to_string(root.join(&rel))
                .unwrap_or_else(|e| panic!("read {rel}: {e}"));
            offending_pipelines(&rel, &text)
        })
        .collect();

    assert!(
        offenders.is_empty(),
        "PMAT-240: {} pipeline(s) under scripts/ feed a command that can exit \
         before its left-hand side finishes. Under `set -o pipefail` the left \
         side takes SIGPIPE and the pipeline returns 141, so a gate reports \
         UNMEASURED at random — measured on gate T at about one run in three.\n\n\
         Capture and use a here-string, do it in one process, or put the \
         early-exit command FIRST so it reads the file rather than a pipe. If a \
         pipeline is genuinely safe, say so on the line with `# sigpipe-ok: \
         <reason>` — an exception nobody wrote down is the same as no rule.\n\n{}",
        offenders.len(),
        offenders.join("\n")
    );
}

/// The rule must cover more than the files one ticket touched.
///
/// A rule scoped to three files is how eighteen sites survived a fix that
/// named them, and a rule scoped to files containing the word `pipefail` is how
/// a sourced LIBRARY survives one — `scripts/dogfood/lib/releases.sh` does not
/// contain it, inherits it from every gate that sources it, and held two of the
/// twelve sites this ticket fixed.
#[test]
fn the_rule_walks_the_whole_of_scripts_including_the_libraries() {
    let root = repo();
    let all = scripts(&root);

    // The three PMAT-239 owned, so its narrower rule's subject is contained in
    // this one; and the library its scope missed.
    for rel in [
        "scripts/dogfood/lib/window.sh",
        "scripts/dogfood/tagged.sh",
        "scripts/release-goal.sh",
        "scripts/dogfood/lib/releases.sh",
    ] {
        assert!(
            all.iter().any(|f| f == rel),
            "{rel} is not in the set this rule walks — a file the narrower \
             rules covered, or a library they missed, must be in this one"
        );
    }

    // And a library that sets no pipefail of its own is still walked, which is
    // the whole difference between this rule and the one before it.
    let lib = "scripts/dogfood/lib/releases.sh";
    let text = std::fs::read_to_string(root.join(lib)).expect("the library must exist");
    assert!(
        !text.contains("pipefail"),
        "{lib} now sets pipefail itself, so this case no longer proves that a \
         library WITHOUT it is walked — point it at one that does not"
    );
    // It is walked: a pipeline planted in its text is found.
    let planted = format!("{text}\nprintf '%s' \"$x\" | grep -q y\n");
    assert_eq!(
        offending_pipelines(lib, &planted).len(),
        1,
        "a pipeline in a library that sets no pipefail of its own is not \
         caught, so the rule has the same hole the one before it had"
    );
}
