//! forjar#1000: parse-time validation for `github_runner` (contract
//! `github-runner-v1`, C6 `scope_is_validated`).
//!
//! Every refusal here is what the operator sees at `forjar validate`, before
//! any registration token is requested, and names the offending field in
//! backticks so the message can be matched on it.
//!
//! A value that still carries a `{{...}}` template cannot be judged yet and is
//! left to the post-resolution pass, the same allowance `tmp_reaper` makes.

use super::*;

fn templated(v: &Option<String>) -> bool {
    v.as_deref().is_some_and(|s| s.contains("{{"))
}

fn blank(v: &Option<String>) -> bool {
    !v.as_deref().is_some_and(|s| !s.trim().is_empty())
}

pub(super) fn validate_github_runner(
    id: &str,
    resource: &Resource,
    errors: &mut Vec<ValidationError>,
) {
    let mut refuse = |why: &str| {
        errors.push(ValidationError {
            message: format!("resource '{id}' (github_runner): {why}"),
        });
    };

    let spec = &resource.runner;
    match spec.scope.as_deref() {
        _ if templated(&spec.scope) || templated(&resource.repo) => {}
        Some("repo") if blank(&resource.repo) => {
            refuse("`repo` is required when `scope` is `repo`");
        }
        Some("org") if !blank(&resource.repo) => {
            refuse("`repo` must not be set when `scope` is `org`");
        }
        Some("org" | "repo") => {}
        Some(other) => refuse(&format!("`scope` is {other:?}, expected `org` or `repo`")),
        None => refuse("`scope` is required (`org` or `repo`)"),
    }

    if spec.labels.is_empty() {
        refuse("`labels` must list at least one label");
    }

    if blank(&spec.sha256) {
        refuse("`sha256` is required: the runner tarball must be pinned");
    }
}
