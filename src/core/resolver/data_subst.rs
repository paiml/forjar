//! forjar#660: write resolved `{{data.x}}` values into the loaded config.
//!
//! The template resolver answers `{{data.x}}` when it renders a resource, so the
//! render path (`show`, `plan`, `apply`) always saw the value. The readers that
//! never render (`pin`, `sbom`, `compare`, `-t`/`-g` filtering, the diagnostics)
//! read `version`, `tags`, `resource_group` and every other string straight off
//! the config, and saw `{{data.x}}` as text.
//!
//! Substitution is a GENERIC walk over the serialized config: every string of
//! every section, whatever its field is called, with one exception, the `data:`
//! block itself (its `value`s are commands, not references). There is no list of
//! fields to forget one of; a field added to a section tomorrow is covered
//! today, and `unknown_data_refs` walks the same tree.
//!
//! A data value is OPAQUE text. Substitution is one pass: the result is built by
//! copying the template and appending each value, and the scan resumes AFTER the
//! value, so a value is never scanned and cannot combine with the text beside it.
//!
//! There are two forms of the loaded config.
//!   READER (`load_resolved_config`, `_lenient`): values are written as plain text.
//!   Nothing renders this config, so the returned config holds no stand-in. (The
//!   `outputs:` pass is a template pass: reader form holds the braces back for it
//!   and decodes before returning, in one place, `finish_resolving`.)
//!   RENDER (`load_resolved_config_for_render`, `_with`): the TEMPLATE resolver
//!   runs later over the config and would scan a written value a second time, so a
//!   value is written with EVERY `{` and `}` replaced by a stand-in character
//!   ([`OPEN`], [`CLOSE`]); no reference can be formed from stand-ins, whatever the
//!   value or its neighbours hold. The resolver restores them as its last step, and
//!   a render-form caller that prints the config calls [`decode_literals`].
//! The stand-ins are reserved: while a manifest declares `data:`, one in a value
//! or in manifest text is an error, so decoding is exact.
//!
//! A `{{data.x}}` in a mapping KEY is rejected, not resolved: a key is a name
//! (a resource id, a param name) that other entries refer to, and renaming it
//! from a command's output would change what those references mean.

use crate::core::types::{DataValues, ForjarConfig};
use serde_yaml_ng::Value;

/// Stands for a `{` that came from a data value.
pub(crate) const OPEN: char = '\u{E001}';
/// Stands for a `}` that came from a data value.
pub(crate) const CLOSE: char = '\u{E002}';

/// The top-level section that is never substituted: data sources are commands.
const SKIPPED_SECTION: &str = "data";

fn encode_value(v: &str, encode: bool) -> String {
    if !encode {
        return v.to_string();
    }
    v.chars()
        .map(|c| match c {
            '{' => OPEN,
            '}' => CLOSE,
            c => c,
        })
        .collect()
}

/// Turn every stand-in back into its brace.
pub(crate) fn decode_str(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            OPEN => '{',
            CLOSE => '}',
            c => c,
        })
        .collect()
}

fn has_reserved(s: &str) -> bool {
    s.contains(OPEN) || s.contains(CLOSE)
}

/// Replace `{{data.KEY}}` (whitespace inside the braces allowed) in one string.
/// Unknown keys are left as they are (they are an error elsewhere).
fn substitute_str(s: &str, data: &DataValues, encode: bool) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(open) = rest.find("{{") {
        let after = &rest[open + 2..];
        let Some(close) = after.find("}}") else {
            break;
        };
        let key = after[..close].trim();
        out.push_str(&rest[..open]);
        match key.strip_prefix("data.").and_then(|k| data.get(k)) {
            Some(v) => out.push_str(&encode_value(v, encode)),
            None => out.push_str(&rest[open..open + 2 + close + 2]),
        }
        rest = &after[close + 2..];
    }
    out.push_str(rest);
    out
}

/// Apply `f` to every string VALUE below `v` (mapping keys are names, not text).
fn map_strings(v: &mut Value, f: &dyn Fn(&str) -> String) {
    match v {
        Value::String(s) => *s = f(s),
        Value::Sequence(items) => items.iter_mut().for_each(|i| map_strings(i, f)),
        Value::Mapping(m) => m.iter_mut().for_each(|(_, i)| map_strings(i, f)),
        Value::Tagged(t) => map_strings(&mut t.value, f),
        _ => {}
    }
}

/// Run `f` over every string of the config except the `data:` block, by a
/// round trip through the generic value tree. The `#[serde(skip)]` fields do not
/// survive the trip, so they are carried over by hand.
fn rewrite_strings(config: &mut ForjarConfig, f: &dyn Fn(&str) -> String) -> Result<(), String> {
    let mut tree = serde_yaml_ng::to_value(&*config).map_err(|e| format!("config: {e}"))?;
    if let Value::Mapping(top) = &mut tree {
        for (k, section) in top.iter_mut() {
            if k.as_str() != Some(SKIPPED_SECTION) {
                map_strings(section, f);
            }
        }
    }
    let mut next: ForjarConfig =
        serde_yaml_ng::from_value(tree).map_err(|e| format!("config: {e}"))?;
    next.include_provenance = std::mem::take(&mut config.include_provenance);
    next.data_values = std::mem::take(&mut config.data_values);
    *config = next;
    Ok(())
}

/// Substitute resolved data values into EVERY string of the config.
/// `outputs` are resolved separately, once, by `state::resolve_outputs`.
pub(super) fn substitute_data_refs(config: &mut ForjarConfig, encode: bool) -> Result<(), String> {
    if config.data_values.is_empty() {
        return Ok(());
    }
    let data = config.data_values.clone();
    if let Some((k, _)) = data.iter().find(|(_, v)| has_reserved(v)) {
        return Err(reserved_msg(&format!("the value of data source '{k}'")));
    }
    rewrite_strings(config, &|s| {
        if s.contains("{{") {
            substitute_str(s, &data, encode)
        } else {
            s.to_string()
        }
    })
}

/// Turn the stand-ins in every string back into braces, for a reader that prints
/// or hashes the config instead of rendering it. With no resolved data nothing
/// was encoded, so nothing is touched.
pub fn decode_literals(config: &mut ForjarConfig) -> Result<(), String> {
    if config.data_values.is_empty() {
        return Ok(());
    }
    rewrite_strings(config, &|s| decode_str(s))
}

/// A readable location for a finding: `resource 'cfg'`, `param 'p'`,
/// `output 'o'`, otherwise the dotted path (`machines.m1.hostname`).
fn label(path: &[String]) -> String {
    match path {
        [sec, name, ..] if sec == "resources" => format!("resource '{name}'"),
        [sec, name] if sec == "params" => format!("param '{name}'"),
        [sec, name, ..] if sec == "params" => format!("param '{name}'"),
        [sec, name, ..] if sec == "outputs" => format!("output '{name}'"),
        _ => path.join("."),
    }
}

fn reserved_msg(at: &str) -> String {
    format!(
        "{at} contains U+E001 or U+E002, which forjar reserves (reserved characters) while a manifest declares `data:`"
    )
}

/// A `{{data.x}}` in a mapping key.
fn key_ref(k: &str) -> bool {
    k.contains("{{") && k.contains("data.")
}

fn scan(path: &mut Vec<String>, v: &Value, config: &ForjarConfig, out: &mut Vec<String>) {
    match v {
        Value::String(s) => {
            if !config.data.is_empty() && has_reserved(s) {
                out.push(reserved_msg(&label(path)));
            }
            let mut rest = s.as_str();
            while let Some(open) = rest.find("{{") {
                let after = &rest[open + 2..];
                let Some(close) = after.find("}}") else {
                    break;
                };
                if let Some(key) = after[..close].trim().strip_prefix("data.") {
                    if !config.data.contains_key(key) {
                        out.push(format!("{}: unknown data source '{key}'", label(path)));
                    }
                }
                rest = &after[close + 2..];
            }
        }
        Value::Sequence(items) => {
            for (i, item) in items.iter().enumerate() {
                path.push(i.to_string());
                scan(path, item, config, out);
                path.pop();
            }
        }
        Value::Mapping(m) => {
            for (k, item) in m {
                if let Some(name) = k.as_str().filter(|n| key_ref(n)) {
                    out.push(format!(
                        "{}: a data reference cannot be a mapping key ('{name}'); a key is a name other entries refer to, so it is not resolved",
                        path.join(".")
                    ));
                }
                path.push(k.as_str().unwrap_or("?").to_string());
                scan(path, item, config, out);
                path.pop();
            }
        }
        Value::Tagged(t) => scan(path, &t.value, config, out),
        _ => {}
    }
}

/// Every `{{data.KEY}}` ANYWHERE in the manifest whose KEY no `data:` source
/// declares.
///
/// `{{params.KEY}}` with no such param is an error; so is this, in every
/// command, because the loader calls it (forjar#660). The walk is over the whole
/// serialized config, not a list of fields. Run BEFORE substitution, so a data
/// VALUE that happens to contain `{{data.x}}` text is not mistaken for a
/// reference.
pub(crate) fn unknown_data_refs(config: &ForjarConfig) -> Vec<String> {
    let tree = match serde_yaml_ng::to_value(config) {
        Ok(t) => t,
        Err(e) => return vec![format!("config cannot be scanned: {e}")],
    };
    let mut out = Vec::new();
    if let Value::Mapping(top) = &tree {
        for (k, section) in top {
            let name = k.as_str().unwrap_or("?");
            if name != SKIPPED_SECTION {
                scan(&mut vec![name.to_string()], section, config, &mut out);
            }
        }
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
