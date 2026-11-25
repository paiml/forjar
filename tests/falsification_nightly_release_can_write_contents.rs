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
//! # Mutations and fixture coverage
//!
//! Applied to the real `.github/workflows/nightly.yml` (verified RED, then reverted):
//! 1. Narrow top-level `permissions.contents` from `write` to `read` (or
//!    `read-all`) → `nightly_release_has_effective_contents_write` goes RED
//!    (also covered by inline-YAML fixture `top_read`).
//!
//! Covered only by inline-YAML fixtures in this file (not measured by mutating
//! `.github/workflows/nightly.yml`):
//! 2. Add a job-level `permissions:` block to `release` without `contents:
//!    write` (`job_replaces`, `job_empty`, `job_read_all` in
//!    `rejects_narrowed_permissions_and_custom_tokens`).
//! 3. Set `with.token` on `softprops/action-gh-release` to another secret
//!    (`bad_action_tok` in `rejects_narrowed_permissions_and_custom_tokens`).
//! 4. Set `env.GH_TOKEN` on a `release` step to another secret
//!    (`bad_env_tok` in `rejects_narrowed_permissions_and_custom_tokens`).
//! 5. Set `GH_TOKEN` or `GITHUB_TOKEN` at workflow-level `env` to a
//!    non-`GITHUB_TOKEN` value (`rejects_workflow_env_custom_token`:
//!    `wf_gh`, `wf_github`).
//! 6. Set `GH_TOKEN` or `GITHUB_TOKEN` at `jobs.release.env` to a
//!    non-`GITHUB_TOKEN` value (`rejects_job_env_custom_token`:
//!    `job_gh`, `job_github`).
//! 7. Set `env.GITHUB_TOKEN` on a `release` step to a non-`GITHUB_TOKEN`
//!    value (`rejects_step_env_custom_github_token`: `step_github`).
//! 8. Run the `gh` CLI in a `release` step without an effective `GH_TOKEN` or
//!    `GITHUB_TOKEN` at step, job, or workflow level
//!    (`rejects_gh_cli_step_without_effective_token`: `gh_start`, `gh_inline`).
//! 9. Passing fixture: `gh` CLI in a `release` step with an effective
//!    `GH_TOKEN` or `GITHUB_TOKEN` from workflow, job, or step `env`
//!    (`accepts_gh_cli_step_with_effective_github_token`:
//!    `wf_pass`, `job_pass`, `step_pass`).
//! 10. Invoke `gh` via subshell, full path, or semicolon-separated command
//!     without a token (`rejects_gh_cli_variants_without_effective_token`),
//!     or set/unset `GH_TOKEN` / `GITHUB_TOKEN` inside a `run` script
//!     (`rejects_token_set_or_unset_in_run_script`).
//! 11. Passing fixtures for trailing comments, full-line comments, and
//!     substring non-matches (`accepts_non_gh_comments_trailing_comments_and_substrings`).
//!
//! # Not covered
//!
//! - A release action step disabled with `if:`.
//! - Publishing through another action with its own token input (for example
//!   `actions/github-script` `github-token`).
//! - An `env` that is an expression string instead of a mapping.

use serde_yaml_ng::Value;

const TOKEN_ENV_VARS: [&str; 2] = ["GH_TOKEN", "GITHUB_TOKEN"];

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

fn release_job(wf: &Value) -> Result<&Value, String> {
    wf.get("jobs")
        .and_then(|j| j.get("release"))
        .ok_or_else(|| "forjar#1163: workflow has no jobs.release".to_string())
}

fn job_steps(job: &Value) -> Result<&[Value], String> {
    job.get("steps")
        .and_then(Value::as_sequence)
        .map(Vec::as_slice)
        .ok_or_else(|| "forjar#1163: workflow has no jobs.release.steps".to_string())
}

fn release_steps(wf: &Value) -> Result<&[Value], String> {
    job_steps(release_job(wf)?)
}

fn perm_allows_contents_write(perm: &Value) -> bool {
    match perm {
        Value::String(s) => s == "write-all",
        Value::Mapping(_) => perm.get("contents").and_then(Value::as_str) == Some("write"),
        _ => false,
    }
}

fn check_contents_write(wf: &Value) -> Result<(), String> {
    let job = release_job(wf)?;
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
        let with = step.get("with").and_then(Value::as_mapping);
        for (key, tok) in with.into_iter().flatten() {
            let is_token = key
                .as_str()
                .is_some_and(|k| k.eq_ignore_ascii_case("token"));
            if is_token && !is_default_github_token(tok) {
                return Err(format!(
                    "forjar#1163: softprops/action-gh-release `with.token` must be omitted or GITHUB_TOKEN, got {tok:?}"
                ));
            }
        }
    }
    Ok(())
}

fn check_env_token_vars(env: Option<&Value>, scope: &str) -> Result<(), String> {
    let Some(env) = env else {
        return Ok(());
    };
    for var in TOKEN_ENV_VARS {
        let Some(tok) = env.get(var) else {
            continue;
        };
        if !is_default_github_token(tok) {
            return Err(format!(
                "forjar#1163: {scope} sets `{var}` to non-GITHUB_TOKEN value {tok:?}"
            ));
        }
    }
    Ok(())
}

fn step_label(step: &Value) -> String {
    let name = step
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("<unnamed>");
    format!("step {name:?}")
}

fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/')
}

fn strip_comment(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    // A `#` opens a comment only outside quotes (`echo "#1" && gh` runs gh).
    let mut quote = None;
    for (i, c) in trimmed.char_indices() {
        match quote {
            Some(q) if c == q => quote = None,
            None if c == '"' || c == '\'' => quote = Some(c),
            None if c == '#' && trimmed[..i].ends_with(' ') => return Some(&trimmed[..i]),
            _ => {}
        }
    }
    Some(trimmed)
}

/// Returns whether `line` invokes `gh` based on word splitting.
///
/// This is deliberately over-strict: a line that only mentions the word `gh`
/// inside a string or heredoc also counts, and the fix for such a step is to
/// give it the default token.
fn line_calls_gh(line: &str) -> bool {
    let Some(code) = strip_comment(line) else {
        return false;
    };
    code.split(|c: char| !is_word_char(c))
        .any(|w| w == "gh" || w.ends_with("/gh"))
}

fn step_calls_gh(step: &Value) -> bool {
    step.get("run")
        .and_then(Value::as_str)
        .is_some_and(|script| script.lines().any(line_calls_gh))
}

fn line_sets_or_unsets_token(line: &str) -> bool {
    let Some(code) = strip_comment(line) else {
        return false;
    };
    let sets = ["GH_TOKEN=", "GITHUB_TOKEN=", "GH_TOKEN<<", "GITHUB_TOKEN<<"];
    let unsets = ["unset", "env -u"].iter().any(|p| code.contains(p));
    sets.iter().any(|p| code.contains(p))
        || (unsets && TOKEN_ENV_VARS.iter().any(|v| code.contains(v)))
}

fn step_sets_or_unsets_token(step: &Value) -> bool {
    step.get("run")
        .and_then(Value::as_str)
        .is_some_and(|script| script.lines().any(line_sets_or_unsets_token))
}

fn env_has_allowed_token(env: Option<&Value>) -> bool {
    let Some(env) = env else {
        return false;
    };
    TOKEN_ENV_VARS
        .iter()
        .any(|var| env.get(*var).is_some_and(is_default_github_token))
}

fn has_effective_gh_token(step: &Value, job: &Value, wf: &Value) -> bool {
    env_has_allowed_token(step.get("env"))
        || env_has_allowed_token(job.get("env"))
        || env_has_allowed_token(wf.get("env"))
}

fn check_step_gh_token_env(step: &Value, job: &Value, wf: &Value) -> Result<(), String> {
    let scope = step_label(step);
    check_env_token_vars(step.get("env"), &scope)?;
    if step_sets_or_unsets_token(step) {
        return Err(format!(
            "forjar#1163: {scope} sets or unsets a GitHub token inside its run script"
        ));
    }
    if step_calls_gh(step) && !has_effective_gh_token(step, job, wf) {
        return Err(format!(
            "forjar#1163: {scope} runs `gh` without an effective GH_TOKEN or GITHUB_TOKEN"
        ));
    }
    Ok(())
}

fn check_gh_token_env(wf: &Value) -> Result<(), String> {
    let job = release_job(wf)?;
    check_env_token_vars(wf.get("env"), "top-level env")?;
    check_env_token_vars(job.get("env"), "jobs.release.env")?;
    for step in job_steps(job)? {
        check_step_gh_token_env(step, job, wf)?;
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

fn assert_rejected_with(res: Result<(), String>, needle: &str, case: &str) {
    match res {
        Ok(()) => panic!("{case}: expected rejection"),
        Err(err) => {
            assert!(err.contains("#1163"), "{case}: missing #1163 in {err}");
            assert!(err.contains(needle), "{case}: missing `{needle}` in {err}");
        }
    }
}

fn assert_rejected(res: Result<(), String>, case: &str) {
    assert_rejected_with(res, "#1163", case);
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

#[test]
fn rejects_workflow_env_custom_token() {
    let wf_gh = "env:\n  GH_TOKEN: ${{ secrets.PAT }}\njobs:\n  release:\n    steps: []\n";
    assert_rejected(check_gh_token_env(&parse_yaml(wf_gh, "wf_gh")), "wf_gh");

    let wf_github = "env:\n  GITHUB_TOKEN: ${{ secrets.PAT }}\njobs:\n  release:\n    steps: []\n";
    assert_rejected(
        check_gh_token_env(&parse_yaml(wf_github, "wf_github")),
        "wf_github",
    );
}

#[test]
fn rejects_job_env_custom_token() {
    let job_gh = "jobs:\n  release:\n    env:\n      GH_TOKEN: ${{ secrets.PAT }}\n    steps: []\n";
    assert_rejected(check_gh_token_env(&parse_yaml(job_gh, "job_gh")), "job_gh");

    let job_github =
        "jobs:\n  release:\n    env:\n      GITHUB_TOKEN: ${{ secrets.PAT }}\n    steps: []\n";
    assert_rejected(
        check_gh_token_env(&parse_yaml(job_github, "job_github")),
        "job_github",
    );
}

#[test]
fn rejects_step_env_custom_github_token() {
    let step_github = "jobs:\n  release:\n    steps:\n      - name: Delete\n        env:\n          GITHUB_TOKEN: ${{ secrets.PAT }}\n        run: echo\n";
    assert_rejected(
        check_gh_token_env(&parse_yaml(step_github, "step_github")),
        "step_github",
    );
}

#[test]
fn rejects_gh_cli_step_without_effective_token() {
    let gh_start = "jobs:\n  release:\n    steps:\n      - name: Delete\n        run: gh release delete nightly --yes\n";
    assert_rejected(
        check_gh_token_env(&parse_yaml(gh_start, "gh_start")),
        "gh_start",
    );

    let gh_inline = "jobs:\n  release:\n    steps:\n      - name: Check\n        run: if ! gh release view nightly; then echo missing; fi\n";
    assert_rejected(
        check_gh_token_env(&parse_yaml(gh_inline, "gh_inline")),
        "gh_inline",
    );
}

#[test]
fn accepts_gh_cli_step_with_effective_github_token() {
    let wf_pass = "env:\n  GITHUB_TOKEN: ${{ github.token }}\njobs:\n  release:\n    steps:\n      - name: Delete\n        run: gh release delete nightly --yes\n";
    assert!(check_gh_token_env(&parse_yaml(wf_pass, "wf_pass")).is_ok());

    let job_pass = "jobs:\n  release:\n    env:\n      GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}\n    steps:\n      - name: Delete\n        run: if ! gh release view nightly; then true; fi\n";
    assert!(check_gh_token_env(&parse_yaml(job_pass, "job_pass")).is_ok());

    let step_pass = "jobs:\n  release:\n    steps:\n      - name: Delete\n        env:\n          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}\n        run: gh release delete nightly --yes\n      - name: Echo\n        run: echo done\n";
    assert!(check_gh_token_env(&parse_yaml(step_pass, "step_pass")).is_ok());
}

fn run_step_yaml(env_gh_token: Option<&str>, script: &str) -> Value {
    let env_block = match env_gh_token {
        Some(tok) => format!("        env:\n          GH_TOKEN: {tok}\n"),
        None => String::new(),
    };
    let yaml = format!(
        "jobs:\n  release:\n    steps:\n      - name: Release\n{env_block}        run: |\n          {script}\n"
    );
    parse_yaml(&yaml, script)
}

#[test]
fn rejects_gh_cli_variants_without_effective_token() {
    for (case, script) in [
        ("subshell", "(gh release delete nightly) || true"),
        ("abs_path", "/usr/bin/gh release create nightly"),
        ("semicolon", "true;gh release list"),
    ] {
        assert_rejected(check_gh_token_env(&run_step_yaml(None, script)), case);
    }
}

#[test]
fn rejects_token_set_or_unset_in_run_script() {
    let cases = [
        (
            "inline_override",
            Some("${{ secrets.GITHUB_TOKEN }}"),
            "GH_TOKEN=${{ secrets.PAT }} gh release delete nightly",
        ),
        (
            "unset_tokens",
            None,
            "unset GH_TOKEN GITHUB_TOKEN; gh release list",
        ),
        (
            "github_env_write",
            None,
            "echo \"GITHUB_TOKEN=${{ secrets.PAT }}\" >> \"$GITHUB_ENV\"",
        ),
    ];
    for (case, env_tok, script) in cases {
        assert_rejected_with(
            check_gh_token_env(&run_step_yaml(env_tok, script)),
            "sets or unsets a GitHub token inside its run script",
            case,
        );
    }
}

#[test]
fn rejects_guard_bypass_repros() {
    let quoted = run_step_yaml(
        None,
        "echo \"Cleaning nightly #1\" && gh release delete nightly --yes",
    );
    assert_rejected_with(
        check_gh_token_env(&quoted),
        "runs `gh` without",
        "quoted_hash",
    );
    let upper = "jobs:\n  release:\n    steps:\n      - uses: softprops/action-gh-release@v3\n        with:\n          TOKEN: ${{ secrets.PAT }}\n";
    assert_rejected(
        check_gh_release_token(&parse_yaml(upper, "upper")),
        "upper_token",
    );
    let heredoc = run_step_yaml(None, "echo \"GITHUB_TOKEN<<EOF\" >> \"$GITHUB_ENV\"");
    assert_rejected_with(
        check_gh_token_env(&heredoc),
        "sets or unsets",
        "heredoc_env",
    );
    let env_u = run_step_yaml(None, "env -u GH_TOKEN gh release list");
    assert_rejected_with(check_gh_token_env(&env_u), "sets or unsets", "env_unset");
}

#[test]
fn accepts_non_gh_comments_trailing_comments_and_substrings() {
    for (case, script) in [
        ("trailing_comment", "ls -la artifacts # no gh call here"),
        ("comment_only", "# gh release list"),
        ("substring_high", "echo high"),
    ] {
        let res = check_gh_token_env(&run_step_yaml(None, script));
        assert!(res.is_ok(), "{case}: unexpected error {res:?}");
    }
}
