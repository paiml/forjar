//! forjar#660: write resolved `{{data.x}}` values into the loaded config.
//!
//! The template resolver answers `{{data.x}}` when it renders a resource, so the
//! render path (`show`, `plan`, `apply`) always saw the value. The readers that
//! never render — `pin`, `sbom`, `compare`, the tag/version diagnostics — read
//! `version`, `tags`, `packages`, `content` and every other string straight off
//! the config, and saw `{{data.x}}` as text. Substituting here, over EVERY string
//! of `resources` and `params`, makes the loaded config say what the manifest
//! means, whichever field a reader looks at.
//!
//! One rule keeps this from changing what the render path produces: the
//! resolver is single-pass — it does not re-scan text it inserted. Substituting
//! early would hand a value containing `{{` to a second scan, so a reference
//! whose value contains `{{` is left in place for the resolver to expand once.

use crate::core::types::{DataValues, ForjarConfig};
use serde_yaml_ng::Value;

/// Replace `{{data.KEY}}` (whitespace inside the braces allowed) in one string.
/// Unknown keys and values that contain `{{` are left as they are.
fn substitute_str(s: &str, data: &DataValues, changed: &mut bool) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(open) = rest.find("{{") {
        let after = &rest[open + 2..];
        let Some(close) = after.find("}}") else {
            break;
        };
        let key = after[..close].trim();
        let replacement = key
            .strip_prefix("data.")
            .and_then(|k| data.get(k))
            .filter(|v| !v.contains("{{"));
        out.push_str(&rest[..open]);
        match replacement {
            Some(v) => {
                out.push_str(v);
                *changed = true;
            }
            None => out.push_str(&rest[open..open + 2 + close + 2]),
        }
        rest = &after[close + 2..];
    }
    out.push_str(rest);
    out
}

fn walk(v: &mut Value, data: &DataValues, changed: &mut bool) {
    match v {
        Value::String(s) if s.contains("{{") => *s = substitute_str(s, data, changed),
        Value::Sequence(items) => items.iter_mut().for_each(|i| walk(i, data, changed)),
        Value::Mapping(m) => m.iter_mut().for_each(|(_, i)| walk(i, data, changed)),
        Value::Tagged(t) => walk(&mut t.value, data, changed),
        _ => {}
    }
}

/// Substitute resolved data values into `config.resources` and `config.params`.
/// `outputs` are resolved separately, once, by `state::resolve_outputs`.
pub(super) fn substitute_data_refs(config: &mut ForjarConfig) -> Result<(), String> {
    if config.data_values.is_empty() {
        return Ok(());
    }
    let data = config.data_values.clone();

    let mut resources =
        serde_yaml_ng::to_value(&config.resources).map_err(|e| format!("resources: {e}"))?;
    let mut changed = false;
    walk(&mut resources, &data, &mut changed);
    if changed {
        config.resources =
            serde_yaml_ng::from_value(resources).map_err(|e| format!("resources: {e}"))?;
    }

    for value in config.params.values_mut() {
        let mut ignored = false;
        walk(value, &data, &mut ignored);
    }
    Ok(())
}

/// Every `{{data.KEY}}` in the manifest whose KEY no `data:` source declares.
///
/// `{{params.KEY}}` with no such param is an error; so is this, in every
/// command, because the loader calls it (forjar#660). Before it, a reader that
/// never rendered the template (`show`, `pin`, `validate`) printed the literal
/// `{{data.KEY}}` and exited 0. Run BEFORE substitution, so a data VALUE that
/// happens to contain `{{data.x}}` text is not mistaken for a reference.
pub(crate) fn unknown_data_refs(config: &ForjarConfig) -> Vec<String> {
    fn scan(what: &str, v: &Value, config: &ForjarConfig, out: &mut Vec<String>) {
        match v {
            Value::String(s) => {
                let mut rest = s.as_str();
                while let Some(open) = rest.find("{{") {
                    let after = &rest[open + 2..];
                    let Some(close) = after.find("}}") else {
                        break;
                    };
                    if let Some(key) = after[..close].trim().strip_prefix("data.") {
                        if !config.data.contains_key(key) {
                            out.push(format!("{what}: unknown data source '{key}'"));
                        }
                    }
                    rest = &after[close + 2..];
                }
            }
            Value::Sequence(items) => items.iter().for_each(|i| scan(what, i, config, out)),
            Value::Mapping(m) => m.iter().for_each(|(_, i)| scan(what, i, config, out)),
            Value::Tagged(t) => scan(what, &t.value, config, out),
            _ => {}
        }
    }
    let mut out = Vec::new();
    for (name, res) in &config.resources {
        match serde_yaml_ng::to_value(res) {
            Ok(v) => scan(&format!("resource '{name}'"), &v, config, &mut out),
            Err(e) => out.push(format!("resource '{name}': cannot be scanned: {e}")),
        }
    }
    for (key, v) in &config.params {
        scan(&format!("param '{key}'"), v, config, &mut out);
    }
    for (key, o) in &config.outputs {
        let v = Value::String(o.value.clone());
        scan(&format!("output '{key}'"), &v, config, &mut out);
    }
    out
}

/// `Err` naming the first unknown `{{data.KEY}}`, `Ok` if every reference is declared.
pub(crate) fn require_known_data_refs(config: &ForjarConfig) -> Result<(), String> {
    match unknown_data_refs(config).first() {
        None => Ok(()),
        Some(first) => Err(first.clone()),
    }
}
