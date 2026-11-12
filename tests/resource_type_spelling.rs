//! forjar#433 falsifier: CLI display sites print ResourceType in its serde spelling
//! (`systemd_unit`), never the Debug spelling, so the output can be typed back into `type:`.
use std::fs;
use std::path::Path;

fn walk(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for e in fs::read_dir(dir).expect("src/cli readable") {
        let p = e.expect("entry").path();
        if p.is_dir() {
            walk(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

fn has_word(hay: &str, word: &str) -> bool {
    hay.match_indices(word).any(|(i, _)| {
        let before = hay[..i].chars().next_back();
        let after = hay[i + word.len()..].chars().next();
        let w = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
        !w(before) && !w(after)
    })
}

/// Every `name!( ... )` macro call, over however many lines, as its argument text.
fn macro_spans(text: &str) -> Vec<String> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(off) = text[i..].find("!(") {
        let open = i + off + 1;
        let mut depth = 0i32;
        let mut end = None;
        for (j, &c) in b.iter().enumerate().skip(open) {
            match c {
                b'(' => depth += 1,
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(j);
                        break;
                    }
                }
                _ => {}
            }
        }
        let Some(end) = end else { break };
        out.push(text[open + 1..end].to_string());
        i = open + 1;
    }
    out
}

/// Split a macro call into its format string and its trailing arguments.
fn literal_and_tail(span: &str) -> Option<(&str, &str)> {
    let rest = &span[span.find('"')? + 1..];
    let rb = rest.as_bytes();
    let mut k = 0;
    while k < rb.len() {
        match rb[k] {
            b'\\' => k += 2,
            b'"' => return Some((&rest[..k], &rest[k + 1..])),
            _ => k += 1,
        }
    }
    None
}

/// Top-level comma-separated arguments.
fn split_args(tail: &str) -> Vec<String> {
    let (mut args, mut depth, mut cur) = (Vec::new(), 0i32, String::new());
    for c in tail.chars() {
        depth += i32::from(matches!(c, '(' | '[' | '{')) - i32::from(matches!(c, ')' | ']' | '}'));
        if c == ',' && depth == 0 {
            args.push(std::mem::take(&mut cur));
        } else {
            cur.push(c);
        }
    }
    args.push(cur);
    let trimmed = args.into_iter().map(|a| a.trim().to_string());
    trimmed.filter(|a| !a.is_empty()).collect()
}

/// The `{name:spec}` placeholders of a format string, `{{` escapes skipped.
fn placeholders(lit: &str) -> Vec<(String, String)> {
    let plain = lit.replace("{{", "").replace("}}", "");
    let mut out = Vec::new();
    for part in plain.split('{').skip(1) {
        let inner = part.split('}').next().unwrap_or("");
        let (name, spec) = inner.split_once(':').unwrap_or((inner, ""));
        out.push((name.to_string(), spec.to_string()));
    }
    out
}

/// What each Debug placeholder (`{:?}`, `{x:?}`, `{:#?}`) in the span's format
/// string is bound to: the matching positional argument, or the inline name.
fn debug_bound_texts(span: &str) -> Vec<String> {
    let Some((lit, tail)) = literal_and_tail(span) else {
        return vec![];
    };
    let args = split_args(tail);
    let mut pos = 0usize;
    let mut out = Vec::new();
    for (name, spec) in placeholders(lit) {
        let bound = if name.is_empty() {
            pos += 1;
            args.get(pos - 1).cloned().unwrap_or_default()
        } else {
            name
        };
        if spec.ends_with('?') {
            out.push(bound);
        }
    }
    out
}

/// Explicit exemptions: these Debug spellings feed hashes, which must stay
/// byte-stable across releases (golden tests in lineage.rs / provenance.rs).
fn is_hash_site(file: &str, span: &str) -> bool {
    let span = span.trim_start();
    match file {
        "model_eval.rs" | "repro_proof.rs" => span.trim() == "\"{rtype:?}\"",
        "lineage.rs" | "provenance.rs" => span.starts_with("\"{:?}\""),
        _ => false,
    }
}

#[test]
fn cli_never_debug_formats_resource_type() {
    let mut files = Vec::new();
    walk(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cli"),
        &mut files,
    );
    assert!(
        files.len() > 50,
        "scanned only {} files: vacuous",
        files.len()
    );
    let mut hits = Vec::new();
    for f in &files {
        let text = fs::read_to_string(f).expect("utf8");
        for span in macro_spans(&text) {
            for bound in debug_bound_texts(&span) {
                // `x.resource_type.to_string()` is the fixed spelling, not a Debug use.
                let bare = bound.replace("resource_type.to_string()", "");
                let names_type = bare.contains("resource_type") || has_word(&bare, "rtype");
                let name = f.file_name().and_then(|n| n.to_str()).unwrap_or("");
                // Hash inputs must stay byte-stable across releases: not a display site.
                let is_hash_input = is_hash_site(name, &span);
                if names_type && !is_hash_input {
                    hits.push(format!("{}: {}", f.display(), span.replace('\n', " ")));
                }
            }
        }
    }
    assert!(
        hits.is_empty(),
        "Debug-formatted ResourceType in CLI:\n{}",
        hits.join("\n")
    );
}

#[test]
fn display_is_serde_spelling() {
    use forjar::core::types::ResourceType;
    assert_eq!(ResourceType::WasmBundle.to_string(), "wasm_bundle");
    assert_eq!(ResourceType::GithubRelease.to_string(), "github_release");
}

#[test]
fn scanner_catches_multiline_and_inline_debug_calls() {
    let hit = |t: &str| {
        macro_spans(t).iter().any(|s| {
            debug_bound_texts(s)
                .iter()
                .any(|b| b.contains("resource_type") || has_word(b, "rtype"))
        })
    };
    let multi = "println!(\n    \"t={:?}\",\n    rlock.resource_type\n);";
    let second = "println!(\"{} {:?}\", a,\n    rl.resource_type);";
    let inline = "let s = format!(\"{rtype:?}\");";
    let clean = "println!(\"t={}\", rlock.resource_type);";
    let other = "println!(\"{},{:?}\", rl.resource_type, rl.status);";
    assert!(hit(multi) && hit(second) && hit(inline));
    assert!(!hit(clean) && !hit(other));
}

/// Lines that compare a ResourceType's text with a literal that starts upper-case.
///
/// `Display` is snake_case, so no such comparison can ever be true: the branch
/// behind it is dead and the check it stands for never runs (review round 1 of
/// #943: `== "Unknown"` in `validate_deep` and `validate_quality`, which had
/// compared the Debug spelling of an enum that has no such variant). The left
/// side is either a `resource_type` expression or a variable bound from one.
fn pascal_compares(text: &str) -> Vec<String> {
    let bound: Vec<String> = text
        .lines()
        .filter(|l| l.contains("resource_type") && l.contains(".to_string()"))
        .filter_map(|l| l.trim().strip_prefix("let ")?.split([' ', ':']).next())
        .map(str::to_string)
        .collect();
    let mut hits = Vec::new();
    for line in text.lines() {
        let Some((lhs, rhs)) = line.split_once(" == \"") else {
            continue;
        };
        if !rhs.starts_with(|c: char| c.is_ascii_uppercase()) {
            continue;
        }
        let lhs_tail = lhs.trim_end();
        let var = lhs_tail
            .rsplit(|c: char| !(c.is_alphanumeric() || c == '_'))
            .next()
            .unwrap_or("");
        if lhs_tail.contains("resource_type") || bound.iter().any(|b| b == var) {
            hits.push(line.trim().to_string());
        }
    }
    hits
}

#[test]
fn cli_never_compares_resource_type_text_to_a_literal_display_cannot_produce() {
    let mut files = Vec::new();
    walk(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cli"),
        &mut files,
    );
    assert!(
        files.len() > 50,
        "scanned only {} files: vacuous",
        files.len()
    );
    let mut hits = Vec::new();
    for f in &files {
        let text = fs::read_to_string(f).expect("utf8");
        for h in pascal_compares(&text) {
            hits.push(format!("{}: {h}", f.display()));
        }
    }
    assert!(
        hits.is_empty(),
        "dead comparison: Display is snake_case, so these are never true:\n{}",
        hits.join("\n")
    );
}

#[test]
fn pascal_compare_scanner_sees_direct_and_bound_forms_only() {
    let direct = "if res.resource_type.to_string() == \"Unknown\" {";
    let bound = "let rt = res.resource_type.to_string();\nif rt == \"Unknown\" {";
    let spelled = "if res.resource_type.to_string() == \"wasm_bundle\" {";
    let other = "let rt = 3;\nif status == \"Converged\" {";
    assert_eq!(pascal_compares(direct).len(), 1);
    assert_eq!(pascal_compares(bound).len(), 1);
    assert!(pascal_compares(spelled).is_empty());
    assert!(pascal_compares(other).is_empty());
}
