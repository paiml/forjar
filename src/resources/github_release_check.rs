//! forjar#1049: the `github_release` check asserts the declared version, not
//! just that an executable exists.
//! * a tag that names a version (`v0.33.2`) must equal the first `N.N.N` the
//!   live `--version` prints, or the check diverges; a pre-release suffix
//!   (`-rc1`) on either side must equal the other side's;
//! * `latest`, no tag, or a tag that names no version (`nightly`) is not
//!   compared: presence-only, exactly as before.
//! * `checksum:` (hex sha256 of the installed file, optional `sha256:` prefix)
//!   must match the file on disk, whatever the tag; it is verified before the
//!   binary is executed.

use crate::core::shell_escape::{is_valid_repo, sh_squote};
use crate::core::types::Resource;
use crate::resources::verdict;
use crate::tripwire::drift::version_pin::{declared_pin, parse_semver_span, Pin};

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
        checksum_guard(resource, &bin_path, repo) + &live_version(&bin_path, false),
        version_assertion(resource, repo),
    )])
}

/// The one reader of a binary's live version. Sets `FJ_PRESENT` (1 for a
/// regular executable file), `FJ_VERLINE` (first line of `--version`) and
/// `FJ_LIVE` (its first `N.N.N` at a token boundary, read bytewise the way
/// `parse_semver` reads it) and `FJ_LIVEPRE` (the `-rc1` suffix right after it).
///
/// `permissive` (the apply-side downgrade guard, forjar#613/#1067) merges stderr
/// and parses a non-zero exit too: a tool that prints its version on stderr or
/// after a warning must not slip a downgrade through. Check stays strict.
///
/// stderr is discarded and the output is read only from a run that exited 0,
/// so a failed run leaves `FJ_LIVE` empty ("version unknown") and the shell's
/// own exec error is never parsed: a path like `.../tool-0.34.2/bin/tool`
/// cannot supply a version.
pub(super) fn live_version(bin_path: &str, permissive: bool) -> String {
    let run = if permissive {
        format!("{bin_path} --version >\"$FJ_OUT\" 2>&1 || true")
    } else {
        format!("{bin_path} --version >\"$FJ_OUT\" 2>/dev/null")
    };
    format!(
        "FJ_PRESENT=0; FJ_LIVE=''; FJ_LIVEPRE=''; FJ_VERLINE=''\n\
         if [ -f {bin_path} ] && [ -x {bin_path} ]; then\n\
         \x20 FJ_PRESENT=1\n\
         \x20 FJ_OUT=$(mktemp) || FJ_OUT=''\n\
         \x20 if [ \"$FJ_SUMOK\" = 1 ] && {{ {run}; }}; then\n\
         \x20\x20\x20 FJ_VERLINE=$( head -n 1 \"$FJ_OUT\" | tr '\\0' ' ' || true )\n\
         \x20\x20\x20 FJ_FULL=$( head -n 20 \"$FJ_OUT\" | tr '\\0' ' ' | \
         LC_ALL=C grep -oE '(^|[^0-9A-Za-z._])[vV]?[0-9]+[.][0-9]+[.][0-9]+(-(alpha|beta|rc|pre|dev)[0-9A-Za-z.]*)?([^.0-9]|[.]([^0-9]|$)|$)' | \
         head -n 1 | grep -oE '[0-9]+[.][0-9]+[.][0-9]+(-(alpha|beta|rc|pre|dev)[0-9A-Za-z.]*)?' || true )\n\
         \x20\x20\x20 FJ_FULL=\"${{FJ_FULL%.}}\"\n\
         \x20\x20\x20 FJ_LIVE=\"${{FJ_FULL%%-*}}\"; FJ_LIVEPRE=\"${{FJ_FULL#\"$FJ_LIVE\"}}\"\n\
         \x20 fi\n\
         \x20 rm -f \"$FJ_OUT\"\n\
         fi\n"
    )
}

/// [`live_version`] for the apply side (post-install read, downgrade guard,
/// state query): no `checksum:` guard runs there, so `FJ_SUMOK` is set to 1
/// explicitly. The reader itself stays fail-closed (`FJ_SUMOK` unset or 0 never
/// runs the binary), so a check script that forgot the guard cannot run one.
pub(super) fn live_version_unguarded(bin_path: &str, permissive: bool) -> String {
    format!("FJ_SUMOK=1\n{}", live_version(bin_path, permissive))
}

/// Diverge when the live version is not the pinned one. Empty for an unpinned
/// resource. Leading zeros are dropped so a CalVer `2024.08.06` equals its pin.
fn version_assertion(resource: &Resource, repo: &str) -> String {
    let Pin::Version(pin) = declared_pin(resource) else {
        return String::new();
    };
    let want = format!("{}.{}.{}", pin.major, pin.minor, pin.patch);
    let pre = pin_prerelease(resource.tag.as_deref().unwrap_or_default());
    format!(
        "\x20 FJ_NORM=$( printf '%s\\n' \"$FJ_LIVE\" | sed -E 's/(^|[.])0+([0-9])/\\1\\2/g' )\n\
         \x20 if [ \"$FJ_NORM\" != '{want}' ] || [ \"$FJ_LIVEPRE\" != '{pre}' ]; then\n\
         \x20\x20\x20 echo \"version-mismatch:{repo}:want {want}{pre}:have ${{FJ_NORM:-unreadable}}$FJ_LIVEPRE\"; __fj_diverged=1\n\
         \x20 fi\n"
    )
}

/// The pre-release suffix of a tag, `-rc1` for `v1.2.3-rc1`, or empty: the
/// `-` directly after the `N.N.N` that `parse_semver` reads (when a letter
/// follows it), plus the `[A-Za-z0-9.-]` run after it. Matches what
/// [`live_version`] stores in `FJ_LIVEPRE`; a `-` before the version
/// (`cli2-v1.2.3`) is not a suffix.
fn pin_prerelease(tag: &str) -> String {
    let Some((_, end)) = parse_semver_span(tag) else {
        return String::new();
    };
    let rest = &tag[end..];
    let run = |c: &char| c.is_ascii_alphanumeric() || *c == '.' || *c == '-';
    match rest.strip_prefix('-') {
        Some(s) if s.starts_with(|c: char| c.is_ascii_alphabetic()) => {
            format!("-{}", s.chars().take_while(run).collect::<String>())
        }
        _ => String::new(),
    }
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
