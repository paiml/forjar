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
use bashrs::linter::{lint_shell, LintResult, Severity};

/// Validate a shell script via bashrs linter.
///
/// Fails only on Error-severity diagnostics. Warnings are acceptable
/// in generated scripts (e.g., SC2162 for `read` without `-r`).
pub fn validate_script(script: &str) -> Result<(), String> {
    let result = lint_shell(script);
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        let msgs: Vec<String> = errors
            .iter()
            .map(|d| format!("[{}] {}: {}", d.severity, d.code, d.message))
            .collect();
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
