//! forjar#417: the count of sites still on the untyped error path, held by a
//! ratchet.
//!
//! # Why this exists
//!
//! `contracts/verb-surface-v1.yaml` (`error_taxonomy_is_total`) allows a
//! prose-matching fallback only as "a NAMED, documented, deliberately-temporary
//! path with a count of the sites still on it". The path is named
//! (`legacy_prose_class` in `src/core/error.rs`); the count did not exist. Every
//! error that leaves a `Result<_, String>` reaches the exit code through that
//! fallback unless it carries a declared marker, and it guesses wrong: on
//! forjar 1.33.0 `forjar undo --resume` with nothing to resume exits 2, the
//! partial-apply class, because its message contains the word "partial".
//!
//! `scripts/ratchets/untyped-error-sites.json` records two ceilings:
//!
//! - `untyped_error_fns`: non-test `fn` signatures under `src/` that return
//!   `Result<_, String>`. Each one is a boundary an error crosses without a
//!   class.
//! - `lock_literals_outside_state`: non-comment, non-test lines under `src/`
//!   outside `src/core/state/` that name `state.lock.yaml`. The issue's target
//!   for these is 0: lock reads belong to the state module.
//!
//! One more site than the ceiling is a REGRESSION. Fewer is green: that is the
//! migration the ratchet exists to let through, and whoever lands it lowers
//! the ceiling. A scan that saw almost no files is UNMEASURED and red, not a
//! count of zero. A ceiling of 0 for `untyped_error_fns` with
//! `legacy_prose_class` still defined, or the reverse, is red: the last
//! migrated site deletes the fallback.
//!
//! # What the predicates see
//!
//! Single-line signatures only; a signature split across lines is not
//! counted. Test code is skipped by path (`tests/`, `tests.rs`, `*_tests.rs`,
//! `test_*.rs`) and by stopping at a file's first `#[cfg(test)]`. Both limits
//! make the count a floor, not the exact number, and both are pinned by the
//! planted tree below so a change to either is a visible change.
#![cfg(unix)]

use std::path::{Path, PathBuf};

const BASELINE: &str = "scripts/ratchets/untyped-error-sites.json";
/// Fewer `.rs` files than this under `src/` means the walk is broken.
const MIN_FILES: usize = 100;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[derive(Debug, Default, PartialEq)]
struct Counts {
    files: usize,
    untyped_error_fns: usize,
    lock_literals_outside_state: usize,
}

fn is_test_file(rel: &str) -> bool {
    let name = rel.rsplit('/').next().unwrap_or(rel);
    rel.contains("/tests/")
        || name == "tests.rs"
        || name.ends_with("_tests.rs")
        || name.ends_with("_test.rs")
        || name.starts_with("test_")
        || name.starts_with("tests_")
}

/// A one-line `fn` signature returning `Result<_, String>`.
fn is_untyped_error_fn(line: &str) -> bool {
    let mut t = line.trim_start();
    if let Some(rest) = t.strip_prefix("pub(") {
        t = rest.split_once(") ").map_or("", |(_, r)| r);
    }
    for kw in ["pub ", "const ", "async ", "unsafe "] {
        t = t.strip_prefix(kw).unwrap_or(t);
    }
    t.starts_with("fn ") && t.contains("-> Result<") && t.contains(", String>")
}

fn is_lock_literal(line: &str) -> bool {
    !line.trim_start().starts_with("//") && line.contains("state.lock.yaml")
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// Count both kinds of site under `root/src`.
fn measure(root: &Path) -> Counts {
    let mut files = Vec::new();
    walk(&root.join("src"), &mut files);
    let mut c = Counts::default();
    for f in files {
        let rel = f
            .strip_prefix(root)
            .expect("walked under root")
            .to_string_lossy()
            .replace('\\', "/");
        c.files += 1;
        if is_test_file(&rel) {
            continue;
        }
        let in_state = rel.starts_with("src/core/state/");
        let body = std::fs::read_to_string(&f).unwrap_or_default();
        for line in body.lines() {
            if line.trim() == "#[cfg(test)]" {
                break;
            }
            if is_untyped_error_fn(line) {
                c.untyped_error_fns += 1;
            }
            if !in_state && is_lock_literal(line) {
                c.lock_literals_outside_state += 1;
            }
        }
    }
    c
}

fn ceiling(key: &str) -> usize {
    let body = std::fs::read_to_string(repo().join(BASELINE)).expect("the baseline is committed");
    let v: serde_json::Value = serde_json::from_str(&body).expect("the baseline is JSON");
    let n = v["ceiling"][key]
        .as_u64()
        .unwrap_or_else(|| panic!("{BASELINE}: ceiling.{key} is missing or not an integer"));
    usize::try_from(n).expect("ceiling fits usize")
}

#[test]
fn the_live_tree_holds_both_ceilings() {
    let c = measure(&repo());
    eprintln!("measured: {c:?}");
    assert!(
        c.files >= MIN_FILES,
        "UNMEASURED: the walk saw {} .rs files under src/, want at least {MIN_FILES}",
        c.files
    );
    for (key, got) in [
        ("untyped_error_fns", c.untyped_error_fns),
        ("lock_literals_outside_state", c.lock_literals_outside_state),
    ] {
        let max = ceiling(key);
        assert!(
            got <= max,
            "REGRESSION: {key} = {got}, ceiling {max} in {BASELINE}. Return a \
             ForjarError, or read the lock through src/core/state/, instead of \
             adding a site."
        );
        if got < max {
            eprintln!("{key}: {got} < ceiling {max}; lower the ceiling in {BASELINE}");
        }
    }
}

#[test]
fn the_fallback_and_the_count_reach_zero_together() {
    let error_rs = std::fs::read_to_string(repo().join("src/core/error.rs")).expect("error.rs");
    let fallback = error_rs.contains("fn legacy_prose_class(");
    let max = ceiling("untyped_error_fns");
    assert_eq!(
        fallback,
        max > 0,
        "legacy_prose_class defined = {fallback}, untyped_error_fns ceiling = {max}: \
         the last migrated site deletes the fallback, and the fallback stays until then"
    );
}

#[test]
fn the_predicates_discriminate() {
    for yes in [
        "fn f() -> Result<(), String> {",
        "    pub fn load(p: &Path) -> Result<Config, String> {",
        "pub(crate) fn cmd_undo(x: u8) -> Result<(), String> {",
        "pub(super) async fn go() -> Result<Vec<String>, String> {",
        "pub const fn c() -> Result<u8, String> {",
    ] {
        assert!(is_untyped_error_fn(yes), "should count: {yes}");
    }
    for no in [
        "fn f() -> Result<(), ForjarError> {",
        "fn f() -> Result<String, ForjarError> {",
        "// fn f() -> Result<(), String> {",
        "let r: Result<(), String> = Ok(());",
        "fn f() -> Option<String> {",
    ] {
        assert!(!is_untyped_error_fn(no), "should not count: {no}");
    }
    assert!(is_lock_literal(r#"let p = dir.join("state.lock.yaml");"#));
    assert!(!is_lock_literal("    // reads state.lock.yaml"));
    assert!(!is_lock_literal("    /// reads state.lock.yaml"));
    for t in [
        "src/a/tests/x.rs",
        "src/a/tests.rs",
        "src/a/b_tests.rs",
        "src/a/test_b.rs",
    ] {
        assert!(is_test_file(t), "should skip: {t}");
    }
    assert!(!is_test_file("src/core/testing.rs"));
}

/// A planted tree with one site of each kind that counts and one of each
/// exclusion. If an exclusion stops working, or the walk stops descending, the
/// totals move and this goes red.
#[test]
fn a_planted_tree_counts_exactly_its_sites() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let put = |rel: &str, body: &str| {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().expect("parent")).expect("mkdir");
        std::fs::write(p, body).expect("write");
    };
    put(
        "src/cli/deep/a.rs",
        "pub(crate) fn a() -> Result<(), String> {\n    std::fs::read(\"state.lock.yaml\");\n    // state.lock.yaml in a comment\n}\n",
    );
    put(
        "src/core/state/b.rs",
        "fn b() -> Result<(), String> { std::fs::read(\"state.lock.yaml\"); }\n",
    );
    put(
        "src/x/tests.rs",
        "fn t() -> Result<(), String> { Ok(()) }\n",
    );
    put(
        "src/c.rs",
        "fn ok() -> Result<(), ForjarError> { Ok(()) }\n#[cfg(test)]\nmod tests {\n    fn t() -> Result<(), String> { Ok(()) }\n    const L: &str = \"state.lock.yaml\";\n}\n",
    );
    assert_eq!(
        measure(root),
        Counts {
            files: 4,
            // a.rs and b.rs; tests.rs and the cfg(test) block are skipped.
            untyped_error_fns: 2,
            // a.rs only; b.rs is inside src/core/state/, the comment is skipped.
            lock_literals_outside_state: 1,
        }
    );
}
