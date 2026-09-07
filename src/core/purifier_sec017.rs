//! PMAT-204: decide a `chmod` line's MODE ARGUMENT ourselves, instead of
//! letting bashrs SEC017 decide it from the line's literal text.
//!
//! SEC017 (`bashrs 6.68.0`, `src/linter/rules/sec017.rs`) does this: for every
//! line containing the word `chmod`, it scans EVERY whitespace-separated word
//! of the line for the substrings `777`, `666`, `664`, `776`, `677`, with a
//! digit-boundary check either side. It never asks which word is the mode. The
//! consequence, measured 2026-09-07 against the pinned bashrs, one line per
//! script:
//!
//! ```text
//! | script line                                | SEC017                          |
//! |--------------------------------------------|---------------------------------|
//! | chmod '0644' '/tmp/.tmp5k666T/target.txt'   | ERROR (false: 666 is in PATH)   |
//! | chmod '0644' '/opt/app666/t'                | ERROR (false positive)          |
//! | chmod '0644' '/tmp/x777y/target.txt'        | ERROR (false positive)          |
//! | chmod '0644' '/tmp/plain/target.txt'        | clean                           |
//! | chmod '0666' '/tmp/plain/t'                 | NO FINDING AT ALL               |
//! | chmod 666 /tmp/real                         | ERROR (true positive)           |
//! ```
//!
//! Rows 1-3 are why `forjar apply` could not write a file under a directory
//! named `app666`: forjar generates `chmod '<mode>' '<path>'`, the gate refused
//! its own correct script, and the resource failed.
//!
//! Row 5 is why this module is a CHANGE OF INSTRUMENT and not a reduction of
//! the gate. The boundary check refuses to match `666` inside `'0666'` because
//! the character before it is the digit `0` — so the one shape forjar actually
//! emits for a world-writable mode is the one shape SEC017 cannot see. Reading
//! the mode argument fixes both directions at once: the path stops being
//! mistaken for a mode (rows 1-3), and the mode stops being invisible (row 5,
//! now [`ChmodVerdict::WorldWritable`], refused by forjar under its own code).
//!
//! The exemption is deliberately narrow. It requires a line with EXACTLY ONE
//! chmod command whose first non-flag argument is a QUOTED octal literal with
//! the world-write bit clear. A bare numeric mode, a symbolic mode (`u+x`), a
//! `--reference` chmod, a second chmod on the same line, a line where `chmod`
//! is an argument rather than the command, or anything this module cannot parse
//! is [`ChmodVerdict::Undecided`] — bashrs keeps the last word and the finding
//! stays an Error.
//!
//! FALSIFY IT: make [`chmod_mode_verdict`] return [`ChmodVerdict::SafeQuotedMode`]
//! unconditionally and `tests/falsification_chmod_path_is_not_a_mode.rs` goes
//! red on the bare-`chmod 666`, second-chmod-on-the-line and non-chmod-line
//! cells.

/// What forjar makes of the `chmod` line a SEC017 diagnostic points at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ChmodVerdict {
    /// Exactly one chmod, mode argument is a quoted octal literal without the
    /// world-write bit. SEC017 matched something that is not the mode.
    SafeQuotedMode,
    /// Exactly one chmod and its mode argument really is world-writable. Carries
    /// the literal as written, so the message can name it.
    WorldWritable(String),
    /// Not decidable here. Whatever bashrs said stands.
    Undecided,
}

/// True where a `chmod` occurrence is a command word rather than a substring
/// (`echo chmod` still counts as a word; `chmodx` and `/bin/chmod-ish` do not).
fn is_word_boundary(line: &str, pos: usize, len: usize) -> bool {
    let bytes = line.as_bytes();
    let before_ok = pos == 0 || matches!(bytes[pos - 1], b' ' | b'\t' | b';' | b'&' | b'|' | b'(');
    let after = pos + len;
    let after_ok = after >= bytes.len() || matches!(bytes[after], b' ' | b'\t' | b';' | b'&' | b'|');
    before_ok && after_ok
}

/// Words that may stand between the start of a command and `chmod` without
/// making `chmod` an argument of something else.
const COMMAND_PREFIX_WORDS: &[&str] = &["sudo", "then", "do", "else", "{", "!", "time"];

/// True where the text between the start of this command and `chmod` leaves
/// `chmod` in command position. `echo chmod 777 /x` is a chmod WORD but not a
/// chmod COMMAND — it has no mode argument to read, so bashrs keeps it.
fn is_command_position(line: &str, pos: usize) -> bool {
    let start = line[..pos]
        .rfind([';', '&', '|', '(', '\n'])
        .map_or(0, |i| i + 1);
    line[start..pos].split_whitespace().all(|w| {
        // `FOO=bar chmod ...` — an environment assignment, still command position.
        w.contains('=') || COMMAND_PREFIX_WORDS.contains(&w)
    })
}

/// Byte offsets of every `chmod` COMMAND on the line.
fn chmod_positions(line: &str) -> Vec<usize> {
    let mut out = Vec::new();
    let mut from = 0usize;
    while let Some(rel) = line[from..].find("chmod") {
        let pos = from + rel;
        if is_word_boundary(line, pos, "chmod".len()) && is_command_position(line, pos) {
            out.push(pos);
        }
        from = pos + "chmod".len();
    }
    out
}

/// Strip one matched pair of surrounding quotes, reporting whether there was one.
fn unquote(token: &str) -> (&str, bool) {
    let bytes = token.as_bytes();
    if bytes.len() >= 2 {
        let first = bytes[0];
        if (first == b'\'' || first == b'"') && bytes[bytes.len() - 1] == first {
            return (&token[1..token.len() - 1], true);
        }
    }
    (token, false)
}

/// An octal file mode written as 3 or 4 digits, or `None` for anything else
/// (symbolic modes, empty strings, `0o644`, a path).
fn parse_octal_mode(text: &str) -> Option<u32> {
    if !(text.len() == 3 || text.len() == 4) {
        return None;
    }
    if !text.bytes().all(|b| (b'0'..=b'7').contains(&b)) {
        return None;
    }
    u32::from_str_radix(text, 8).ok()
}

/// The first argument to chmod that is not an option, or `None` if an option
/// makes the mode argument meaningless (`--reference=FILE` takes the mode from
/// another file, so there is nothing on this line to judge).
fn mode_token(rest: &str) -> Option<&str> {
    for token in rest.split_whitespace() {
        if token.starts_with("--reference") {
            return None;
        }
        if token.starts_with('-') && token.len() > 1 {
            continue;
        }
        return Some(token);
    }
    None
}

/// Read the mode argument of the single `chmod` command on `line`.
///
/// Everything this cannot resolve is [`ChmodVerdict::Undecided`] on purpose:
/// this function may only ever EXEMPT a finding, so an unparsable line must
/// leave the gate exactly as strict as it was.
pub(crate) fn chmod_mode_verdict(line: &str) -> ChmodVerdict {
    let positions = chmod_positions(line);
    // Two chmods share one SEC017 finding (the rule reports once per line), so
    // judging only the first would let `chmod '0644' '/a'; chmod 666 /b` pass.
    if positions.len() != 1 {
        return ChmodVerdict::Undecided;
    }
    let rest = &line[positions[0] + "chmod".len()..];
    let token = match mode_token(rest) {
        Some(t) => t,
        None => return ChmodVerdict::Undecided,
    };
    let (inner, quoted) = unquote(token);
    let mode = match parse_octal_mode(inner) {
        Some(m) => m,
        None => return ChmodVerdict::Undecided,
    };
    if mode & 0o002 != 0 {
        return ChmodVerdict::WorldWritable(inner.to_string());
    }
    if quoted {
        ChmodVerdict::SafeQuotedMode
    } else {
        // A bare `chmod 644 /x` is not a shape forjar generates, and an
        // unquoted argument is not a literal we can be sure of. Leave it.
        ChmodVerdict::Undecided
    }
}

#[cfg(test)]
mod chmod_mode_tests {
    use super::*;

    /// The shape forjar generates, on the paths that broke: the mode is safe
    /// and quoted, so the SEC017 hit is on the path.
    #[test]
    fn forjar_generated_lines_are_exempt() {
        for line in [
            "chmod '0644' '/tmp/.tmp5k666T/target.txt'",
            "chmod '0644' '/opt/app666/t'",
            "chmod '0755' '/tmp/x777y/bin'",
            "chmod -R '0750' '/srv/app666'",
            "chmod \"0644\" \"/opt/app666/t\"",
        ] {
            assert_eq!(
                chmod_mode_verdict(line),
                ChmodVerdict::SafeQuotedMode,
                "not exempted: {line}"
            );
        }
    }

    /// Row 5 of the measurement table: bashrs sees nothing, forjar must.
    #[test]
    fn a_quoted_world_writable_mode_is_named() {
        assert_eq!(
            chmod_mode_verdict("chmod '0666' '/tmp/plain/t'"),
            ChmodVerdict::WorldWritable("0666".into())
        );
        assert_eq!(
            chmod_mode_verdict("chmod '0777' '/tmp/plain/t'"),
            ChmodVerdict::WorldWritable("0777".into())
        );
        assert_eq!(
            chmod_mode_verdict("chmod 0666 /tmp/plain/t"),
            ChmodVerdict::WorldWritable("0666".into())
        );
        // 0662: world-write bit set, no dangerous substring anywhere. bashrs
        // has no rule that reaches it; the bit is what forjar judges.
        assert_eq!(
            chmod_mode_verdict("chmod '0662' '/tmp/plain/t'"),
            ChmodVerdict::WorldWritable("0662".into())
        );
    }

    /// Everything the parser cannot resolve must leave bashrs in charge.
    #[test]
    fn unresolvable_lines_are_undecided() {
        for line in [
            // bare mode — not a shape forjar generates
            "chmod 644 /srv/data",
            // symbolic mode
            "chmod u+x '/tmp/666/t'",
            // chmod is an argument, not the command
            "echo chmod 777 /var/www",
            // two chmods share one SEC017 finding
            "chmod '0644' '/srv/a'; chmod 666 /srv/b",
            // the mode comes from another file
            "chmod --reference='/srv/a666' '/srv/b'",
            // no argument at all
            "chmod",
            // not a chmod line
            "echo '/opt/app666/t'",
        ] {
            assert_eq!(
                chmod_mode_verdict(line),
                ChmodVerdict::Undecided,
                "wrongly decided: {line}"
            );
        }
    }

    #[test]
    fn octal_parsing_rejects_non_modes() {
        assert_eq!(parse_octal_mode("0644"), Some(0o644));
        assert_eq!(parse_octal_mode("644"), Some(0o644));
        assert_eq!(parse_octal_mode("1777"), Some(0o1777));
        assert_eq!(parse_octal_mode("68"), None);
        assert_eq!(parse_octal_mode("0888"), None);
        assert_eq!(parse_octal_mode("00644"), None);
        assert_eq!(parse_octal_mode(""), None);
    }

    #[test]
    fn quoting_is_read_not_assumed() {
        assert_eq!(unquote("'0644'"), ("0644", true));
        assert_eq!(unquote("\"0644\""), ("0644", true));
        assert_eq!(unquote("0644"), ("0644", false));
        assert_eq!(unquote("'0644"), ("'0644", false));
        assert_eq!(unquote("'"), ("'", false));
    }
}
