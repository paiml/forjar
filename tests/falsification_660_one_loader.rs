//! forjar#660 round 4: every `cmd_*` that takes a manifest loads it through the
//! ONE resolving loader, found by structure.
//!
//! The scan walks every `fn cmd_*` of `src/` whose signature takes a `&Path`,
//! prints how many it looked at and how each one is classified:
//!   LOADER    names a function that ends in the resolving loader;
//!   BYPASS    names a function that parses or resolves by hand;
//!   DELEGATE  names neither (it hands the path on, or does not read a manifest).
//! A BYPASS with no allowlist row is red. A row that no longer matches is red.
//! A planted mutant (a `cmd_*` that calls the unresolved parser) must go red,
//! or the scan proves nothing.

mod common_660;

use common_660::rust_scan::*;
use std::collections::BTreeSet;
use std::path::Path;

/// Functions that load a manifest and resolve its `data:` block.
const LOADERS: &[&str] = &[
    "load_resolved_config_for_render",
    "parse_and_validate_for_render(",
    "load_resolved_config",
    "load_resolved_config_with",
    "load_resolved_config_lenient",
    "load_resolved_value",
    "parse_and_validate(",
    "load_apply_config(",
    "prepare_config(",
    "load_for_contracts(",
];

/// Functions that read a manifest WITHOUT resolving it, or resolve by hand.
const BYPASSES: &[&str] = &[
    "parse_and_validate_unresolved(",
    "resolve_data_sources_with_base(",
    "parse_config_file(",
    "parse_config(",
];

/// `path:fn` of a `cmd_*` that may bypass the loader, with the reason.
const BYPASS_ALLOWLIST: &[(&str, &str)] = &[
    (
        "src/cli/complexity_analysis.rs:cmd_complexity",
        "static analysis of the manifest as written; a data value is not complexity",
    ),
    (
        "src/cli/config_merge.rs:cmd_config_merge",
        "writes a MANIFEST: it merges the unresolved files (data: included) and the output validates",
    ),
    (
        "src/cli/extract.rs:cmd_extract",
        "writes a MANIFEST subset: resolving first would bake data values into the output",
    ),
    (
        "src/cli/init.rs:cmd_fmt",
        "rewrites the file in place: it must keep the references, not their values",
    ),
];

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Kind {
    Loader,
    Bypass,
    Delegate,
}

fn classify(body: &str) -> Kind {
    if BYPASSES.iter().any(|t| body.contains(t)) {
        Kind::Bypass
    } else if LOADERS.iter().any(|t| body.contains(t)) {
        Kind::Loader
    } else {
        Kind::Delegate
    }
}

/// `(key, kind)` for every `cmd_*` taking a `&Path` in one file's code.
fn scan_source(rel: &str, code: &str) -> Vec<(String, Kind)> {
    let spans = fn_spans(code);
    let mut out = Vec::new();
    for (start, end, name) in &spans {
        if !name.starts_with("cmd_") {
            continue;
        }
        // The signature is the text between `fn name` and the body's `{`.
        let head_from = code[..*start].rfind(&format!("fn {name}")).unwrap_or(0);
        let signature = &code[head_from..*start];
        if !signature.contains("&Path") && !signature.contains("&std::path::Path") {
            continue;
        }
        out.push((format!("{rel}:{name}"), classify(&code[*start..*end])));
    }
    out
}

fn scan_tree() -> Vec<(String, Kind)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    rust_files(&root.join("src"), &mut files);
    let test_only = test_only_files(&files);
    let mut out = Vec::new();
    for f in &files {
        if test_only.contains(f) {
            continue;
        }
        let rel = f
            .strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let code = production_code(&std::fs::read_to_string(f).expect("read"));
        out.extend(scan_source(&rel, &code));
    }
    out
}

fn verdict(found: &[(String, Kind)]) -> Vec<String> {
    let allowed: BTreeSet<&str> = BYPASS_ALLOWLIST.iter().map(|(k, _)| *k).collect();
    let bypasses: BTreeSet<&str> = found
        .iter()
        .filter(|(_, k)| *k == Kind::Bypass)
        .map(|(k, _)| k.as_str())
        .collect();
    let mut problems = Vec::new();
    for k in bypasses.difference(&allowed) {
        problems.push(format!(
            "BYPASS `{k}` reads a manifest without the one loader"
        ));
    }
    for k in allowed.difference(&bypasses) {
        problems.push(format!("STALE row `{k}` matches no bypass"));
    }
    problems
}

#[test]
fn every_file_command_loads_through_the_one_loader() {
    let found = scan_tree();
    let count = |k: Kind| found.iter().filter(|(_, x)| *x == k).count();
    println!(
        "scanned {} cmd_* fns taking a &Path: {} loader, {} bypass, {} delegate",
        found.len(),
        count(Kind::Loader),
        count(Kind::Bypass),
        count(Kind::Delegate)
    );
    for (k, kind) in &found {
        println!("  {kind:?} {k}");
    }
    assert!(
        found.len() >= 40,
        "the scan saw only {} commands; it is blind",
        found.len()
    );
    assert!(count(Kind::Loader) >= 20, "too few loader commands found");
    let problems = verdict(&found);
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// The planted mutant: a command that bypasses the loader must be red.
#[test]
fn a_planted_bypass_goes_red() {
    let mutant = "pub(crate) fn cmd_planted(file: &Path) -> Result<(), String> {\n    \
                  let config = parse_and_validate_unresolved(file)?;\n    Ok(())\n}\n";
    let found = scan_source("src/cli/planted.rs", &production_code(mutant));
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].1, Kind::Bypass);
    assert!(
        verdict(&found).iter().any(|p| p.contains("BYPASS")),
        "the planted bypass did not go red"
    );
    let clean = "pub(crate) fn cmd_clean(file: &Path) -> Result<(), String> {\n    \
                 let config = parse_and_validate(file)?;\n    Ok(())\n}\n";
    let found = scan_source("src/cli/clean.rs", &production_code(clean));
    assert_eq!(found[0].1, Kind::Loader);
}
