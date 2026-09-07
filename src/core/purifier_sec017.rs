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
//! WHAT THE REFUTER ROUND CHANGED. Three refuter lanes attacked the redaction
//! rule and killed it twice. A backslash-escaped quote (`echo \\" ; (chmod 777
//! /foo) ; echo \\"`) re-paired the quotes and swallowed a real `chmod 777`,
//! which the pre-PMAT-204 gate refused — a regression, fixed by honouring
//! backslash escapes here. And a comma-separated symbolic mode (`u=rwx,o=w`)
//! grants world write in its second clause, which the first version of
//! [`symbolic_grants_world_write`] never read; that shape was accepted before
//! PMAT-204 too, so closing it makes this gate stricter than the one it
//! replaced. Twenty-one shapes are now measured, each against the pre-PMAT-204
//! baseline first; the table lives in
//! `tests/falsification_chmod_path_is_not_a_mode.rs`.
//!
//! THE FOURTH ROUND. Three more judge lanes returned FAIL on seven
//! counterexamples; every one was re-run against both commits and NONE
//! reproduced: five (a backslash inside single quotes, process substitution, an
//! escaped `c\hmod`) are refused at both, and two — `chmod 0000666 /foo` and
//! `chmod $EMPTY_VAR '0666' /foo` — are accepted at both, so they are shapes
//! neither instrument ever saw rather than anything this change opened. The
//! width one is closed anyway: [`parse_octal_mode`] now reads any width chmod
//! accepts. A mode held in a variable stays undecidable and stays declared.
//!
//! WHAT THE JUDGE ROUND CHANGED. Three judge lanes returned FAIL and their
//! counterexamples were re-run against both the baseline and the fix: a quoted
//! mode padded with whitespace (`chmod " 777" /foo`, `' 777'`, `'777 '`, a tab)
//! parsed as "not a mode", was redacted away, and a script the baseline REFUSED
//! became accepted. Redaction is now restricted to plain path literals — a `/`,
//! no shell metacharacter, no `chmod` — so a quoted token that is not a path
//! stays on the line. Forty-five shapes are measured against the baseline and
//! against this code: three go refused -> accepted (the ticket's bug: a safe
//! mode on a path containing 666 or 777), eleven go accepted -> refused (every
//! world-writable mode the baseline could not see), and the rest are unchanged.
//! No shape the baseline refused is accepted here.
//!
//! FALSIFY IT: make [`sec017_is_path_only`] return `true` unconditionally, or
//! make [`world_writable_modes`] return an empty vector, and
//! `tests/falsification_chmod_path_is_not_a_mode.rs` goes red.

use bashrs::linter::{lint_shell, Severity};

/// True where a quoted argument is a plain path literal: a `/` in it, no shell
/// metacharacter that could make it executable text, and no `chmod`.
///
/// Deliberately conservative in the direction that keeps findings: anything
/// this is unsure about is left on the line for bashrs to judge.
fn is_plain_path_literal(inner: &str) -> bool {
    inner.contains('/')
        && !inner.contains("chmod")
        && !inner.contains(['$', '`', ';', '&', '|', '\n'])
}

/// Replace quoted PATH LITERALS with a fixed path, and nothing else.
///
/// Redaction is the only thing forjar does to the line before handing it back
/// to bashrs, so what it may remove has to be narrow enough that removing it
/// cannot hide a finding. A quoted argument is redacted only when it is a plain
/// path literal: it contains a `/`, it carries no shell metacharacter (`$`, a
/// backtick, `;`, `&`, `|`, a newline) that could make it executable text, and
/// it does not contain the word `chmod`.
///
/// Everything else is left exactly as written, which is what keeps the gate
/// honest in the shapes the judge round found: `chmod " 777" /foo`,
/// `chmod ' 777' /foo`, `chmod '777 ' /foo` and a tab-padded mode all have no
/// `/` in the quoted argument, so they are not redacted, SEC017 still fires and
/// the script is still refused — as it was before PMAT-204. An earlier version
/// redacted any quoted token that did not PARSE as a mode, and those four
/// shapes went from refused to accepted (measured against the baseline).
fn redact_quoted_paths(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        match c {
            // A backslash escapes the next character, quote or not. Without
            // this, `echo \\" ; chmod 777 /foo ; echo \\"` re-pairs the two
            // ESCAPED quotes, swallows the real `chmod 777` between them and
            // exempts a line that bashrs refused before PMAT-204 — measured,
            // and the reason this scanner is not a `find(['\'', '"'])` loop.
            '\\' => {
                out.push(c);
                if let Some((_, next)) = chars.next() {
                    out.push(next);
                }
            }
            '\'' | '"' => {
                let quote = c;
                let start = i + quote.len_utf8();
                let mut end = None;
                let mut escaped = false;
                for (j, d) in chars.by_ref() {
                    if escaped {
                        escaped = false;
                        continue;
                    }
                    if d == '\\' {
                        escaped = true;
                        continue;
                    }
                    if d == quote {
                        end = Some(j);
                        break;
                    }
                }
                match end {
                    Some(j) => {
                        let inner = &line[start..j];
                        if is_plain_path_literal(inner) {
                            out.push_str("'/x'");
                        } else {
                            out.push(quote);
                            out.push_str(inner);
                            out.push(quote);
                        }
                    }
                    None => {
                        // An unbalanced quote: leave the remainder verbatim.
                        out.push_str(&line[i..]);
                        return out;
                    }
                }
            }
            _ => out.push(c),
        }
    }
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

/// An octal file mode of any width `chmod` itself accepts, or `None`.
///
/// The width is wide on purpose. A 3-or-4-digit rule read `chmod '00666'` as
/// "not a mode" and let it through; a 6-digit cap did the same for
/// `chmod 0000666`, which the final judge round found (both measured, both
/// missed by bashrs too, so neither was a regression — but both are modes).
/// Leading zeros are what makes the widths vary, so only the low twelve bits
/// are kept: everything above them is padding.
fn parse_octal_mode(text: &str) -> Option<u32> {
    if text.is_empty() || text.len() > 12 {
        return None;
    }
    if !text.bytes().all(|b| (b'0'..=b'7').contains(&b)) {
        return None;
    }
    u32::from_str_radix(text, 8).ok().map(|m| m & 0o7777)
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
///
/// Every comma-separated clause is judged, not just the first: `u=rwx,o=w`
/// grants world write in its SECOND clause, and a rule that read only the
/// leading `who` group called it safe (found by three refuter lanes, measured).
fn symbolic_grants_world_write(text: &str) -> bool {
    text.split(',').any(|clause| {
        let (who, op_rest) = match clause.find(['+', '=']) {
            Some(i) => (&clause[..i], &clause[i..]),
            None => return false,
        };
        if !op_rest.contains('w') {
            return false;
        }
        who.is_empty() || who.contains('a') || who.contains('o')
    })
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
        if unquote(tok).trim_matches(|c: char| ";&|(){}`".contains(c)) != "chmod" {
            continue;
        }
        // Every argument up to the next command separator is examined, not
        // only the first: `chmod '0644' '/x' 0666 '/y'` carries a second mode
        // that a first-argument-only rule missed and that SEC017's
        // digit-boundary check cannot see either (measured, accepted at the
        // pre-PMAT-204 baseline too). A path never parses as an octal literal,
        // so reading them all costs nothing but refuses more.
        for arg in tokens.by_ref() {
            let inner = unquote(arg);
            if inner.starts_with("--reference") {
                break;
            }
            if inner.contains([';', '&', '|']) {
                break;
            }
            match parse_octal_mode(inner) {
                Some(mode) if mode & 0o002 != 0 => found.push(inner.to_string()),
                Some(_) => {}
                None if symbolic_grants_world_write(inner) => found.push(inner.to_string()),
                None => {}
            }
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
        // Wider than four digits is still a mode: leading zeros are padding,
        // and only the low twelve bits are kept.
        assert_eq!(parse_octal_mode("0000666"), Some(0o666));
        assert_eq!(parse_octal_mode("000000662"), Some(0o662));
        assert_eq!(parse_octal_mode("0000000"), Some(0));
        assert_eq!(parse_octal_mode("0123456789012"), None);
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
