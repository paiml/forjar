//! FJ-3307: Secret leakage detection in generated shell scripts.
//!
//! Scans shell script text for patterns that leak secrets — echo/printf
//! of secret variables, curl with inline credentials, redirection of
//! secrets to files. Uses the same regex patterns as `secret_scan.rs`
//! plus shell-specific patterns (echo, env export, curl -u).

use std::sync::OnceLock;

/// A detected secret leakage in a shell script.
#[derive(Debug, Clone, PartialEq)]
pub struct ScriptLeakFinding {
    /// Pattern that matched.
    pub pattern_name: String,
    /// Redacted matched text.
    pub matched_text: String,
    /// Line number in script (1-based).
    pub line: usize,
}

/// Result of scanning a script for secret leakage.
#[derive(Debug, Clone)]
pub struct ScriptLeakResult {
    /// Detected leakage findings.
    pub findings: Vec<ScriptLeakFinding>,
    /// Lines scanned.
    pub lines_scanned: usize,
}

impl ScriptLeakResult {
    /// True if no leaks found.
    pub fn clean(&self) -> bool {
        self.findings.is_empty()
    }
}

/// Compiled pattern for script leak detection.
struct LeakPattern {
    name: &'static str,
    regex: regex::Regex,
}

fn compiled_patterns() -> &'static Vec<LeakPattern> {
    static PATTERNS: OnceLock<Vec<LeakPattern>> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        let defs: Vec<(&str, &str)> = vec![
            // Shell echo/printf of secret variables
            (
                "echo_secret_var",
                r#"(?i)(echo|printf)\s+.*\$\{?(password|passwd|secret|token|api_key|apikey|db_pass|private_key)\b"#,
            ),
            // Export of secret variables (exposes to child processes)
            (
                "export_secret_inline",
                r#"(?i)export\s+(password|passwd|secret|token|api_key|apikey|db_pass)="#,
            ),
            // curl/wget with inline credentials
            (
                "curl_inline_creds",
                r#"(?i)curl\s+.*(-u|--user)\s+\S+:\S+"#,
            ),
            // wget with inline password
            (
                "wget_inline_password",
                r"(?i)wget\s+.*--password[= ]\S+",
            ),
            // Redirect secret to file
            (
                "redirect_secret_to_file",
                r#"(?i)\$\{?(password|passwd|secret|token|api_key|apikey|db_pass|private_key)\}?\s*>\s*\S+"#,
            ),
            // sshpass inline password
            ("sshpass_inline", r"(?i)sshpass\s+-p\s+\S+"),
            // mysql/psql with inline password
            (
                "db_inline_password",
                r"(?i)(mysql|psql|mongosh?)\s+.*-p\S+",
            ),
            // Hardcoded AWS keys
            ("aws_key_in_script", r"AKIA[0-9A-Z]{16}"),
            // Hardcoded tokens (gh, sk_, etc.)
            ("hardcoded_token", r"gh[ps]_[A-Za-z0-9_]{36,}"),
            ("hardcoded_stripe", r"[sr]k_(live|test)_[A-Za-z0-9]{20,}"),
            // Private key content in script
            (
                "private_key_inline",
                r"-----BEGIN (RSA|EC|DSA|OPENSSH) PRIVATE KEY-----",
            ),
            // Env assignment of long hex secrets
            (
                "hex_secret_assign",
                r"(?i)(secret|key|token)=['\x22]?[0-9a-f]{32,}",
            ),
            // Database URL with embedded password
            (
                "db_url_embedded_pass",
                r"(?i)(mysql|postgres|mongodb)://[^:]+:[^@]+@",
            ),
        ];
        defs.into_iter()
            .filter_map(|(name, pattern)| {
                regex::Regex::new(pattern)
                    .ok()
                    .map(|regex| LeakPattern { name, regex })
            })
            .collect()
    })
}

/// Scan a shell script for secret leakage patterns.
///
/// Returns findings per line. Each finding includes the pattern name,
/// redacted matched text, and line number.
pub fn scan_script(script: &str) -> ScriptLeakResult {
    let mut findings = Vec::new();
    let lines: Vec<&str> = script.lines().collect();

    for (idx, line) in lines.iter().enumerate() {
        // Skip comments
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }

        for pattern in compiled_patterns() {
            if let Some(m) = pattern.regex.find(line) {
                let matched = m.as_str();
                let redacted = if matched.len() > 12 {
                    format!("{}...", &matched[..12])
                } else {
                    format!("{matched}...")
                };
                findings.push(ScriptLeakFinding {
                    pattern_name: pattern.name.to_string(),
                    matched_text: redacted,
                    line: idx + 1,
                });
            }
        }
    }

    ScriptLeakResult {
        findings,
        lines_scanned: lines.len(),
    }
}

/// Validate a script has no secret leakage.
///
/// Returns Ok(()) if clean, Err with details if secrets detected.
/// Intended to be called alongside `purifier::validate_script()`.
pub fn validate_no_leaks(script: &str) -> Result<(), String> {
    let result = scan_script(script);
    if result.clean() {
        return Ok(());
    }

    let msgs: Vec<String> = result
        .findings
        .iter()
        .map(|f| format!("  line {}: [{}] {}", f.line, f.pattern_name, f.matched_text))
        .collect();
    Err(format!(
        "secret leakage detected in script ({} finding(s)):\n{}",
        result.findings.len(),
        msgs.join("\n")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_script() {
        let script = "#!/bin/bash\nset -euo pipefail\napt-get install -y nginx\n";
        let result = scan_script(script);
        assert!(result.clean());
        assert_eq!(result.lines_scanned, 3);
    }

    #[test]
    fn echo_password_detected() {
        let script = "echo $PASSWORD > /tmp/log\n";
        let result = scan_script(script);
        assert!(!result.clean());
        assert_eq!(result.findings[0].pattern_name, "echo_secret_var");
        assert_eq!(result.findings[0].line, 1);
    }

    #[test]
    fn printf_secret_detected() {
        let script = "printf '%s' \"${SECRET}\" > conf.yml\n";
        let result = scan_script(script);
        assert!(!result.clean());
    }

    #[test]
    fn curl_inline_creds() {
        let script = "curl -u admin:hunter2 https://api.example.com\n";
        let result = scan_script(script);
        assert!(!result.clean());
        assert_eq!(result.findings[0].pattern_name, "curl_inline_creds");
    }

    #[test]
    fn sshpass_detected() {
        let script = "sshpass -p mypassword ssh user@host\n";
        let result = scan_script(script);
        assert!(!result.clean());
        assert_eq!(result.findings[0].pattern_name, "sshpass_inline");
    }

    #[test]
    fn aws_key_detected() {
        let script = "export AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE\n";
        let result = scan_script(script);
        assert!(!result.clean());
        // Should find the AKIA pattern
        let names: Vec<&str> = result
            .findings
            .iter()
            .map(|f| f.pattern_name.as_str())
            .collect();
        assert!(names.contains(&"aws_key_in_script"));
    }

    #[test]
    fn private_key_detected() {
        let script = "cat <<'EOF'\n-----BEGIN RSA PRIVATE KEY-----\nMIIE...\nEOF\n";
        let result = scan_script(script);
        assert!(!result.clean());
        assert_eq!(result.findings[0].pattern_name, "private_key_inline");
    }

    #[test]
    fn db_url_password_detected() {
        let script = "DATABASE_URL=postgres://app:s3cret@db.internal:5432/prod\n";
        let result = scan_script(script);
        assert!(!result.clean());
        assert_eq!(result.findings[0].pattern_name, "db_url_embedded_pass");
    }

    #[test]
    fn comments_skipped() {
        let script = "# echo $PASSWORD\n# sshpass -p secret ssh host\n";
        let result = scan_script(script);
        assert!(result.clean());
    }

    #[test]
    fn redirect_secret() {
        let script = "$PASSWORD > /etc/app.conf\n";
        let result = scan_script(script);
        assert!(!result.clean());
        assert_eq!(result.findings[0].pattern_name, "redirect_secret_to_file");
    }

    #[test]
    fn export_secret() {
        let script = "export PASSWORD=hunter2\n";
        let result = scan_script(script);
        assert!(!result.clean());
        assert_eq!(result.findings[0].pattern_name, "export_secret_inline");
    }

    #[test]
    fn hex_secret_assign() {
        let script = "SECRET=abcdef0123456789abcdef0123456789\n";
        let result = scan_script(script);
        assert!(!result.clean());
        assert_eq!(result.findings[0].pattern_name, "hex_secret_assign");
    }

    #[test]
    fn validate_no_leaks_clean() {
        let script = "#!/bin/bash\necho hello\n";
        assert!(validate_no_leaks(script).is_ok());
    }

    #[test]
    fn validate_no_leaks_fail() {
        let script = "echo $PASSWORD\n";
        let result = validate_no_leaks(script);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("secret leakage detected"));
        assert!(err.contains("echo_secret_var"));
    }

    #[test]
    fn multiple_findings_per_script() {
        let script = "echo $TOKEN\ncurl -u admin:pass https://api.com\nsshpass -p pw ssh h\n";
        let result = scan_script(script);
        assert!(result.findings.len() >= 3);
    }

    #[test]
    fn github_token_detected() {
        let script = "TOKEN=ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghij\n";
        let result = scan_script(script);
        assert!(!result.clean());
        let names: Vec<&str> = result
            .findings
            .iter()
            .map(|f| f.pattern_name.as_str())
            .collect();
        assert!(names.contains(&"hardcoded_token"));
    }
}
