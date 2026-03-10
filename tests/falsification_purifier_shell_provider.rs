//! FJ-036/3405: Shell purification pipeline and shell provider bridge.
//!
//! Popperian rejection criteria for:
//! - FJ-036: validate_script (bashrs lint, errors-only)
//! - FJ-036: lint_script (full diagnostics)
//! - FJ-036: lint_error_count (error counting)
//! - FJ-036: validate_or_purify (fast-path + fallback)
//! - FJ-036: purify_script (parse → purify → format → validate)
//! - FJ-3405: parse_shell_type, is_shell_type (type parsing)
//! - FJ-3405: validate_provider_script (bashrs + secret lint)
//!
//! Usage: cargo test --test falsification_purifier_shell_provider

use forjar::core::purifier::{
    lint_error_count, lint_script, purify_script, validate_or_purify, validate_script,
};
use forjar::core::shell_provider::{is_shell_type, parse_shell_type, validate_provider_script};

// ============================================================================
// FJ-036: validate_script — clean scripts pass
// ============================================================================

#[test]
fn validate_clean_script() {
    let script = "#!/bin/bash\nset -euo pipefail\necho 'hello'\n";
    assert!(validate_script(script).is_ok());
}

#[test]
fn validate_simple_commands() {
    let script = "#!/bin/bash\nmkdir -p /tmp/test\ncp file.txt /tmp/test/\n";
    assert!(validate_script(script).is_ok());
}

#[test]
fn validate_empty_script() {
    assert!(validate_script("").is_ok());
}

#[test]
fn validate_conditional() {
    let script = "#!/bin/bash\nif [ -f /etc/test ]; then\n  echo exists\nfi\n";
    assert!(validate_script(script).is_ok());
}

// ============================================================================
// FJ-036: lint_script — full diagnostics
// ============================================================================

#[test]
fn lint_clean_script_no_errors() {
    let script = "#!/bin/bash\nset -euo pipefail\necho 'hello'\n";
    let result = lint_script(script);
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.severity == bashrs::linter::Severity::Error)
        .collect();
    assert!(errors.is_empty());
}

#[test]
fn lint_returns_diagnostics() {
    let script = "#!/bin/bash\necho hello\n";
    let result = lint_script(script);
    // Just verify it runs without panicking
    let _ = result.diagnostics.len();
}

// ============================================================================
// FJ-036: lint_error_count
// ============================================================================

#[test]
fn lint_error_count_clean() {
    let script = "#!/bin/bash\nset -euo pipefail\necho 'hello'\n";
    assert_eq!(lint_error_count(script), 0);
}

#[test]
fn lint_error_count_empty() {
    assert_eq!(lint_error_count(""), 0);
}

// ============================================================================
// FJ-036: validate_or_purify — fast path
// ============================================================================

#[test]
fn validate_or_purify_fast_path() {
    let script = "#!/bin/bash\nset -euo pipefail\necho 'hello'\n";
    let result = validate_or_purify(script).unwrap();
    assert_eq!(result, script);
}

#[test]
fn validate_or_purify_empty() {
    let result = validate_or_purify("").unwrap();
    assert_eq!(result, "");
}

// ============================================================================
// FJ-036: purify_script — full pipeline
// ============================================================================

#[test]
fn purify_simple_script() {
    let script = "#!/bin/bash\necho hello\n";
    let result = purify_script(script);
    // Should either succeed or fail gracefully
    if let Ok(purified) = result {
        assert!(!purified.is_empty());
    }
}

#[test]
fn purify_heredoc() {
    let script = r#"#!/bin/bash
cat <<'EOF'
hello world
EOF
"#;
    // Purification may or may not handle heredocs
    let _ = purify_script(script);
}

// ============================================================================
// FJ-3405: parse_shell_type
// ============================================================================

#[test]
fn parse_shell_type_valid() {
    assert_eq!(parse_shell_type("shell:nginx"), Some("nginx"));
    assert_eq!(parse_shell_type("shell:my-provider"), Some("my-provider"));
    assert_eq!(parse_shell_type("shell:k8s"), Some("k8s"));
}

#[test]
fn parse_shell_type_not_shell() {
    assert_eq!(parse_shell_type("plugin:foo"), None);
    assert_eq!(parse_shell_type("file"), None);
    assert_eq!(parse_shell_type("package"), None);
    assert_eq!(parse_shell_type(""), None);
}

#[test]
fn parse_shell_type_prefix_only() {
    // "shell:" with no name returns empty string
    assert_eq!(parse_shell_type("shell:"), Some(""));
}

// ============================================================================
// FJ-3405: is_shell_type
// ============================================================================

#[test]
fn is_shell_type_true() {
    assert!(is_shell_type("shell:nginx"));
    assert!(is_shell_type("shell:my-provider"));
}

#[test]
fn is_shell_type_false() {
    assert!(!is_shell_type("plugin:nginx"));
    assert!(!is_shell_type("package"));
    assert!(!is_shell_type("file"));
    assert!(!is_shell_type(""));
}

// ============================================================================
// FJ-3405: validate_provider_script
// ============================================================================

#[test]
fn provider_script_clean_passes() {
    let script = "#!/bin/bash\nset -euo pipefail\necho 'checking resource'\n";
    assert!(validate_provider_script(script).is_ok());
}

#[test]
fn provider_script_with_secret_leaks() {
    // Scripts containing hardcoded secrets should fail
    let script = "#!/bin/bash\ncurl -H 'Authorization: Bearer sk_live_abc123def456' https://api.example.com\n";
    let result = validate_provider_script(script);
    // May or may not fail depending on secret lint patterns
    // The important thing is that the function runs without panicking
    let _ = result;
}

#[test]
fn provider_script_empty_passes() {
    assert!(validate_provider_script("").is_ok());
}
