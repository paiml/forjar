use crate::core::types::*;

/// Get the value field or return an error for data sources that require it.
fn require_value<'a>(key: &str, source: &'a DataSource) -> Result<&'a str, String> {
    source
        .value
        .as_deref()
        .ok_or_else(|| format!("data source '{}' requires 'value' field", key))
}

fn resolve_file_source(key: &str, source: &DataSource) -> Result<String, String> {
    let path = require_value(key, source)?;
    std::fs::read_to_string(path)
        .map(|s| s.trim().to_string())
        .or_else(|e| {
            source
                .default
                .clone()
                .ok_or_else(|| format!("data source '{}' file error: {}", key, e))
        })
}

fn resolve_command_source(key: &str, source: &DataSource) -> Result<String, String> {
    let cmd = require_value(key, source)?;
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output()
        .map_err(|e| format!("data source '{}' command error: {}", key, e))?;
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
    let addr_str = format!("{}:0", host);
    match addr_str.to_socket_addrs() {
        Ok(mut addrs) => {
            if let Some(addr) = addrs.next() {
                Ok(addr.ip().to_string())
            } else {
                source
                    .default
                    .clone()
                    .ok_or_else(|| format!("data source '{}' DNS: no addresses", key))
            }
        }
        Err(e) => source
            .default
            .clone()
            .ok_or_else(|| format!("data source '{}' DNS error: {}", key, e)),
    }
}

/// FJ-1250: Resolve forjar-state data source (read outputs from another config's state).
fn resolve_forjar_state_source(_key: &str, source: &DataSource) -> Result<String, String> {
    // forjar-state sources resolve to a placeholder until apply-time
    // The actual resolution requires reading another config's state directory
    let config_name = source
        .config
        .as_deref()
        .unwrap_or("unknown");
    Ok(format!("{{{{data.{}}}}}", config_name))
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
            format!("__data__{}", key),
            serde_yaml_ng::Value::String(value),
        );
    }
    Ok(())
}
