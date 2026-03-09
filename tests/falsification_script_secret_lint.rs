//! FJ-3307: Script secret leakage detection falsification.
//!
//! Popperian rejection criteria for:
//! - 13 regex patterns detecting secret leakage in shell scripts
//! - Comment line skipping
//! - Clean script detection
//! - Multi-finding accumulation
//! - validate_no_leaks error formatting
//! - Redaction of matched text
//!
//! Usage: cargo test --test falsification_script_secret_lint

use forjar::core::script_secret_lint::{scan_script, validate_no_leaks, ScriptLeakResult};

// ============================================================================
// FJ-3307: Clean Scripts
// ============================================================================

#[test]
fn clean_script_no_findings() {
    let script =
        "#!/bin/bash\nset -euo pipefail\napt-get install -y nginx\nsystemctl restart nginx\n";
    let result = scan_script(script);
    assert!(result.clean());
    assert_eq!(result.findings.len(), 0);
    assert_eq!(result.lines_scanned, 4);
}

#[test]
fn clean_script_empty_string() {
    let result = scan_script("");
    assert!(result.clean());
    assert_eq!(result.lines_scanned, 0);
}

#[test]
fn clean_script_only_comments() {
    let script = "# echo $PASSWORD\n# sshpass -p secret ssh host\n# curl -u admin:pass url\n";
    let result = scan_script(script);
    assert!(result.clean());
}

#[test]
fn clean_script_safe_variable_usage() {
    let script = "#!/bin/bash\nUSER=deploy\nHOME_DIR=/opt/app\necho \"Deploying to $HOME_DIR\"\n";
    let result = scan_script(script);
    assert!(result.clean());
}

// ============================================================================
// FJ-3307: Pattern — echo_secret_var
// ============================================================================

#[test]
fn detect_echo_password() {
    let result = scan_script("echo $PASSWORD > /tmp/log\n");
    assert!(!result.clean());
    assert_eq!(result.findings[0].pattern_name, "echo_secret_var");
    assert_eq!(result.findings[0].line, 1);
}

#[test]
fn detect_echo_secret() {
    let result = scan_script("echo ${SECRET}\n");
    assert!(!result.clean());
    assert_eq!(result.findings[0].pattern_name, "echo_secret_var");
}

#[test]
fn detect_echo_token() {
    let result = scan_script("echo $TOKEN\n");
    assert!(!result.clean());
    assert_eq!(result.findings[0].pattern_name, "echo_secret_var");
}

#[test]
fn detect_echo_api_key() {
    let result = scan_script("echo $API_KEY\n");
    assert!(!result.clean());
    assert_eq!(result.findings[0].pattern_name, "echo_secret_var");
}

#[test]
fn detect_printf_secret() {
    let result = scan_script("printf '%s' \"${SECRET}\" > conf.yml\n");
    assert!(!result.clean());
    assert_has_pattern(&result, "echo_secret_var");
}

// ============================================================================
// FJ-3307: Pattern — export_secret_inline
// ============================================================================

#[test]
fn detect_export_password() {
    let result = scan_script("export PASSWORD=hunter2\n");
    assert!(!result.clean());
    assert_eq!(result.findings[0].pattern_name, "export_secret_inline");
}

#[test]
fn detect_export_token() {
    let result = scan_script("export TOKEN=abc123\n");
    assert!(!result.clean());
    assert_has_pattern(&result, "export_secret_inline");
}

#[test]
fn detect_export_api_key() {
    let result = scan_script("export APIKEY=xyz\n");
    assert!(!result.clean());
    assert_has_pattern(&result, "export_secret_inline");
}

// ============================================================================
// FJ-3307: Pattern — curl_inline_creds
// ============================================================================

#[test]
fn detect_curl_u_flag() {
    let result = scan_script("curl -u admin:hunter2 https://api.example.com\n");
    assert!(!result.clean());
    assert_eq!(result.findings[0].pattern_name, "curl_inline_creds");
}

#[test]
fn detect_curl_user_flag() {
    let result = scan_script("curl --user admin:pass https://api.example.com\n");
    assert!(!result.clean());
    assert_has_pattern(&result, "curl_inline_creds");
}

// ============================================================================
// FJ-3307: Pattern — wget_inline_password
// ============================================================================

#[test]
fn detect_wget_password() {
    let result = scan_script("wget --password=secret http://example.com/file\n");
    assert!(!result.clean());
    assert_has_pattern(&result, "wget_inline_password");
}

// ============================================================================
// FJ-3307: Pattern — redirect_secret_to_file
// ============================================================================

#[test]
fn detect_redirect_password() {
    let result = scan_script("$PASSWORD > /etc/app.conf\n");
    assert!(!result.clean());
    assert_eq!(result.findings[0].pattern_name, "redirect_secret_to_file");
}

#[test]
fn detect_redirect_token() {
    let result = scan_script("$TOKEN > /tmp/token.txt\n");
    assert!(!result.clean());
    assert_has_pattern(&result, "redirect_secret_to_file");
}

// ============================================================================
// FJ-3307: Pattern — sshpass_inline
// ============================================================================

#[test]
fn detect_sshpass() {
    let result = scan_script("sshpass -p mypassword ssh user@host\n");
    assert!(!result.clean());
    assert_eq!(result.findings[0].pattern_name, "sshpass_inline");
}

// ============================================================================
// FJ-3307: Pattern — db_inline_password
// ============================================================================

#[test]
fn detect_mysql_password() {
    let result = scan_script("mysql -psecretpass -h db.internal mydb\n");
    assert!(!result.clean());
    assert_has_pattern(&result, "db_inline_password");
}

#[test]
fn detect_psql_password() {
    let result = scan_script("psql -psecretpass\n");
    assert!(!result.clean());
    assert_has_pattern(&result, "db_inline_password");
}

// ============================================================================
// FJ-3307: Pattern — aws_key_in_script
// ============================================================================

#[test]
fn detect_aws_access_key() {
    let result = scan_script("export AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE\n");
    assert!(!result.clean());
    assert_has_pattern(&result, "aws_key_in_script");
}

#[test]
fn detect_aws_key_inline() {
    let result = scan_script("aws s3 cp s3://bucket/key . --access-key AKIAIOSFODNN7EXAMPLE\n");
    assert!(!result.clean());
    assert_has_pattern(&result, "aws_key_in_script");
}

// ============================================================================
// FJ-3307: Pattern — hardcoded_token (GitHub)
// ============================================================================

#[test]
fn detect_github_pat() {
    let result = scan_script("TOKEN=ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghij\n");
    assert!(!result.clean());
    assert_has_pattern(&result, "hardcoded_token");
}

#[test]
fn detect_github_server_token() {
    let result = scan_script("GH_TOKEN=ghs_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghij\n");
    assert!(!result.clean());
    assert_has_pattern(&result, "hardcoded_token");
}

// ============================================================================
// FJ-3307: Pattern — hardcoded_stripe
// ============================================================================

#[test]
fn detect_stripe_live_key() {
    let result = scan_script("STRIPE_KEY=sk_live_abcdefghij0123456789\n");
    assert!(!result.clean());
    assert_has_pattern(&result, "hardcoded_stripe");
}

#[test]
fn detect_stripe_test_key() {
    let result = scan_script("STRIPE_KEY=rk_test_abcdefghij0123456789\n");
    assert!(!result.clean());
    assert_has_pattern(&result, "hardcoded_stripe");
}

// ============================================================================
// FJ-3307: Pattern — private_key_inline
// ============================================================================

#[test]
fn detect_rsa_private_key() {
    let result = scan_script("cat <<'EOF'\n-----BEGIN RSA PRIVATE KEY-----\nMIIE...\nEOF\n");
    assert!(!result.clean());
    assert_eq!(result.findings[0].pattern_name, "private_key_inline");
}

#[test]
fn detect_ec_private_key() {
    let result = scan_script("echo '-----BEGIN EC PRIVATE KEY-----'\n");
    assert!(!result.clean());
    assert_has_pattern(&result, "private_key_inline");
}

#[test]
fn detect_openssh_private_key() {
    let result = scan_script("printf '-----BEGIN OPENSSH PRIVATE KEY-----'\n");
    assert!(!result.clean());
    assert_has_pattern(&result, "private_key_inline");
}

// ============================================================================
// FJ-3307: Pattern — hex_secret_assign
// ============================================================================

#[test]
fn detect_hex_secret() {
    let result = scan_script("SECRET=abcdef0123456789abcdef0123456789\n");
    assert!(!result.clean());
    assert_eq!(result.findings[0].pattern_name, "hex_secret_assign");
}

#[test]
fn detect_hex_key_assign() {
    let result = scan_script("KEY='abcdef0123456789abcdef0123456789abcdef01'\n");
    assert!(!result.clean());
    assert_has_pattern(&result, "hex_secret_assign");
}

// ============================================================================
// FJ-3307: Pattern — db_url_embedded_pass
// ============================================================================

#[test]
fn detect_postgres_url() {
    let result = scan_script("DATABASE_URL=postgres://app:s3cret@db.internal:5432/prod\n");
    assert!(!result.clean());
    assert_eq!(result.findings[0].pattern_name, "db_url_embedded_pass");
}

#[test]
fn detect_mysql_url() {
    let result = scan_script("DATABASE_URL=mysql://root:password@localhost/mydb\n");
    assert!(!result.clean());
    assert_has_pattern(&result, "db_url_embedded_pass");
}

#[test]
fn detect_mongodb_url() {
    let result = scan_script("MONGO_URL=mongodb://user:pass@mongo.host:27017/db\n");
    assert!(!result.clean());
    assert_has_pattern(&result, "db_url_embedded_pass");
}

// ============================================================================
// FJ-3307: Multi-finding and Accumulation
// ============================================================================

#[test]
fn multiple_findings_per_script() {
    let script = "echo $TOKEN\ncurl -u admin:pass https://api.com\nsshpass -p pw ssh h\n";
    let result = scan_script(script);
    assert!(result.findings.len() >= 3);
}

#[test]
fn findings_on_correct_lines() {
    let script = "echo hello\necho $PASSWORD\necho world\nsshpass -p pass ssh x\n";
    let result = scan_script(script);
    assert!(!result.clean());
    let lines: Vec<usize> = result.findings.iter().map(|f| f.line).collect();
    assert!(lines.contains(&2), "PASSWORD echo on line 2");
    assert!(lines.contains(&4), "sshpass on line 4");
}

#[test]
fn lines_scanned_counts_all() {
    let script = "line1\nline2\nline3\nline4\nline5\n";
    let result = scan_script(script);
    assert_eq!(result.lines_scanned, 5);
}

// ============================================================================
// FJ-3307: Comment Skipping
// ============================================================================

#[test]
fn comments_are_skipped() {
    let script = "# echo $PASSWORD\n# sshpass -p secret ssh host\n";
    let result = scan_script(script);
    assert!(result.clean());
}

#[test]
fn inline_comment_still_detected() {
    // Leading whitespace before # does count as comment (trimmed)
    let script = "  # echo $PASSWORD\n";
    let result = scan_script(script);
    assert!(result.clean());
}

// ============================================================================
// FJ-3307: Redaction
// ============================================================================

#[test]
fn matched_text_redacted_long() {
    let result = scan_script("curl -u adminuser:longpassword123 https://api.example.com\n");
    assert!(!result.clean());
    // Long matches get truncated to 12 chars + "..."
    let text = &result.findings[0].matched_text;
    assert!(text.ends_with("..."), "redacted: {text}");
}

// ============================================================================
// FJ-3307: validate_no_leaks
// ============================================================================

#[test]
fn validate_no_leaks_clean_ok() {
    assert!(validate_no_leaks("#!/bin/bash\necho hello\n").is_ok());
}

#[test]
fn validate_no_leaks_fail_error() {
    let result = validate_no_leaks("echo $PASSWORD\n");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("secret leakage detected"), "err: {err}");
    assert!(err.contains("1 finding"), "err: {err}");
    assert!(err.contains("echo_secret_var"), "err: {err}");
}

#[test]
fn validate_no_leaks_multiple_findings() {
    let result = validate_no_leaks("echo $PASSWORD\nsshpass -p pass ssh x\n");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("2 finding"), "err: {err}");
}

#[test]
fn validate_no_leaks_includes_line_numbers() {
    let result = validate_no_leaks("echo hello\necho $SECRET\n");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("line 2"), "err: {err}");
}

// ============================================================================
// FJ-3307: ScriptLeakResult API
// ============================================================================

#[test]
fn script_leak_result_clean_method() {
    let clean = ScriptLeakResult {
        findings: vec![],
        lines_scanned: 5,
    };
    assert!(clean.clean());
}

// ============================================================================
// Helpers
// ============================================================================

fn assert_has_pattern(result: &ScriptLeakResult, pattern: &str) {
    let names: Vec<&str> = result
        .findings
        .iter()
        .map(|f| f.pattern_name.as_str())
        .collect();
    assert!(
        names.contains(&pattern),
        "expected pattern '{pattern}' in {names:?}"
    );
}
