//! forjar#1049: the `github_release` check asserts the declared version and
//! checksum, not just that an executable exists.
//!
//! Before this, `check_script` tested `[ -x bin ]` and echoed `--version` as
//! text. Change `tag: v1.0.0` to `v2.0.0` and the check still read converged,
//! so nothing was re-downloaded. Now:
//!
//! * a tag that names a version (`v0.33.2`) must equal the first `N.N.N` the
//!   live `--version` prints, or the check diverges;
//! * `latest`, no tag, or a tag that names no version (`nightly`) is not
//!   compared: presence-only, exactly as before;
//! * `checksum:` (hex sha256 of the installed file, optional `sha256:` prefix)
//!   must match the file on disk, whatever the tag.

use crate::core::shell_escape::{is_valid_repo, sh_squote};
use crate::core::types::Resource;
use crate::resources::verdict;
use crate::tripwire::drift::version_pin::{declared_pin, Pin};

/// Generate shell script to check the binary from a GitHub release.
///
/// Checks: the binary exists and is executable; a versioned pin equals the
/// live version; a declared sha256 equals the file's sha256.
pub fn check_script(resource: &Resource, reject_bad_repo: fn(&str) -> String) -> String {
    let repo = resource.repo.as_deref().unwrap_or("unknown/unknown");
    if !is_valid_repo(repo) {
        return reject_bad_repo(repo);
    }
    let binary = resource.binary.as_deref().unwrap_or("unknown");
    let install_dir = resource.install_dir.as_deref().unwrap_or("/usr/local/bin");
    let bin_path = sh_squote(&format!("{install_dir}/{binary}"));

    verdict::check_script_from(&[format!(
        "if [ -x {bin_path} ]; then\n\
         \x20 VER=$( {bin_path} --version 2>/dev/null | head -1 || echo 'unknown' )\n\
         \x20 echo \"installed:{repo}:$VER\"\n\
         {}{}\
         else\n\
         \x20 echo 'missing:{repo}'; __fj_diverged=1\n\
         fi",
        version_assertion(resource, &bin_path, repo),
        checksum_assertion(resource, &bin_path, repo),
    )])
}

/// Diverge when the live version is not the pinned one. Empty for an unpinned
/// resource.
fn version_assertion(resource: &Resource, bin_path: &str, repo: &str) -> String {
    let Pin::Version(pin) = declared_pin(resource) else {
        return String::new();
    };
    let want = format!("{}.{}.{}", pin.major, pin.minor, pin.patch);
    format!(
        "\x20 FJ_LIVE=$( {bin_path} --version 2>&1 | head -n 20 | tr -d '\\0' | \
         LC_ALL=C grep -oE '(^|[^0-9A-Za-z._])[vV]?[0-9]+[.][0-9]+[.][0-9]+([^.0-9]|[.]([^0-9]|$)|$)' | \
         head -n 1 | grep -oE '[0-9]+[.][0-9]+[.][0-9]+' | sed -E 's/(^|[.])0+([0-9])/\\1\\2/g' || true )\n\
         \x20 if [ \"$FJ_LIVE\" != '{want}' ]; then\n\
         \x20\x20\x20 echo \"version-mismatch:{repo}:want {want}:have ${{FJ_LIVE:-unreadable}}\"; __fj_diverged=1\n\
         \x20 fi\n"
    )
}

/// Diverge when the installed file's sha256 differs from `checksum:`.
fn checksum_assertion(resource: &Resource, bin_path: &str, repo: &str) -> String {
    let Some(raw) = resource.checksum.as_deref() else {
        return String::new();
    };
    let lowered = raw.trim().to_ascii_lowercase();
    let want = lowered
        .strip_prefix("sha256:")
        .unwrap_or(&lowered)
        .to_string();
    // Hex only: the value is interpolated into the script.
    if want.len() != 64 || !want.bytes().all(|b| b.is_ascii_hexdigit()) {
        return format!(
            "\x20 echo 'checksum-invalid:{repo}:checksum must be 64 hex chars of sha256'; __fj_diverged=1\n"
        );
    }
    format!(
        "\x20 FJ_SUM=$( {{ sha256sum < {bin_path} 2>/dev/null || shasum -a 256 < {bin_path} 2>/dev/null; }} | cut -d ' ' -f 1 || true )\n\
         \x20 if [ \"$FJ_SUM\" != '{want}' ]; then\n\
         \x20\x20\x20 echo \"checksum-mismatch:{repo}:want {want}:have ${{FJ_SUM:-unreadable}}\"; __fj_diverged=1\n\
         \x20 fi\n"
    )
}
