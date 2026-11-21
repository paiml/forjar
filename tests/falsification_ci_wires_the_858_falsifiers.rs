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

/// A shell line up to its first UNQUOTED `#` that starts a comment (line start
/// or after a blank). Single and double quotes and backslash escapes are tracked.
fn strip_comment(line: &str) -> &str {
    let mut quote: Option<char> = None;
    let mut prev_blank = true;
    let mut chars = line.char_indices();
    while let Some((i, c)) = chars.next() {
        match (quote, c) {
            (None, '#') if prev_blank => return &line[..i],
            (None, '\\') | (Some('"'), '\\') => {
                chars.next();
            }
            (None, '"' | '\'') => quote = Some(c),
            (Some(q), _) if q == c => quote = None,
            _ => {}
        }
        prev_blank = c.is_ascii_whitespace();
    }
    line
}

/// Shell `run:` text as logical commands: comments dropped, backslash
/// continuations joined, split on newline `;` `&&` `||` `|`.
fn logical_commands(run: &str) -> Vec<String> {
    let mut joined = Vec::new();
    let mut cur = String::new();
    for line in run.lines() {
        let code = strip_comment(line).trim_end();
        match code.strip_suffix('\\') {
            Some(head) => {
                cur.push_str(head);
                cur.push(' ');
            }
            None => {
                cur.push_str(code);
                joined.push(std::mem::take(&mut cur));
            }
        }
    }
    joined.push(cur);
    let mut out = Vec::new();
    for l in joined {
        let l = l.replace("&&", ";").replace("||", ";").replace('|', ";");
        out.extend(l.split(';').map(str::to_string));
    }
    out
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
fn run_targets(run: &str) -> Vec<String> {
    let cmds = logical_commands(run);
    let toks = cmds
        .iter()
        .map(|c| c.split_whitespace().collect::<Vec<_>>());
    toks.filter(|t| is_cargo_test(t))
        .flat_map(|t| {
            t.windows(2)
                .filter(|w| w[0] == "--test")
                .map(|w| w[1].to_string())
                .collect::<Vec<_>>()
        })
        .collect()
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
