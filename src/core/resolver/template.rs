use crate::core::{secrets, types::*};
use std::borrow::Cow;
use std::collections::HashMap;

/// FJ-2300: Resolve a secret value using the configured provider.
///
/// Default (env): `FORJAR_SECRET_<KEY>` (uppercase, hyphens → underscores).
/// File provider: reads `/run/secrets/<key>` (or configured path prefix).
pub(super) fn resolve_secret(key: &str) -> Result<String, String> {
    resolve_secret_with_provider(key, None, None)
}

/// Resolve secret with explicit provider config.
pub fn resolve_secret_with_provider(
    key: &str,
    provider: Option<&str>,
    path_prefix: Option<&str>,
) -> Result<String, String> {
    match provider.unwrap_or("env") {
        "file" => {
            let prefix = path_prefix.unwrap_or("/run/secrets");
            let path = std::path::Path::new(prefix).join(key);
            std::fs::read_to_string(&path)
                .map(|s| s.trim_end().to_string())
                .map_err(|e| format!("secret '{key}' not found at {}: {e}", path.display()))
        }
        _ => {
            // Default: env provider
            let env_key = format!("FORJAR_SECRET_{}", key.to_uppercase().replace('-', "_"));
            std::env::var(&env_key).map_err(|_| {
                format!("secret '{key}' not found (set env var {env_key} or use a secrets file)")
            })
        }
    }
}

/// FJ-2300: Redact secret values from a string.
///
/// Replaces all occurrences of secret values with `***`.
pub fn redact_secrets(text: &str, secret_values: &[String]) -> String {
    let mut result = text.to_string();
    for secret in secret_values {
        if !secret.is_empty() {
            result = result.replace(secret.as_str(), "***");
        }
    }
    result
}

/// Resolve a single template variable key to its value.
fn resolve_variable<'a>(
    key: &str,
    params: &HashMap<String, serde_yaml_ng::Value>,
    machines: &'a indexmap::IndexMap<String, Machine>,
) -> Result<Cow<'a, str>, String> {
    if let Some(param_key) = key.strip_prefix("params.") {
        return Ok(Cow::Owned(
            params
                .get(param_key)
                .map(yaml_value_to_string)
                .ok_or_else(|| format!("unknown param: {param_key}"))?,
        ));
    }
    if let Some(secret_key) = key.strip_prefix("secrets.") {
        return Ok(Cow::Owned(resolve_secret(secret_key)?));
    }
    if key.starts_with("machine.") {
        return resolve_machine_ref(key, machines);
    }
    if let Some(data_key) = key.strip_prefix("data.") {
        return Ok(Cow::Owned(
            params
                .get(&format!("__data__{data_key}"))
                .map(yaml_value_to_string)
                .ok_or_else(|| format!("unknown data source: {data_key}"))?,
        ));
    }
    if key.contains('(') {
        return Ok(Cow::Owned(super::functions::resolve_function(
            key, params, machines,
        )?));
    }
    Err(format!("unknown template variable: {key}"))
}

/// Resolve a machine.NAME.FIELD reference.
fn resolve_machine_ref<'a>(
    key: &str,
    machines: &'a indexmap::IndexMap<String, Machine>,
) -> Result<Cow<'a, str>, String> {
    let parts: Vec<&str> = key.splitn(3, '.').collect();
    if parts.len() != 3 {
        return Err(format!("invalid machine ref: {key}"));
    }
    let machine = machines
        .get(parts[1])
        .ok_or_else(|| format!("unknown machine: {}", parts[1]))?;
    match parts[2] {
        "addr" => Ok(Cow::Borrowed(&machine.addr)),
        "hostname" => Ok(Cow::Borrowed(&machine.hostname)),
        "user" => Ok(Cow::Borrowed(&machine.user)),
        "arch" => Ok(Cow::Borrowed(&machine.arch)),
        _ => Err(format!("unknown machine field: {}", parts[2])),
    }
}

/// Resolve all template variables in a string.
pub fn resolve_template(
    template: &str,
    params: &HashMap<String, serde_yaml_ng::Value>,
    machines: &indexmap::IndexMap<String, Machine>,
) -> Result<String, String> {
    let mut result = template.to_string();
    let mut start = 0;

    while let Some(open) = result[start..].find("{{") {
        let open = start + open;
        let close = result[open..]
            .find("}}")
            .ok_or_else(|| format!("unclosed template at position {open}"))?;
        let close = open + close + 2;
        let key = result[open + 2..close - 2].trim();

        let value = resolve_variable(key, params, machines)?;
        result.replace_range(open..close, &value);
        start = open + value.len();
    }

    // FJ-200: Decrypt any ENC[age,...] markers after template resolution
    #[cfg(feature = "encryption")]
    if secrets::has_encrypted_markers(&result) {
        let identities = secrets::load_identities(None)?;
        result = secrets::decrypt_all(&result, &identities)?;
    }
    #[cfg(not(feature = "encryption"))]
    if secrets::has_encrypted_markers(&result) {
        return Err("ENC[age,...] markers found but forjar was compiled without encryption support. Rebuild with `--features encryption`.".to_string());
    }

    Ok(result)
}
