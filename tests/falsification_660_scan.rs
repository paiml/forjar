//! forjar#660: ONE loader, found by structure, deny by default.
//!
//! Three scans over `src/`, each keyed `path:fn` with a written reason:
//!   1. a raw YAML parse of a manifest outside the loader module;
//!   2. a call to the UNRESOLVED loader;
//!   3. a load/resolve result that is swallowed (`.ok()`, `.unwrap_or*`, `let _ =`).
//!
//! A hit with no row is RED; a row with no hit is RED (stale). The scanner is
//! `common_660::rust_scan`: it blanks comments and literals, blanks the body of
//! every `#[cfg(test)]` item by brace matching, and names the enclosing `fn`.
//! The previous scan stopped at the first `#[cfg(test)]` line, so a test helper
//! in the middle of a file hid every production line after it.

mod common_660;

use common_660::lists::*;
use common_660::rust_scan::*;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// The loader module: the parser, and the resolver files that wrap it.
fn is_loader_module(rel: &str) -> bool {
    rel.starts_with("src/core/parser/")
        || rel == "src/core/resolver/data.rs"
        || rel == "src/core/resolver/data_subst.rs"
}

type Spans = Vec<(usize, usize, String)>;
type Scanned = (String, String, Spans);

/// `(rel path, production code, fn spans)` for every non-test file in `src/`.
struct Tree {
    scanned: Vec<Scanned>,
    total: usize,
}

fn tree() -> Tree {
    tree_at(Path::new(env!("CARGO_MANIFEST_DIR")))
}

/// What is a test file is decided by structure: the target of a `#[cfg(test)]
/// mod`, never a file NAME. `src/cli/check_test.rs` is production code.
fn tree_at(root: &Path) -> Tree {
    let mut files = Vec::new();
    rust_files(&root.join("src"), &mut files);
    let test_only = test_only_files(&files);
    let mut scanned = Vec::new();
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
        let spans = fn_spans(&code);
        scanned.push((rel, code, spans));
    }
    Tree {
        scanned,
        total: files.len(),
    }
}

/// path:fn of every raw manifest parse in `rel`'s production code.
fn raw_parse_hits(rel: &str, code: &str, spans: &[(usize, usize, String)]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    if is_loader_module(rel) {
        return out;
    }
    for pat in ["serde_yaml_ng::from_", "serde_yaml::from_"] {
        let mut at = 0;
        while let Some(p) = code[at..].find(pat) {
            let pos = at + p;
            at = pos + pat.len();
            let st = statement_at(code, pos);
            // A manifest, a generic document, or a type nobody wrote down. A
            // visibly typed non-manifest (a lock, a log line) is not a manifest.
            let manifest = st.contains("ForjarConfig") || st.contains("Value");
            let typed = st.contains("::<") || (st.contains(": ") && st.contains(" = "));
            if manifest || !typed {
                out.insert(format!("{rel}:{}", enclosing_fn(spans, pos)));
            }
        }
    }
    out
}

const UNRESOLVED_CALLS: &[&str] = &[
    "parse_and_validate_unresolved(",
    "parse_and_validate_opts(",
    "parser::parse_and_validate(",
    "parser::parse_config(",
    "parser::parse_config_file(",
];

fn unresolved_loader_hits(
    rel: &str,
    code: &str,
    spans: &[(usize, usize, String)],
) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    if is_loader_module(rel) {
        return out;
    }
    for pat in UNRESOLVED_CALLS {
        let mut at = 0;
        while let Some(p) = code[at..].find(pat) {
            let pos = at + p;
            at = pos + pat.len();
            if code[..pos].trim_end().ends_with("fn") {
                continue;
            }
            out.insert(format!("{rel}:{}", enclosing_fn(spans, pos)));
        }
    }
    out
}

/// Names of every `fn load_*` / `prepare_*` in the resolver and the CLI.
fn derive_loaders(files: &[Scanned]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for (rel, _code, spans) in files {
        if !(rel.starts_with("src/core/resolver/") || rel.starts_with("src/cli/")) {
            continue;
        }
        for (_, _, name) in spans {
            if name.starts_with("load_") || name.starts_with("prepare_") {
                out.insert(name.clone());
            }
        }
    }
    out
}

/// Loader fns with no classification, and classifications with no fn.
fn unclassified_loaders(derived: &BTreeSet<String>) -> Vec<String> {
    let known: BTreeSet<&str> = MANIFEST_LOADERS
        .iter()
        .copied()
        .chain(NOT_A_MANIFEST_LOADER.iter().map(|(n, _)| *n))
        .collect();
    let mut bad = Vec::new();
    for d in derived {
        if !known.contains(d.as_str()) {
            bad.push(format!(
                "loader fn `{d}` is in neither MANIFEST_LOADERS nor NOT_A_MANIFEST_LOADER"
            ));
        }
    }
    for k in &known {
        if !derived.contains(*k) {
            bad.push(format!("STALE loader row `{k}` matches no fn"));
        }
    }
    bad
}

fn swallow_hits(rel: &str, code: &str, spans: &[(usize, usize, String)]) -> BTreeSet<String> {
    swallow_hits_for(rel, code, spans, MANIFEST_LOADERS)
}

fn swallow_hits_for(
    rel: &str,
    code: &str,
    spans: &[(usize, usize, String)],
    loaders: &[&str],
) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for tok in BASE_TOKENS.iter().chain(loaders) {
        for (pos, _why) in swallowed_results(code, tok) {
            out.insert(format!("{rel}:{}", enclosing_fn(spans, pos)));
        }
    }
    out
}

fn judge(kind: &str, hits: &BTreeSet<String>, allow: &[(&str, &str)]) -> Vec<String> {
    let mut bad = Vec::new();
    let allowed: BTreeMap<&str, &str> = allow.iter().copied().collect();
    assert_eq!(
        allowed.len(),
        allow.len(),
        "{kind}: duplicate allowlist rows"
    );
    for (key, why) in allow {
        if why.trim().is_empty() {
            bad.push(format!("{kind}: `{key}` has no reason"));
        }
    }
    for h in hits {
        if !allowed.contains_key(h.as_str()) {
            bad.push(format!("{kind}: NEW `{h}` is not allowlisted"));
        }
    }
    for key in allowed.keys() {
        if !hits.contains(*key) {
            bad.push(format!("{kind}: STALE row `{key}` matches nothing"));
        }
    }
    bad
}

fn collect(
    f: impl Fn(&str, &str, &[(usize, usize, String)]) -> BTreeSet<String>,
) -> BTreeSet<String> {
    let t = tree();
    // Count first: "0 violations" over 0 files is not a pass.
    assert!(
        !t.scanned.is_empty() && t.scanned.len() < t.total,
        "scanned {} of {} files",
        t.scanned.len(),
        t.total
    );
    t.scanned
        .iter()
        .flat_map(|(rel, code, spans)| f(rel, code, spans))
        .collect()
}

#[test]
fn no_raw_manifest_parse_outside_the_loader() {
    let hits = collect(raw_parse_hits);
    let bad = judge("raw parse", &hits, RAW_PARSE_ALLOWLIST);
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
fn unresolved_loader_callers_are_allowlisted() {
    let hits = collect(unresolved_loader_hits);
    let bad = judge("unresolved loader", &hits, UNRESOLVED_LOADER_ALLOWLIST);
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
fn no_load_or_resolve_result_is_swallowed() {
    let hits = collect(swallow_hits);
    let bad = judge("swallowed result", &hits, SWALLOW_ALLOWLIST);
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

fn planted(src: &str) -> (String, Vec<(usize, usize, String)>) {
    let code = production_code(src);
    let spans = fn_spans(&code);
    (code, spans)
}

/// The scanner must be able to FAIL, in the shapes the tree uses, and must see
/// production code that comes AFTER a `#[cfg(test)]` item.
#[test]
fn the_scans_go_red_on_planted_code() {
    for (name, src) in [
        ("typed let", "fn f(s: &str) {\n    let c: ForjarConfig = serde_yaml_ng::from_str(s).unwrap();\n}\n"),
        ("turbofish", "fn f(s: &str) {\n    let c = serde_yaml_ng::from_str::<types::ForjarConfig>(s).unwrap();\n}\n"),
        ("untyped", "fn f(s: &str) {\n    let c = serde_yaml_ng::from_str(s).unwrap();\n}\n"),
        ("serde_yaml", "fn f(s: &str) {\n    let c: Value = serde_yaml::from_str(s).unwrap();\n}\n"),
        (
            "after a cfg(test) helper",
            "#[cfg(test)]\nfn helper() -> u8 {\n    let x = '{';\n    1\n}\nfn f(s: &str) {\n    let c: ForjarConfig = serde_yaml_ng::from_str(s).unwrap();\n}\n",
        ),
        (
            "after a cfg(test) mod",
            "#[cfg(test)]\nmod tests {\n    fn t() { let s = \"}\"; }\n}\nfn f(s: &str) {\n    let c: Value = serde_yaml_ng::from_str(s).unwrap();\n}\n",
        ),
    ] {
        let (code, spans) = planted(src);
        let hits = raw_parse_hits("src/cli/planted.rs", &code, &spans);
        assert_eq!(hits.len(), 1, "scan missed the planted `{name}` parse: {hits:?}");
        assert!(!judge("t", &hits, &[]).is_empty(), "`{name}` was accepted");
    }
    let (code, spans) = planted("fn f() {\n    let c = parse_and_validate_unresolved(file)?;\n}\n");
    assert_eq!(
        unresolved_loader_hits("src/cli/p.rs", &code, &spans).len(),
        1
    );
    for src in [
        "fn f() { let c = load_resolved_config(p).ok(); }",
        "fn f() { let c = parse_and_validate(p).unwrap_or_default(); }",
        "fn f() { let _ = parse_and_validate(p); }",
        "fn f() { if let Ok(c) = load_resolved_config(p) { } }",
        "fn f() { let c = resolve_resource_templates(r, a, b, c).map_err(|e| e).ok(); }",
    ] {
        let (code, spans) = planted(src);
        assert_eq!(
            swallow_hits("src/cli/p.rs", &code, &spans).len(),
            1,
            "{src}"
        );
    }
}

/// And it must not flag what is not production code or not a swallow.
#[test]
fn the_scans_ignore_tests_comments_strings_and_propagation() {
    for src in [
        "#[cfg(test)]\nmod tests {\n    fn t() { let c: ForjarConfig = serde_yaml_ng::from_str(s).unwrap(); }\n}\n",
        "#[cfg(test)]\nfn t() {\n    let c: ForjarConfig = serde_yaml_ng::from_str(s).unwrap();\n}\n",
        "// let c: ForjarConfig = serde_yaml_ng::from_str(s).unwrap();\nfn f() {}\n",
        "fn f() { let s = \"serde_yaml_ng::from_str::<ForjarConfig>(x)\"; }\n",
        "fn f() { let s = r#\"serde_yaml_ng::from_str::<Value>(x)\"#; }\n",
        "fn f() { let l: StateLock = serde_yaml_ng::from_str(s).unwrap(); }\n",
    ] {
        let (code, spans) = planted(src);
        assert!(raw_parse_hits("src/cli/p.rs", &code, &spans).is_empty(), "{src}");
    }
    let (code, spans) =
        planted("fn f() { let c: ForjarConfig = serde_yaml_ng::from_str(s).unwrap(); }");
    assert!(raw_parse_hits("src/core/parser/x.rs", &code, &spans).is_empty());
    for src in [
        "fn f() { let c = load_resolved_config(p)?; }",
        "fn f() { let _ = parse_and_validate(p)?; }",
        "fn f() { let c = parse_and_validate(p).map_err(|e| e)?; }",
        "#[cfg(test)]\nfn t() { let c = load_resolved_config(p).ok(); }",
    ] {
        let (code, spans) = planted(src);
        assert!(
            swallow_hits("src/cli/p.rs", &code, &spans).is_empty(),
            "{src}"
        );
    }
}

// ---- round 5 --------------------------------------------------------------

/// A loader fn nobody classified is RED, so a new `load_*` cannot dodge the
/// swallow scan by not being in a hand-kept list.
#[test]
fn every_load_or_prepare_fn_is_classified() {
    let t = tree();
    let derived = derive_loaders(&t.scanned);
    assert!(
        derived.len() > 20,
        "derived only {} loader fns",
        derived.len()
    );
    let bad = unclassified_loaders(&derived);
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

/// Plant a mutant: a new `pub fn load_*` that returns a config, and a caller
/// that swallows it. The classification must go red, and the swallow scan
/// must see the caller once the loader is listed.
#[test]
fn a_planted_unlisted_loader_goes_red() {
    let src = "pub fn load_planted_config(p: &Path) -> Result<ForjarConfig, String> { todo!() }\nfn cmd_x(p: &Path) { let _ = load_planted_config(p); }\n";
    let (code, spans) = planted(src);
    let files = vec![(
        "src/cli/planted.rs".to_string(),
        code.clone(),
        spans.clone(),
    )];
    let derived = derive_loaders(&files);
    assert!(derived.contains("load_planted_config"));
    let bad = unclassified_loaders(&derived);
    assert!(
        bad.iter().any(|b| b.contains("load_planted_config")),
        "the unlisted loader was accepted: {bad:?}"
    );
    // The swallow scan is blind to it with today's list: that is what the
    // classification above protects against.
    assert!(swallow_hits("src/cli/planted.rs", &code, &spans).is_empty());
    let with = swallow_hits_for(
        "src/cli/planted.rs",
        &code,
        &spans,
        &["load_planted_config"],
    );
    assert_eq!(with.len(), 1, "{with:?}");
}

#[test]
fn the_listed_manifest_loaders_are_each_watched() {
    for name in MANIFEST_LOADERS {
        for swallow in [
            format!("fn f() {{ let _ = {name}(p); }}"),
            format!("fn f() {{ let c = {name}(p).ok(); }}"),
            format!("fn f() {{ match {name}(p) {{ Ok(c) => c, Err(_) => return }}; }}"),
        ] {
            let (code, spans) = planted(&swallow);
            assert_eq!(
                swallow_hits("src/cli/p.rs", &code, &spans).len(),
                1,
                "{swallow}"
            );
        }
    }
}

/// A production file whose NAME ends `_test` is scanned like any other. The
/// planted tree has one `_test.rs` that bypasses the loader and one real test
/// module (`#[cfg(test)] mod`) that does the same.
#[test]
fn a_file_named_like_a_test_is_scanned_by_structure_not_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let cli = dir.path().join("src/cli");
    std::fs::create_dir_all(&cli).unwrap();
    let bypass = "pub fn cmd_x(f: &Path) {\n    let c = parse_and_validate_unresolved(f);\n}\n";
    std::fs::write(
        cli.join("mod.rs"),
        "mod check_test;\nmod other;\n#[cfg(test)]\nmod tests_real;\n",
    )
    .unwrap();
    std::fs::write(cli.join("check_test.rs"), bypass).unwrap();
    std::fs::write(cli.join("other.rs"), "pub fn ok() {}\n").unwrap();
    std::fs::write(cli.join("tests_real.rs"), bypass).unwrap();
    let t = tree_at(dir.path());
    let hits: BTreeSet<String> = t
        .scanned
        .iter()
        .flat_map(|(rel, code, spans)| unresolved_loader_hits(rel, code, spans))
        .collect();
    assert_eq!(
        hits.iter().cloned().collect::<Vec<_>>(),
        vec!["src/cli/check_test.rs:cmd_x".to_string()],
        "the `_test.rs` bypass must be seen and the cfg(test) module must not"
    );
    assert!(
        !judge("t", &hits, &[]).is_empty(),
        "the bypass was accepted"
    );
}

/// A function that renders resources must load the config in render form,
/// or the template resolver scans a data value a second time.
fn reader_load_in_a_renderer(
    rel: &str,
    code: &str,
    spans: &[(usize, usize, String)],
) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    if rel.starts_with("src/core/resolver/") || rel.starts_with("src/core/parser/") {
        return out;
    }
    let renders = |s: &str| {
        [
            "resolve_resource_templates(",
            "resolve_resource_templates_with_secrets(",
            "resolve_template_with_data(",
            "resolve_template(",
        ]
        .iter()
        .any(|t| s.contains(t))
    };
    for (start, end, name) in spans {
        let body = &code[*start..*end];
        if !renders(body) {
            continue;
        }
        for pat in [
            "parse_and_validate(",
            "load_resolved_config(",
            "load_resolved_config_lenient(",
        ] {
            if body.contains(pat) && !body.contains(&format!("fn {pat}")) {
                out.insert(format!("{rel}:{name}"));
            }
        }
    }
    out
}

#[test]
fn a_renderer_loads_in_render_form() {
    let hits = collect(reader_load_in_a_renderer);
    let bad = judge("renderer on a reader load", &hits, &[]);
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
fn a_planted_renderer_on_the_reader_loader_goes_red() {
    let (code, spans) = planted(
        "fn cmd_x(f: &Path) {\n    let c = parse_and_validate(f)?;\n    let r = resolve_resource_templates(&a, &b, &c, &d)?;\n}\n",
    );
    assert_eq!(
        reader_load_in_a_renderer("src/cli/p.rs", &code, &spans).len(),
        1
    );
    let (code, spans) = planted(
        "fn cmd_x(f: &Path) {\n    let c = parse_and_validate_for_render(f)?;\n    let r = resolve_resource_templates(&a, &b, &c, &d)?;\n}\n",
    );
    assert!(reader_load_in_a_renderer("src/cli/p.rs", &code, &spans).is_empty());
}
