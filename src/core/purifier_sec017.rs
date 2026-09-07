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
//! Both instruments were measured: the linked library (bashrs 6.68.0, which is
//! what `validate_script` calls, through `tests/falsification_chmod_path_is_not_a_mode.rs`)
//! and the standalone `bashrs lint` CLI at 7.0.1. They agree on every row.
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
//! HOW THE EXEMPTION DECIDES, AND WHY IT IS NOT A SHELL PARSER. The first
//! version of this module tried to find the chmod COMMAND on the line and read
//! its mode argument. Three blind review lanes and a direct re-run refuted it:
//! `chmod '0644' '/srv/a'; sudo -u root chmod 777 /srv/b`, the same line with
//! the second chmod inside backticks, and `env chmod 777 /srv/b` were all
//! ACCEPTED, because the second chmod was not recognised as a command position
//! and the line was therefore exempted whole. Those three lines were REFUSED
//! before PMAT-204: that version weakened the gate it was meant to keep.
//!
//! So forjar no longer judges the line. It REDACTS the path text and asks
//! bashrs again: every quoted argument whose content is not an octal mode is
//! replaced with `'/x'`, and the SEC017 finding is dropped only if the redacted
//! line no longer produces one. Everything else on the line — a second chmod, a
//! `sudo` prefix, backticks, `env`, an unquoted `777` — survives redaction
//! untouched and is still judged by the rule that always judged it. The
//! decision is bashrs's; forjar only removes the text that cannot be a mode.
//!
//! WHAT FORJAR JUDGES ITSELF is the other direction, where bashrs is blind
//! (row 5): [`world_writable_modes`] reads EVERY `chmod` word on a line, takes
//! the following non-flag token, and refuses an octal mode with the o+w bit or
//! a symbolic mode granting world write (`a+w`, `o+w`, `+w`). It is deliberately
//! over-eager — it does not care whether the chmod is in command position,
//! because refusing too much is the safe direction for a gate.
//!
//! WHAT NEITHER CAN DECIDE, recorded rather than claimed: a mode in a variable
//! (`chmod "$MODE" /x`), a mode taken from another file (`--reference=`), and a
//! mode computed at run time. Those pass this module exactly as they passed it
//! before PMAT-204.
//!
//! FALSIFY IT: make [`sec017_is_path_only`] return `true` unconditionally, or
//! make [`world_writable_modes`] return an empty vector, and
//! `tests/falsification_chmod_path_is_not_a_mode.rs` goes red.

use bashrs::linter::{lint_shell, Severity};

/// Replace every quoted argument that cannot be a mode with a fixed path.
///
/// A quoted token whose content parses as an octal mode is left alone — that is
/// the one quoted thing on a chmod line that IS a mode. Everything else quoted
/// is path-shaped text, and path text is what SEC017 mistakes for a mode.
fn redact_quoted_paths(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(open) = rest.find(['\'', '"']) {
        let quote = rest.as_bytes()[open] as char;
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        match after.find(quote) {
            Some(close) => {
                let inner = &after[..close];
                if parse_octal_mode(inner).is_some() {
                    out.push(quote);
                    out.push_str(inner);
                    out.push(quote);
                } else {
                    out.push_str("'/x'");
                }
                rest = &after[close + 1..];
            }
            None => {
                // An unbalanced quote: leave the remainder exactly as written.
                out.push_str(&rest[open..]);
                return out;
            }
        }
    }
    out.push_str(rest);
    out
}

/// True where SEC017 fired on this line only because of PATH text.
///
/// Redact the paths, ask bashrs again, and exempt the finding only if the rule
/// itself stops reporting. A line that still trips SEC017 with its paths
/// neutralised is a line whose chmod really is unsafe.
pub(crate) fn sec017_is_path_only(line: &str) -> bool {
    let redacted = redact_quoted_paths(line);
    if redacted == line {
        // Nothing was redacted, so no path text can be to blame.
        return false;
    }
    !lint_shell(&redacted)
        .diagnostics
        .iter()
        .any(|d| d.code == "SEC017" && d.severity == Severity::Error)
}

/// An octal file mode of 1 to 6 digits, or `None` for anything else.
///
/// The width is wide on purpose: `chmod '00666'` is a real world-writable mode
/// that a 3-or-4-digit rule read as "not a mode" and let through (measured).
fn parse_octal_mode(text: &str) -> Option<u32> {
    if text.is_empty() || text.len() > 6 {
        return None;
    }
    if !text.bytes().all(|b| (b'0'..=b'7').contains(&b)) {
        return None;
    }
    u32::from_str_radix(text, 8).ok()
}

/// Strip one matched pair of surrounding quotes.
fn unquote(token: &str) -> &str {
    let bytes = token.as_bytes();
    if bytes.len() >= 2 {
        let first = bytes[0];
        if (first == b'\'' || first == b'"') && bytes[bytes.len() - 1] == first {
            return &token[1..token.len() - 1];
        }
    }
    token
}

/// True where a symbolic mode grants write to everyone (`a+w`, `o+w`, `+w`).
fn symbolic_grants_world_write(text: &str) -> bool {
    let (who, op_rest) = match text.find(['+', '=']) {
        Some(i) => (&text[..i], &text[i..]),
        None => return false,
    };
    if !op_rest.contains('w') {
        return false;
    }
    who.is_empty() || who.contains('a') || who.contains('o')
}

/// Every world-writable mode written on `line`, as it was written.
///
/// Reads every `chmod` word, not only a command-position one: a mode inside
/// `find -exec chmod '0666' {} \;` is as world-writable as any other, and this
/// function may only ever ADD a refusal.
pub(crate) fn world_writable_modes(line: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut tokens = line.split_whitespace().peekable();
    while let Some(tok) = tokens.next() {
        if unquote(tok).trim_end_matches(';') != "chmod" {
            continue;
        }
        for arg in tokens.by_ref() {
            let inner = unquote(arg);
            if inner.starts_with("--reference") {
                break;
            }
            if inner.starts_with('-') && inner.len() > 1 && parse_octal_mode(inner).is_none() {
                continue;
            }
            match parse_octal_mode(inner) {
                Some(mode) if mode & 0o002 != 0 => found.push(inner.to_string()),
                Some(_) => {}
                None if symbolic_grants_world_write(inner) => found.push(inner.to_string()),
                None => {}
            }
            break;
        }
    }
    found
}

#[cfg(test)]
mod chmod_mode_tests {
    use super::*;

    /// The shape forjar generates, on the paths that broke.
    #[test]
    fn forjar_generated_lines_are_exempt() {
        for line in [
            "chmod '0644' '/tmp/.tmp5k666T/target.txt'",
            "chmod '0644' '/opt/app666/t'",
            "chmod '0755' '/tmp/x777y/bin'",
            "chmod -R '0750' '/srv/app666'",
            "chmod \"0644\" \"/opt/app666/t\"",
        ] {
            assert!(sec017_is_path_only(line), "not exempted: {line}");
        }
    }

    /// The refuted shapes: a second chmod that the first version of this module
    /// did not see. Each was ACCEPTED by that version and is refused here.
    #[test]
    fn a_real_chmod_elsewhere_on_the_line_is_never_exempt() {
        for line in [
            "chmod '0644' '/srv/a'; sudo -u root chmod 777 /srv/b",
            "chmod '0644' '/srv/a'; `chmod 777 /srv/b`",
            "chmod '0644' '/srv/a'; env chmod 777 /srv/b",
            "chmod '0644' '/srv/a'; chmod 666 /srv/b",
            "chmod '0644' '/srv/a' && chmod 777 /srv/b",
            "chmod '0644' '/opt/app666/t'; chmod 777 /srv/b",
        ] {
            assert!(!sec017_is_path_only(line), "wrongly exempted: {line}");
        }
    }

    /// A line with no quoting has nothing to redact, so nothing to exempt.
    #[test]
    fn an_unquoted_line_is_never_exempt() {
        assert!(!sec017_is_path_only("chmod 666 /srv/data"));
        assert!(!sec017_is_path_only("echo chmod 777 /var/www"));
    }

    /// Row 5 of the measurement table: bashrs sees nothing, forjar must.
    #[test]
    fn world_writable_modes_are_named_wherever_they_sit() {
        assert_eq!(
            world_writable_modes("chmod '0666' '/tmp/plain/t'"),
            ["0666"]
        );
        assert_eq!(world_writable_modes("chmod 0777 /tmp/plain/t"), ["0777"]);
        assert_eq!(
            world_writable_modes("chmod '0662' '/tmp/plain/t'"),
            ["0662"]
        );
        // 5 digits: the width the first version read as "not a mode".
        assert_eq!(world_writable_modes("chmod '00666' '/srv/b'"), ["00666"]);
        // Second chmod on the line, quoted: invisible to SEC017 either way.
        assert_eq!(
            world_writable_modes("chmod '0644' '/srv/a'; chmod '0666' '/srv/b'"),
            ["0666"]
        );
        // Not in command position, still a world-writable mode.
        assert_eq!(
            world_writable_modes("find /srv -type f -exec chmod '0666' {} \\;"),
            ["0666"]
        );
        // Symbolic.
        assert_eq!(world_writable_modes("chmod 'a+w' '/srv/b'"), ["a+w"]);
        assert_eq!(world_writable_modes("chmod o+w /srv/b"), ["o+w"]);
        // Flags are skipped, the mode after them is read.
        assert_eq!(world_writable_modes("chmod -R '0666' '/srv/b'"), ["0666"]);
    }

    #[test]
    fn safe_and_undecidable_modes_are_not_named() {
        for line in [
            "chmod '0644' '/srv/a'",
            "chmod 0755 /srv/a",
            "chmod u+x '/srv/a'",
            "chmod g+w '/srv/a'",
            "chmod --reference='/srv/a' '/srv/b'",
            "chmod \"$MODE\" '/srv/b'",
            "echo '/opt/app666/t'",
            "chmod",
        ] {
            assert!(
                world_writable_modes(line).is_empty(),
                "wrongly named world-writable: {line}"
            );
        }
    }

    #[test]
    fn octal_parsing_reads_the_widths_chmod_accepts() {
        assert_eq!(parse_octal_mode("0644"), Some(0o644));
        assert_eq!(parse_octal_mode("644"), Some(0o644));
        assert_eq!(parse_octal_mode("1777"), Some(0o1777));
        assert_eq!(parse_octal_mode("00666"), Some(0o666));
        assert_eq!(parse_octal_mode("68"), None);
        assert_eq!(parse_octal_mode("0888"), None);
        assert_eq!(parse_octal_mode(""), None);
        assert_eq!(parse_octal_mode("0000000"), None);
    }

    #[test]
    fn redaction_keeps_the_mode_and_replaces_the_path() {
        assert_eq!(
            redact_quoted_paths("chmod '0644' '/opt/app666/t'"),
            "chmod '0644' '/x'"
        );
        assert_eq!(
            redact_quoted_paths("chmod '0644' '/srv/a'; chmod 777 /srv/b"),
            "chmod '0644' '/x'; chmod 777 /srv/b"
        );
        // An unbalanced quote is left exactly as written.
        assert_eq!(redact_quoted_paths("chmod '0644"), "chmod '0644");
    }
}
