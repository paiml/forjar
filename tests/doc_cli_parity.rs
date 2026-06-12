//! DOCS-1 (PMAT-086): CLI <-> book parity test.
//!
//! Parses the `Commands` enum in `src/cli/commands/mod.rs` at test time and
//! asserts that every subcommand appears at least once across
//! `docs/book/src/*.md` as `forjar <name>`.
//!
//! Deterministic and offline: reads only files in this repository.

use std::fs;
use std::path::{Path, PathBuf};

/// Convert a PascalCase enum variant name to clap's default kebab-case
/// command name (e.g. `UndoDestroy` -> `undo-destroy`).
fn kebab_case(variant: &str) -> String {
    let mut out = String::with_capacity(variant.len() + 4);
    for (i, ch) in variant.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if i > 0 {
                out.push('-');
            }
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

/// Extract the body of `pub enum Commands { ... }` from the module source.
fn commands_enum_body(src: &str) -> &str {
    let start = src
        .find("pub enum Commands {")
        .expect("Commands enum not found in src/cli/commands/mod.rs");
    let body_start = start + "pub enum Commands {".len();
    // Variants are unit or tuple style, so the first line starting with `}`
    // terminates the enum body.
    let body_end = src[body_start..]
        .find("\n}")
        .map(|i| body_start + i)
        .expect("Commands enum closing brace not found");
    &src[body_start..body_end]
}

/// Parse a `#[command(name = "...")]` attribute line, returning the override.
fn parse_name_override(line: &str) -> Option<String> {
    let line = line.trim();
    if !line.starts_with("#[command(") || !line.contains("name") {
        return None;
    }
    let after = line.split("name").nth(1)?;
    let first_quote = after.find('"')?;
    let rest = &after[first_quote + 1..];
    let second_quote = rest.find('"')?;
    Some(rest[..second_quote].to_string())
}

/// Parse an enum variant line, returning the variant identifier.
/// Matches `Init(InitArgs),` and unit variants like `Schema,`.
fn parse_variant(line: &str) -> Option<String> {
    let line = line.trim();
    let first = line.chars().next()?;
    if !first.is_ascii_uppercase() {
        return None;
    }
    let ident: String = line
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric())
        .collect();
    let next = line[ident.len()..].chars().next();
    match next {
        Some('(') | Some(',') => Some(ident),
        _ => None,
    }
}

/// Extract all CLI subcommand names from the Commands enum source,
/// honoring `#[command(name = "...")]` overrides.
fn extract_command_names(src: &str) -> Vec<String> {
    let body = commands_enum_body(src);
    let mut names = Vec::new();
    let mut pending_override: Option<String> = None;
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("///") || trimmed.is_empty() {
            continue;
        }
        if let Some(name) = parse_name_override(trimmed) {
            pending_override = Some(name);
            continue;
        }
        if trimmed.starts_with("#[") {
            continue;
        }
        if let Some(variant) = parse_variant(trimmed) {
            let name = pending_override
                .take()
                .unwrap_or_else(|| kebab_case(&variant));
            names.push(name);
        }
    }
    names
}

/// Concatenate the contents of every `.md` file in `docs/book/src/`.
fn load_book_text(dir: &Path) -> String {
    let mut text = String::new();
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .expect("docs/book/src not readable")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "md"))
        .collect();
    entries.sort();
    for path in entries {
        text.push_str(&fs::read_to_string(&path).expect("doc file not readable"));
        text.push('\n');
    }
    text
}

/// True if `forjar <cmd>` appears in the book with a word boundary after it
/// (so `forjar state` is not satisfied by `forjar state-list`).
fn is_documented(book: &str, cmd: &str) -> bool {
    let needle = format!("forjar {cmd}");
    let mut from = 0;
    while let Some(pos) = book[from..].find(&needle) {
        let end = from + pos + needle.len();
        let boundary = match book[end..].chars().next() {
            Some(c) => !c.is_ascii_alphanumeric() && c != '-' && c != '_',
            None => true,
        };
        if boundary {
            return true;
        }
        from = end;
    }
    false
}

fn repo_path(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)
}

#[test]
fn kebab_case_handles_multi_word_variants() {
    assert_eq!(kebab_case("Init"), "init");
    assert_eq!(kebab_case("UndoDestroy"), "undo-destroy");
    assert_eq!(kebab_case("RetryFailed"), "retry-failed");
}

#[test]
fn name_override_is_parsed() {
    assert_eq!(
        parse_name_override(r##"#[command(name = "state-list")]"##),
        Some("state-list".to_string())
    );
    assert_eq!(parse_name_override("#[command(subcommand)]"), None);
    assert_eq!(parse_name_override("Init(InitArgs),"), None);
}

#[test]
fn commands_enum_parses_known_names() {
    let src = fs::read_to_string(repo_path("src/cli/commands/mod.rs"))
        .expect("src/cli/commands/mod.rs not readable");
    let names = extract_command_names(&src);
    assert!(
        names.len() >= 100,
        "parser found only {} commands — parsing logic likely broken",
        names.len()
    );
    for expected in [
        "apply",
        "state-list",
        "query",
        "agent",
        "schema",
        "undo-destroy",
    ] {
        assert!(
            names.iter().any(|n| n == expected),
            "expected command `{expected}` not extracted from Commands enum"
        );
    }
    // Overrides must replace, not duplicate, the kebab-cased variant name.
    assert!(
        !names.iter().any(|n| n == "infra-query"),
        "name override for InfraQuery (`query`) was not honored"
    );
}

#[test]
fn every_cli_subcommand_is_documented_in_the_book() {
    let src = fs::read_to_string(repo_path("src/cli/commands/mod.rs"))
        .expect("src/cli/commands/mod.rs not readable");
    let names = extract_command_names(&src);
    let book = load_book_text(&repo_path("docs/book/src"));

    let missing: Vec<&String> = names.iter().filter(|n| !is_documented(&book, n)).collect();
    assert!(
        missing.is_empty(),
        "{} subcommand(s) missing from docs/book/src/*.md (add a section with \
         `forjar <name>` to docs/book/src/06b-cli-reference-appendix.md): {:?}",
        missing.len(),
        missing
    );
}
