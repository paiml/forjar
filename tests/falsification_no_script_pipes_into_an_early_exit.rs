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

/// The pipeline stages of one shell line, with quoting and comments honoured.
///
/// Three review lanes broke the naive version, each in one line of shell:
///
/// | input | naive result | why |
/// |---|---|---|
/// | `echo " \| head "` | flagged | split on a pipe inside a string |
/// | `cat f \| grep " # " \| head -1` | missed | `split_once(" # ")` cut the line at a `#` inside a string, taking the `head` with it |
/// | `cat f \` (newline) `  \| head -1` | missed | a continuation is one pipeline written on two lines |
///
/// So this walks the line: a `'` or `"` toggles its quote, a `#` outside quotes
/// ends the line, and only a single `|` outside quotes splits a stage — `||`
/// is an operator and is consumed whole, so the command after it is a fallback
/// rather than the right-hand side of a pipe.
fn stages(line: &str) -> Vec<String> {
    let mut w = Walk::default();
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if w.step(c, &mut chars) {
            break;
        }
    }
    w.out
}

/// One pass over a shell line, carrying its quote state.
///
/// A struct rather than a loop body because the loop body was cognitive 33
/// against a limit of 25 — the complexity gate catching the same "one function
/// doing three jobs" it caught in this file once already.
#[derive(Default)]
struct Walk {
    out: Vec<String>,
    single: bool,
    double: bool,
}

impl Walk {
    /// Consume one character. Returns true when the line is over (a comment).
    fn step(&mut self, c: char, rest: &mut std::iter::Peekable<std::str::Chars<'_>>) -> bool {
        if self.out.is_empty() {
            self.out.push(String::new());
        }
        match c {
            // An escape takes the next character with it, whatever it is.
            '\\' if !self.single => {
                self.push(c);
                if let Some(n) = rest.next() {
                    self.push(n);
                }
            }
            '\'' if !self.double => {
                self.single = !self.single;
                self.push(c);
            }
            '"' if !self.single => {
                self.double = !self.double;
                self.push(c);
            }
            _ if self.single || self.double => self.push(c),
            // `||` IS AN OPERATOR, NOT TWO PIPES. Consumed whole, so
            // `cmd || head -1` has one stage and the `head` after it is a
            // fallback command rather than the right-hand side of a pipe.
            '|' if rest.peek() == Some(&'|') => {
                rest.next();
                self.push('|');
                self.push('|');
            }
            '|' => self.out.push(String::new()),
            '#' => return true,
            _ => self.push(c),
        }
        false
    }

    fn push(&mut self, c: char) {
        self.out.last_mut().expect("a stage").push(c);
    }
}

/// The offending pipelines in one script's text, as `line:code`.
///
/// Split out so the walk over the tree stays a loop and this stays a
/// predicate — the first version was one function and the complexity gate
/// refused it at 38 against a limit of 25, which is the same "one function
/// doing three jobs" the gate exists to catch.
fn offending_pipelines(rel: &str, text: &str) -> Vec<String> {
    // EVERY script, not only those whose text contains `pipefail`.
    //
    // That filter skipped `scripts/dogfood/lib/releases.sh` — a LIBRARY,
    // sourced by gates that do set it, so it inherits pipefail and dies the
    // same way. Two of the sites this ticket fixed were in it.
    //
    // And pipefail is not the whole hazard. Without it the left side still
    // takes SIGPIPE and its output is still TRUNCATED, silently. That is how a
    // 459-line proof log in this very session landed as 9 lines.
    //
    // A CONTINUATION IS ONE PIPELINE. `cat f \` on one line and `| head -1` on
    // the next is the same hazard written differently, and a lane walked out
    // through exactly that.
    let joined = text.replace("\\\n", " ");
    joined
        .lines()
        .enumerate()
        .filter_map(|(n, line)| {
            let t = line.trim_start();
            // A whole-line comment is not code, and an exception that was
            // WRITTEN DOWN is an exception. One nobody wrote down is the same
            // as no rule.
            if t.starts_with('#') || line.contains("# sigpipe-ok:") {
                return None;
            }
            let st = stages(t);
            // The leftmost command is never the early exit.
            let fatal = st.iter().skip(1).any(|stage| {
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
    // It is walked: a pipeline planted in the MIDDLE of its text is found.
    //
    // A lane pointed out that planting at the END proves almost nothing — a
    // rule that only read a file's last line would pass. So it goes in the
    // middle, and the case asserts the reported LINE NUMBER, which only a rule
    // that actually walks the file can get right.
    let lines: Vec<&str> = text.lines().collect();
    let at = lines.len() / 2;
    let mut planted: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
    planted.insert(at, "printf '%s' \"$x\" | grep -q y".to_string());
    let found = offending_pipelines(lib, &planted.join("\n"));
    assert_eq!(
        found.len(),
        1,
        "a pipeline in a library that sets no pipefail of its own is not \
         caught, so the rule has the same hole the one before it had"
    );
    assert!(
        found[0].starts_with(&format!("{lib}:{}", at + 1)),
        "the rule found the planted pipeline but reports it at the wrong line \
         ({}), so it is not walking the file — it is pattern-matching the \
         whole text",
        found[0]
    );
}

/// The three shapes three review lanes walked out through.
///
/// Each is one line of shell and each defeated the naive version of this rule.
/// They are here as cases rather than as a comment because a rule's holes are
/// the part worth regression-testing: the fixes are one function, and the next
/// person to touch it will not have read the round.
#[test]
fn the_rule_cannot_be_walked_out_of_by_quoting_or_continuing() {
    // A `#` INSIDE A STRING is not a comment. Cutting the line there took the
    // `head` stage with it and the pipeline vanished.
    let hidden = "set -o pipefail\ncat f | grep \" # \" | head -1\n";
    assert_eq!(
        offending_pipelines("x.sh", hidden).len(),
        1,
        "a `#` inside a string hides the rest of the pipeline from the rule"
    );

    // A `|` INSIDE A STRING is not a pipe. This one is safe and was flagged.
    let quoted = "set -o pipefail\necho \" | head \"\n";
    assert!(
        offending_pipelines("x.sh", quoted).is_empty(),
        "a pipe inside a string is reported as a pipeline, so the rule fires \
         on text that runs nothing"
    );

    // A CONTINUATION is one pipeline written on two lines.
    let continued = "set -o pipefail\ncat f \\\n  | head -1\n";
    assert_eq!(
        offending_pipelines("x.sh", continued).len(),
        1,
        "a pipeline split across a line continuation is invisible to the rule"
    );

    // And the safe shapes stay safe: `||` is an operator, the early-exit
    // command as the LEFTMOST stage reads a file rather than a pipe, and a
    // written exception is an exception.
    for safe in [
        "set -o pipefail\ncmd || head -1\n",
        "set -o pipefail\nhead -12 f | sed 's/^/  /'\n",
        "set -o pipefail\ncat f | head -1  # sigpipe-ok: f is one line\n",
        "set -o pipefail\n# cat f | head -1\n",
    ] {
        assert!(
            offending_pipelines("x.sh", safe).is_empty(),
            "the rule fires on a safe shape:\n{safe}"
        );
    }
}
