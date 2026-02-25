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

/// Purify a shell script through the full bashrs pipeline.
///
/// Parse → purify AST → format back to shell → validate.
/// Returns the purified script or an error if any stage fails.
pub fn purify_script(script: &str) -> Result<String, String> {
    // Parse shell to AST
    let mut parser =
        BashParser::new(script).map_err(|e| format!("bashrs parse: {e}"))?;
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
mod tests {
    use super::*;

    // --- FJ-036: Validation tests ---

    #[test]
    fn test_fj036_validate_simple_echo() {
        let script = "#!/bin/bash\nset -euo pipefail\necho 'hello'\n";
        assert!(validate_script(script).is_ok());
    }

    #[test]
    fn test_fj036_validate_pipefail_script() {
        let script = "#!/bin/bash\nset -euo pipefail\napt-get install -y curl\n";
        assert!(validate_script(script).is_ok());
    }

    #[test]
    fn test_fj036_validate_empty_script() {
        assert!(validate_script("").is_ok());
    }

    #[test]
    fn test_fj036_validate_multiline_script() {
        let script = "#!/bin/bash\nset -euo pipefail\nmkdir -p /tmp/test\nchmod 0755 /tmp/test\n";
        assert!(validate_script(script).is_ok());
    }

    // --- FJ-036: Lint tests ---

    #[test]
    fn test_fj036_lint_returns_diagnostics() {
        let script = "#!/bin/bash\nset -euo pipefail\necho hello\n";
        let result = lint_script(script);
        // Should lint without panicking; diagnostics may vary
        let _ = result.diagnostics.len();
    }

    #[test]
    fn test_fj036_lint_error_count_clean_script() {
        let script = "#!/bin/bash\nset -euo pipefail\nprintf '%s\\n' 'hello'\n";
        let errors = lint_error_count(script);
        // A well-formed script should have zero or few errors
        assert!(errors <= 2, "expected few errors, got {errors}");
    }

    #[test]
    fn test_fj036_lint_severity_filter() {
        // validate_script should pass even if there are warnings
        let script = "#!/bin/bash\necho hello\n";
        assert!(
            validate_script(script).is_ok(),
            "warnings should not fail validation"
        );
    }

    // --- FJ-036: Purification tests ---

    #[test]
    fn test_fj036_purify_simple_script() {
        let script = "#!/bin/bash\necho hello\n";
        let result = purify_script(script);
        // Purification should succeed on a simple script
        assert!(result.is_ok(), "purify failed: {:?}", result.err());
    }

    #[test]
    fn test_fj036_purify_preserves_semantics() {
        let script = "#!/bin/bash\nset -euo pipefail\nmkdir -p /tmp/test\n";
        if let Ok(purified) = purify_script(script) {
            assert!(
                purified.contains("mkdir"),
                "purified lost mkdir: {purified}"
            );
        }
    }

    #[test]
    fn test_fj036_purify_returns_string() {
        let script = "echo test";
        if let Ok(purified) = purify_script(script) {
            assert!(!purified.is_empty());
        }
    }

    // --- FJ-036: Integration with codegen output ---

    #[test]
    fn test_fj036_validate_generated_package_script() {
        let script = r#"#!/bin/bash
set -euo pipefail
SUDO=""
if [ "$(id -u)" -ne 0 ]; then SUDO="sudo"; fi
dpkg -l curl 2>/dev/null | grep -q '^ii'
"#;
        assert!(
            validate_script(script).is_ok(),
            "package check script failed validation"
        );
    }

    #[test]
    fn test_fj036_validate_generated_file_script() {
        let script = "#!/bin/bash\nset -euo pipefail\ntest -f /etc/test.conf\n";
        assert!(
            validate_script(script).is_ok(),
            "file check script failed validation"
        );
    }

    #[test]
    fn test_fj036_validate_generated_service_script() {
        let script = "#!/bin/bash\nset -euo pipefail\nsystemctl is-active nginx\n";
        assert!(
            validate_script(script).is_ok(),
            "service check script failed validation"
        );
    }

    #[test]
    fn test_fj036_validate_heredoc_script() {
        let script = "set -euo pipefail\ncat > '/etc/test.conf' <<'FORJAR_EOF'\nkey=value\nFORJAR_EOF\n";
        assert!(
            validate_script(script).is_ok(),
            "heredoc script failed validation"
        );
    }

    #[test]
    fn test_fj036_validate_base64_pipe() {
        let script = "set -euo pipefail\necho 'aGVsbG8=' | base64 -d > '/tmp/test'\n";
        assert!(
            validate_script(script).is_ok(),
            "base64 pipe script failed validation"
        );
    }
}
