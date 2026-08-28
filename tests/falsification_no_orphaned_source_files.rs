//! GH-292: a `.rs` file checked into `src/` that rustc never compiles.
//!
//! Nothing in the build or the test suite asserted that a source file is
//! reachable from the crate root. rustc compiles only what a `mod`, a
//! `#[path]` or an `include!` names, so a file that loses — or never gains —
//! its declaration goes silently dark: it stops being type-checked, it stops
//! being linted, and if it holds tests they stop running. It still counts as
//! source to every reader and to every external tool.
//!
//! Three files were in that state on main:
//!
//! * `src/cli/commands/status_args_ext.rs` — a bare struct-body fragment left
//!   by a mechanical file split. It does not parse as Rust at all
//!   (`visibility 'pub' is not followed by an item`), which is how the sweep
//!   in GH-292 found it. Every one of its 101 fields already existed in
//!   `status_args.rs`.
//! * `src/core/planner/tests_proof_obligation.rs` — 12 tests superseded by the
//!   28 in `tests_proof_cov.rs`, which IS declared.
//! * `src/core/planner/tests_sat_deps_b.rs` — 10 tests on the SAT solver added
//!   by a commit whose message claimed SAT-solver coverage. The `mod` line was
//!   never written, so that coverage was never delivered.
//!
//! And two more were the same file compiled twice: `tests_container_b.rs` and
//! `tests_container_c.rs` under `src/transport/` were byte-identical, both
//! declared, so eleven container tests built and ran twice per `cargo test`
//! while looking like twenty-two.
//!
//! The reachability rule below is deliberately exact rather than convenient.
//! A naive "grep the whole tree for the stem" is too loose and a naive
//! "only look at the sibling mod.rs" reports nine false positives, because
//! this crate legitimately uses all three declaration forms: `mod x;` in a
//! sibling `mod.rs`, `#[path = "x.rs"]` from an arbitrary sibling, and an
//! `include!` chain (`src/cli/mod.rs` hides roughly 250 `mod` lines behind
//! three `mod_test_decl*.rs` fragments). All three are honoured here.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// File stems that are declaration sites themselves, not declared items.
const ROOT_STEMS: &[&str] = &["mod", "lib", "main"];

fn src_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// Every `.rs` file under `root`, recursively.
///
/// `.pmat` is skipped: pmat leaves an analysis cache on disk under `src/`
/// which is not source and is not git-tracked.
fn rs_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in entries.filter_map(Result::ok) {
            let p = e.path();
            if p.is_dir() {
                if p.file_name().is_some_and(|n| n == ".pmat") {
                    continue;
                }
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

/// The `x.rs` targets of every `include!("x.rs")` in `text`.
fn include_targets(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for chunk in text.split("include!(").skip(1) {
        let Some(rest) = chunk.trim_start().strip_prefix('"') else {
            continue;
        };
        let Some(end) = rest.find('"') else { continue };
        out.push(rest[..end].to_string());
    }
    out
}

/// `path`'s text with every `include!` expanded, transitively.
///
/// Load-bearing: without it `src/cli/` reports ~250 false orphans, because the
/// `mod` lines that declare them live in files reached only by `include!`.
fn read_expanded(path: &Path) -> String {
    let mut text = String::new();
    let mut seen: Vec<PathBuf> = Vec::new();
    let mut stack = vec![path.to_path_buf()];
    while let Some(p) = stack.pop() {
        if seen.contains(&p) {
            continue;
        }
        let Ok(body) = std::fs::read_to_string(&p) else {
            continue;
        };
        let dir = p.parent().map(Path::to_path_buf).unwrap_or_default();
        for target in include_targets(&body) {
            stack.push(dir.join(target));
        }
        seen.push(p);
        text.push_str(&body);
        text.push('\n');
    }
    text
}

/// Everything that could legally declare a module living in `dir`.
///
/// That is every `.rs` file sitting in `dir` (a sibling may carry the
/// `#[path]` attribute or the `mod` line) plus `dir.rs` itself, which is the
/// 2018-edition form — e.g. `src/core/state/process_lock.rs` parents
/// `src/core/state/process_lock/tests.rs`.
fn declaration_text(dir: &Path, exclude: &Path) -> String {
    let mut parents: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|x| x == "rs"))
        .filter(|p| p != exclude)
        .collect();
    let as_file = dir.with_extension("rs");
    if as_file.is_file() {
        parents.push(as_file);
    }
    parents.sort();
    parents.iter().map(|p| read_expanded(p)).collect()
}

/// Does `decl` name `stem` as a module, by any of the three forms rustc honours?
fn is_declared(decl: &str, stem: &str) -> bool {
    decl.contains(&format!("mod {stem};"))
        || decl.contains(&format!("mod {stem} {{"))
        // Covers both `#[path = "stem.rs"]` and `include!("stem.rs")`.
        || decl.contains(&format!("\"{stem}.rs\""))
}

#[test]
fn every_source_file_is_declared_by_its_parent_module() {
    let src = src_dir();
    let mut cache: HashMap<PathBuf, String> = HashMap::new();
    let mut orphans = Vec::new();

    for path in rs_files(&src) {
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        if ROOT_STEMS.contains(&stem.as_ref()) {
            continue;
        }
        let dir = path.parent().expect("a file has a parent").to_path_buf();
        // Cached per (dir, file) is wrong — the excluded file differs per
        // candidate — but a file never declares itself, so keying on the dir
        // and excluding nothing would be equivalent. Exclude self anyway so a
        // stray self-mention cannot vouch for the file.
        let decl = cache
            .entry(dir.clone())
            .or_insert_with(|| declaration_text(&dir, Path::new("")))
            .clone();
        if is_declared(&decl, &stem) {
            continue;
        }
        // Re-check without the file itself in the pool, in case it named
        // its own stem in a comment.
        let strict = declaration_text(&dir, &path);
        if is_declared(&strict, &stem) {
            continue;
        }
        orphans.push(format!(
            "{} is compiled by nothing — no `mod {stem};`, `#[path]` or \
             `include!` in {}",
            path.display(),
            dir.display()
        ));
    }

    assert!(
        orphans.is_empty(),
        "source files in src/ that rustc never sees. They are not linted, not \
         type-checked, and any tests in them do not run — declare them or \
         delete them:\n  {}",
        orphans.join("\n  ")
    );
}

#[test]
fn there_are_source_files_to_check() {
    // Guards the guard. If the walk above ever matched nothing — a renamed
    // directory, a broken `read_dir` — every orphan would pass by not being
    // looked at, which is the exact vacuous-green shape GH-292 is about.
    let n = rs_files(&src_dir()).len();
    assert!(
        n > 1000,
        "expected the src/ walk to find over a thousand .rs files, found {n} \
         — the walk is broken, not the tree"
    );
}

#[test]
fn no_two_source_files_are_byte_identical() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut seen: HashMap<Vec<u8>, PathBuf> = HashMap::new();
    let mut twins = Vec::new();

    let mut all = rs_files(&root.join("src"));
    all.extend(rs_files(&root.join("tests")));
    for path in all {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        if let Some(first) = seen.insert(bytes, path.clone()) {
            twins.push(format!("{} == {}", first.display(), path.display()));
        }
    }

    assert!(
        twins.is_empty(),
        "these files are byte-identical. If both are declared the same tests \
         build and run twice while the count reads as twice the coverage; if \
         one is not, it is dead weight. Keep one:\n  {}",
        twins.join("\n  ")
    );
}
