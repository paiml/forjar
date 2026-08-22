use crate::core::types::*;

/// Get the value field or return an error for data sources that require it.
fn require_value<'a>(key: &str, source: &'a DataSource) -> Result<&'a str, String> {
    source
        .value
        .as_deref()
        .ok_or_else(|| format!("data source '{key}' requires 'value' field"))
}

fn resolve_file_source(key: &str, source: &DataSource) -> Result<String, String> {
    let path = require_value(key, source)?;
    std::fs::read_to_string(path)
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
fn resolve_forjar_state_source(key: &str, source: &DataSource) -> Result<String, String> {
    let state_dir = source.state_dir.as_deref().unwrap_or("state");
    let lock_path = std::path::Path::new(state_dir).join("forjar.lock.yaml");

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

/// FJ-223: Resolve all data sources and inject values into config params.
/// Data sources are stored with `__data__` prefix to avoid conflicts.
pub fn resolve_data_sources(config: &mut ForjarConfig) -> Result<(), String> {
    for (key, source) in &config.data {
        let value = match source.source_type {
            DataSourceType::File => resolve_file_source(key, source),
            DataSourceType::Command => resolve_command_source(key, source),
            DataSourceType::Dns => resolve_dns_source(key, source),
            DataSourceType::ForjarState => resolve_forjar_state_source(key, source),
        }?;

        config.params.insert(
            format!("__data__{key}"),
            serde_yaml_ng::Value::String(value),
        );
    }
    Ok(())
}
