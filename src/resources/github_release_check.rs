//! forjar#1049: the `github_release` check asserts the declared version, not
//! just that an executable exists.
//! * a tag that names a version (`v0.33.2`) must equal the first `N.N.N` the
//!   live `--version` prints, or the check diverges;
//! * `latest`, no tag, or a tag that names no version (`nightly`) is not
//!   compared: presence-only, exactly as before.
//! * `checksum:` (hex sha256 of the installed file, optional `sha256:` prefix)
//!   must match the file on disk, whatever the tag; it is verified before the
//!   binary is executed.

use crate::core::shell_escape::{is_valid_repo, sh_squote};
use crate::core::types::Resource;
use crate::resources::verdict;
use crate::tripwire::drift::version_pin::{declared_pin, Pin};

/// Check script: the binary is a regular executable file and a versioned pin
/// equals the live version. The binary runs once, in [`live_version`].
pub(super) fn check_script(resource: &Resource, reject_bad_repo: fn(&str) -> String) -> String {
    let repo = resource.repo.as_deref().unwrap_or("unknown/unknown");
    if !is_valid_repo(repo) {
        return reject_bad_repo(repo);
    }
    let binary = resource.binary.as_deref().unwrap_or("unknown");
    let install_dir = resource.install_dir.as_deref().unwrap_or("/usr/local/bin");
    let bin_path = sh_squote(&format!("{install_dir}/{binary}"));

    verdict::check_script_from(&[format!(
        "{}\
         if [ \"$FJ_PRESENT\" = 1 ]; then\n\
         \x20 echo \"installed:{repo}:${{FJ_VERLINE:-unknown}}\"\n\
         {}\
         else\n\
         \x20 echo 'missing:{repo}'; __fj_diverged=1\n\
         fi",
        checksum_guard(resource, &bin_path, repo) + &live_version(&bin_path),
        version_assertion(resource, repo),
    )])
}

/// The one reader of a binary's live version. Sets `FJ_PRESENT` (1 for a
/// regular executable file), `FJ_VERLINE` (first line of `--version`) and
/// `FJ_LIVE` (its first `N.N.N` at a token boundary, read bytewise the way
/// `parse_semver` reads it).
///
/// stderr is discarded and the output is read only from a run that exited 0,
/// so a failed run leaves `FJ_LIVE` empty ("version unknown") and the shell's
/// own exec error is never parsed: a path like `.../tool-0.34.2/bin/tool`
/// cannot supply a version.
fn live_version(bin_path: &str) -> String {
    format!(
        "FJ_PRESENT=0; FJ_LIVE=''; FJ_VERLINE=''\n\
         if [ -f {bin_path} ] && [ -x {bin_path} ]; then\n\
         \x20 FJ_PRESENT=1\n\
         \x20 FJ_OUT=$(mktemp) || FJ_OUT=''\n\
         \x20 if [ \"$FJ_SUMOK\" = 1 ] && {bin_path} --version >\"$FJ_OUT\" 2>/dev/null; then\n\
         \x20\x20\x20 FJ_VERLINE=$( head -n 1 \"$FJ_OUT\" | tr '\\0' ' ' || true )\n\
         \x20\x20\x20 FJ_LIVE=$( head -n 20 \"$FJ_OUT\" | tr '\\0' ' ' | \
         LC_ALL=C grep -oE '(^|[^0-9A-Za-z._])[vV]?[0-9]+[.][0-9]+[.][0-9]+([^.0-9]|[.]([^0-9]|$)|$)' | \
         head -n 1 | grep -oE '[0-9]+[.][0-9]+[.][0-9]+' || true )\n\
         \x20 fi\n\
         \x20 rm -f \"$FJ_OUT\"\n\
         fi\n"
    )
}

/// Diverge when the live version is not the pinned one. Empty for an unpinned
/// resource. Leading zeros are dropped so a CalVer `2024.08.06` equals its pin.
fn version_assertion(resource: &Resource, repo: &str) -> String {
    let Pin::Version(pin) = declared_pin(resource) else {
        return String::new();
    };
    let want = format!("{}.{}.{}", pin.major, pin.minor, pin.patch);
    format!(
        "\x20 FJ_NORM=$( printf '%s\\n' \"$FJ_LIVE\" | sed -E 's/(^|[.])0+([0-9])/\\1\\2/g' )\n\
         \x20 if [ \"$FJ_NORM\" != '{want}' ]; then\n\
         \x20\x20\x20 echo \"version-mismatch:{repo}:want {want}:have ${{FJ_NORM:-unreadable}}\"; __fj_diverged=1\n\
         \x20 fi\n"
    )
}

/// Set `FJ_SUMOK` (1 unless a declared `checksum:` fails) before the binary is
/// run. The hash fails closed: one tool reading the file on stdin, no fallback
/// that could hash nothing. A malformed value never reaches the script.
fn checksum_guard(resource: &Resource, bin_path: &str, repo: &str) -> String {
    let Some(raw) = resource.checksum.as_deref() else {
        return "FJ_SUMOK=1\n".to_string();
    };
    let lowered = raw.trim().to_ascii_lowercase();
    let want = lowered.strip_prefix("sha256:").unwrap_or(&lowered);
    if want.len() != 64 || !want.bytes().all(|b| b.is_ascii_hexdigit()) {
        return format!(
            "FJ_SUMOK=0; echo 'checksum-invalid:{repo}:want 64 hex chars'; __fj_diverged=1\n"
        );
    }
    format!(
        "FJ_SUMOK=1; FJ_SUM=''\n\
         if [ -f {bin_path} ]; then\n\
         \x20 if command -v sha256sum >/dev/null 2>&1; then\n\
         \x20\x20\x20 FJ_SUM=$( sha256sum < {bin_path} 2>/dev/null | cut -d ' ' -f 1 )\n\
         \x20 elif command -v shasum >/dev/null 2>&1; then\n\
         \x20\x20\x20 FJ_SUM=$( shasum -a 256 < {bin_path} 2>/dev/null | cut -d ' ' -f 1 )\n\
         \x20 fi\n\
         \x20 if [ \"$FJ_SUM\" != '{want}' ]; then\n\
         \x20\x20\x20 FJ_SUMOK=0; __fj_diverged=1\n\
         \x20\x20\x20 echo \"checksum-mismatch:{repo}:want {want}:have ${{FJ_SUM:-unreadable}}\"\n\
         \x20 fi\n\
         fi\n"
    )
}
