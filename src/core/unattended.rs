//! forjar#372: strip everything a config can make a READ verb EXECUTE.
//!
//! # The promise that was not kept
//!
//! `src/verb/registry.rs` publishes all nine verbs with `Effects::ReadOnly`,
//! and `src/verb/spec.rs` says what that means: *safe for an agent to call
//! unattended*. MCP publishes the same bit as `readOnlyHint: true`, which is
//! the only signal an agent has before deciding to call a tool without a human
//! watching.
//!
//! `plan` did not keep it. Three ordinary config keys reach a subprocess from
//! inside `planner::plan`, with no flag involved and nothing to opt into:
//!
//! | config key | what runs | where |
//! |---|---|---|
//! | `ambient_inputs: [cmd]` | `bash -c cmd` | `core/task/ambient.rs` |
//! | `secrets.provider: sops` / `op` | `sops` / `op` | `core/resolver/template.rs` |
//! | `output_equivalence: !command cmd` | `bash -c cmd` | `core/task/output_hash.rs` |
//!
//! Measured over real `forjar mcp` stdio on 1.21.0, one verb per fresh
//! fixture: a config declaring `ambient_inputs: ["touch AMBIENT_FIRED; echo
//! v1"]` created the file when — and only when — `forjar_plan` was called. So
//! an agent asked to *inspect* an untrusted repository executed whatever that
//! repository declared.
//!
//! # Why not just call `plan` Mutating
//!
//! Because it is not. `plan` changes no machine and writes no lock; declaring
//! it `Mutating` would discard the one accurate signal an agent has about the
//! other eight verbs' neighbour, and would not stop the execution either.
//!
//! Instead the unattended surface plans over a config with those three keys
//! REMOVED, and says so in its output. That makes the plan explicitly
//! lock-relative for the parts it could not compute — the same disclosure
//! `forjar plan` already makes for its other blind spot (forjar#342, "run
//! `forjar drift` for what the machines actually hold").
//!
//! # What is deliberately NOT changed
//!
//! The CLI. `forjar plan` still probes, still runs `ambient_inputs`, still
//! shells out to `sops`. `ambient_inputs` (#244) is a good feature and the
//! operator who typed `forjar plan` chose their own config; the defect is a
//! surface that promises the opposite exposing it.

use crate::core::types::{ForjarConfig, OutputEquivalence, Resource};

/// The secrets provider substituted for any provider implemented as a
/// subprocess.
///
/// It must not silently become `env`: the `_ =>` arm of
/// `resolve_secret_with_provider` reads `FORJAR_SECRET_<KEY>`, so downgrading
/// `sops` to the default would resolve a DIFFERENT value under the same name
/// and hash it into the plan. This provider resolves to an error instead, which
/// `resolver::resolve_or_fallback` turns into "leave the resource unresolved" —
/// visible, and disclosed.
pub const NO_EXEC_SECRET_PROVIDER: &str = "unattended-no-exec";

/// True when a secrets provider is implemented by spawning a process.
///
/// `env` reads an environment variable and `file` reads a path; neither runs
/// anything a config author chose.
#[must_use]
pub fn provider_executes(provider: Option<&str>) -> bool {
    matches!(provider, Some("sops" | "op"))
}

/// A config with every config-declared subprocess removed, plus one line per
/// thing removed.
///
/// The returned notes are the disclosure. An empty vector means the config
/// declared nothing this surface had to skip, so the unattended plan and the
/// CLI plan are computing the same thing.
#[must_use]
pub fn sanitize_config(config: &ForjarConfig) -> (ForjarConfig, Vec<String>) {
    let mut out = config.clone();
    let mut skipped = Vec::new();

    strip_secret_provider(&mut out, &mut skipped);
    for (id, resource) in out.resources.iter_mut() {
        strip_ambient_inputs(id, resource, &mut skipped);
        strip_output_normalisers(id, resource, &mut skipped);
    }

    (out, skipped)
}

/// Replace a subprocess secrets provider with the non-executing one.
fn strip_secret_provider(config: &mut ForjarConfig, skipped: &mut Vec<String>) {
    if !provider_executes(config.secrets.provider.as_deref()) {
        return;
    }
    let name = config.secrets.provider.take().unwrap_or_default();
    config.secrets.provider = Some(NO_EXEC_SECRET_PROVIDER.to_string());
    skipped.push(format!(
        "secrets: provider '{name}' not invoked; `{{{{secrets.*}}}}` left unresolved"
    ));
}

/// Drop `ambient_inputs`, which are shell commands run to fingerprint the host.
fn strip_ambient_inputs(id: &str, resource: &mut Resource, skipped: &mut Vec<String>) {
    if resource.ambient_inputs.is_empty() {
        return;
    }
    let n = resource.ambient_inputs.len();
    resource.ambient_inputs.clear();
    skipped.push(format!(
        "{id}: {n} ambient_inputs command(s) not executed; staleness from ambient state not checked"
    ));
}

/// Downgrade `output_equivalence: !command` to `none`.
///
/// `none` is the honest substitute: it means "this artifact's CONTENT does not
/// participate in the staleness hash", which is exactly true of an artifact
/// whose declared normaliser we refused to run. It is recorded distinctly from
/// `bytes` by `hash_outputs_with`, so the skip cannot alias with a real byte
/// comparison.
fn strip_output_normalisers(id: &str, resource: &mut Resource, skipped: &mut Vec<String>) {
    let named: Vec<String> = resource
        .output_equivalence
        .iter()
        .filter(|(_, rule)| matches!(rule, OutputEquivalence::Command(_)))
        .map(|(artifact, _)| artifact.clone())
        .collect();
    for artifact in named {
        resource
            .output_equivalence
            .insert(artifact.clone(), OutputEquivalence::None);
        skipped.push(format!(
            "{id}: output_equivalence normaliser for '{artifact}' not executed; \
             its content is not compared"
        ));
    }
}

/// The prose disclosure for a set of skipped declarations.
///
/// `None` when nothing was skipped, so the field is absent exactly when the
/// unattended plan and the CLI plan agree — the same biconditional
/// `scope_disclosure` uses for forjar#342.
#[must_use]
pub fn disclosure(skipped: &[String]) -> Option<String> {
    if skipped.is_empty() {
        return None;
    }
    Some(format!(
        "this surface never executes what a config declares, so {} declaration(s) \
         were skipped ({}); the plan is lock-relative for those — run `forjar plan` \
         from a checkout you trust, or `forjar drift`, for what the machines \
         actually hold",
        skipped.len(),
        skipped.join("; ")
    ))
}

/// Join the two plan disclosures into the one string a consumer reads.
///
/// forjar#342 named what the plan did not CONSULT; this file names what it did
/// not EXECUTE. A consumer that reads only `disclosure` must learn both, so
/// they compose rather than overwrite.
#[must_use]
pub fn merge_disclosures(scope: Option<String>, unattended: Option<String>) -> Option<String> {
    match (scope, unattended) {
        (Some(a), Some(b)) => Some(format!("{a} Also: {b}.")),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests;
