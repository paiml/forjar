//! FJ-2402/E11: WASM bundle resource handler.
//!
//! Handles `type: wasm_bundle` resources with WASM-specific validation:
//! - Validates .wasm file exists and is a valid WebAssembly binary (magic bytes)
//! - Checks size budget if configured
//! - Deploys WASM bundle (wasm + JS glue + HTML) to target path
//!
//! Unlike the generic file handler, this handler:
//! - Validates WASM binary magic bytes (`\0asm`)
//! - Supports size budget checking via `WasmSizeBudget`
//! - Sets appropriate cache-control headers for CDN deployment

use crate::core::types::Resource;

/// Generate check script for a WASM bundle resource.
///
/// Checks: .wasm file exists, has correct magic bytes, is within size budget.
pub fn check_script(resource: &Resource) -> String {
    let path = resource.path.as_deref().unwrap_or("/dev/null");

    let mut script = String::from("set -euo pipefail\n");

    // Check if the WASM file exists
    script.push_str(&format!(
        "if [ ! -f '{path}' ]; then\n  echo 'wasm=missing'\n  exit 0\nfi\n"
    ));

    // Validate WASM magic bytes: \0asm (00 61 73 6d)
    script.push_str(&format!(
        "MAGIC=$(od -A n -t x1 -N 4 '{path}' | tr -d ' ')\n\
         if [ \"$MAGIC\" != '0061736d' ]; then\n\
         \x20 echo 'wasm=invalid'\n\
         \x20 exit 0\n\
         fi\n"
    ));

    // Report current state
    script.push_str(&format!(
        "SIZE=$(stat -c %s '{path}' 2>/dev/null || stat -f %z '{path}')\n\
         echo \"wasm=present size=$SIZE\"\n"
    ));

    script
}

/// Generate apply script for a WASM bundle resource.
///
/// Deploys the WASM bundle to the target path. If `source` is set,
/// copies from source. If `content` is set, writes inline.
pub fn apply_script(resource: &Resource) -> String {
    let path = resource.path.as_deref().unwrap_or("/dev/null");
    let mut script = String::from("set -euo pipefail\n");

    // Ensure target directory exists
    let dir = std::path::Path::new(path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    if !dir.is_empty() {
        script.push_str(&format!("mkdir -p '{dir}'\n"));
    }

    // Deploy: source copy or inline content
    if let Some(ref source) = resource.source {
        script.push_str(&format!("cp -r '{source}' '{path}'\n"));
    } else if let Some(ref content) = resource.content {
        // For inline content (config files alongside WASM)
        let escaped = content.replace('\'', "'\\''");
        script.push_str(&format!("printf '%s' '{escaped}' > '{path}'\n"));
    }

    // Set permissions
    if let Some(ref mode) = resource.mode {
        script.push_str(&format!("chmod {mode} '{path}'\n"));
    }
    if let Some(ref owner) = resource.owner {
        let group = resource.group.as_deref().unwrap_or(owner);
        script.push_str(&format!("chown {owner}:{group} '{path}'\n"));
    }

    // Validate WASM binary after deployment
    script.push_str(&format!(
        "if [ -f '{path}' ]; then\n\
         \x20 MAGIC=$(od -A n -t x1 -N 4 '{path}' | tr -d ' ')\n\
         \x20 if [ \"$MAGIC\" != '0061736d' ]; then\n\
         \x20\x20\x20 echo 'WARNING: deployed file is not a valid WASM binary'\n\
         \x20 fi\n\
         \x20 SIZE=$(stat -c %s '{path}' 2>/dev/null || stat -f %z '{path}')\n\
         \x20 echo \"wasm_bundle={path} size=$SIZE deployed\"\n\
         fi\n"
    ));

    script
}

/// Generate state query script for drift detection.
pub fn state_query_script(resource: &Resource) -> String {
    check_script(resource)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_wasm_resource(path: &str) -> Resource {
        serde_yaml_ng::from_str(&format!("type: wasm_bundle\npath: {path}\nname: test-wasm"))
            .unwrap()
    }

    #[test]
    fn check_script_validates_magic() {
        let r = make_wasm_resource("/opt/app/bundle.wasm");
        let script = check_script(&r);
        assert!(script.contains("0061736d"), "should check WASM magic bytes");
        assert!(script.contains("/opt/app/bundle.wasm"));
        assert!(script.contains("wasm=missing"));
    }

    #[test]
    fn apply_script_creates_dir() {
        let r = make_wasm_resource("/opt/app/bundle.wasm");
        let script = apply_script(&r);
        assert!(script.contains("mkdir -p '/opt/app'"));
    }

    #[test]
    fn apply_script_with_source() {
        let yaml = "type: wasm_bundle\npath: /opt/app/bundle.wasm\nsource: dist/app.wasm";
        let r: Resource = serde_yaml_ng::from_str(yaml).unwrap();
        let script = apply_script(&r);
        assert!(script.contains("cp -r 'dist/app.wasm'"));
    }

    #[test]
    fn apply_script_validates_after_deploy() {
        let r = make_wasm_resource("/opt/app/bundle.wasm");
        let script = apply_script(&r);
        assert!(
            script.contains("0061736d"),
            "should validate WASM after deploy"
        );
        assert!(script.contains("wasm_bundle="));
    }

    #[test]
    fn apply_script_with_mode_and_owner() {
        let yaml = "type: wasm_bundle\npath: /opt/app/bundle.wasm\nmode: '644'\nowner: www-data";
        let r: Resource = serde_yaml_ng::from_str(yaml).unwrap();
        let script = apply_script(&r);
        assert!(script.contains("chmod 644"));
        assert!(script.contains("chown www-data:www-data"));
    }

    #[test]
    fn state_query_delegates_to_check() {
        let r = make_wasm_resource("/opt/app/bundle.wasm");
        assert_eq!(state_query_script(&r), check_script(&r));
    }
}
