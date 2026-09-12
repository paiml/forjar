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
    // Only scripts that actually set pipefail can die this way.
    if !text.contains("pipefail") {
        return Vec::new();
    }
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
            let (_, rhs) = code.split_once('|')?;
            // `||` is a shell operator, not a pipeline.
            if rhs.starts_with('|') {
                return None;
            }
            let rhs = rhs.trim_start();
            EARLY
                .iter()
                .any(|e| rhs.starts_with(e))
                .then(|| format!("{rel}:{}  {}", n + 1, line.trim()))
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

/// The rule must cover more than the three files PMAT-239 owned.
///
/// A rule scoped to the files one ticket touched is how eighteen sites
/// survived a fix that named them. This asserts the census is repository-wide
/// by counting what it actually walks.
#[test]
fn the_rule_walks_the_whole_of_scripts_and_not_one_ticket_s_files() {
    let root = repo();
    let all = scripts(&root);
    let with_pipefail = all
        .iter()
        .filter(|rel| {
            std::fs::read_to_string(root.join(rel))
                .map(|t| t.contains("pipefail"))
                .unwrap_or(false)
        })
        .count();
    assert!(
        with_pipefail >= 10,
        "only {with_pipefail} of {} scripts set pipefail, which is fewer than \
         the set PMAT-239's census found — either the census shrank or this \
         test is looking in the wrong place",
        all.len()
    );
    // And the three PMAT-239 owned are among them, so the narrow rule's
    // subject is genuinely contained in this one.
    for rel in [
        "scripts/dogfood/lib/window.sh",
        "scripts/dogfood/tagged.sh",
        "scripts/release-goal.sh",
    ] {
        assert!(
            all.iter().any(|f| f == rel),
            "{rel} is not in the set this rule walks, so PMAT-239's narrower \
             rule covers a file this one does not"
        );
    }
}
