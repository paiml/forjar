//! Citation parsing for the contract corpus, shared by the citation tests.
//!
//! Extracted from `falsification_contract_citations_resolve.rs` when adding
//! comma-continuation support (#298) pushed that file past the repo's 500-line
//! ceiling. The parser is the part worth naming on its own: it decides what the
//! corpus is understood to be CLAIMING, and every count the suite reports is
//! downstream of it.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

/// Files under `contracts/` that are registries rather than contracts, and so
/// carry no citations to resolve.
const NOT_CONTRACTS: &[&str] = &["binding.yaml"];
/// A resolved reference into the source tree: a repo-relative `.rs` path and,
/// optionally, the item inside it. A trailing `*` on the item is a prefix.
#[derive(Debug)]
pub struct Citation {
    pub file: String,
    pub item: Option<String>,
}

pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

pub fn contracts_dir() -> PathBuf {
    repo_root().join("contracts")
}

pub fn contract_files() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(contracts_dir())
        .expect("contracts/ must exist")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "yaml"))
        .filter(|p| {
            let name = p.file_name().unwrap_or_default().to_string_lossy();
            !NOT_CONTRACTS.contains(&name.as_ref())
        })
        .collect();
    files.sort();
    files
}

fn is_path_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '/' | '-')
}

/// The `.rs` path ending at `end` (exclusive), walking back over path chars.
fn path_ending_at(s: &str, end: usize) -> Option<String> {
    let head = &s[..end];
    let start = head
        .char_indices()
        .rev()
        .take_while(|(_, c)| is_path_char(*c))
        .map(|(i, _)| i)
        .last()?;
    let file = &s[start..end];
    // ".rs" alone, or a token whose stem is empty, is not a path.
    (file.len() > 3).then(|| file.to_string())
}

/// The item named immediately after a `.rs`, in either the `::item` form or
/// the space-separated `mod::item` form the corpus also uses.
fn item_after(rest: &str) -> Option<String> {
    let tok = if let Some(after) = rest.strip_prefix("::") {
        after.split([' ', '\t', '\n', ',', ';', ')', '"']).next()?
    } else if rest.starts_with([' ', '\t', '\n']) {
        let next = rest
            .trim_start()
            .split([' ', '\t', '\n', ',', ';', ')', '"'])
            .next()?;
        if !next.contains("::") {
            return None;
        }
        next
    } else {
        return None;
    };
    let seg = tok.rsplit("::").next()?.trim_end_matches('.');
    let stem = seg.strip_suffix('*').unwrap_or(seg);
    let ok = !stem.is_empty()
        && stem.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        && stem.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    ok.then(|| seg.to_string())
}

/// Every item named for ONE `.rs` path: the first, plus any comma-separated
/// continuation names that follow it.
///
/// The corpus writes a shared path once and lists several items under it:
///
/// ```text
/// src/core/tests_webhook_http.rs::every_response_body_is_valid_json,
/// response_escapes_hostile_codes;
/// ```
///
/// `item_after` reads only the first. Ten function names in the corpus are
/// written as continuations and were therefore never resolved — so a citation
/// naming a function that does not exist passed this test green, which is the
/// exact defect #298 exists to catch: a guard reporting a total it did not
/// measure. Verified by replacing `response_escapes_hostile_codes` with a name
/// present nowhere in the tree and watching every test still pass.
///
/// A continuation stops at a `;`, at anything carrying `.` or `:` (a new path
/// or a qualified item), and at any token that is not a bare Rust identifier.
fn items_after(rest: &str) -> Vec<String> {
    let Some(first) = item_after(rest) else {
        return Vec::new();
    };
    let mut out = vec![first.clone()];
    let mut tail = match rest.find(first.as_str()) {
        Some(i) => &rest[i + first.len()..],
        None => return out,
    };
    loop {
        let Some(after_comma) = tail.trim_start().strip_prefix(',') else {
            break;
        };
        let c = after_comma.trim_start();
        let cand = c
            .split([' ', '\t', '\n', '\r', ',', ';', ')', '"'])
            .next()
            .unwrap_or("");
        let bare = !cand.is_empty()
            && !cand.contains('.')
            && !cand.contains(':')
            && cand.starts_with(|ch: char| ch.is_ascii_alphabetic() || ch == '_')
            && cand
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_');
        if !bare {
            break;
        }
        out.push(cand.to_string());
        tail = &c[cand.len()..];
    }
    out
}

/// Every citation in `s`. Anchored on `.rs`, so a string with no `.rs` token
/// yields nothing — which is how shell snippets and equation names are
/// skipped without an exemption list.
pub fn citations_in(s: &str) -> Vec<Citation> {
    let mut out = Vec::new();
    let mut from = 0usize;
    while let Some(rel) = s[from..].find(".rs") {
        let at = from + rel;
        let end = at + 3;
        from = end;
        // ".rs" must end the token: `x.rsomething` is not a path.
        if s[end..].starts_with(|c: char| c.is_ascii_alphanumeric() || c == '_') {
            continue;
        }
        let Some(file) = path_ending_at(s, end) else {
            continue;
        };
        let items = items_after(&s[end..]);
        if items.is_empty() {
            out.push(Citation { item: None, file });
        } else {
            for item in items {
                out.push(Citation {
                    item: Some(item),
                    file: file.clone(),
                });
            }
        }
    }
    out
}
