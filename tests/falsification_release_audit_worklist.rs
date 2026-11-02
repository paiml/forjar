//! Release Audit's work list must never read "could not measure" as "skip".
//!
//! forjar#672. `release-audit.yml` asked `releases/tags/<tag>` once per tag:
//!
//! ```sh
//! n=$(gh api "repos/$R/releases/tags/$t" --jq '.assets | length' 2>/dev/null || echo 0)
//! ```
//!
//! `v1.33.0-rc.1` has a DRAFT release, which that endpoint does not serve. gh
//! wrote the 404 body to stdout, `|| echo 0` appended a 0, `n` became
//! `{"message":"Not Found",...}0`, the skip never fired, and the lane was red
//! every day on a tag with no published release. The same line failed OPEN on
//! any error that left stdout empty: n=0, printed as `skip: has no assets`.
//!
//! `release-object-audit.sh worklist` reads the release list once, drafts
//! excluded, and exits 2 when it cannot measure. These tests drive it offline
//! through its env hooks; none of them touches the network.

use std::process::Command;

fn script() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/release-object-audit.sh")
}

fn workflow() -> String {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(".github/workflows/release-audit.yml");
    std::fs::read_to_string(p).expect("release-audit.yml is readable")
}

/// One release object, as the releases list serves it.
fn release(tag: &str, draft: bool, assets: usize) -> String {
    let names: Vec<String> = (0..assets)
        .map(|i| format!(r#"{{"name":"forjar-{tag}-{i}.tar.gz"}}"#))
        .collect();
    format!(
        r#"{{"tag_name":"{tag}","draft":{draft},"assets":[{}]}}"#,
        names.join(",")
    )
}

struct Run {
    code: i32,
    stdout: String,
    stderr: String,
}

fn worklist(releases_json: &str, tags: &[&str]) -> Run {
    let dir = tempfile::tempdir().expect("tempdir");
    let rel = dir.path().join("releases.json");
    let tag = dir.path().join("tags.txt");
    std::fs::write(&rel, releases_json).expect("write releases");
    std::fs::write(&tag, tags.join("\n") + "\n").expect("write tags");
    let out = Command::new("bash")
        .arg(script())
        .arg("worklist")
        .env("FORJAR_AUDIT_RELEASES_FILE", &rel)
        .env("FORJAR_AUDIT_TAGS_FILE", &tag)
        .output()
        .expect("run release-object-audit.sh");
    Run {
        code: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

const TAGS: &[&str] = &["v1.0.0", "v1.32.0", "v1.33.0-rc.1"];

/// The recorded shape: v1.0.0 published with no assets, v1.32.0 published
/// with assets, v1.33.0-rc.1 a draft.
fn recorded() -> String {
    format!(
        "[{},{},{}]",
        release("v1.33.0-rc.1", true, 0),
        release("v1.32.0", false, 13),
        release("v1.0.0", false, 0)
    )
}

#[test]
fn a_draft_release_is_skipped_as_unpublished_not_audited() {
    let r = worklist(&recorded(), TAGS);
    assert_eq!(r.code, 0, "stderr: {}", r.stderr);
    assert!(
        r.stdout
            .contains("skip: v1.33.0-rc.1 has no published release (none, or a draft)"),
        "{}",
        r.stdout
    );
    assert!(!r.stdout.contains("audit v1.33.0-rc.1"), "{}", r.stdout);
}

#[test]
fn a_published_release_with_assets_is_audited_and_one_without_is_skipped() {
    let r = worklist(&recorded(), TAGS);
    assert_eq!(r.code, 0, "stderr: {}", r.stderr);
    assert!(r.stdout.contains("audit v1.32.0\n"), "{}", r.stdout);
    assert!(
        r.stdout.contains("skip: v1.0.0 has no assets"),
        "{}",
        r.stdout
    );
}

#[test]
fn every_tag_gets_exactly_one_line() {
    let r = worklist(&recorded(), TAGS);
    assert_eq!(r.stdout.lines().count(), TAGS.len(), "{}", r.stdout);
}

#[test]
fn every_page_of_a_paginated_list_is_read() {
    // `gh api --paginate` emits one array per page, back to back.
    let paged = format!(
        "[{}][{}]",
        release("v1.0.0", false, 0),
        release("v1.32.0", false, 13)
    );
    let r = worklist(&paged, &["v1.0.0", "v1.32.0"]);
    assert_eq!(r.code, 0, "stderr: {}", r.stderr);
    assert!(r.stdout.contains("audit v1.32.0\n"), "{}", r.stdout);
}

#[test]
fn an_error_body_is_cannot_measure_never_a_skip() {
    let r = worklist(
        r#"{"message":"Not Found","documentation_url":"https://docs.github.com","status":"404"}"#,
        TAGS,
    );
    assert_eq!(r.code, 2, "stdout: {}", r.stdout);
    assert!(r.stderr.contains("cannot measure"), "{}", r.stderr);
    assert!(!r.stdout.contains("skip:"), "{}", r.stdout);
}

#[test]
fn an_empty_read_is_cannot_measure_never_a_skip() {
    // The fail-open half of #672: an error that leaves stdout empty.
    let r = worklist("", TAGS);
    assert_eq!(r.code, 2, "stdout: {}", r.stdout);
    assert!(!r.stdout.contains("skip:"), "{}", r.stdout);
}

#[test]
fn a_list_holding_only_drafts_is_cannot_measure() {
    let r = worklist(&format!("[{}]", release("v1.33.0-rc.1", true, 0)), TAGS);
    assert_eq!(r.code, 2, "stdout: {}", r.stdout);
    assert!(!r.stdout.contains("skip:"), "{}", r.stdout);
}

#[test]
fn the_workflow_reads_the_work_list_and_never_one_tag_at_a_time() {
    let wf = workflow();
    assert!(
        wf.contains("release-object-audit.sh worklist"),
        "release-audit.yml must take its tags from the worklist"
    );
    assert!(
        !wf.contains("releases/tags/"),
        "release-audit.yml must not ask releases/tags/<tag>: it 404s on a draft (#672)"
    );
    assert!(
        !wf.contains("|| echo 0"),
        "release-audit.yml must not turn an error into a count (#672)"
    );
}
