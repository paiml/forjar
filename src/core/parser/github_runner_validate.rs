//! forjar#1000: parse-time validation for `github_runner` (contract
//! `github-runner-v1`, C6 `scope_is_validated`).
//!
//! Every refusal here is what the operator sees at `forjar validate`, before
//! any registration token is requested, and names the offending field in
//! backticks so the message can be matched on it.
//!
//! A templated value is resolved against the config's own `params` and
//! `machines` before it is judged, so `repo: "{{params.repo}}"` is held to the
//! same rules as the literal it expands to. A value that does not resolve is
//! refused. The one value left for later is a `{{item}}`/`{{index}}` on a
//! `for_each`/`count` resource: expansion substitutes it, and the pass over the
//! expanded config judges the copies.

use super::*;

/// A `github_runner` field as far as this pass can see it.
enum Field {
    /// Unset or blank.
    Unset,
    /// The value, with every template resolved.
    Value(String),
    /// Carries `{{item}}`/`{{index}}` on a resource that expansion copies; the
    /// copies are judged in the pass over the expanded config.
    Deferred,
}

/// Resolve `v` against the config's `params` and `machines`, or say why it
/// cannot be.
fn resolve_field(
    config: &ForjarConfig,
    resource: &Resource,
    v: &Option<String>,
) -> Result<Field, String> {
    let Some(raw) = v.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(Field::Unset);
    };
    if !raw.contains("{{") {
        return Ok(Field::Value(raw.to_string()));
    }
    let expands = resource.for_each.is_some() || resource.count.is_some();
    if expands && (raw.contains("{{item}}") || raw.contains("{{index}}")) {
        return Ok(Field::Deferred);
    }
    if raw.contains("{{secrets.") {
        return Err(format!(
            "is {raw:?}; a `{{{{secrets.*}}}}` value cannot be judged here, use a literal \
             or `{{{{params.*}}}}`"
        ));
    }
    let text = crate::core::resolver::resolve_template(raw, &config.params, &config.machines)
        .map_err(|e| format!("is {raw:?}, which does not resolve: {e}"))?;
    if text.contains("{{") {
        return Err(format!(
            "is {raw:?}, which resolves to {text:?}, still a template"
        ));
    }
    let text = text.trim();
    Ok(if text.is_empty() {
        Field::Unset
    } else {
        Field::Value(text.to_string())
    })
}

pub(super) fn validate_github_runner(
    config: &ForjarConfig,
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

    // Anything but `absent` would be applied as present: `state: removed` must
    // not register a runner.
    match resource.state.as_deref() {
        None | Some("present" | "absent") => {}
        Some(other) => refuse(&format!(
            "`state` is {other:?}, expected `present` or `absent`"
        )),
    }

    let spec = &resource.runner;
    let mut field = |name: &str, v: &Option<String>| match resolve_field(config, resource, v) {
        Ok(f) => Some(f),
        Err(why) => {
            refuse(&format!("`{name}` {why}"));
            None
        }
    };
    let scope = field("scope", &spec.scope);
    let repo = field("repo", &resource.repo);
    let sha256 = field("sha256", &spec.sha256);

    if let (Some(scope), Some(repo)) = (scope, repo) {
        check_scope(&scope, &repo, &mut refuse);
    }

    if spec.labels.is_empty() {
        refuse("`labels` must list at least one label");
    }

    match sha256 {
        Some(Field::Unset) => refuse("`sha256` is required: the runner tarball must be pinned"),
        Some(Field::Value(s))
            if !crate::resources::github_runner::is_sha256_hex(&s.to_ascii_lowercase()) =>
        {
            refuse("`sha256` must be exactly 64 hex characters");
        }
        _ => {}
    }
}

/// C6: `scope` is `org` or `repo`; a repo seat names its `repo`, an org seat
/// names none.
fn check_scope(scope: &Field, repo: &Field, refuse: &mut impl FnMut(&str)) {
    match (scope, repo) {
        (Field::Deferred, _) => {}
        (Field::Unset, _) => refuse("`scope` is required (`org` or `repo`)"),
        (Field::Value(s), Field::Unset) if s == "repo" => {
            refuse("`repo` is required when `scope` is `repo`");
        }
        // A deferred repo is still a repo: an org seat must not carry one.
        (Field::Value(s), Field::Value(_) | Field::Deferred) if s == "org" => {
            refuse("`repo` must not be set when `scope` is `org`");
        }
        (Field::Value(s), _) if s == "org" || s == "repo" => {}
        (Field::Value(other), _) => {
            refuse(&format!("`scope` is {other:?}, expected `org` or `repo`"));
        }
    }
}

/// The identity GitHub gives a seat: its namespace (an org, or one repo) and
/// its runner `name`. Owner, repo and runner names are compared without case,
/// so two spellings GitHub may treat as one seat are refused, not trusted.
///
/// A templated field is resolved against the config's own params and machines
/// first, so `name: "{{params.seat}}"` and the literal it expands to are one
/// seat. A field that does not resolve is refused by `validate_github_runner`;
/// a deferred `{{item}}`/`{{index}}` is compared in the pass over the expanded
/// config, where each copy carries its own value.
fn seat_identity(config: &ForjarConfig, resource: &Resource) -> Option<String> {
    let field = |v: &Option<String>| -> Option<String> {
        match resolve_field(config, resource, v) {
            Ok(Field::Value(text)) => Some(text.to_ascii_lowercase()),
            _ => None,
        }
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
