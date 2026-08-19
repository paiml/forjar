//! GH-211 / FALSIFY-FLAG-A16, part 1 of 2: reading the flag surface out of the
//! source.
//!
//! The guard in `tests_flag_consumer_guard` needs two sets it cannot get from
//! the type system: every flag DECLARED on a `clap::Args` struct, and every
//! field actually READ by a dispatcher. Rust offers no reflection over struct
//! fields, and clap's generated `Command` gives flag names but not which field
//! backs them, so both sets are recovered from the source text here.
//!
//! Deterministic and offline: it reads only files inside this repository, via
//! `CARGO_MANIFEST_DIR`. Every helper has its own unit test at the bottom,
//! because a parser that silently finds nothing would make the guard above
//! vacuously green — the exact failure mode this whole contract exists to
//! catch.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Absolute path to a file in the repository being tested.
pub(super) fn repo_path(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)
}

/// One `pub` field on a `clap::Args` struct.
#[derive(Debug, Clone)]
pub(super) struct FieldDecl {
    /// Args struct the field belongs to, e.g. `ApplyArgs`.
    pub(super) ty: String,
    /// Field name, e.g. `notify_file`.
    pub(super) name: String,
    /// True when the doc comment carries the `[UNIMPLEMENTED` marker, i.e.
    /// `--help` admits the flag does nothing.
    pub(super) marked_unimplemented: bool,
}

/// Read every `.rs` file under `src/`, keyed by repo-relative path.
pub(super) fn read_sources() -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let mut stack = vec![repo_path("src")];
    while let Some(dir) = stack.pop() {
        let entries = std::fs::read_dir(&dir).expect("src/ is readable");
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "rs") {
                let rel = p
                    .strip_prefix(repo_path(""))
                    .unwrap_or(&p)
                    .to_string_lossy()
                    .into_owned();
                out.insert(rel, std::fs::read_to_string(&p).expect("source is readable"));
            }
        }
    }
    out
}

/// Return the body of the brace-delimited block that starts at `open`
/// (the index of its `{`), excluding the braces themselves.
pub(super) fn block_body(src: &str, open: usize) -> &str {
    let bytes = src.as_bytes();
    let mut depth = 0usize;
    let mut i = open;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return &src[open + 1..i];
                }
            }
            _ => {}
        }
        i += 1;
    }
    &src[open + 1..]
}

/// True when `s[..at]` ends with `needle` at a token boundary.
pub(super) fn ident_at(s: &str, at: usize, needle: &str) -> bool {
    if at < needle.len() || &s[at - needle.len()..at] != needle {
        return false;
    }
    let before = s[..at - needle.len()].chars().next_back();
    !before.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == ':' || c == '.')
}

/// Parse one struct body into its `pub` fields, carrying the doc-comment marker.
pub(super) fn parse_fields(ty: &str, body: &str) -> Vec<FieldDecl> {
    let mut out = Vec::new();
    let mut marked = false;
    for line in body.lines() {
        let t = line.trim();
        if t.starts_with("///") {
            if t.contains("[UNIMPLEMENTED") {
                marked = true;
            }
            continue;
        }
        if t.starts_with("#[") || t.is_empty() {
            continue;
        }
        if let Some(rest) = t.strip_prefix("pub ") {
            if let Some(name) = rest.split(':').next() {
                let name = name.trim();
                if !name.is_empty() && !name.contains(' ') {
                    out.push(FieldDecl {
                        ty: ty.to_string(),
                        name: name.to_string(),
                        marked_unimplemented: marked,
                    });
                }
            }
        }
        marked = false;
    }
    out
}

/// Parse `src/cli/commands/*.rs` for `#[derive(.. clap::Args ..)] pub struct X`
/// and collect every declared flag.
pub(super) fn declared_flags(sources: &BTreeMap<String, String>) -> Vec<FieldDecl> {
    let mut out = Vec::new();
    for (path, src) in sources {
        if !is_declaration(path) {
            continue;
        }
        for (i, _) in src.match_indices("clap::Args") {
            let Some(rel) = src[i..].find("pub struct ") else {
                continue;
            };
            // Anything between the derive and the struct keyword must be
            // attributes, never another item.
            if src[i..i + rel].contains("fn ") {
                continue;
            }
            let name_at = i + rel + "pub struct ".len();
            let ty: String = src[name_at..]
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            let Some(open) = src[name_at..].find('{').map(|o| name_at + o) else {
                continue;
            };
            out.extend(parse_fields(&ty, block_body(src, open)));
        }
    }
    out
}

/// Map `Commands::Variant(..)` to the args struct the variant carries, so a
/// binding introduced by `Commands::Bootstrap(a)` is attributed to
/// `BootstrapArgs`.
pub(super) fn variant_types(sources: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let src = sources
        .get("src/cli/commands/mod.rs")
        .expect("Commands enum module exists");
    for line in src.lines() {
        let l = line.trim();
        let Some(open) = l.find('(') else { continue };
        let Some(close) = l.find(')') else { continue };
        if close < open {
            continue;
        }
        let variant = &l[..open];
        let ty = &l[open + 1..close];
        if ty.ends_with("Args") && variant.chars().all(|c| c.is_alphanumeric()) && !variant.is_empty()
        {
            out.insert(variant.to_string(), ty.to_string());
        }
    }
    out
}

/// Every binding name through which values of `<Struct>` are reached: typed
/// parameters (`args: &ApplyArgs`) and enum patterns (`Commands::Apply(args)`).
pub(super) fn bindings_for(text: &str, variants: &BTreeMap<String, String>) -> BTreeMap<String, BTreeSet<String>> {
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (i, _) in text.match_indices("Args") {
        let end = i + "Args".len();
        // `Args` must end an identifier.
        if text[end..].starts_with(|c: char| c.is_alphanumeric() || c == '_') {
            continue;
        }
        let start = text[..end]
            .rfind(|c: char| !(c.is_alphanumeric() || c == '_'))
            .map_or(0, |p| p + 1);
        let ty = &text[start..end];
        if !ty.chars().next().is_some_and(char::is_uppercase) {
            continue;
        }
        if let Some(name) = typed_param_name(&text[..start]) {
            out.entry(ty.to_string()).or_default().insert(name);
        }
    }
    for (variant, ty) in variants {
        let needle = format!("Commands::{variant}(");
        for (i, _) in text.match_indices(&needle) {
            if let Some(name) = pattern_binding(&text[i + needle.len()..]) {
                out.entry(ty.clone()).or_default().insert(name);
            }
        }
    }
    out
}

/// Strip a module path qualifier from the end of `head`, so
/// `args: &super::commands::` reduces to `args: &`.
pub(super) fn strip_path_qualifier(mut head: &str) -> &str {
    while let Some(rest) = head.strip_suffix("::") {
        let cut = rest
            .rfind(|c: char| !(c.is_alphanumeric() || c == '_'))
            .map_or(0, |p| p + 1);
        head = rest[..cut].trim_end();
    }
    head
}

/// Given everything to the left of a type name, return the parameter binding
/// when the text has the shape `name : & mut? <path>`.
pub(super) fn typed_param_name(before: &str) -> Option<String> {
    let head = strip_path_qualifier(before.trim_end());
    let head = head.strip_suffix("mut").unwrap_or(head).trim_end();
    let head = head.strip_suffix('&').unwrap_or(head).trim_end();
    let head = head.strip_suffix(':')?;
    if head.ends_with(':') {
        // `<expr>::Type` is a path, not a `name: Type` binding.
        return None;
    }
    let ident: String = head
        .chars()
        .rev()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    (!ident.is_empty()).then(|| ident.chars().rev().collect())
}

/// Given everything after `Commands::Variant(`, return the binding it
/// introduces. Handles `ref args` and `ref mut args`; rejects `_` and `..`.
pub(super) fn pattern_binding(after: &str) -> Option<String> {
    let after = after.trim_start();
    let after = after.strip_prefix("ref ").unwrap_or(after).trim_start();
    let after = after.strip_prefix("mut ").unwrap_or(after).trim_start();
    let ident: String = after
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    if ident.is_empty() || ident == "_" || !after[ident.len()..].starts_with(')') {
        return None;
    }
    Some(ident)
}

/// Fields bound by name in a `<Struct> { .. }` destructuring pattern.
///
/// `foo` and `foo: bar` count (rustc's unused-variable lint then guarantees a
/// real use); `foo: _foo` and a field elided by `..` do not.
pub(super) fn destructured(text: &str, ty: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let needle = format!("{ty} ");
    for (i, _) in text.match_indices(ty) {
        if !ident_at(text, i + ty.len(), ty) {
            continue;
        }
        let after = text[i + ty.len()..].trim_start();
        if !after.starts_with('{') {
            continue;
        }
        let open = i + ty.len() + text[i + ty.len()..].find('{').unwrap();
        for part in block_body(text, open).split(',') {
            let part = part.trim();
            if part.is_empty() || part.starts_with("..") {
                continue;
            }
            match part.split_once(':') {
                Some((f, bind)) => {
                    if !bind.trim().starts_with('_') {
                        out.insert(f.trim().to_string());
                    }
                }
                None => {
                    out.insert(part.to_string());
                }
            }
        }
    }
    let _ = needle;
    out
}

/// Which fields of `ty` are read in `text`, by any route.
pub(super) fn fields_read(text: &str, ty: &str, bindings: &BTreeMap<String, BTreeSet<String>>) -> BTreeSet<String> {
    let mut out = destructured(text, ty);
    let empty = BTreeSet::new();
    for bind in bindings.get(ty).unwrap_or(&empty) {
        let needle = format!("{bind}.");
        for (i, _) in text.match_indices(&needle) {
            if !ident_at(text, i + bind.len(), bind) {
                continue;
            }
            let field: String = text[i + needle.len()..]
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !field.is_empty() {
                out.insert(field);
            }
        }
    }
    out
}

/// Source files that count as declaration (not consumption).
pub(super) fn is_declaration(path: &str) -> bool {
    path.starts_with("src/cli/commands/")
}

/// Source files that count as neither declaration nor consumption: tests,
/// generated code, and the refusal module (whose reads mean "refused").
pub(super) fn is_excluded(path: &str) -> bool {
    let base = path.rsplit('/').next().unwrap_or(path);
    base.starts_with("tests_")
        || base.starts_with("test_")
        || path == "src/generated_contracts.rs"
        || path == "src/cli/inert_flags.rs"
}

#[test]
fn block_body_extracts_the_brace_delimited_body() {
    let s = "struct X { a: u8, b: (u8, u8) }";
    assert_eq!(block_body(s, s.find('{').unwrap()), " a: u8, b: (u8, u8) ");
}

#[test]
fn destructured_ignores_underscore_bindings_and_rest_patterns() {
    let text = "let LintArgs { file, rules: _rules, .. } = args;";
    let got = destructured(text, "LintArgs");
    assert!(got.contains("file"));
    assert!(
        !got.contains("rules"),
        "`rules: _rules` silences rustc, not the operator"
    );
}

#[test]
fn parse_fields_carries_the_unimplemented_marker() {
    let body = "\n    /// FJ-1: does nothing [UNIMPLEMENTED]\n    #[arg(long)]\n    pub a: bool,\n\n    /// FJ-2: works\n    #[arg(long)]\n    pub b: bool,\n";
    let got = parse_fields("T", body);
    assert_eq!(got.len(), 2);
    assert!(got[0].marked_unimplemented);
    assert!(!got[1].marked_unimplemented);
}

#[test]
fn typed_param_name_sees_through_paths_and_references() {
    assert_eq!(
        typed_param_name("pub(crate) fn cmd_dist(args: &super::commands::"),
        Some("args".to_string())
    );
    assert_eq!(
        typed_param_name("fn f(a: &mut "),
        Some("a".to_string())
    );
    // A path expression is not a binding.
    assert_eq!(typed_param_name("let x = super::commands::"), None);
}

#[test]
fn pattern_binding_handles_ref_and_rejects_wildcards() {
    assert_eq!(pattern_binding("ref args)"), Some("args".to_string()));
    assert_eq!(pattern_binding("ref mut a)"), Some("a".to_string()));
    assert_eq!(pattern_binding("args)"), Some("args".to_string()));
    assert_eq!(pattern_binding("_)"), None);
    assert_eq!(pattern_binding("..)"), None);
    assert_eq!(pattern_binding("Args { file, .. })"), None);
}
