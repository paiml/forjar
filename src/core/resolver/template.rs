use crate::core::{secrets, types::*};
use std::borrow::Cow;
use std::collections::HashMap;

/// Resolve a secret value from environment variables.
///
/// Looks for `FORJAR_SECRET_<KEY>` (uppercase, hyphens become underscores).
/// Example: `{{secrets.db-password}}` resolves from `FORJAR_SECRET_DB_PASSWORD`.
pub(super) fn resolve_secret(key: &str) -> Result<String, String> {
    let env_key = format!("FORJAR_SECRET_{}", key.to_uppercase().replace('-', "_"));
    std::env::var(&env_key).map_err(|_| {
        format!(
            "secret '{}' not found (set env var {} or use a secrets file)",
            key, env_key
        )
    })
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
            .ok_or_else(|| format!("unclosed template at position {}", open))?;
        let close = open + close + 2;
        let key = result[open + 2..close - 2].trim();

        let value: Cow<str> = if let Some(param_key) = key.strip_prefix("params.") {
            Cow::Owned(
                params
                    .get(param_key)
                    .map(yaml_value_to_string)
                    .ok_or_else(|| format!("unknown param: {}", param_key))?,
            )
        } else if let Some(secret_key) = key.strip_prefix("secrets.") {
            Cow::Owned(resolve_secret(secret_key)?)
        } else if key.starts_with("machine.") {
            let parts: Vec<&str> = key.splitn(3, '.').collect();
            if parts.len() != 3 {
                return Err(format!("invalid machine ref: {}", key));
            }
            let machine = machines
                .get(parts[1])
                .ok_or_else(|| format!("unknown machine: {}", parts[1]))?;
            Cow::Borrowed(match parts[2] {
                "addr" => &machine.addr,
                "hostname" => &machine.hostname,
                "user" => &machine.user,
                "arch" => &machine.arch,
                _ => return Err(format!("unknown machine field: {}", parts[2])),
            })
        } else if let Some(data_key) = key.strip_prefix("data.") {
            Cow::Owned(
                params
                    .get(&format!("__data__{}", data_key))
                    .map(yaml_value_to_string)
                    .ok_or_else(|| format!("unknown data source: {}", data_key))?,
            )
        } else if key.contains('(') {
            // FJ-250: Template function call — e.g., upper(params.name)
            Cow::Owned(super::functions::resolve_function(key, params, machines)?)
        } else {
            return Err(format!("unknown template variable: {}", key));
        };

        result.replace_range(open..close, &value);
        start = open + value.len();
    }

    // FJ-200: Decrypt any ENC[age,...] markers after template resolution
    if secrets::has_encrypted_markers(&result) {
        let identities = secrets::load_identities(None)?;
        result = secrets::decrypt_all(&result, &identities)?;
    }

    Ok(result)
}
