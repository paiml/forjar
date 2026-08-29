//! Byte-range-anchored, in-place edits to a YAML document (paiml/forjar#359).
//!
//! Every transformation forjar had that "fixes" a config re-serialised the
//! whole document: `serde_yaml_ng::from_str::<Value>` then `to_string`, or a
//! round-trip through `ForjarConfig`. Neither type carries comments, so both
//! delete every comment in the user's file — silently, as a side effect of a
//! flag whose contract was to sort some keys. `serde_yaml_ng` additionally
//! rewrites quote style, so the diff an operator reads is dominated by noise
//! unrelated to the change they asked for.
//!
//! This module edits the SOURCE TEXT instead. It locates the byte range of the
//! node being changed and replaces exactly those bytes; every other byte —
//! comments, quote style, key order, blank lines, trailing whitespace — is
//! copied through untouched.
//!
//! The scanner is a hand-rolled block-mapping walk, which is exactly the sort
//! of code that corrupts a config file, so it is built to **refuse rather than
//! guess**. Flow style, block scalars, anchors/aliases/tags, duplicate keys and
//! multi-line values all return an [`AnchorError`] instead of an edit. Callers
//! pair that with [`verify::changed_paths`], which re-parses both documents and
//! proves the only semantic difference is the intended one. A wrong anchor
//! cannot land — it can only fail closed.

pub mod blocks;
pub mod verify;

#[cfg(test)]
mod tests_anchor;
#[cfg(test)]
mod tests_blocks;

/// Why an edit was refused.
///
/// Every variant is a REFUSAL, never a guess: the scanner reports what it could
/// not prove instead of editing bytes it did not understand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnchorError {
    /// The path names no key in the source text.
    NotFound,
    /// The node is written in flow style (`{...}` / `[...]`).
    FlowStyle,
    /// The value is a block scalar (`|` / `>`).
    BlockScalar,
    /// The value is a YAML anchor, alias or tag.
    Alias,
    /// The key appears more than once at that path.
    Duplicate,
    /// The value spans more than one line.
    Multiline,
    /// The path does not name a block mapping.
    NotAMapping,
    /// The region's last line has no terminating newline.
    Unterminated,
}

impl AnchorError {
    /// A sentence an operator can act on. Used verbatim as the `reason` on an
    /// unfixable violation, so it must say what forjar refused and why.
    pub fn reason(self) -> &'static str {
        match self {
            Self::NotFound => {
                "no such key in the document text — the value may come from an \
                 include, a recipe, or a {{template}} expansion"
            }
            Self::FlowStyle => {
                "the node is written in flow style ({...} / [...]); editing it by \
                 byte range is not sound, so forjar refuses rather than guesses"
            }
            Self::BlockScalar => "the value is a block scalar (| or >)",
            Self::Alias => "the value is a YAML anchor, alias or tag",
            Self::Duplicate => "the key appears more than once at that path",
            Self::Multiline => "the value spans more than one line",
            Self::NotAMapping => "the path does not name a block mapping",
            Self::Unterminated => "the last line of the region has no terminating newline",
        }
    }
}

/// The byte range of one scalar VALUE in the source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScalarSpan {
    /// 1-based line number, for reporting.
    pub line: usize,
    /// Byte offset of the first byte of the value.
    pub byte_start: usize,
    /// Byte offset one past the last byte of the value.
    pub byte_end: usize,
}

/// One source line, with the byte offsets needed to splice around it.
pub(crate) struct Line<'a> {
    /// The line without its terminator (and without a CR before it).
    pub(crate) text: &'a str,
    /// Byte offset of the line's first byte.
    pub(crate) start: usize,
    /// Byte offset one past the line's terminator.
    pub(crate) next: usize,
    /// Leading-space count, or [`usize::MAX`] for a line that is transparent to
    /// the walk: blank, comment-only, or tab-indented (tabs are invalid YAML
    /// indentation, so refusing to match them is fail-closed).
    pub(crate) indent: usize,
}

pub(crate) const TRANSPARENT: usize = usize::MAX;

/// Split `text` into lines, preserving byte offsets and CRLF terminators.
pub(crate) fn scan_lines(text: &str) -> Vec<Line<'_>> {
    let mut out = Vec::new();
    let mut start = 0usize;
    for raw in text.split_inclusive('\n') {
        let body = raw.strip_suffix('\n').unwrap_or(raw);
        let body = body.strip_suffix('\r').unwrap_or(body);
        out.push(Line {
            text: body,
            start,
            next: start + raw.len(),
            indent: indent_of(body),
        });
        start += raw.len();
    }
    out
}

fn indent_of(body: &str) -> usize {
    let n = body.len() - body.trim_start_matches(' ').len();
    let rest = &body[n..];
    if rest.is_empty() || rest.starts_with('#') || rest.starts_with('\t') {
        TRANSPARENT
    } else {
        n
    }
}

/// Whether a line is a comment-only line (blank lines are NOT comments).
pub(crate) fn is_comment(line: &Line<'_>) -> bool {
    line.text.trim_start().starts_with('#')
}

/// Characters allowed in a plain scalar key. Deliberately narrow: a key made of
/// anything else is refused rather than parsed by guesswork.
fn is_key_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | '/' | '+' | '@')
}

/// Parse `key:` at the start of `rest`. Returns the key and the byte index just
/// past the colon. `key:value` (no space) is a plain scalar in YAML, not a
/// mapping entry, and is rejected here.
fn parse_key(rest: &str) -> Option<(String, usize)> {
    let (raw, end) = match rest.chars().next()? {
        '"' => quoted_key(rest, '"')?,
        '\'' => quoted_key(rest, '\'')?,
        _ => {
            let n = rest.find(|c| !is_key_char(c))?;
            if n == 0 {
                return None;
            }
            (rest[..n].to_string(), n)
        }
    };
    let after = &rest[end..];
    let tail = after.strip_prefix(':')?;
    if tail.is_empty() || tail.starts_with(' ') {
        Some((raw, end + 1))
    } else {
        None
    }
}

fn quoted_key(rest: &str, quote: char) -> Option<(String, usize)> {
    let close = rest[1..].find(quote)? + 1;
    Some((rest[1..close].to_string(), close + 1))
}

/// The key on this line, if it is a key line at exactly `indent`.
pub(crate) fn key_at(line: &Line<'_>, indent: usize) -> Option<(String, usize)> {
    if line.indent != indent {
        return None;
    }
    let rest = &line.text[indent..];
    if rest.starts_with('-') {
        return None;
    }
    parse_key(rest).map(|(k, consumed)| (k, indent + consumed))
}

/// The inline value on a key line: its absolute byte offset and its text.
/// A trailing `# comment` with no value before it reads as "no value".
pub(crate) fn value_of<'a>(line: &Line<'a>, key_end: usize) -> (usize, &'a str) {
    let rest = &line.text[key_end..];
    let ws = rest.len() - rest.trim_start_matches(' ').len();
    let val = rest[ws..].trim_end();
    if val.starts_with('#') {
        return (line.start + line.text.len(), "");
    }
    (line.start + key_end + ws, val)
}

/// Classify an inline value that appeared where a nested mapping was expected.
pub(crate) fn classify_inline(val: &str) -> AnchorError {
    classify_prefix(val).unwrap_or(AnchorError::NotAMapping)
}

/// Refuse the value shapes a byte-range splice cannot reason about.
fn classify_prefix(val: &str) -> Option<AnchorError> {
    match val.chars().next()? {
        '{' | '[' => Some(AnchorError::FlowStyle),
        '|' | '>' => Some(AnchorError::BlockScalar),
        '&' | '*' | '!' => Some(AnchorError::Alias),
        _ => None,
    }
}

/// Where the final key of `path` lives.
pub(crate) struct Located {
    pub(crate) line: usize,
    pub(crate) indent: usize,
    pub(crate) range_end: usize,
    pub(crate) key_end: usize,
}

/// Walk the block mappings named by `path` and return the final key's position.
pub(crate) fn locate_key(lines: &[Line<'_>], path: &[&str]) -> Result<Located, AnchorError> {
    if path.is_empty() {
        return Err(AnchorError::NotFound);
    }
    let (mut lo, mut hi) = (0usize, lines.len());
    let mut indent = first_key_indent(lines, lo, hi).ok_or(AnchorError::NotAMapping)?;
    for (i, seg) in path.iter().enumerate() {
        let (idx, key_end) = find_key(lines, lo, hi, indent, seg)?;
        if i + 1 == path.len() {
            return Ok(Located {
                line: idx,
                indent,
                range_end: hi,
                key_end,
            });
        }
        let (_, val) = value_of(&lines[idx], key_end);
        if !val.is_empty() {
            return Err(classify_inline(val));
        }
        let (clo, chi) = child_range(lines, idx, indent, hi);
        indent = first_key_indent(lines, clo, chi).ok_or(AnchorError::NotAMapping)?;
        lo = clo;
        hi = chi;
    }
    Err(AnchorError::NotFound)
}

/// The one line in `[lo, hi)` at `indent` whose key is `key`.
/// Two matches is [`AnchorError::Duplicate`] — never "the first one".
fn find_key(
    lines: &[Line<'_>],
    lo: usize,
    hi: usize,
    indent: usize,
    key: &str,
) -> Result<(usize, usize), AnchorError> {
    let mut found: Option<(usize, usize)> = None;
    for (j, line) in lines.iter().enumerate().take(hi).skip(lo) {
        if let Some((k, key_end)) = key_at(line, indent) {
            if k == key {
                if found.is_some() {
                    return Err(AnchorError::Duplicate);
                }
                found = Some((j, key_end));
            }
        }
    }
    found.ok_or(AnchorError::NotFound)
}

/// The line range nested under the key line at `idx`.
pub(crate) fn child_range(
    lines: &[Line<'_>],
    idx: usize,
    indent: usize,
    hi: usize,
) -> (usize, usize) {
    let lo = (idx + 1).min(hi);
    let mut end = lo;
    for (j, line) in lines.iter().enumerate().take(hi).skip(lo) {
        if line.indent != TRANSPARENT && line.indent <= indent {
            break;
        }
        end = j + 1;
    }
    (lo, end)
}

/// The indent of the mapping starting at `lo`, or `None` if the first
/// non-transparent line there is not a key line.
pub(crate) fn first_key_indent(lines: &[Line<'_>], lo: usize, hi: usize) -> Option<usize> {
    for line in lines.iter().take(hi).skip(lo) {
        if line.indent == TRANSPARENT {
            continue;
        }
        return parse_key(&line.text[line.indent..]).map(|_| line.indent);
    }
    None
}

/// The byte range of the scalar value at `path`, or the reason it was refused.
pub fn find_scalar(text: &str, path: &[&str]) -> Result<ScalarSpan, AnchorError> {
    let lines = scan_lines(text);
    let at = locate_key(&lines, path)?;
    let (vs, val) = value_of(&lines[at.line], at.key_end);
    if val.is_empty() {
        return Err(AnchorError::NotFound);
    }
    if let Some(e) = classify_prefix(val) {
        return Err(e);
    }
    if next_line_is_deeper(&lines, at.line, at.indent) {
        return Err(AnchorError::Multiline);
    }
    let len = scalar_len(val)?;
    Ok(ScalarSpan {
        line: at.line + 1,
        byte_start: vs,
        byte_end: vs + len,
    })
}

/// A key line with an inline value cannot have children, so a deeper next line
/// means the scalar continues onto it.
fn next_line_is_deeper(lines: &[Line<'_>], idx: usize, indent: usize) -> bool {
    lines
        .get(idx + 1)
        .is_some_and(|n| n.indent != TRANSPARENT && n.indent > indent)
}

/// How many bytes of `val` are the scalar, excluding a trailing ` # comment`.
fn scalar_len(val: &str) -> Result<usize, AnchorError> {
    match val.chars().next() {
        Some('"') => quoted_len(val, '"', true),
        Some('\'') => quoted_len(val, '\'', false),
        _ => Ok(val.find(" #").unwrap_or(val.len())),
    }
    .map(|n| val[..n].trim_end().len())
}

fn quoted_len(val: &str, quote: char, escapes: bool) -> Result<usize, AnchorError> {
    let bytes = val.as_bytes();
    let mut i = 1;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if escapes && c == '\\' {
            i += 2;
            continue;
        }
        if c == quote {
            return Ok(i + 1);
        }
        i += 1;
    }
    Err(AnchorError::Multiline)
}

/// The source text of a located scalar, quotes included.
pub fn scalar_text<'a>(text: &'a str, span: &ScalarSpan) -> &'a str {
    &text[span.byte_start..span.byte_end]
}

/// Strip one layer of matching quotes. Used only to compare a document scalar
/// against the value the parser resolved for the same field.
pub fn unquote(s: &str) -> String {
    for q in ['"', '\''] {
        if s.len() >= 2 && s.starts_with(q) && s.ends_with(q) {
            return s[1..s.len() - 1].to_string();
        }
    }
    s.to_string()
}

/// Replace the bytes of `span` with `new_value`. Every other byte is copied.
pub fn splice(text: &str, span: &ScalarSpan, new_value: &str) -> String {
    let mut out = String::with_capacity(text.len() + new_value.len());
    out.push_str(&text[..span.byte_start]);
    out.push_str(new_value);
    out.push_str(&text[span.byte_end..]);
    out
}

/// Render `value` as a YAML scalar.
///
/// The emitter decides the quoting, not forjar: `0644` must come back as
/// `'0644'` and `hello` as `hello`, and a hand-rolled rule for that would be
/// one more thing to get wrong. A value that cannot be written on one line is
/// refused.
pub fn emit_scalar(value: &str) -> Result<String, AnchorError> {
    let rendered = serde_yaml_ng::to_string(&serde_yaml_ng::Value::String(value.to_string()))
        .map_err(|_| AnchorError::Multiline)?;
    let one_line = rendered.trim_end_matches('\n');
    if one_line.contains('\n') {
        return Err(AnchorError::Multiline);
    }
    Ok(one_line.to_string())
}
