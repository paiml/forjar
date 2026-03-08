use crate::core::{secrets, types::*};
use std::borrow::Cow;
use std::collections::HashMap;

/// FJ-2300: Resolve a secret value using the configured provider.
///
/// Providers:
/// - `env` (default): `FORJAR_SECRET_<KEY>` (uppercase, hyphens → underscores)
/// - `file`: reads `<path>/<key>` (default path: `/run/secrets/`)
/// - `sops`: runs `sops -d --extract '["<key>"]' <file>` to decrypt
/// - `op`: runs `op read "op://forjar/<key>"` (or custom vault via path)
pub(super) fn resolve_secret(key: &str, secrets_cfg: &SecretsConfig) -> Result<String, String> {
    resolve_secret_with_provider(
        key,
        secrets_cfg.provider.as_deref(),
        secrets_cfg.path.as_deref(),
        secrets_cfg.file.as_deref(),
    )
}

/// Resolve secret with explicit provider config.
pub fn resolve_secret_with_provider(
    key: &str,
    provider: Option<&str>,
    path_prefix: Option<&str>,
    sops_file: Option<&str>,
) -> Result<String, String> {
    match provider.unwrap_or("env") {
        "file" => resolve_secret_file(key, path_prefix),
        "sops" => resolve_secret_sops(key, sops_file),
        "op" => resolve_secret_op(key, path_prefix),
        _ => resolve_secret_env(key),
    }
}

/// Resolve from environment variable `FORJAR_SECRET_<KEY>`.
fn resolve_secret_env(key: &str) -> Result<String, String> {
    let env_key = format!("FORJAR_SECRET_{}", key.to_uppercase().replace('-', "_"));
    std::env::var(&env_key).map_err(|_| {
        format!("secret '{key}' not found (set env var {env_key} or use a secrets file)")
    })
}

/// Resolve from a file at `<prefix>/<key>`.
fn resolve_secret_file(key: &str, path_prefix: Option<&str>) -> Result<String, String> {
    let prefix = path_prefix.unwrap_or("/run/secrets");
    let path = std::path::Path::new(prefix).join(key);
    std::fs::read_to_string(&path)
        .map(|s| s.trim_end().to_string())
        .map_err(|e| format!("secret '{key}' not found at {}: {e}", path.display()))
}

/// Resolve via `sops -d --extract '["<key>"]' <file>`.
fn resolve_secret_sops(key: &str, sops_file: Option<&str>) -> Result<String, String> {
    let file = sops_file.unwrap_or("secrets.enc.yaml");
    let output = std::process::Command::new("sops")
        .args(["-d", "--extract", &format!("[\"{key}\"]"), file])
        .output()
        .map_err(|e| format!("sops: failed to execute: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("sops: decrypt '{key}' from {file}: {stderr}"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Resolve via 1Password CLI: `op read "op://<vault>/<key>"`.
fn resolve_secret_op(key: &str, vault: Option<&str>) -> Result<String, String> {
    let vault = vault.unwrap_or("forjar");
    let ref_path = format!("op://{vault}/{key}");
    let output = std::process::Command::new("op")
        .args(["read", &ref_path])
        .output()
        .map_err(|e| format!("op: failed to execute: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("op: read '{ref_path}': {stderr}"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
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
    secrets_cfg: &SecretsConfig,
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
        return Ok(Cow::Owned(resolve_secret(secret_key, secrets_cfg)?));
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
    resolve_template_with_secrets(template, params, machines, &SecretsConfig::default())
}

/// Resolve all template variables with explicit secrets configuration.
pub fn resolve_template_with_secrets(
    template: &str,
    params: &HashMap<String, serde_yaml_ng::Value>,
    machines: &indexmap::IndexMap<String, Machine>,
    secrets_cfg: &SecretsConfig,
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

        let value = resolve_variable(key, params, machines, secrets_cfg)?;
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
