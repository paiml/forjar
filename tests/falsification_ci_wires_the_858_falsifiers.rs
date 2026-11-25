//! #858: seven integration-test binaries ran in no CI job, so they rotted
//! unseen (one failed on main for weeks). Each is a `--test` argument of a
//! `cargo test` step in `.github/workflows/ci.yml`. Dropping one must fail here.
//!
//! The checker reads the parsed workflow, collects every `--test NAME` token of
//! every step `run:`, and names each required target that is absent. It does
//! not read comments: a commented-out `--test` line is not a wiring.
//!
//! mutation: each row of `mutants()` removes one required target from the
//! parsed text and the checker must name exactly that target.

use serde_yaml_ng::Value;
use std::collections::BTreeSet;

const REQUIRED: [&str; 7] = [
    "falsification_390_one_stream_reporter",
    "falsification_release_workflow_shape",
    "falsification_untyped_error_sites_ratchet",
    "falsification_e02_controlmaster_before_the_drift_gate",
    "falsification_replace_running_binary",
    "falsification_io_hash_answers_for_its_machine",
    "falsification_budget_dry_run",
];

fn ci_yml() -> String {
    let p = format!("{}/.github/workflows/ci.yml", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {p}: {e}"))
}

/// One pass over a whole `run:` text (quote and escape state carried across
/// lines), yielding commands as (line, words). It models only: words, single
/// and double quotes, backslash escapes and continuations, `#` comments (an
/// unquoted `#` at a word start), and the separators newline `;` `&` `|`.
/// Anything else it cannot read faithfully is refused (see `Scan::bad`).
#[derive(Default)]
struct Scan {
    cmds: Vec<(usize, Vec<String>)>,
    cur: Vec<String>,
    tok: String,
    started: bool,
    line: usize,
}

impl Scan {
    fn push(&mut self, c: char) {
        self.tok.push(c);
        self.started = true;
    }
    fn end_tok(&mut self) {
        if self.started {
            self.cur.push(std::mem::take(&mut self.tok));
            self.started = false;
        }
    }
    fn end_cmd(&mut self) {
        self.end_tok();
        if !self.cur.is_empty() {
            self.cmds.push((self.line, std::mem::take(&mut self.cur)));
        }
    }
    fn bad(&self, what: &str) -> String {
        format!(
            "ci.yml run block, line {}: unsupported {what}",
            self.line + 1
        )
    }
    /// Chars consumed after a backslash outside single quotes.
    fn escape(&mut self, nx: Option<char>) -> usize {
        match nx {
            Some('\n') => {
                self.line += 1;
                1
            }
            Some(c) => {
                self.push(c);
                1
            }
            None => 0,
        }
    }
    fn unquoted(
        &mut self,
        q: &mut Option<char>,
        c: char,
        nx: Option<char>,
    ) -> Result<usize, String> {
        if let Some(what) = unsupported(c, nx, false) {
            return Err(self.bad(what));
        }
        match c {
            '\\' => return Ok(self.escape(nx)),
            '\'' | '"' => {
                *q = Some(c);
                self.started = true;
            }
            '\n' | ';' | '&' | '|' => self.end_cmd(),
            c if c.is_whitespace() => self.end_tok(),
            _ => self.push(c),
        }
        Ok(0)
    }
    fn dquoted(
        &mut self,
        q: &mut Option<char>,
        c: char,
        nx: Option<char>,
    ) -> Result<usize, String> {
        if let Some(what) = unsupported(c, nx, true) {
            return Err(self.bad(what));
        }
        match (c, nx) {
            ('"', _) => *q = None,
            ('\\', Some('\n')) => return Ok(self.escape(nx)),
            ('\\', Some(n @ ('$' | '`' | '"' | '\\'))) => {
                self.push(n);
                return Ok(1);
            }
            _ => self.push(c),
        }
        Ok(0)
    }
}

/// The construct (if any) the scanner refuses to read at `c`.
fn unsupported(c: char, nx: Option<char>, in_dq: bool) -> Option<&'static str> {
    match (c, nx) {
        ('$', Some('(')) => Some("$( command substitution"),
        ('`', _) => Some("backtick substitution"),
        ('$', Some('\'')) if !in_dq => Some("$'...' quoting"),
        ('<', Some('<')) if !in_dq => Some("heredoc <<"),
        _ => None,
    }
}

/// Commands of one `run:` text, or the reason it cannot be read faithfully.
fn commands(run: &str) -> Result<Vec<(usize, Vec<String>)>, String> {
    let chars: Vec<char> = run.chars().collect();
    let (mut s, mut q, mut i) = (Scan::default(), None::<char>, 0);
    while i < chars.len() {
        let (c, nx) = (chars[i], chars.get(i + 1).copied());
        i += 1;
        if q.is_none() && c == '#' && !s.started {
            i += chars[i..].iter().take_while(|&&k| k != '\n').count();
            continue;
        }
        i += match q {
            None => s.unquoted(&mut q, c, nx)?,
            Some('"') => s.dquoted(&mut q, c, nx)?,
            Some(_) => {
                if c == '\'' {
                    q = None;
                } else {
                    s.push(c);
                }
                0
            }
        };
        if c == '\n' {
            s.line += 1;
        }
    }
    if q.is_some() {
        return Err(s.bad("unbalanced quote at end of text"));
    }
    s.end_cmd();
    Ok(s.cmds)
}

/// Commands that re-parse a string as shell are beyond the scanner: refuse.
fn refuse_reparse(cmds: &[(usize, Vec<String>)]) -> Result<(), String> {
    for (line, t) in cmds {
        let w: Vec<&str> = t.iter().map(String::as_str).collect();
        let eval = w.first() == Some(&"eval");
        let dash_c = w
            .windows(2)
            .any(|p| matches!(p[0], "sh" | "bash" | "zsh") && p[1] == "-c");
        if eval || dash_c {
            let what = if eval { "eval" } else { "sh -c" };
            return Err(format!(
                "ci.yml run block, line {}: unsupported {what}",
                line + 1
            ));
        }
    }
    Ok(())
}

/// `[ENV=v ...] cargo [+toolchain] test ...`
fn is_cargo_test(toks: &[&str]) -> bool {
    let mut it = toks
        .iter()
        .skip_while(|t| !t.starts_with('-') && t.contains('='));
    if it.next() != Some(&"cargo") {
        return false;
    }
    let mut rest = it.skip_while(|t| t.starts_with('+'));
    rest.next() == Some(&"test")
}

/// `--test NAME` targets of the `cargo test` commands in one `run:` block.
/// A step naming `--test` that the scanner cannot read PANICS: it never
/// silently drops or invents targets. The pinned GUARDS_RUN literal in
/// falsification_ci_light_jobs_are_hardened.rs remains the backstop.
fn run_targets(run: &str) -> Vec<String> {
    if !run.contains("--test") {
        return Vec::new();
    }
    let cmds = commands(run)
        .and_then(|c| refuse_reparse(&c).map(|()| c))
        .unwrap_or_else(|e| panic!("{e}"));
    let mut out = Vec::new();
    for (_, t) in &cmds {
        let w: Vec<&str> = t.iter().map(String::as_str).collect();
        if is_cargo_test(&w) {
            let args = w.windows(2).filter(|p| p[0] == "--test");
            out.extend(args.map(|p| p[1].to_string()));
        }
    }
    out
}

/// Every `--test NAME` token in every job step's `run:`.
fn wired(text: &str) -> BTreeSet<String> {
    let doc: Value = serde_yaml_ng::from_str(text).expect("ci.yml parses");
    let mut out = BTreeSet::new();
    let jobs = doc["jobs"].as_mapping().expect("jobs mapping");
    for (_, job) in jobs {
        for step in job["steps"].as_sequence().into_iter().flatten() {
            let Some(run) = step["run"].as_str() else {
                continue;
            };
            out.extend(run_targets(run));
        }
    }
    out
}

fn missing(text: &str) -> Vec<&'static str> {
    let have = wired(text);
    REQUIRED
        .iter()
        .copied()
        .filter(|t| !have.contains(*t))
        .collect()
}

/// `text` with every `--test <target>` occurrence deleted (a dropped wiring).
fn drop_target(text: &str, target: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(i) = rest.find("--test") {
        let after = &rest[i + "--test".len()..];
        let tail = after.trim_start_matches([' ', '\t']);
        let hit = tail.strip_prefix(target).filter(|t| {
            let c = t.chars().next();
            !c.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        });
        out.push_str(&rest[..i]);
        match hit {
            Some(t) => rest = t,
            None => {
                out.push_str("--test");
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

#[test]
fn every_858_falsifier_is_wired_into_ci() {
    assert_eq!(missing(&ci_yml()), Vec::<&str>::new(), "not run by ci.yml");
}

#[test]
fn dropping_any_one_target_is_named() {
    let text = ci_yml();
    for t in REQUIRED {
        let mutated = drop_target(&text, t);
        assert_ne!(
            mutated, text,
            "mutant for {t} changed nothing: the probe is vacuous"
        );
        assert_eq!(
            missing(&mutated),
            vec![t],
            "dropping {t} must name exactly {t}"
        );
    }
}

#[test]
fn a_commented_out_target_is_not_wired() {
    let text = ci_yml().replace(
        "--test falsification_budget_dry_run",
        "# --test falsification_budget_dry_run",
    );
    assert!(missing(&text).contains(&"falsification_budget_dry_run"));
}

#[test]
fn every_required_target_exists_on_disk() {
    for t in REQUIRED {
        let p = format!("{}/tests/{t}.rs", env!("CARGO_MANIFEST_DIR"));
        assert!(std::path::Path::new(&p).exists(), "{p} is missing");
    }
}

fn one_step(run_block: &str) -> String {
    let body: String = run_block
        .lines()
        .map(|l| format!("          {l}\n"))
        .collect();
    format!("jobs:\n  j:\n    steps:\n      - run: |\n{body}")
}

/// A mutant must not depend on how ci.yml spaces its arguments, or a reformat
/// turns the probe vacuous (or, here, loud for the wrong reason).
#[test]
fn drop_target_survives_odd_spacing_and_keeps_prefix_siblings() {
    let text = "cargo test \\\n\t--test  foo\t\\\n  --test foo_bar \\\n  --test baz\n";
    let m = drop_target(text, "foo");
    assert_ne!(m, text, "tab / double-space spelling was not removed");
    assert!(m.contains("--test foo_bar"), "a prefix sibling was eaten");
    assert_ne!(
        drop_target(text, "baz"),
        text,
        "last line without backslash"
    );
}

/// `--test X` only counts as wiring when the same step runs `cargo test`.
#[test]
fn a_test_flag_outside_cargo_test_is_not_wired() {
    let echoed = one_step("echo --test falsification_budget_dry_run");
    assert!(wired(&echoed).is_empty(), "echo is not a cargo test run");
    let real = one_step("cargo test --locked --test a # --test b\ncargo test --test c");
    let got: Vec<String> = wired(&real).into_iter().collect();
    assert_eq!(got, ["a", "c"], "comment tail or real target misread");
}

/// If the parser silently lost the steps, `missing` would still report seven
/// absent targets; this pins that the live file yields a populated set.
#[test]
fn the_live_workflow_yields_a_populated_wired_set() {
    let w = wired(&ci_yml());
    assert!(w.contains("examples_validate"), "pre-existing target gone");
    assert!(w.len() >= 25, "only {} targets parsed from ci.yml", w.len());
}

/// Only a logical command that itself is `cargo test` wires a target: an echo
/// in the same step, or a second command after `&&` / `;` / `|`, does not.
#[test]
fn only_the_cargo_test_command_itself_wires_a_target() {
    let echo_after = one_step("cargo test --lib\necho --test x1");
    assert!(wired(&echo_after).is_empty(), "echo line counted");
    let and_then = one_step("cargo test --lib && echo --test x2; true | cat --test x3");
    assert!(wired(&and_then).is_empty(), "chained command counted");
    let cont = one_step("cargo test --locked \\\n  --test a1 \\\n  --test a2");
    assert_eq!(wired(&cont).len(), 2, "continuation not joined");
    let env = one_step("RUST_LOG=x cargo +nightly test --test e1 && cargo fmt --test z");
    assert_eq!(wired(&env).into_iter().collect::<Vec<_>>(), ["e1"]);
}

/// A `#` inside quotes is data, not a comment: it must not hide later `--test`
/// arguments, and a quoted `# --test X` in a non-cargo command is still no wiring.
#[test]
fn a_quoted_hash_is_not_a_comment() {
    let q = one_step("cargo test --test a \"x # y\" --test b 'p #q' --test c");
    assert_eq!(wired(&q).into_iter().collect::<Vec<_>>(), ["a", "b", "c"]);
    let esc = one_step("cargo test --test a \\# --test b # --test z");
    assert_eq!(wired(&esc).into_iter().collect::<Vec<_>>(), ["a", "b"]);
    let tail = one_step("cargo test --test a # \"--test z\"");
    assert_eq!(wired(&tail).into_iter().collect::<Vec<_>>(), ["a"]);
    let echoed = one_step("echo \"# --test x\"");
    assert!(wired(&echoed).is_empty(), "echo is not cargo test");
}

fn w(run: &str) -> Vec<String> {
    wired(&one_step(run)).into_iter().collect()
}

/// `;#` starts a comment: what follows on that line is not live code.
#[test]
fn a_hash_right_after_a_separator_is_a_comment() {
    assert!(w("cargo test --lib;# c; cargo test --test X").is_empty());
    assert!(w("cargo test --lib &# c\ncargo fmt --test Y").is_empty());
}

/// Quote state survives a newline: a `#` inside a multi-line string is data.
#[test]
fn quote_state_carries_across_lines() {
    assert_eq!(w("cargo test --test a \"x\n# y\" --test c"), ["a", "c"]);
}

/// Separators inside quotes do not split; a bare `&` does.
#[test]
fn only_unquoted_separators_split_commands() {
    assert_eq!(w("cargo test --test a \"p;q\" --test b"), ["a", "b"]);
    assert!(w("cargo test --lib & echo --test x").is_empty());
}

// Fail closed: a construct the scanner does not model must stop the test, never
// silently drop or invent targets. (The pinned GUARDS_RUN literal in
// falsification_ci_light_jobs_are_hardened.rs stays the backstop for the step.)
#[test]
#[should_panic(expected = "$'")]
fn ansi_c_quote_is_refused() {
    w("cargo test --test a $'it\\'s' --test b");
}
#[test]
#[should_panic(expected = "heredoc")]
fn heredoc_is_refused() {
    w("cargo test --test a\ncat <<EOF\nx\nEOF");
}
#[test]
#[should_panic(expected = "backtick")]
fn backtick_is_refused() {
    w("cargo test --test a `echo b`");
}
#[test]
#[should_panic(expected = "$(")]
fn command_substitution_is_refused() {
    w("cargo test --test a \"$(echo b)\"");
}
#[test]
#[should_panic(expected = "eval")]
fn eval_is_refused() {
    w("cargo test --test a\neval \"cargo test --test b\"");
}
#[test]
#[should_panic(expected = "sh -c")]
fn shell_dash_c_is_refused() {
    w("cargo test --test a\nbash -c 'cargo test --test b'");
}
#[test]
#[should_panic(expected = "unbalanced quote")]
fn unbalanced_quote_is_refused() {
    w("cargo test --test a \"b");
}
