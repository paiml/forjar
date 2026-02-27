use crate::core::types::*;

fn resolve_file_source(key: &str, source: &DataSource) -> Result<String, String> {
    std::fs::read_to_string(&source.value)
        .map(|s| s.trim().to_string())
        .or_else(|e| {
            source
                .default
                .clone()
                .ok_or_else(|| format!("data source '{}' file error: {}", key, e))
        })
}

fn resolve_command_source(key: &str, source: &DataSource) -> Result<String, String> {
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(&source.value)
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
    let addr_str = format!("{}:0", source.value);
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

/// FJ-223: Resolve all data sources and inject values into config params.
/// Data sources are stored with `__data__` prefix to avoid conflicts.
pub fn resolve_data_sources(config: &mut ForjarConfig) -> Result<(), String> {
    for (key, source) in &config.data {
        let value = match source.source_type {
            DataSourceType::File => resolve_file_source(key, source),
            DataSourceType::Command => resolve_command_source(key, source),
            DataSourceType::Dns => resolve_dns_source(key, source),
        }?;

        config.params.insert(
            format!("__data__{}", key),
            serde_yaml_ng::Value::String(value),
        );
    }
    Ok(())
}
