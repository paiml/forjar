//! The stubbed `gh` for the PUBLISHED fixture — the two questions gate R asks
//! it after a tag exists, and the answers each case needs it to give.
//!
//! Split out of `mod.rs` because that file reached the 500-line gate while
//! PMAT-534 was adding to it. Nothing here asserts anything.

use super::{HEAD_REF, PR, PREV_TAG};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

/// The body of every stub: the PR array `gh pr list` answers with, and the
/// release object `gh release view` answers with. Only `$1` tells them apart.
fn stub_body(head: &str) -> String {
    format!(
        r#"{{"number":{PR},"mergedAt":"2026-09-05T00:00:00Z","mergeCommit":{{"oid":"{head}"}},"headRefName":"{HEAD_REF}"}}"#
    )
}

/// Write `script` as an executable `gh` at `dir/name` and return its path.
fn install(dir: &Path, name: &str, script: &str) -> String {
    let p = dir.join(name);
    std::fs::write(&p, script).expect("write stub");
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    p.to_string_lossy().into_owned()
}

/// The three-branch stub every case below shares: `api` first, then `release`,
/// then the PR list. `api_branch` is the body of the `api` case — what
/// `gh api repos/<r>/releases/latest --jq .tag_name` does.
///
/// The `api` branch must come FIRST: `gh api …` and `gh release view …` are
/// told apart by `$1`, and a stub that answered the PR list for `api` would
/// make the gate read a JSON array where it wants a tag name.
fn stub_script(head: &str, prerelease: &str, draft: &str, api_branch: &str) -> String {
    let body = stub_body(head);
    let release =
        format!(r#"{{"tagName":"{PREV_TAG}","isPrerelease":{prerelease},"isDraft":{draft}}}"#);
    format!(
        "#!/usr/bin/env bash\nif [ \"${{1:-}}\" = api ]; then\n{api_branch}\nfi\nif [ \"${{1:-}}\" = release ]; then\n  printf '%s' '{release}'\n  exit 0\nfi\nprintf '%s' '[{body}]'\n"
    )
}

/// A `gh` that answers BOTH questions the published state asks it: the release
/// object for `gh release view`, and the PR window for everything else. The
/// pre-tag fixture's stub answers every invocation with the PR array, which
/// `jq -r .isPrerelease` cannot index — a stub that answers the wrong question
/// is a fixture defect, not a gate finding.
pub(crate) fn stub_gh_published(dir: &Path, head: &str, draft: &str) -> String {
    stub_gh_published_as(dir, head, draft, "false", PREV_TAG)
}

/// The same stub with the two fields PMAT-534's arm reads under the caller's
/// control: whether the release is a prerelease, and what
/// `gh api repos/<r>/releases/latest --jq .tag_name` answers.
pub(crate) fn stub_gh_published_as(
    dir: &Path,
    head: &str,
    draft: &str,
    prerelease: &str,
    latest: &str,
) -> String {
    let api = format!("  printf '%s\\n' '{latest}'\n  exit 0");
    install(
        dir,
        &format!("gh-published-{prerelease}-{latest}"),
        &stub_script(head, prerelease, draft, &api),
    )
}

/// A `gh` whose `api` call FAILS — the shape of a token without the scope, a
/// rate limit, or a repository with no release `/releases/latest` can serve.
pub(crate) fn stub_gh_published_api_broken(dir: &Path, head: &str) -> String {
    stub_gh_published_api_broken_as(dir, head, "false")
}

/// The failing `api` with the release's own kind under the caller's control.
///
/// `prerelease = "true"` is the state MEASURED on a repository whose releases
/// are all prereleases: `/releases/latest` serves only non-prerelease,
/// non-draft releases and 404s when there is none. That 404 is ordinary there,
/// not a broken instrument, and the arm must report it rather than refuse it.
pub(crate) fn stub_gh_published_api_broken_as(dir: &Path, head: &str, prerelease: &str) -> String {
    let api = "  echo 'HTTP 404: Not Found' >&2\n  exit 1";
    install(
        dir,
        &format!("gh-published-api-broken-{prerelease}"),
        &stub_script(head, prerelease, "false", api),
    )
}

/// A `gh` whose `api` call SUCCEEDS and says nothing.
///
/// The one answer that is neither a tag nor an error: exit 0, empty stdout. A
/// gate that only checks the exit code reads it as "the pointer answered" and
/// then compares the empty string to the tag.
pub(crate) fn stub_gh_published_api_empty(dir: &Path, head: &str) -> String {
    let api = "  printf ''\n  exit 0";
    install(
        dir,
        "gh-published-api-empty",
        &stub_script(head, "false", "false", api),
    )
}
