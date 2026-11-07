//! A small structural Rust scanner for the forjar#660 source-scan tests.
//!
//! No `syn` in this crate, so it does the part the tests need by hand and does
//! it structurally rather than line-wise: it blanks comments, string and char
//! literals, blanks the body of every `#[cfg(test)]` item (found by brace
//! matching from the attribute to the item's closing `}` or `;`), and tells
//! each remaining byte which `fn` it sits in. The earlier scan took lines
//! `while != "#[cfg(test)]"`, so a test helper in the MIDDLE of a file hid
//! every production line after it (`status_connectivity.rs` line 159).
#![allow(dead_code)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// `src` with comments, string and char literals replaced by spaces (newlines
/// kept, so byte offsets and line numbers survive).
pub fn blank_literals(src: &str) -> String {
    let b = src.as_bytes();
    let mut out = b.to_vec();
    let mut i = 0;
    let blank = |out: &mut Vec<u8>, from: usize, to: usize| {
        for byte in out.iter_mut().take(to).skip(from) {
            if *byte != b'\n' {
                *byte = b' ';
            }
        }
    };
    while i < b.len() {
        match b[i] {
            b'/' if b.get(i + 1) == Some(&b'/') => {
                let end = src[i..].find('\n').map_or(b.len(), |n| i + n);
                blank(&mut out, i, end);
                i = end;
            }
            b'/' if b.get(i + 1) == Some(&b'*') => {
                let (mut depth, mut j) = (1, i + 2);
                while j < b.len() && depth > 0 {
                    if b[j] == b'/' && b.get(j + 1) == Some(&b'*') {
                        depth += 1;
                        j += 2;
                    } else if b[j] == b'*' && b.get(j + 1) == Some(&b'/') {
                        depth -= 1;
                        j += 2;
                    } else {
                        j += 1;
                    }
                }
                blank(&mut out, i, j);
                i = j;
            }
            b'r' | b'b' if raw_string_at(b, i).is_some() => {
                let (hashes, open) = raw_string_at(b, i).unwrap();
                let close: String = std::iter::once('"')
                    .chain(std::iter::repeat_n('#', hashes))
                    .collect();
                let end = src[open..]
                    .find(&close)
                    .map_or(b.len(), |n| open + n + close.len());
                blank(&mut out, i, end);
                i = end;
            }
            b'"' => {
                let mut j = i + 1;
                while j < b.len() && b[j] != b'"' {
                    j += if b[j] == b'\\' { 2 } else { 1 };
                }
                blank(&mut out, i, (j + 1).min(b.len()));
                i = j + 1;
            }
            b'\'' => {
                // A char literal ('a', '\n', '{') or a lifetime ('a). A literal
                // closes within a few bytes; a lifetime does not.
                let rest = &b[i + 1..];
                let close = if rest.first() == Some(&b'\\') {
                    rest.iter().skip(2).position(|&c| c == b'\'').map(|p| p + 3)
                } else if rest.get(1) == Some(&b'\'') {
                    Some(2)
                } else {
                    None
                };
                match close {
                    Some(n) => {
                        blank(&mut out, i, i + 1 + n);
                        i += 1 + n;
                    }
                    None => i += 1,
                }
            }
            _ => i += 1,
        }
    }
    String::from_utf8(out).expect("blanking keeps utf-8")
}

/// `r"`, `r#"`, `br"`... at `i`: (number of #, offset just past the opening quote).
fn raw_string_at(b: &[u8], i: usize) -> Option<(usize, usize)> {
    if i > 0 && (b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_') {
        return None;
    }
    let mut j = i;
    if b[j] == b'b' {
        j += 1;
    }
    if b.get(j) != Some(&b'r') {
        return None;
    }
    j += 1;
    let mut hashes = 0;
    while b.get(j) == Some(&b'#') {
        hashes += 1;
        j += 1;
    }
    (b.get(j) == Some(&b'"')).then_some((hashes, j + 1))
}

/// Is the attribute text `#[cfg(...)]` one that selects test builds only?
fn is_test_cfg(attr: &str) -> bool {
    let inner = attr.replace(' ', "");
    inner.starts_with("#[cfg(")
        && !inner.contains("not(test)")
        && (inner == "#[cfg(test)]" || inner.contains("(test,") || inner.contains(",test)"))
}

/// Byte ranges of every `#[cfg(test)]` item: attribute through the item's
/// closing `}` (or its `;`). Input must already be literal-blanked.
pub fn cfg_test_ranges(code: &str) -> Vec<(usize, usize)> {
    let b = code.as_bytes();
    let mut ranges = Vec::new();
    let mut i = 0;
    while let Some(p) = code[i..].find("#[cfg(") {
        let start = i + p;
        let Some(close) = code[start..].find(']').map(|n| start + n + 1) else {
            break;
        };
        i = close;
        if !is_test_cfg(&code[start..close]) {
            continue;
        }
        let (mut j, mut depth) = (close, 0usize);
        let mut paren = 0usize;
        let mut end = b.len();
        while j < b.len() {
            match b[j] {
                b'(' | b'[' => paren += 1,
                b')' | b']' => paren = paren.saturating_sub(1),
                b'{' => depth += 1,
                b'}' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        end = j + 1;
                        break;
                    }
                }
                b';' if depth == 0 && paren == 0 => {
                    end = j + 1;
                    break;
                }
                _ => {}
            }
            j += 1;
        }
        ranges.push((start, end));
        i = end;
    }
    ranges
}

/// Production code: literals blanked, `#[cfg(test)]` items blanked.
pub fn production_code(src: &str) -> String {
    let code = blank_literals(src);
    let mut out = code.clone().into_bytes();
    for (s, e) in cfg_test_ranges(&code) {
        for byte in out.iter_mut().take(e).skip(s) {
            if *byte != b'\n' {
                *byte = b' ';
            }
        }
    }
    String::from_utf8(out).expect("utf-8")
}

/// Name of the `fn` enclosing each byte offset in `code` (literal-blanked), as a
/// sorted list of `(start, end, name)` body spans.
pub fn fn_spans(code: &str) -> Vec<(usize, usize, String)> {
    let b = code.as_bytes();
    let mut spans = Vec::new();
    let mut stack: Vec<Option<(usize, String)>> = Vec::new();
    let mut pending: Option<String> = None;
    let mut i = 0;
    while i < b.len() {
        let word_start = i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_');
        if word_start && code[i..].starts_with("fn ") {
            let rest = code[i + 3..].trim_start();
            let name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                pending = Some(name);
            }
            i += 3;
            continue;
        }
        match b[i] {
            b'{' => stack.push(pending.take().map(|n| (i, n))),
            b'}' => {
                if let Some(Some((s, n))) = stack.pop() {
                    spans.push((s, i + 1, n));
                }
            }
            b';' => pending = None,
            _ => {}
        }
        i += 1;
    }
    spans.sort();
    spans
}

/// The innermost fn containing `at`, or `<module>`.
pub fn enclosing_fn(spans: &[(usize, usize, String)], at: usize) -> String {
    spans
        .iter()
        .filter(|(s, e, _)| *s <= at && at < *e)
        .max_by_key(|(s, _, _)| *s)
        .map_or_else(|| "<module>".to_string(), |(_, _, n)| n.clone())
}

pub fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .expect("read_dir")
        .map(|e| e.expect("entry").path())
        .collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            rust_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// Files that are compiled only under `cfg(test)`: the target of a
/// `#[cfg(test)] mod name;` in some other file (`name.rs` or `name/mod.rs` next
/// to it), and everything those files declare in turn.
pub fn test_only_files(files: &[PathBuf]) -> BTreeSet<PathBuf> {
    let includes = include_map(files);
    let mut found = BTreeSet::new();
    let mut work: Vec<PathBuf> = Vec::new();
    for f in files {
        let raw = std::fs::read_to_string(f).expect("read");
        let code = blank_literals(&raw);
        for (s, e) in cfg_test_ranges(&code) {
            let item = code[s..e].trim_end_matches(';');
            let after_attr = item.rsplit(']').next().unwrap_or("").trim();
            if let Some(name) = after_attr
                .strip_prefix("pub(crate) mod ")
                .or_else(|| after_attr.strip_prefix("pub mod "))
                .or_else(|| after_attr.strip_prefix("mod "))
            {
                if name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                    // `#[path = "x.rs"]` names the file; offsets survive blanking.
                    match path_attr(&raw[s..e]) {
                        Some(p) => work.push(f.parent().expect("parent").join(p)),
                        None => work.push(mod_file(f, name, &includes)),
                    }
                }
            }
        }
    }
    while let Some(f) = work.pop() {
        if !f.exists() || !found.insert(f.clone()) {
            continue;
        }
        let code = blank_literals(&std::fs::read_to_string(&f).expect("read"));
        for line in code.lines() {
            let t = line.trim();
            if let Some(n) = t
                .strip_prefix("pub(crate) mod ")
                .or_else(|| t.strip_prefix("pub mod "))
                .or_else(|| t.strip_prefix("mod "))
                .and_then(|r| r.strip_suffix(';'))
            {
                work.push(mod_file(&f, n.trim(), &includes));
            }
        }
    }
    found
}

/// Where `mod name;` declared in `decl` lives.
fn mod_file(
    decl: &Path,
    name: &str,
    includes: &std::collections::BTreeMap<PathBuf, PathBuf>,
) -> PathBuf {
    let mut decl = decl;
    while let Some(parent) = includes.get(decl) {
        decl = parent;
    }
    let dir = decl.parent().expect("parent");
    let stem = decl.file_stem().unwrap().to_string_lossy();
    let base = if matches!(stem.as_ref(), "mod" | "lib" | "main") {
        dir.to_path_buf()
    } else {
        dir.join(stem.as_ref())
    };
    let flat = base.join(format!("{name}.rs"));
    if flat.exists() {
        flat
    } else {
        base.join(name).join("mod.rs")
    }
}

/// The statement around `at`: from the previous `;`, `{` or `}` to the next `;`.
pub fn statement_at(code: &str, at: usize) -> String {
    let b = code.as_bytes();
    let mut s = at;
    while s > 0 && !matches!(b[s - 1], b';' | b'{' | b'}') {
        s -= 1;
    }
    let mut e = at;
    while e < b.len() && b[e] != b';' {
        e += 1;
    }
    code[s..e].split_whitespace().collect::<Vec<_>>().join(" ")
}

/// `include!("x.rs")` edges: included file -> the file that includes it. An
/// included file's `mod` lines resolve as if they were written in the includer.
pub fn include_map(files: &[PathBuf]) -> std::collections::BTreeMap<PathBuf, PathBuf> {
    let mut map = std::collections::BTreeMap::new();
    for f in files {
        let src = std::fs::read_to_string(f).expect("read");
        let mut rest = src.as_str();
        while let Some(p) = rest.find("include!(\"") {
            let tail = &rest[p + 10..];
            if let Some(q) = tail.find('"') {
                map.insert(f.parent().unwrap().join(&tail[..q]), f.clone());
            }
            rest = &rest[p + 10..];
        }
    }
    map
}

/// Index just past the `)` that closes the `(` at `open`.
fn close_paren(code: &str, open: usize) -> usize {
    let b = code.as_bytes();
    let mut depth = 0usize;
    for (i, &c) in b.iter().enumerate().skip(open) {
        match c {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return i + 1;
                }
            }
            _ => {}
        }
    }
    b.len()
}

/// Calls to `token(` whose result is thrown away: `.ok()`, `.unwrap_or*`,
/// `.is_ok()` on the call chain, or `let _ =` / `if let Ok(..) =` in front of it.
/// Returns `(byte offset, why)`. A load or resolve error must propagate: a
/// command that swallows it reports on a manifest it never loaded (forjar#660).
pub fn swallowed_results(code: &str, token: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(p) = code[at..].find(token) {
        let pos = at + p;
        at = pos + token.len();
        let before_ok = pos == 0 || !(code.as_bytes()[pos - 1].is_ascii_alphanumeric());
        let rest = &code[pos + token.len()..];
        if !before_ok || !rest.starts_with('(') {
            continue;
        }
        if code[..pos].trim_end().ends_with("fn") {
            continue;
        }
        let open = pos + token.len();
        let mut end = close_paren(code, open);
        // Walk the `.method(...)` / `?` chain after the call.
        let mut propagated = false;
        let mut why = Vec::new();
        loop {
            let tail = code[end..].trim_start();
            let skipped = code[end..].len() - tail.len();
            if let Some(m) = tail.strip_prefix('.') {
                let name: String = m
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if matches!(
                    name.as_str(),
                    "ok" | "unwrap_or"
                        | "unwrap_or_else"
                        | "unwrap_or_default"
                        | "is_ok"
                        | "is_err"
                        | "err"
                ) {
                    why.push(format!(".{name}()"));
                }
                let after_name = end + skipped + 1 + name.len();
                end = if code[after_name..].starts_with('(') {
                    close_paren(code, after_name)
                } else {
                    after_name
                };
            } else if tail.starts_with('?') {
                propagated = true;
                // `?` propagates; whatever follows is on the Ok value.
                break;
            } else {
                break;
            }
        }
        let stmt_start = code[..pos].rfind([';', '{', '}']).map_or(0, |i| i + 1);
        let lead: String = code[stmt_start..pos]
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        for pat in [
            "let _ =",
            "if let Ok(",
            "let Ok(",
            "while let Ok(",
            "if let Some(",
        ] {
            if !propagated && lead.contains(pat) {
                why.push(pat.to_string());
            }
        }
        if !propagated && lead.ends_with("match") && match_arm_drops_err(&code[end..]) {
            why.push("match Err(_)".to_string());
        }
        if !why.is_empty() {
            out.push((pos, why.join(" ")));
        }
    }
    out
}

/// The file named by a `#[path = "..."]` attribute in the text of one item.
fn path_attr(item: &str) -> Option<String> {
    let at = item.find("#[path")?;
    let rest = &item[at..];
    let q1 = rest.find('"')?;
    let q2 = rest[q1 + 1..].find('"')?;
    Some(rest[q1 + 1..q1 + 1 + q2].to_string())
}

/// Does the `{ ... }` block at the start of `rest` have an `Err(_)` arm?
fn match_arm_drops_err(rest: &str) -> bool {
    let t = rest.trim_start();
    if !t.starts_with('{') {
        return false;
    }
    let mut depth = 0usize;
    for (i, c) in t.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return t[..i].replace(' ', "").contains("Err(_)=>");
                }
            }
            _ => {}
        }
    }
    false
}
