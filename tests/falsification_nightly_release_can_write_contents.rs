//! forjar#1163 (parent #913): the nightly `release` job must have effective
//! `contents: write` permission and use the `GITHUB_TOKEN` it applies to.
//!
//! # The defect
//!
//! On 2026-11-17 the nightly `release` job failed at "Create nightly release"
//! (`softprops/action-gh-release`) with HTTP 403 `Resource not accessible by
//! integration`. Because a job-level `permissions:` block in GitHub Actions
//! replaces the top-level `permissions:` block rather than merging with it,
//! and a custom `token:` / `GH_TOKEN:` secret bypasses `GITHUB_TOKEN`'s
//! workflow permissions, any narrowing or token override breaks publishing.
//!
//! # mutations — each measured to redden its own assertion
//!
//! 1. Narrow top-level `permissions.contents` from `write` to `read` (or
//!    `read-all`) → `nightly_release_has_effective_contents_write` goes RED.
//! 2. Add a job-level `permissions:` block to `release` without `contents:
//!    write` (e.g., `actions: read` or `{}`) →
//!    `nightly_release_has_effective_contents_write` goes RED.
//! 3. Set `with.token` on `softprops/action-gh-release` to another secret
//!    → `nightly_release_steps_use_github_token` goes RED.
//! 4. Set `env.GH_TOKEN` on a `release` step to another secret
//!    → `nightly_release_steps_use_github_token` goes RED.

use serde_yaml_ng::Value;

fn parse_yaml(text: &str, label: &str) -> Value {
    serde_yaml_ng::from_str(text).unwrap_or_else(|e| panic!("forjar#1163: parse {label}: {e}"))
}

fn nightly() -> Value {
    let path = format!(
        "{}/.github/workflows/nightly.yml",
        env!("CARGO_MANIFEST_DIR")
    );
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("forjar#1163: read {path}: {e}"));
    parse_yaml(&text, &path)
}

fn release_steps(wf: &Value) -> Result<&[Value], String> {
    wf.get("jobs")
        .and_then(|j| j.get("release"))
        .and_then(|j| j.get("steps"))
        .and_then(Value::as_sequence)
        .map(Vec::as_slice)
        .ok_or_else(|| "forjar#1163: workflow has no jobs.release.steps".to_string())
}

fn perm_allows_contents_write(perm: &Value) -> bool {
    match perm {
        Value::String(s) => s == "write-all",
        Value::Mapping(_) => perm.get("contents").and_then(Value::as_str) == Some("write"),
        _ => false,
    }
}

fn check_contents_write(wf: &Value) -> Result<(), String> {
    let job = wf
        .get("jobs")
        .and_then(|j| j.get("release"))
        .ok_or_else(|| "forjar#1163: workflow has no jobs.release".to_string())?;
    let (scope, perm) = match job.get("permissions") {
        Some(p) => ("jobs.release.permissions", Some(p)),
        None => ("top-level permissions", wf.get("permissions")),
    };
    if perm.is_some_and(perm_allows_contents_write) {
        Ok(())
    } else {
        Err(format!(
            "forjar#1163: {scope} must grant effective `contents: write` to the `release` job"
        ))
    }
}

fn is_default_github_token(val: &Value) -> bool {
    let Some(inner) = val
        .as_str()
        .and_then(|s| s.trim().strip_prefix("${{"))
        .and_then(|s| s.strip_suffix("}}"))
        .map(str::trim)
    else {
        return false;
    };
    matches!(inner, "secrets.GITHUB_TOKEN" | "github.token")
}

fn check_gh_release_token(wf: &Value) -> Result<(), String> {
    let gh_releases: Vec<&Value> = release_steps(wf)?
        .iter()
        .filter(|s| {
            s.get("uses").and_then(Value::as_str).is_some_and(|u| {
                u == "softprops/action-gh-release" || u.starts_with("softprops/action-gh-release@")
            })
        })
        .collect();
    if gh_releases.is_empty() {
        return Err("forjar#1163: jobs.release has no softprops/action-gh-release step".into());
    }
    for step in gh_releases {
        if let Some(tok) = step.get("with").and_then(|w| w.get("token")) {
            if !is_default_github_token(tok) {
                return Err(format!(
                    "forjar#1163: softprops/action-gh-release `with.token` must be omitted or GITHUB_TOKEN, got {tok:?}"
                ));
            }
        }
    }
    Ok(())
}

fn check_gh_token_env(wf: &Value) -> Result<(), String> {
    for step in release_steps(wf)? {
        if let Some(tok) = step.get("env").and_then(|e| e.get("GH_TOKEN")) {
            if !is_default_github_token(tok) {
                let name = step
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("<unnamed>");
                return Err(format!(
                    "forjar#1163: step {name:?} sets `env.GH_TOKEN` to non-GITHUB_TOKEN {tok:?}"
                ));
            }
        }
    }
    Ok(())
}

#[test]
fn nightly_release_has_effective_contents_write() {
    let wf = nightly();
    check_contents_write(&wf).unwrap_or_else(|e| panic!("{e}"));
}

#[test]
fn nightly_release_steps_use_github_token() {
    let wf = nightly();
    check_gh_release_token(&wf).unwrap_or_else(|e| panic!("{e}"));
    check_gh_token_env(&wf).unwrap_or_else(|e| panic!("{e}"));
}

fn assert_rejected(res: Result<(), String>, case: &str) {
    match res {
        Ok(()) => panic!("{case}: expected rejection"),
        Err(err) => assert!(err.contains("#1163"), "{case}: missing #1163 in {err}"),
    }
}

#[test]
fn rejects_narrowed_permissions_and_custom_tokens() {
    let top_read = "permissions:\n  contents: read\njobs:\n  release:\n    steps: []\n";
    assert_rejected(
        check_contents_write(&parse_yaml(top_read, "top_read")),
        "top_read",
    );

    let job_replaces = "permissions:\n  contents: write\njobs:\n  release:\n    permissions:\n      actions: read\n    steps: []\n";
    assert_rejected(
        check_contents_write(&parse_yaml(job_replaces, "job_replaces")),
        "job_replaces",
    );

    let job_empty =
        "permissions: write-all\njobs:\n  release:\n    permissions: {}\n    steps: []\n";
    assert_rejected(
        check_contents_write(&parse_yaml(job_empty, "job_empty")),
        "job_empty",
    );

    let job_read_all =
        "permissions: write-all\njobs:\n  release:\n    permissions: read-all\n    steps: []\n";
    assert_rejected(
        check_contents_write(&parse_yaml(job_read_all, "job_read_all")),
        "job_read_all",
    );

    let top_write_all = "permissions: write-all\njobs:\n  release:\n    steps: []\n";
    assert!(check_contents_write(&parse_yaml(top_write_all, "top_write_all")).is_ok());

    let bad_action_tok = "jobs:\n  release:\n    steps:\n      - uses: softprops/action-gh-release@v3\n        with:\n          token: ${{ secrets.PAT }}\n";
    assert_rejected(
        check_gh_release_token(&parse_yaml(bad_action_tok, "bad_action_tok")),
        "bad_action_tok",
    );

    let good_github_tok = "jobs:\n  release:\n    steps:\n      - uses: softprops/action-gh-release@v3\n        with:\n          token: ${{ github.token }}\n";
    assert!(check_gh_release_token(&parse_yaml(good_github_tok, "good_github_tok")).is_ok());

    let bad_env_tok = "jobs:\n  release:\n    steps:\n      - name: Delete\n        env:\n          GH_TOKEN: ${{ secrets.PAT }}\n        run: echo\n";
    assert_rejected(
        check_gh_token_env(&parse_yaml(bad_env_tok, "bad_env_tok")),
        "bad_env_tok",
    );
}
