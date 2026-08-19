//! GH-210 / GH-212: one bounded, status-checked HTTP POST for every notification.
//!
//! Every notification channel shelled out to `curl` and decided delivery
//! success from the *process* exit status alone. Two defects fall straight out
//! of that, both measured on the published 1.12.3 binary:
//!
//! ```text
//!   receiver replies HTTP 500   ->  exit=0, no warning at all
//!   nothing listening           ->  exit=0, "Warning: webhook POST ... failed (exit 7)"
//! ```
//!
//! A 5xx *is* a failed delivery — the receiver refused the notification — yet
//! only transport-level errors were ever reported, because `curl` without
//! `--fail` exits 0 for any completed HTTP transaction. forjar's own behaviour
//! in the unreachable case is the contract; this module makes the rejected case
//! match it, and names the status: `failed (HTTP 500)`.
//!
//! The second defect is worse in CI. `curl` has no default transfer timeout, so
//! a receiver that accepts the connection and then stalls holds `forjar apply`
//! open forever — *after* the apply itself has finished and printed "Apply
//! complete". Measured: apply done in 0.0s, process still blocked 70s later and
//! killed by the wrapper. A post-apply notification must never be able to
//! outlive the work it reports on, so every POST here is bounded by
//! [`MAX_TIME_SECS`] and a timeout is reported as the delivery failure it is.
//!
//! Both fixes live in ONE helper on purpose: the shipped code had five separate
//! `Command::new("curl")` argv literals, and the next channel added would have
//! copied whichever one was nearest.

use std::process::Command;

/// Seconds allowed for the TCP/TLS connect phase of a notification POST.
pub(crate) const CONNECT_TIMEOUT_SECS: u32 = 5;

/// Total seconds allowed for a notification POST, connect included.
///
/// Deliberately small: this runs after the apply has already succeeded, so the
/// only thing a longer bound can buy is a hung pipeline.
pub(crate) const MAX_TIME_SECS: u32 = 10;

/// `curl` exit code for "operation timed out" (`--max-time` / `--connect-timeout`).
const CURL_EXIT_TIMEOUT: i32 = 28;

/// Build the argv for a bounded JSON POST.
///
/// Split out from [`post_json`] so the timeout and status-reporting flags can
/// be asserted without a network: a regression here is invisible at runtime
/// until a receiver misbehaves in production.
pub(crate) fn curl_post_argv(url: &str, payload: &str, headers: &[String]) -> Vec<String> {
    let mut argv = vec![
        "-s".to_string(),
        "-o".to_string(),
        "/dev/null".to_string(),
        // Report the status line so a 4xx/5xx is distinguishable from success.
        "-w".to_string(),
        "%{http_code}".to_string(),
        "--connect-timeout".to_string(),
        CONNECT_TIMEOUT_SECS.to_string(),
        "--max-time".to_string(),
        MAX_TIME_SECS.to_string(),
        "-X".to_string(),
        "POST".to_string(),
        "-H".to_string(),
        "Content-Type: application/json".to_string(),
    ];
    for h in headers {
        argv.push("-H".to_string());
        argv.push(h.clone());
    }
    argv.push("-d".to_string());
    argv.push(payload.to_string());
    argv.push(url.to_string());
    argv
}

/// Turn a finished `curl` run into a delivery verdict.
///
/// `exit_code` is the process status; `http_code` is what `-w '%{http_code}'`
/// printed. Kept pure so every branch is testable.
pub(crate) fn classify_curl_result(exit_code: Option<i32>, http_code: &str) -> Result<(), String> {
    match exit_code {
        Some(0) => {}
        Some(CURL_EXIT_TIMEOUT) => {
            return Err(format!("timed out after {MAX_TIME_SECS}s"));
        }
        Some(c) => return Err(format!("exit {c}")),
        None => return Err("killed by signal".to_string()),
    }
    let status: u16 = http_code.trim().parse().unwrap_or(0);
    if (200..300).contains(&status) {
        return Ok(());
    }
    if status == 0 {
        return Err("no HTTP status returned".to_string());
    }
    Err(format!("HTTP {status}"))
}

/// POST `payload` as JSON, bounded in time and checked for an HTTP 2xx.
///
/// `Ok(())` means the receiver ACCEPTED the notification. Every other outcome —
/// connection refused, DNS failure, timeout, 4xx, 5xx — is an `Err` naming the
/// cause, which callers put in their warning text.
pub(crate) fn post_json(url: &str, payload: &str, headers: &[String]) -> Result<(), String> {
    let argv = curl_post_argv(url, payload, headers);
    let out = Command::new("curl")
        .args(&argv)
        .output()
        .map_err(|e| format!("curl could not be run: {e}"))?;
    let http_code = String::from_utf8_lossy(&out.stdout).to_string();
    classify_curl_result(out.status.code(), &http_code)
}

/// Parse a `{"Header":"Value"}` JSON object into `Header: Value` curl arguments.
///
/// Malformed input is an error rather than a silent drop: the shipped code
/// dropped custom headers entirely, so a webhook that authenticates via a
/// header was delivered unauthenticated and the resulting 401 was swallowed
/// too. Failing loudly is the only way the operator finds out.
pub(crate) fn parse_header_json(raw: &str) -> Result<Vec<String>, String> {
    let value: serde_json::Value = serde_json::from_str(raw)
        .map_err(|e| format!("expected a JSON object of headers, got {raw:?}: {e}"))?;
    let obj = value
        .as_object()
        .ok_or_else(|| format!("expected a JSON object of headers, got {raw:?}"))?;
    let mut headers = Vec::with_capacity(obj.len());
    for (k, v) in obj {
        let rendered = match v {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            other => return Err(format!("header '{k}' must be a scalar, got {other}")),
        };
        if k.trim().is_empty() {
            return Err("header name must not be empty".to_string());
        }
        headers.push(format!("{k}: {rendered}"));
    }
    Ok(headers)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_post_is_time_bounded() {
        // GH-210: an unresponsive receiver held `forjar apply` open for 70s
        // AFTER the apply had printed "Apply complete". Without these two flags
        // curl waits forever and the defect returns silently.
        let argv = curl_post_argv("http://127.0.0.1:1/x", "{}", &[]);
        assert!(argv.iter().any(|a| a == "--max-time"), "{argv:?}");
        assert!(argv.iter().any(|a| a == "--connect-timeout"), "{argv:?}");
        assert!(argv.iter().any(|a| a == MAX_TIME_SECS.to_string().as_str()));
    }

    #[test]
    fn status_is_requested_so_5xx_is_visible() {
        let argv = curl_post_argv("http://h/x", "{}", &[]);
        assert!(argv.iter().any(|a| a == "%{http_code}"), "{argv:?}");
    }

    #[test]
    fn custom_headers_reach_the_argv() {
        let argv = curl_post_argv("http://h/x", "{}", &["X-Auth: SECRET123".to_string()]);
        assert!(
            argv.iter().any(|a| a == "X-Auth: SECRET123"),
            "a header flag that never reaches curl is the defect: {argv:?}"
        );
    }

    #[test]
    fn payload_and_url_are_still_sent() {
        // Non-regression: "bounded" must not mean "sends nothing".
        let argv = curl_post_argv("http://h/x", r#"{"a":1}"#, &[]);
        assert_eq!(argv.last().map(String::as_str), Some("http://h/x"));
        assert!(argv.iter().any(|a| a == r#"{"a":1}"#), "{argv:?}");
    }

    #[test]
    fn a_5xx_rejection_is_a_failed_delivery() {
        // GH-210: this was Ok(()) — the receiver rejected the notification and
        // forjar said nothing at all.
        let err = classify_curl_result(Some(0), "500").expect_err("500 is not delivery");
        assert_eq!(err, "HTTP 500");
    }

    #[test]
    fn a_4xx_rejection_is_a_failed_delivery() {
        assert_eq!(
            classify_curl_result(Some(0), "401").expect_err("401 is not delivery"),
            "HTTP 401"
        );
    }

    #[test]
    fn a_2xx_is_a_successful_delivery() {
        // Non-regression guard: "reports failures" must not mean "reports
        // everything as a failure".
        assert!(classify_curl_result(Some(0), "200").is_ok());
        assert!(classify_curl_result(Some(0), "204\n").is_ok());
    }

    #[test]
    fn a_timeout_names_itself() {
        let err = classify_curl_result(Some(CURL_EXIT_TIMEOUT), "").expect_err("timeout");
        assert!(err.contains("timed out"), "{err}");
    }

    #[test]
    fn a_transport_failure_is_still_reported() {
        // curl exit 7 = connection refused; this case already worked and must
        // keep working.
        let err = classify_curl_result(Some(7), "").expect_err("refused");
        assert_eq!(err, "exit 7");
    }

    #[test]
    fn header_json_becomes_curl_headers() {
        let h = parse_header_json(r#"{"X-Auth":"SECRET123"}"#).expect("valid");
        assert_eq!(h, vec!["X-Auth: SECRET123".to_string()]);
    }

    #[test]
    fn malformed_header_json_is_rejected_not_dropped() {
        let err = parse_header_json("not json").expect_err("must not silently drop headers");
        assert!(err.contains("JSON object"), "{err}");
        assert!(parse_header_json("[1,2]").is_err());
    }
}
