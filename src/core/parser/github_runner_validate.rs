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
    v.as_deref().is_none_or(|s| s.trim().is_empty())
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

    // The seat's verdict is keyed on the runner `name` alone, so a second
    // machine would find the first one's seat listed and be reported adopted
    // with nothing installed on it.
    if let MachineTarget::Multiple(machines) = &resource.machine {
        if machines.len() > 1 {
            refuse(&format!(
                "`machine` lists {} machines ({}); a seat is one runner on one machine, \
                 declare one resource per machine, each with its own `name`",
                machines.len(),
                machines.join(", ")
            ));
        }
    }

    // The seat script elevates with its own `sudo -n`; a declared `sudo:` would
    // be silently ignored for this type.
    if resource.sudo {
        refuse("`sudo` is not supported: the seat script runs its own `sudo -n`");
    }

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
    } else if !templated(&spec.sha256)
        && !crate::resources::github_runner::is_sha256_hex(
            &spec
                .sha256
                .as_deref()
                .unwrap_or_default()
                .to_ascii_lowercase(),
        )
    {
        refuse("`sha256` must be exactly 64 hex characters");
    }
}

/// The identity GitHub gives a seat: its namespace (an org, or one repo) and
/// its runner `name`. Owner, repo and runner names are compared without case,
/// so two spellings GitHub may treat as one seat are refused, not trusted.
///
/// A templated field is resolved against the config's own params and machines
/// first, so `name: "{{params.seat}}"` and the literal it expands to are one
/// seat. A field that cannot be resolved here is compared as written.
fn seat_identity(config: &ForjarConfig, resource: &Resource) -> Option<String> {
    let field = |v: &Option<String>| -> Option<String> {
        let raw = v.as_deref()?.trim();
        let text = if raw.contains("{{") && !raw.contains("{{secrets.") {
            crate::core::resolver::resolve_template(raw, &config.params, &config.machines)
                .unwrap_or_else(|_| raw.to_string())
        } else {
            raw.to_string()
        };
        let text = text.trim().to_ascii_lowercase();
        (!text.is_empty()).then_some(text)
    };
    let (owner, name) = (field(&resource.owner)?, field(&resource.name)?);
    match field(&resource.runner.scope)?.as_str() {
        "org" => Some(format!("org {owner}, runner `{name}`")),
        "repo" => Some(format!(
            "repo {owner}/{}, runner `{name}`",
            field(&resource.repo)?
        )),
        _ => None,
    }
}

/// forjar#1000: one seat identity, one resource. The seat's verdict is keyed
/// on its identity alone, so a second resource holding the same identity, on
/// another machine, under a second machine key for the same host, or from a
/// `for_each`/`count`/recipe copy, would find the first one's runner listed and
/// be reported adopted with nothing installed, or, declared `absent`, would
/// deregister the first one's runner. Runs over the expanded config too.
pub(super) fn validate_github_runner_seats_unique(
    config: &ForjarConfig,
    errors: &mut Vec<ValidationError>,
) {
    let mut seen: Vec<(String, &str)> = Vec::new();
    for (id, resource) in &config.resources {
        if resource.resource_type != ResourceType::GithubRunner {
            continue;
        }
        let Some(identity) = seat_identity(config, resource) else {
            continue;
        };
        if let Some((_, first)) = seen.iter().find(|(seat, _)| *seat == identity) {
            errors.push(ValidationError {
                message: format!(
                    "resource '{id}' (github_runner): the seat ({identity}) is already declared \
                     by resource '{first}'; a seat is one runner on one machine, give each \
                     resource its own `name`"
                ),
            });
        } else {
            seen.push((identity, id.as_str()));
        }
    }
}
