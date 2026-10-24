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

use common_660::rust_scan::*;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// The loader module: the parser, and the resolver files that wrap it.
fn is_loader_module(rel: &str) -> bool {
    rel.starts_with("src/core/parser/")
        || rel == "src/core/resolver/data.rs"
        || rel == "src/core/resolver/data_subst.rs"
}

fn is_test_name(path: &Path) -> bool {
    let stem = path.file_stem().unwrap().to_string_lossy();
    stem.starts_with("tests") || stem.ends_with("_tests") || stem.ends_with("_test")
}

type Spans = Vec<(usize, usize, String)>;
type Scanned = (String, String, Spans);

/// `(rel path, production code, fn spans)` for every non-test file in `src/`.
struct Tree {
    scanned: Vec<Scanned>,
    total: usize,
}

fn tree() -> Tree {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    rust_files(&root.join("src"), &mut files);
    let test_only = test_only_files(&files);
    let mut scanned = Vec::new();
    for f in &files {
        if test_only.contains(f) || is_test_name(f) {
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

const LOAD_TOKENS: &[&str] = &[
    "parse_and_validate",
    "parse_and_validate_unresolved",
    "load_resolved_config",
    "load_resolved_config_lenient",
    "load_resolved_value",
    "resolve_resource_templates",
    "resolve_resource_templates_with_secrets",
    "resolve_data_sources_with_base",
    "resolve_outputs",
    "require_data_sources",
    "finish_resolving",
];

fn swallow_hits(rel: &str, code: &str, spans: &[(usize, usize, String)]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for tok in LOAD_TOKENS {
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

const RAW_PARSE_ALLOWLIST: &[(&str, &str)] = &[
    ("src/cli/apply_helpers.rs:load_env_params", "parses the flat --env-file params mapping, not a manifest"),
    ("src/cli/destroy.rs:cmd_rollback", "parses a PREVIOUS git revision of the manifest to diff resource sets; data: sources must not run against history"),
    ("src/cli/lint_fix.rs:lint_auto_fix", "lint --fix rewrites the manifest text; it edits the raw document and must not run data: commands"),
    ("src/cli/lsp_validate.rs:validate_yaml", "checks YAML syntax of an editor buffer, the unsaved text of a file"),
    ("src/cli/reseal.rs:reseal_one", "reseal rewrites the manifest bytes in place; resolving would bake data: output into the file"),
    ("src/cli/state_identity.rs:applied_under_before", "reads only `name:` of a stored applied-config snapshot under the state dir"),
    ("src/cli/status_trends.rs:count_changes_from_logs", "parses one event-log line, not a manifest"),
    ("src/cli/template_cmd.rs:classify", "asks which top-level keys the file has (recipe vs config) before the loader runs; no value is read"),
    ("src/cli/template_cmd.rs:yaml_scalar", "parses one `-V KEY=VALUE` scalar, not a manifest"),
    ("src/cli/undo_helpers.rs:parse_replay_resource", "parses a resource fragment recorded in a destroy log"),
    ("src/cli/validate_policy.rs:cmd_validate_policy_file", "reads a separate POLICY file, not the manifest"),
    ("src/cli/verify.rs:recorded_output_hash", "reads a lock/attestation document under the state dir, not the manifest"),
    ("src/core/remediate/mod.rs:emit_in_style", "round-trips one candidate scalar to choose a quoting style"),
    ("src/core/state/integrity.rs:check_lock_file", "checks that a LOCK file is well-formed YAML, not a manifest"),
    ("src/core/store/convert_exec.rs:apply_conversion", "WRITES the converted manifest in place after a backup; resolving would bake data: output into the file"),
    ("src/core/store/derivation.rs:parse_derivation", "reads a derivation document of the store, not a manifest"),
    ("src/core/store/ingest.rs:ingest_generations", "reads a stored generation snapshot, not the live manifest"),
    ("src/core/store/ingest.rs:ingest_lock_file", "reads a state LOCK file into the store index"),
    ("src/core/store/ingest.rs:upsert_machine", "reads a state LOCK file into the store index"),
    ("src/core/store/meta.rs:read_meta", "reads store metadata, not a manifest"),
    ("src/core/store/provider.rs:parse_import_config", "reads an import-provider config, not a manifest"),
    ("src/core/store/secret_scan.rs:scan_yaml_str", "scans arbitrary YAML text for secrets; the text is the caller's, nothing resolves"),
    ("src/core/yaml_edit/verify.rs:changed_paths_of_text", "diffs two texts of an edit (before/after); the edit is made on the raw document"),
];

const UNRESOLVED_LOADER_ALLOWLIST: &[(&str, &str)] = &[
    ("src/cli/helpers.rs:parse_and_validate_unresolved", "defines the unresolved loader the rows below call"),
    ("src/mcp/handlers.rs:handle", "ValidateHandler parses unresolved FIRST only to tell an invalid manifest (a `valid: false` result) from a failing data: source (an error); it then calls the resolving loader"),
    ("src/cli/contracts.rs:load_for_contracts", "an UNPARSABLE manifest is graceful (codebase contracts still print); one that parses is then loaded through the resolving loader and its failure is returned"),
    ("src/cli/complexity_analysis.rs:cmd_complexity", "counts structure on the unresolved manifest; a failing data: source still fails it via helpers::require_data_sources"),
    ("src/cli/extract.rs:cmd_extract", "EMITS a sub-manifest: templates and `data:` sources stay in it; a failing source still fails it via helpers::require_data_sources"),
    ("src/cli/config_merge.rs:cmd_config_merge", "WRITES the merged manifest: resolving would bake data: output into the file; a failing source still fails it via helpers::require_data_sources"),
    ("src/cli/infra_bench.rs:run_benchmarks", "times parse+validate; the sources are checked once, outside the timed loops, via helpers::require_data_sources"),
    ("src/cli/init.rs:cmd_fmt", "fmt REWRITES the file, so it serializes the unresolved manifest; a failing source still fails it via helpers::require_data_sources"),
    ("src/cli/lsp_validate.rs:validate_yaml", "validates an editor buffer that is not on disk; no base dir, no data: run"),
    ("src/cli/undo_helpers.rs:build_replay_config", "rebuilds a config from a destroy-log record; the record holds already-resolved resources"),
];

const SWALLOW_ALLOWLIST: &[(&str, &str)] = &[
    ("src/cli/apply_variants.rs:refreshed_live_hash", "the config was loaded through the resolving loader, which already failed on an unknown reference; the fallback only covers a secrets-provider error, and the refresh query then runs the raw resource"),
    ("src/core/resolver/resource.rs:resolve_or_fallback", "documented fallback: warns on stderr and keeps the raw resource for destroy/refresh scripts"),
    ("src/cli/print_helpers.rs:print_plan_change", "display only: falls back to the raw text for a resource the planner already reported as unresolvable"),
    ("src/core/executor/refresh.rs:refresh_check_fails", "an unresolvable resource counts as a failing check, i.e. it is re-applied, which then reports the error"),
    ("src/core/executor/refresh_seed.rs:check_passes_on", "an unresolvable resource does not pass its check, which then reports the error on apply"),
];

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
