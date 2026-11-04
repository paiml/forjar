//! The `file` resource's content and mode assertions, split out of `file.rs`
//! to hold that file under its line ceiling.

use super::file::read_source_file;
use crate::core::shell_escape::sh_squote;
use crate::core::types::Resource;
use crate::resources::verdict;

pub(super) fn content(resource: &Resource, p: &str) -> Option<String> {
    let bytes = match declared_bytes(resource)? {
        Ok(bytes) => bytes,
        Err(e) => return Some(verdict::always_diverged(&format!("unreadable source: {e}"))),
    };
    let want = sha256_hex(&bytes);
    Some(verdict::assert_that(
        &format!(
            "forjar_sha=$( {{ sha256sum {p} 2>/dev/null || shasum -a 256 {p} 2>/dev/null; }} | cut -d' ' -f1)\n  [ \"$forjar_sha\" = '{want}' ]"
        ),
        "content:declared",
        "stale:content",
    ))
}

pub(super) fn mode(
    resource: &Resource,
    p: &str,
    octal_mode: fn(&str) -> Option<String>,
) -> Option<String> {
    let mode = resource.mode.as_deref().and_then(octal_mode)?;
    Some(verdict::assert_that(
        &format!(
            "forjar_mode=$(stat -c %a {p} 2>/dev/null || stat -f %Lp {p} 2>/dev/null)\n  [ \"$forjar_mode\" = {} ]",
            sh_squote(&mode)
        ),
        "mode:declared",
        "stale:mode",
    ))
}

/// The bytes the apply script would write, or `None` when the resource
/// declares no content (existence is then the whole declaration).
fn declared_bytes(resource: &Resource) -> Option<Result<Vec<u8>, String>> {
    if let Some(ref source) = resource.source {
        Some(read_source_file(source))
    } else {
        resource.content.as_ref().map(|c| Ok(c.as_bytes().to_vec()))
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}
