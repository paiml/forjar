use crate::core::types::*;
use std::path::Path;

/// Get the value field or return an error for data sources that require it.
fn require_value<'a>(key: &str, source: &'a DataSource) -> Result<&'a str, String> {
    source
        .value
        .as_deref()
        .ok_or_else(|| format!("data source '{key}' requires 'value' field"))
}

fn resolve_file_source(key: &str, source: &DataSource, base: &Path) -> Result<String, String> {
    let path = require_value(key, source)?;
    std::fs::read_to_string(base.join(path))
        .map(|s| s.trim().to_string())
        .or_else(|e| {
            source
                .default
                .clone()
                .ok_or_else(|| format!("data source '{key}' file error: {e}"))
        })
}

fn resolve_command_source(key: &str, source: &DataSource) -> Result<String, String> {
    let cmd = require_value(key, source)?;
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output()
        .map_err(|e| format!("data source '{key}' command error: {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        source.default.clone().ok_or_else(|| {
            format!(
                "data source '{}' command failed (exit {})",
                key,
                output.status.code().unwrap_or(-1)
            )
        })
    }
}

fn resolve_dns_source(key: &str, source: &DataSource) -> Result<String, String> {
    use std::net::ToSocketAddrs;
    let host = require_value(key, source)?;
    let addr_str = format!("{host}:0");
    match addr_str.to_socket_addrs() {
        Ok(mut addrs) => {
            if let Some(addr) = addrs.next() {
                Ok(addr.ip().to_string())
            } else {
                source
                    .default
                    .clone()
                    .ok_or_else(|| format!("data source '{key}' DNS: no addresses"))
            }
        }
        Err(e) => source
            .default
            .clone()
            .ok_or_else(|| format!("data source '{key}' DNS error: {e}")),
    }
}

/// FJ-1270: Warn when the state lock was applied longer ago than `max_staleness`.
///
/// Decides only whether a warning is printed — never which value is returned —
/// which is why it is split out of `resolve_forjar_state_source`. Returns an
/// error only when `max_staleness` itself cannot be parsed, and (as before)
/// never parses it at all when the lock carries no `last_apply`.
fn warn_if_state_is_stale(
    key: &str,
    source: &DataSource,
    doc: &serde_yaml_ng::Value,
) -> Result<(), String> {
    let Some(max_staleness) = source.max_staleness.as_ref() else {
        return Ok(());
    };
    let Some(last_apply) = doc.get("last_apply").and_then(|v| v.as_str()) else {
        return Ok(());
    };
    let max_secs = super::staleness::parse_duration_secs(max_staleness)
        .map_err(|e| format!("data source '{key}': invalid max_staleness: {e}"))?;
    if super::staleness::is_stale(last_apply, max_secs) {
        eprintln!(
            "warning: data source '{key}' is stale (last_apply: {last_apply}, max_staleness: {max_staleness})"
        );
    }
    Ok(())
}

/// Pick the value for a data source that asked for specific outputs by name.
///
/// Decides which of the requested names wins: the first one the state lock
/// actually carries. Falls back to the data source's `default` when none of
/// them are present, and errors when there is no default.
fn select_requested_output(
    key: &str,
    source: &DataSource,
    outputs: &serde_yaml_ng::Mapping,
) -> Result<String, String> {
    for output_name in &source.outputs {
        if let Some(val) = outputs.get(serde_yaml_ng::Value::String(output_name.clone())) {
            return Ok(val.as_str().unwrap_or("").to_string());
        }
    }
    source.default.clone().ok_or_else(|| {
        format!(
            "data source '{}': none of requested outputs ({}) found in state",
            key,
            source.outputs.join(", ")
        )
    })
}

/// Render every string-valued output in the state lock as a JSON object.
///
/// This is the answer when a data source names no specific outputs. Entries
/// whose key or value is not a string are dropped, as they always were.
fn outputs_as_json(key: &str, outputs: &serde_yaml_ng::Mapping) -> Result<String, String> {
    let json_map: std::collections::HashMap<String, String> = outputs
        .iter()
        .filter_map(|(k, v)| Some((k.as_str()?.to_string(), v.as_str()?.to_string())))
        .collect();
    serde_json::to_string(&json_map)
        .map_err(|e| format!("data source '{key}': serialize outputs: {e}"))
}

/// FJ-1260: Resolve forjar-state data source by reading outputs from another config's state.
/// Reads the global lock (`forjar.lock.yaml`) in the given state directory and extracts
/// stored output values. Falls back to `default` if the state is unavailable.
fn resolve_forjar_state_source(
    key: &str,
    source: &DataSource,
    base: &Path,
) -> Result<String, String> {
    let state_dir = source.state_dir.as_deref().unwrap_or("state");
    let lock_path = base.join(state_dir).join("forjar.lock.yaml");

    if !lock_path.exists() {
        return source.default.clone().ok_or_else(|| {
            format!(
                "data source '{}': state lock not found at {} (no default)",
                key,
                lock_path.display()
            )
        });
    }

    let content = std::fs::read_to_string(&lock_path)
        .map_err(|e| format!("data source '{key}': read state lock: {e}"))?;
    let doc: serde_yaml_ng::Value = serde_yaml_ng::from_str(&content)
        .map_err(|e| format!("data source '{key}': parse state lock: {e}"))?;

    // FJ-1270: Check staleness if max_staleness is configured
    warn_if_state_is_stale(key, source, &doc)?;

    // Extract output values from the lock's "outputs" section
    let outputs = match doc.get("outputs") {
        Some(serde_yaml_ng::Value::Mapping(m)) => m,
        _ => {
            return source
                .default
                .clone()
                .ok_or_else(|| format!("data source '{key}': state lock has no outputs section"));
        }
    };

    // If specific outputs requested, return the first matching one
    if !source.outputs.is_empty() {
        return select_requested_output(key, source, outputs);
    }

    // No specific outputs requested — return all as JSON
    outputs_as_json(key, outputs)
}

/// The directory a manifest's relative `data:` paths are read from: the
/// manifest's own directory, never the process's working directory (forjar#660).
pub fn manifest_dir(file: &Path) -> &Path {
    file.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

/// Resolve one source by type, reading relative `file`/`forjar-state` paths from `base`.
fn resolve_one_source(key: &str, source: &DataSource, base: &Path) -> Result<String, String> {
    match source.source_type {
        DataSourceType::File => resolve_file_source(key, source, base),
        DataSourceType::Command => resolve_command_source(key, source),
        DataSourceType::Dns => resolve_dns_source(key, source),
        DataSourceType::ForjarState => resolve_forjar_state_source(key, source, base),
    }
}

/// FJ-223: Resolve all data sources into `config.data_values`, reading relative
/// `file`/`forjar-state` paths from `base` (the manifest's directory).
///
/// Values land in `config.data_values`, never in `params` (forjar#660). There is
/// deliberately NO variant without a base: the one that existed let `plan`,
/// `apply` and `lock` resolve against the working directory while `show`
/// resolved against the manifest, so one manifest meant two things.
/// Strict: the first failing source aborts. See [`resolve_data_sources_isolated`].
pub fn resolve_data_sources_with_base(
    config: &mut ForjarConfig,
    base: &Path,
) -> Result<(), String> {
    // An undeclared `{{data.KEY}}` is checked BEFORE any source runs, in every
    // caller, not just the ones that go through `finish_resolving`.
    super::data_subst::require_known_data_refs(config)?;
    let mut resolved = DataValues::new();
    for (key, source) in &config.data {
        resolved.insert(key.clone(), resolve_one_source(key, source, base)?);
    }
    config.data_values = resolved;
    Ok(())
}

/// forjar#660: the one load path for any command that resolves templates.
///
/// `{{data.x}}` is not a template the resolver can answer from the file: it
/// reads it out of `config.data_values`, which only `resolve_data_sources_with_base`
/// fills. That precondition used to be re-typed by hand in each subcommand and
/// the ones that forgot it failed with `unknown data source`, or, where errors
/// were swallowed, reported on a config with the reference left unresolved.
/// Parsing and resolving are now one call, so a reader cannot get the first
/// without the second.
///
/// The config comes back in READER form: every data value as written, no
/// stand-in character in any string. A command that renders resources from it
/// loads with [`load_resolved_config_for_render`] instead.
pub fn load_resolved_config(path: &Path) -> Result<ForjarConfig, String> {
    let config = crate::core::parser::parse_and_validate(path)?;
    finish_resolving(config, manifest_dir(path), true, Form::Reader)
        .map_err(crate::core::error::ForjarError::into_message)
}

/// [`load_resolved_config`] in RENDER form, for a command that hands the config's
/// resources to the template resolver (`resolve_resource_templates`,
/// `resolve_template*`). Data values are written with their braces held back so
/// the resolver cannot scan them a second time; the resolver restores them as its
/// last step. A command that prints or hashes this config without rendering must
/// call `decode_literals` first. `tests/falsification_660_scan.rs` requires every
/// file that renders to load through this function or `load_resolved_config_with`.
pub fn load_resolved_config_for_render(path: &Path) -> Result<ForjarConfig, String> {
    let config = crate::core::parser::parse_and_validate(path)?;
    finish_resolving(config, manifest_dir(path), true, Form::Render)
        .map_err(crate::core::error::ForjarError::into_message)
}

/// Which side of the loader a caller is on. Reader form is decoded; render form
/// keeps the stand-ins for the template resolver's last step.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Form {
    Reader,
    Render,
}

/// The same load for a command that changes the params first (`--env-file`,
/// `--workspace`, `--what-if`) and renders its own `outputs:` later (plan, apply,
/// lock, refresh). `prepare` runs after parse and validation and BEFORE any
/// `data:` source runs, so a source can template those params; `outputs:` have
/// their `{{data.x}}` written in but are left for `state::resolve_outputs` to
/// resolve once, when they are persisted.
pub fn load_resolved_config_with(
    path: &Path,
    prepare: impl FnOnce(&mut ForjarConfig) -> Result<(), String>,
) -> Result<ForjarConfig, String> {
    let mut config = crate::core::parser::parse_and_validate(path)?;
    prepare(&mut config)?;
    finish_resolving(config, manifest_dir(path), false, Form::Render)
        .map_err(crate::core::error::ForjarError::into_message)
}

/// The same resolution over a manifest that is only PARSED, not validated.
///
/// For the diagnostics (`graph *`, `status *`, `validate --check-*`, scoring)
/// whose job is to report on a manifest that may be structurally wrong: they
/// used to read it with a bare serde parse, and putting `validate_config` in
/// front of them would turn "report what is wrong" into "refuse to look". They
/// still resolve `data:` sources and `outputs:` (forjar#660); what they skip is
/// the structural validation and recipe expansion they never had.
pub fn load_resolved_config_lenient(
    path: &Path,
) -> Result<ForjarConfig, crate::core::error::ForjarError> {
    let config = crate::core::parser::parse_config_file(path)
        .map_err(crate::core::error::ForjarError::validation)?;
    finish_resolving(config, manifest_dir(path), true, Form::Reader)
}

/// Resolve the `data:` sources AND write their values into EVERY string of the
/// config (forjar#660), whichever section it is in.
///
/// `pin`, `sbom`, `compare`, `-t`/`-g` and the tag/version diagnostics read
/// those fields straight off the loaded config, so a `version: "{{data.v}}"`
/// that was only resolved on the render path hashed, listed and compared as the
/// literal reference. With `outputs` true they are resolved here exactly once
/// and a reader must not run `state::resolve_outputs` over them again.
pub(crate) fn finish_resolving(
    mut config: ForjarConfig,
    base: &Path,
    outputs: bool,
    form: Form,
) -> Result<ForjarConfig, crate::core::error::ForjarError> {
    resolve_data_sources_with_base(&mut config, base)
        .map_err(crate::core::error::ForjarError::validation)?;
    finish_substituting(config, outputs, form)
}

/// Everything in [`finish_resolving`] after the `data:` sources have run: write their
/// values into the config's strings and, with `outputs`, resolve `outputs:` once.
fn finish_substituting(
    mut config: ForjarConfig,
    outputs: bool,
    form: Form,
) -> Result<ForjarConfig, crate::core::error::ForjarError> {
    super::data_subst::substitute_data_refs(&mut config, form == Form::Render || outputs)
        .map_err(crate::core::error::ForjarError::validation)?;
    if outputs {
        // `outputs:` carry templates too (`{{data.x}}`, `{{params.y}}`); resolving them
        // here means every reader that prints the config prints resolved outputs, and
        // a template that cannot resolve fails the load instead of printing raw.
        let resolved = crate::core::state::resolve_outputs(&config)?;
        for (k, v) in resolved {
            if let Some(o) = config.outputs.get_mut(&k) {
                o.value = v;
            }
        }
        // Outputs were resolved by the template resolver, which scans every string it
        // is given; the values were held back for that pass, so a reader gets them back
        // now. The reader form never leaves a stand-in in a returned config.
        if form == Form::Reader {
            super::data_subst::decode_literals(&mut config)
                .map_err(crate::core::error::ForjarError::validation)?;
        }
    }
    Ok(config)
}

/// A data source that failed to resolve (recorded, not fatal).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataSourceFailure {
    pub key: String,
    pub message: String,
}

/// Fault-isolating variant of [`resolve_data_sources_with_base`]: every source is tried,
/// failures are returned instead of aborting, and healthy sources land in
/// `config.data_values`.
pub fn resolve_data_sources_isolated(
    config: &mut ForjarConfig,
    base: &Path,
) -> Vec<DataSourceFailure> {
    let mut failures = Vec::new();
    let mut resolved = DataValues::new();
    for (key, source) in &config.data {
        match resolve_one_source(key, source, base) {
            Ok(v) => {
                resolved.insert(key.clone(), v);
            }
            Err(message) => failures.push(DataSourceFailure {
                key: key.clone(),
                message,
            }),
        }
    }
    config.data_values = resolved;
    failures
}

/// [`load_resolved_config_with`] for `plan` and `apply`, which isolate data-source
/// faults (PMAT-828): a failing source no longer aborts the load. `drop_dependents`
/// receives the config and the failures BEFORE any `{{data.x}}` is written in, removes
/// the resources a failed source feeds, and returns the messages to report. Dropping
/// them first is what lets substitution run over the rest: a failed key has no value.
/// An undeclared `{{data.KEY}}` is still a hard error, checked before any source runs.
pub fn load_resolved_config_isolated(
    path: &Path,
    prepare: impl FnOnce(&mut ForjarConfig) -> Result<(), String>,
    drop_dependents: impl FnOnce(&mut ForjarConfig, &[DataSourceFailure]) -> Vec<String>,
) -> Result<(ForjarConfig, Vec<String>), String> {
    let mut config = crate::core::parser::parse_and_validate(path)?;
    prepare(&mut config)?;
    super::data_subst::require_known_data_refs(&config)?;
    let failures = resolve_data_sources_isolated(&mut config, manifest_dir(path));
    let messages = if failures.is_empty() {
        Vec::new()
    } else {
        drop_dependents(&mut config, &failures)
    };
    let config = finish_substituting(config, false, Form::Render)
        .map_err(crate::core::error::ForjarError::into_message)?;
    Ok((config, messages))
}

/// True when `text` holds a `{{ ... data.<key> ... }}` template reference.
fn references_data_key(text: &str, key: &str) -> bool {
    let needle = format!("data.{key}");
    text.split("{{").skip(1).any(|rest| {
        let inner = rest.split("}}").next().unwrap_or("");
        inner.match_indices(&needle).any(|(i, _)| {
            let before_ok = inner[..i]
                .chars()
                .next_back()
                .is_none_or(|c| !(c.is_alphanumeric() || c == '_' || c == '-' || c == '.'));
            let after_ok = inner[i + needle.len()..]
                .chars()
                .next()
                .is_none_or(|c| !(c.is_alphanumeric() || c == '_' || c == '-'));
            before_ok && after_ok
        })
    })
}

/// Resources that cannot be planned because a failed data source feeds them:
/// those whose templates reference it, plus (transitively) anything that
/// `depends_on` one of them. Maps resource id -> message naming the source.
pub fn dependents_of_failures(
    config: &ForjarConfig,
    failures: &[DataSourceFailure],
) -> std::collections::BTreeMap<String, String> {
    let mut hit = std::collections::BTreeMap::new();
    for (id, res) in &config.resources {
        let text = serde_yaml_ng::to_string(res).unwrap_or_default();
        if let Some(f) = failures.iter().find(|f| references_data_key(&text, &f.key)) {
            hit.insert(
                id.clone(),
                format!("data source '{}' failed: {}", f.key, f.message),
            );
        }
    }
    loop {
        let next: Vec<(String, String)> = config
            .resources
            .iter()
            .filter(|(id, _)| !hit.contains_key(*id))
            .filter_map(|(id, res)| {
                let dep = res.depends_on.iter().find(|d| hit.contains_key(*d))?;
                Some((id.clone(), format!("depends on skipped resource '{dep}'")))
            })
            .collect();
        if next.is_empty() {
            return hit;
        }
        hit.extend(next);
    }
}

/// `outputs:` entries that cannot be resolved because a failed data source feeds them.
/// Maps output name -> message naming the source. They are skipped with the resources, so a
/// failed source never aborts the apply's post-actions at `resolve_outputs`.
pub fn outputs_of_failures(
    config: &ForjarConfig,
    failures: &[DataSourceFailure],
) -> std::collections::BTreeMap<String, String> {
    config
        .outputs
        .iter()
        .filter_map(|(name, out)| {
            let f = failures
                .iter()
                .find(|f| references_data_key(&out.value, &f.key))?;
            Some((
                name.clone(),
                format!("data source '{}' failed: {}", f.key, f.message),
            ))
        })
        .collect()
}
