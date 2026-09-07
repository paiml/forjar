//! FJ-036: Shell purification pipeline — bashrs integration.
//!
//! Invariant I8: No raw shell execution — all shell is bashrs-purified.
//!
//! Three levels of shell safety:
//! - `validate_script()` — lint-based validation, errors only (warnings pass)
//! - `lint_script()` — full linter pass, returns all diagnostics
//! - `purify_script()` — parse → purify AST → reformat (strongest guarantee)

use bashrs::bash_parser::BashParser;
use bashrs::bash_quality::Formatter;
use bashrs::bash_transpiler::{PurificationOptions, Purifier};
use bashrs::linter::{lint_shell, Diagnostic, LintResult, Severity};

use super::purifier_sec017::{chmod_mode_verdict, ChmodVerdict};

/// forjar's own code for a chmod whose declared mode grants world write.
/// Not a bashrs code: bashrs has no finding for this shape (see
/// [`super::purifier_sec017`], measurement row 5).
const WORLD_WRITABLE_CODE: &str = "FJ-CHMOD-WW";

/// The source line a diagnostic points at, if the span is inside the script.
///
/// Spans are 1-indexed. The script here is the one the caller linted — for the
/// I8 gate that is `strip_data_payloads`' output, which is the text bashrs
/// judged and the text `numbered_for_diagnosis` prints, so the line numbers
/// agree with the diagnostic the operator sees.
fn source_line<'a>(lines: &[&'a str], one_indexed: usize) -> Option<&'a str> {
    one_indexed
        .checked_sub(1)
        .and_then(|i| lines.get(i))
        .copied()
}

/// True where a SEC017 Error is a hit on the PATH rather than on the mode.
///
/// The judgement is made by parsing the chmod line's mode argument
/// ([`chmod_mode_verdict`]), never by matching on the message text: the message
/// says "chmod 666" for `chmod '0644' '/opt/app666/t'`, which is precisely the
/// text that is wrong.
fn is_path_false_positive(lines: &[&str], diag: &Diagnostic) -> bool {
    if diag.code != "SEC017" {
        return false;
    }
    match source_line(lines, diag.span.start_line) {
        Some(line) => chmod_mode_verdict(line) == ChmodVerdict::SafeQuotedMode,
        None => false,
    }
}

/// forjar's own findings: a chmod line declaring a world-writable mode.
///
/// This is the half of the trade that makes PMAT-204 a change of instrument
/// rather than a loosened gate. bashrs SEC017 produces NO finding for
/// `chmod '0666' '/tmp/plain/t'` — its digit-boundary check refuses to see
/// `666` behind the leading `0` — so the one shape forjar generates for a
/// world-writable mode was invisible to the gate. It is not invisible now.
fn world_writable_chmod_errors(lines: &[&str]) -> Vec<String> {
    lines
        .iter()
        .enumerate()
        .filter_map(|(i, line)| match chmod_mode_verdict(line) {
            ChmodVerdict::WorldWritable(mode) => Some(format!(
                "[error] {WORLD_WRITABLE_CODE}: line {}: chmod mode '{mode}' is world-writable \
                 (the o+w bit is set) — every user on the target could rewrite this path",
                i + 1
            )),
            _ => None,
        })
        .collect()
}

/// Validate a shell script via bashrs linter.
///
/// Fails only on Error-severity diagnostics. Warnings are acceptable
/// in generated scripts (e.g., SC2162 for `read` without `-r`).
pub fn validate_script(script: &str) -> Result<(), String> {
    let result = lint_shell(script);
    let lines: Vec<&str> = script.lines().collect();
    // SC1xxx (SYNTAX) rules were excluded here for a long time, with the note:
    // "bashrs has false positives on generated scripts (SC1035 on `in` in quoted
    // strings, SC1020 on `]` in heredocs)". Both of those were real, and both
    // are the quote-blindness class fixed upstream — SC1035/SC1020 by bashrs
    // 6.67.0 (GH-226, which resolves quoting once and hands the shell-syntax
    // rules a source where literal text is inert filler), SC1028/SC1078 by
    // 6.68.0 (paiml/bashrs#243, #245).
    //
    // Excluding them cost more than it saved. SC1xxx is the SYNTAX-error family,
    // which is precisely what a gate over GENERATED shell most wants to catch —
    // forjar was blind to malformed output of its own making. The SC2135/SC2136
    // pair in #281 was caught only because it happened to land in SC2*; the same
    // defect under an SC1 code would have shipped silently.
    //
    // Measured before removing (forjar#285), against every resource this fleet
    // declares: 437 resources x 3 phases = 1,311 generated scripts, emitted with
    // `forjar codegen` from all nine machine YAMLs and linted at Error severity.
    // SC1* findings: ZERO. That sweep did NOT apply `strip_data_payloads`, so it
    // is stricter than this call site, which lints the sanitised script.
    //
    // PMAT-204: SEC017 is the one rule whose verdict forjar re-derives, because
    // it decides from the LITERAL TEXT of the whole chmod line and forjar puts
    // a config-supplied PATH on that line. `chmod '0644' '/opt/app666/config'`
    // was refused as world-writable; the file could not be written at all.
    // Measured 2026-09-07 against the pinned bashrs (6.68.0):
    //
    //   chmod '0644' '/tmp/.tmp5k666T/target.txt'  ERROR  false — 666 in PATH
    //   chmod '0644' '/opt/app666/t'               ERROR  false positive
    //   chmod '0644' '/tmp/x777y/target.txt'       ERROR  false positive
    //   chmod '0644' '/tmp/plain/target.txt'       clean
    //   chmod '0666' '/tmp/plain/t'                NO FINDING AT ALL
    //   chmod 666 /tmp/real                        ERROR  true positive
    //
    // Row 5 is why this is a change of INSTRUMENT, not a reduction: SEC017's
    // digit-boundary check cannot see `666` inside `'0666'`, so the shape
    // forjar generates for a genuinely world-writable mode was the one shape
    // the gate never caught. Reading the mode argument fixes both directions —
    // the exemption below drops rows 1-3, and `world_writable_chmod_errors`
    // adds row 5 under forjar's own code. Row 6, the true positive, is
    // untouched: a bare numeric mode is never exempted.
    //
    // Nothing else changes. Every other Error-severity diagnostic, including
    // SEC017 on a line this cannot parse, still refuses the script. To falsify:
    // see `super::purifier_sec017`.
    let mut msgs: Vec<String> = result
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .filter(|d| !is_path_false_positive(&lines, d))
        .map(|d| format!("[{}] {}: {}", d.severity, d.code, d.message))
        .collect();
    msgs.extend(world_writable_chmod_errors(&lines));
    if msgs.is_empty() {
        Ok(())
    } else {
        Err(format!("bashrs lint errors:\n{}", msgs.join("\n")))
    }
}

/// Lint a shell script and return the full diagnostic result.
pub fn lint_script(script: &str) -> LintResult {
    lint_shell(script)
}

/// Count lint errors (severity == Error) in a script.
pub fn lint_error_count(script: &str) -> usize {
    let result = lint_shell(script);
    result
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .count()
}

/// Validate first, falling back to full purification if validation fails.
///
/// This is the recommended entry point for scripts that might need fixing:
/// - If `validate_script()` passes, return the script as-is (fast path)
/// - If validation fails, attempt `purify_script()` to fix it
/// - If purification also fails, return the error
pub fn validate_or_purify(script: &str) -> Result<String, String> {
    if validate_script(script).is_ok() {
        return Ok(script.to_string());
    }
    purify_script(script)
}

/// Purify a shell script through the full bashrs pipeline.
///
/// Parse → purify AST → format back to shell → validate.
/// Returns the purified script or an error if any stage fails.
pub fn purify_script(script: &str) -> Result<String, String> {
    // Parse shell to AST
    let mut parser = BashParser::new(script).map_err(|e| format!("bashrs parse: {e}"))?;
    let ast = parser.parse().map_err(|e| format!("bashrs parse: {e}"))?;

    // Purify AST (injection prevention, proper quoting, determinism)
    let options = PurificationOptions::default();
    let mut purifier = Purifier::new(options);
    let purified_ast = purifier
        .purify(&ast)
        .map_err(|e| format!("bashrs purify: {e}"))?;

    // Format purified AST back to shell code
    let formatter = Formatter::new();
    let purified = formatter
        .format(&purified_ast)
        .map_err(|e| format!("bashrs format: {e}"))?;

    // Final validation pass (errors only)
    validate_script(&purified)?;

    Ok(purified)
}

#[cfg(test)]
mod sc1_gate_tests {
    use super::*;

    /// The SC1xxx family must be LIVE, not filtered away.
    ///
    /// forjar#285: this gate excluded every `SC1*` finding for a long time,
    /// which made it blind to the syntax-error family — precisely what a gate
    /// over GENERATED shell most wants to catch. A test that only asserted the
    /// good cases pass would go green with the family switched back off, so
    /// this one requires a real syntax error to be REJECTED.
    #[test]
    fn a_real_syntax_error_is_rejected() {
        // Unterminated double-quoted string: SC1078, an SC1* code.
        let broken = "echo \"this string never closes\n";
        let err = validate_script(broken).expect_err(
            "a script with an unterminated string was accepted — the SC1 family is filtered again",
        );
        assert!(
            err.contains("SC1"),
            "rejected, but not by an SC1 rule: {err}"
        );
    }

    /// Guard the guard: the shapes that JUSTIFIED the exclusion must still pass,
    /// or re-enabling the family trades a blind spot for a false positive that
    /// aborts `forjar apply` fleet-wide.
    ///
    /// Both are named in the original comment: SC1035 on `in` inside a quoted
    /// string, SC1020 on `]` inside a heredoc. Both are the quote-blindness
    /// class fixed by bashrs 6.67.0 (GH-226) and 6.68.0.
    #[test]
    fn the_false_positives_that_justified_the_exclusion_are_gone() {
        let quoted_in = "grep \"^Diff in\" \"$f\"\necho \"select a in b\"\n";
        assert!(
            validate_script(quoted_in).is_ok(),
            "SC1035-shape false positive is back: `in` inside a quoted string"
        );

        let bracket_in_heredoc =
            "cat > /tmp/x <<'PAYLOAD'\nsome text with ] a bracket\nand [ 0-9 ] regex-ish\nPAYLOAD\n";
        assert!(
            validate_script(bracket_in_heredoc).is_ok(),
            "SC1020-shape false positive is back: `]` inside a heredoc"
        );
    }

    /// The generated shape that #281 was about must still pass — a folded YAML
    /// scalar inlined into a check script.
    #[test]
    fn a_folded_condition_check_script_still_passes() {
        let script = crate::resources::verdict::single(
            "sh -c 'for u in a b; do test -n \"$u\"; done; exit 0'",
            "forjar=converged",
            "forjar=diverged",
        );
        assert!(
            validate_script(&script).is_ok(),
            "forjar generates a check script its own gate rejects"
        );
    }
}
