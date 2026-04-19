#![allow(clippy::field_reassign_with_default)]
//! FJ-005/200: Codegen dispatch and secrets marker falsification.
//!
//! Popperian rejection criteria for:
//! - FJ-005: Script generation dispatch
//!   - check_script for all resource types (Package, File, Service, etc.)
//!   - apply_script for all resource types
//!   - state_query_script for all resource types
//!   - Recipe type returns error (must expand first)
//!   - Scripts produce non-empty output
//!   - sudo wrapping (resource.sudo = true)
//! - FJ-200: Age-encrypted secrets
//!   - has_encrypted_markers: real vs fake markers
//!   - find_markers: position tracking
//!   - ENC_PREFIX/ENC_SUFFIX constants
//!   - decrypt_all_inline without encryption feature
//!
//! Usage: cargo test --test falsification_codegen_secrets

use forjar::core::codegen::{apply_script, check_script, state_query_script};
use forjar::core::secrets;
use forjar::core::types::{Resource, ResourceType};

// ============================================================================
// Helpers
// ============================================================================

fn resource_of_type(rt: ResourceType) -> Resource {
    let mut r = Resource::default();
    r.resource_type = rt;
    r
}

fn file_resource() -> Resource {
    let mut r = resource_of_type(ResourceType::File);
    r.path = Some("/etc/test.conf".into());
    r.content = Some("hello world".into());
    r
}

fn package_resource() -> Resource {
    let mut r = resource_of_type(ResourceType::Package);
    r.packages = vec!["nginx".into()];
    r
}

fn service_resource() -> Resource {
    let mut r = resource_of_type(ResourceType::Service);
    r.name = Some("nginx".into());
    r
}

// ============================================================================
// FJ-005: check_script — all types produce output
// ============================================================================

#[test]
fn check_script_package() {
    let r = package_resource();
    let script = check_script(&r).unwrap();
    assert!(!script.is_empty());
}

#[test]
fn check_script_file() {
    let r = file_resource();
    let script = check_script(&r).unwrap();
    assert!(!script.is_empty());
}

#[test]
fn check_script_service() {
    let r = service_resource();
    let script = check_script(&r).unwrap();
    assert!(!script.is_empty());
}

#[test]
fn check_script_mount() {
    let mut r = resource_of_type(ResourceType::Mount);
    r.path = Some("/mnt/data".into());
    let script = check_script(&r).unwrap();
    assert!(!script.is_empty());
}

#[test]
fn check_script_user() {
    let mut r = resource_of_type(ResourceType::User);
    r.name = Some("deploy".into());
    let script = check_script(&r).unwrap();
    assert!(!script.is_empty());
}

#[test]
fn check_script_docker() {
    let r = resource_of_type(ResourceType::Docker);
    let script = check_script(&r).unwrap();
    assert!(!script.is_empty());
}

#[test]
fn check_script_cron() {
    let mut r = resource_of_type(ResourceType::Cron);
    r.content = Some("0 * * * * echo hello".into());
    let script = check_script(&r).unwrap();
    assert!(!script.is_empty());
}

#[test]
fn check_script_network() {
    let r = resource_of_type(ResourceType::Network);
    let script = check_script(&r).unwrap();
    assert!(!script.is_empty());
}

#[test]
fn check_script_recipe_errors() {
    let r = resource_of_type(ResourceType::Recipe);
    let result = check_script(&r);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("recipe"));
}

// ============================================================================
// FJ-005: apply_script — types produce output
// ============================================================================

#[test]
fn apply_script_package() {
    let r = package_resource();
    let script = apply_script(&r).unwrap();
    assert!(!script.is_empty());
    assert!(script.contains("nginx"));
}

#[test]
fn apply_script_file() {
    let r = file_resource();
    let script = apply_script(&r).unwrap();
    assert!(!script.is_empty());
    assert!(script.contains("/etc/test.conf") || script.contains("hello world"));
}

#[test]
fn apply_script_service() {
    let r = service_resource();
    let script = apply_script(&r).unwrap();
    assert!(!script.is_empty());
}

#[test]
fn apply_script_recipe_errors() {
    let r = resource_of_type(ResourceType::Recipe);
    let result = apply_script(&r);
    assert!(result.is_err());
}

// ============================================================================
// FJ-005: apply_script — sudo wrapping
// ============================================================================

#[test]
fn apply_script_sudo_wraps() {
    let mut r = file_resource();
    r.sudo = true;
    let script = apply_script(&r).unwrap();
    assert!(
        script.contains("sudo") || script.contains("FORJAR_SUDO"),
        "sudo resource should have elevated wrapper"
    );
}

#[test]
fn apply_script_no_sudo() {
    let mut r = file_resource();
    r.sudo = false;
    let script = apply_script(&r).unwrap();
    assert!(
        !script.contains("FORJAR_SUDO"),
        "non-sudo resource should not have sudo wrapper"
    );
}

// ============================================================================
// FJ-005: state_query_script — types produce output
// ============================================================================

#[test]
fn state_query_package() {
    let r = package_resource();
    let script = state_query_script(&r).unwrap();
    assert!(!script.is_empty());
}

#[test]
fn state_query_file() {
    let r = file_resource();
    let script = state_query_script(&r).unwrap();
    assert!(!script.is_empty());
}

#[test]
fn state_query_service() {
    let r = service_resource();
    let script = state_query_script(&r).unwrap();
    assert!(!script.is_empty());
}

#[test]
fn state_query_recipe_errors() {
    let r = resource_of_type(ResourceType::Recipe);
    let result = state_query_script(&r);
    assert!(result.is_err());
}

// ============================================================================
// FJ-200: has_encrypted_markers — real markers
// ============================================================================

#[test]
fn has_markers_real_base64() {
    // A valid base64 string long enough to be real age ciphertext
    let marker = "ENC[age,YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBGY0V3SGtMcjhmbQ==]";
    assert!(
        secrets::has_encrypted_markers(marker),
        "should detect real base64 marker"
    );
}

#[test]
fn has_markers_short_base64_rejected() {
    // Too short to be real ciphertext (< 20 chars)
    let marker = "ENC[age,abc]";
    assert!(
        !secrets::has_encrypted_markers(marker),
        "short marker should be rejected"
    );
}

#[test]
fn has_markers_no_markers() {
    assert!(!secrets::has_encrypted_markers("just plain text"));
    assert!(!secrets::has_encrypted_markers("password: mysecret"));
}

#[test]
fn has_markers_invalid_base64() {
    // Invalid base64 characters
    let marker = "ENC[age,!!!not_valid_base64_at_all!!!]";
    assert!(
        !secrets::has_encrypted_markers(marker),
        "invalid base64 should be rejected"
    );
}

#[test]
fn has_markers_comment_example() {
    // Comments mentioning ENC[age,...] should not trigger
    let text = "# values use ENC[age,...] markers";
    assert!(
        !secrets::has_encrypted_markers(text),
        "comment examples should not trigger"
    );
}

// ============================================================================
// FJ-200: decrypt_all_inline (no encryption feature)
// `decrypt_all_inline` only exists when encryption is OFF — see secrets.rs:204.
// When `--features encryption` is set, the real impl is `decrypt_all_counted`.
// ============================================================================

#[cfg(not(feature = "encryption"))]
#[test]
fn decrypt_inline_no_markers_passthrough() {
    let input = "just plain text with no markers";
    let result = secrets::decrypt_all_inline(input).unwrap();
    assert_eq!(result, input);
}

#[cfg(not(feature = "encryption"))]
#[test]
fn decrypt_inline_with_real_markers_errors() {
    // Real base64 marker should error since encryption feature is not compiled
    let input = "password: ENC[age,YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBGY0V3SGtMcjhmbQ==]";
    let result = secrets::decrypt_all_inline(input);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("encryption"));
}

// ============================================================================
// FJ-200: Multiple markers in string
// ============================================================================

#[test]
fn has_markers_multiple() {
    let input = "a: ENC[age,YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBGY0V3SGtMcjhmbQ==] b: ENC[age,YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBGY0V3SGtMcjhmbQ==]";
    assert!(secrets::has_encrypted_markers(input));
}

// ============================================================================
// FJ-200: Mixed content
// ============================================================================

#[test]
fn has_markers_mixed_with_plain() {
    let input = "host: example.com\npassword: ENC[age,YWdlLWVuY3J5cHRpb24ub3JnL3YxCi0+IFgyNTUxOSBGY0V3SGtMcjhmbQ==]\nport: 5432";
    assert!(secrets::has_encrypted_markers(input));
}

#[test]
fn has_markers_prefix_only() {
    // Unclosed marker should not crash
    let input = "ENC[age,unclosed without bracket";
    assert!(!secrets::has_encrypted_markers(input));
}
