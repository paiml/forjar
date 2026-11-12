//! forjar#1000: the slice of the GitHub REST API the resource uses.
//!
//! The base URL is `FORJAR_GITHUB_API_URL` (default `https://api.github.com`)
//! so the falsifiers can point it at a local fake. Errors carry the HTTP status
//! and GitHub's own `message`, truncated; never a header, a request body or a
//! token.

use crate::core::store::registry_http::agent_for_url;
use std::io::Read;
use ureq::Agent;

const DEFAULT_BASE: &str = "https://api.github.com";
const MAX_BODY: u64 = 4 << 20;
const MAX_MESSAGE: usize = 200;

pub(super) struct Github {
    base: String,
    agent: Agent,
}

impl Github {
    pub(super) fn from_env() -> Self {
        let base = std::env::var("FORJAR_GITHUB_API_URL")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_BASE.to_string());
        let base = base.trim_end_matches('/').to_string();
        let agent = agent_for_url(&base);
        Self { base, agent }
    }

    pub(super) fn get(&self, path: &str, bearer: &str) -> Result<serde_json::Value, String> {
        self.call("GET", path, bearer)
    }

    pub(super) fn post(&self, path: &str, bearer: &str) -> Result<serde_json::Value, String> {
        self.call("POST", path, bearer)
    }

    fn call(&self, method: &str, path: &str, bearer: &str) -> Result<serde_json::Value, String> {
        let url = format!("{}{path}", self.base);
        let request = ureq::http::Request::builder()
            .method(method)
            .uri(&url)
            .header("Authorization", format!("Bearer {bearer}"))
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "forjar")
            .body(())
            .map_err(|_| format!("github_runner: malformed request for {method} {path}"))?;
        let mut response = self
            .agent
            .run(request)
            .map_err(|e| format!("github_runner: {method} {path} failed: {e}"))?;
        let status = response.status().as_u16();
        let mut buf = Vec::new();
        let _ = response
            .body_mut()
            .as_reader()
            .take(MAX_BODY)
            .read_to_end(&mut buf);
        let json: serde_json::Value = serde_json::from_slice(&buf).unwrap_or_default();
        if (200..300).contains(&status) {
            return Ok(json);
        }
        Err(format!(
            "github_runner: {method} {path} returned {status}: {}",
            message_of(&json)
        ))
    }
}

/// GitHub's `message` field, cut to a short single line.
fn message_of(json: &serde_json::Value) -> String {
    let m = json
        .get("message")
        .and_then(|m| m.as_str())
        .unwrap_or("no message");
    m.chars()
        .take(MAX_MESSAGE)
        .collect::<String>()
        .replace('\n', " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_is_truncated_and_single_line() {
        let long = "x".repeat(500);
        let m = message_of(&serde_json::json!({ "message": format!("a\n{long}") }));
        assert!(m.len() <= MAX_MESSAGE && !m.contains('\n'));
        assert_eq!(message_of(&serde_json::json!({})), "no message");
    }
}
