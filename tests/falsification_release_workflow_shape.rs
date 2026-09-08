//! PMAT-166: falsification tests for the shape of the release workflows.
//!
//! WHAT THIS GUARDS. release.yml (self-hosted clean-room) is the sole
//! producer of a tag-triggered GitHub Release: it must create the release as
//! a DRAFT PRERELEASE right after the clean-room `verify` gate, record
//! whether THIS run created it, assert `prerelease=true draft=true` for a
//! run that created it, and only un-draft it (assert
//! `prerelease=true draft=false`) once every asset job (`build-binaries`,
//! `checksums`, `dist-artifacts`) has finished. binary-release.yml (PMAT-170)
//! is the manual backfill path only — it must never fire on `push` or
//! `release` (that raced release.yml on the same tag: overlapping Linux
//! targets, clobbered SHA256SUMS), and its backfill release is a prerelease
//! too.
//!
//! Each test below asserts one rule from the PMAT-166 ticket, named in its
//! function name and panic message, by reading the workflow YAML at
//! `env!("CARGO_MANIFEST_DIR")` — the same bytes GitHub Actions parses —
//! rather than trusting a description of what the file is supposed to say.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn manifest_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn workflows_dir() -> PathBuf {
    manifest_dir().join(".github/workflows")
}

fn read(rel: &str) -> String {
    let p = manifest_dir().join(rel);
    fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

fn yaml(rel: &str) -> serde_yaml_ng::Value {
    let text = read(rel);
    serde_yaml_ng::from_str(&text).unwrap_or_else(|e| panic!("parse {rel} as YAML: {e}"))
}

/// Every `.yml` workflow file under `.github/workflows/`.
fn all_workflow_files() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = fs::read_dir(workflows_dir())
        .expect("read .github/workflows")
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("yml"))
        .collect();
    out.sort();
    out
}

/// Slice out one top-level job's block (2-space-indented `name:` key up to,
/// but not including, the next 2-space-indented key or EOF). Jobs in both
/// release.yml and binary-release.yml are declared at exactly this
/// indentation, so this is a reliable way to scope a check to one job
/// without a full GitHub-Actions-aware YAML schema.
fn job_block<'a>(text: &'a str, job_name: &str) -> &'a str {
    let marker = format!("\n  {job_name}:\n");
    let start = text
        .find(&marker)
        .unwrap_or_else(|| panic!("no job named `{job_name}` found"));
    let body_start = start + 1; // skip the leading \n so we search from the header line
    let rest = &text[body_start..];
    // Find the next line that starts a new 2-space-indented top-level key
    // (i.e. `  word` with no further leading whitespace), searching after
    // the job's own header line.
    let after_header = rest.find('\n').map(|i| i + 1).unwrap_or(rest.len());
    let tail = &rest[after_header..];
    let end = tail
        .match_indices('\n')
        .find(|(i, _)| {
            let line_start = i + 1;
            if line_start >= tail.len() {
                return false;
            }
            let remainder = &tail[line_start..];
            let is_top_level_key = remainder.starts_with("  ")
                && !remainder.starts_with("   ")
                && remainder
                    .as_bytes()
                    .get(2)
                    .is_some_and(|b| b.is_ascii_alphabetic() || *b == b'_' || *b == b'-');
            is_top_level_key
        })
        .map(|(i, _)| after_header + i + 1)
        .unwrap_or(rest.len());
    &rest[..end]
}

/// Slice out one step's block within an already-sliced job block, by its
/// `- name: <name>` line, up to the next `- name:` step or the end of the
/// job block.
fn step_block<'a>(job_text: &'a str, step_name: &str) -> &'a str {
    let marker = format!("- name: {step_name}");
    let start = job_text
        .find(&marker)
        .unwrap_or_else(|| panic!("no step named `{step_name}` found in job block"));
    let rest = &job_text[start..];
    let end = rest[marker.len()..]
        .find("- name:")
        .map(|i| marker.len() + i)
        .unwrap_or(rest.len());
    &rest[..end]
}

fn non_comment_lines(text: &str) -> impl Iterator<Item = &str> {
    text.lines().filter(|l| !l.trim_start().starts_with('#'))
}

// ---------------------------------------------------------------------
// Rule 1: binary-release.yml's `on` mapping is exactly {workflow_dispatch}.
// ---------------------------------------------------------------------
#[test]
fn rule1_binary_release_fires_only_on_workflow_dispatch() {
    let doc = yaml(".github/workflows/binary-release.yml");
    let on = doc
        .get("on")
        .unwrap_or_else(|| panic!("binary-release.yml has no `on` key"));
    let on_map = on
        .as_mapping()
        .unwrap_or_else(|| panic!("binary-release.yml's `on` is not a mapping: {on:?}"));
    let keys: BTreeSet<String> = on_map
        .keys()
        .map(|k| k.as_str().expect("on-key is a string").to_string())
        .collect();
    let expected: BTreeSet<String> = ["workflow_dispatch".to_string()].into_iter().collect();
    assert_eq!(
        keys, expected,
        "PMAT-166 rule 1: binary-release.yml's `on` mapping must be exactly \
         {{workflow_dispatch}} (no push, no release — PMAT-170 removed both \
         to stop the same-tag race with release.yml) — got {keys:?}"
    );
}

// ---------------------------------------------------------------------
// Rule 2: release.yml is the only workflow whose on.push.tags contains 'v*'.
// ---------------------------------------------------------------------
#[test]
fn rule2_release_yml_is_the_only_v_star_tag_push_producer() {
    let mut owners: Vec<String> = Vec::new();
    for path in all_workflow_files() {
        let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
        let doc: serde_yaml_ng::Value = match serde_yaml_ng::from_str(&text) {
            Ok(d) => d,
            Err(_) => continue,
        };
        let has_v_star_tag = doc
            .get("on")
            .and_then(|on| on.as_mapping())
            .and_then(|m| m.get("push"))
            .and_then(|push| push.as_mapping())
            .and_then(|m| m.get("tags"))
            .and_then(|tags| tags.as_sequence())
            .is_some_and(|seq| seq.iter().any(|t| t.as_str() == Some("v*")));
        if has_v_star_tag {
            owners.push(
                path.file_name()
                    .expect("workflow file has a name")
                    .to_string_lossy()
                    .to_string(),
            );
        }
    }
    assert_eq!(
        owners,
        vec!["release.yml".to_string()],
        "PMAT-166 rule 2: release.yml must be the ONLY file under \
         .github/workflows/ whose on.push.tags contains 'v*' — found {owners:?}"
    );
}

// ---------------------------------------------------------------------
// Rule 3: create-release creates a draft prerelease and records `created`.
// ---------------------------------------------------------------------
#[test]
fn rule3_create_release_step_creates_a_draft_prerelease_and_records_created() {
    let text = read(".github/workflows/release.yml");
    let job = job_block(&text, "create-release");

    assert!(
        job.contains("outputs:") && job.contains("created: ${{ steps.create.outputs.created }}"),
        "PMAT-166 rule 3: the `create-release` job must expose \
         `outputs.created` sourced from `steps.create.outputs.created`:\n{job}"
    );

    let create_step = step_block(job, "Create or update GitHub Release");
    for needle in ["--draft", "--prerelease", "--verify-tag"] {
        assert!(
            create_step.contains(needle),
            "PMAT-166 rule 3: the `gh release create` invocation in \
             create-release must carry `{needle}`:\n{create_step}"
        );
    }
    assert!(
        create_step.contains("gh release create"),
        "PMAT-166 rule 3: create-release must actually invoke `gh release create`:\n{create_step}"
    );
    for needle in [
        "echo \"created=true\" >> \"$GITHUB_OUTPUT\"",
        "echo \"created=false\" >> \"$GITHUB_OUTPUT\"",
    ] {
        assert!(
            create_step.contains(needle),
            "PMAT-166 rule 3: the create step must write `{needle}`:\n{create_step}"
        );
    }
}

// ---------------------------------------------------------------------
// Rule 4: the assert step requires the exact draft-prerelease string, gated
// on `steps.create.outputs.created`.
// ---------------------------------------------------------------------
#[test]
fn rule4_assert_step_requires_exact_draft_prerelease_string_when_created() {
    let text = read(".github/workflows/release.yml");
    let job = job_block(&text, "create-release");
    let assert_step = step_block(
        job,
        "Assert the release this run created is a draft prerelease",
    );

    assert!(
        assert_step.contains("prerelease=true draft=true"),
        "PMAT-166 rule 4: the assert step must require the exact string \
         `prerelease=true draft=true`:\n{assert_step}"
    );
    assert!(
        assert_step.contains("steps.create.outputs.created"),
        "PMAT-166 rule 4: the `prerelease=true draft=true` assertion must be \
         gated on `steps.create.outputs.created` (only a release THIS run \
         created is asserted; a pre-existing release is left alone):\n{assert_step}"
    );
}

// ---------------------------------------------------------------------
// Rule 5: publish-release un-drafts only after every asset job, asserts the
// published state.
// ---------------------------------------------------------------------
#[test]
fn rule5_publish_release_job_shape() {
    let text = read(".github/workflows/release.yml");
    let job = job_block(&text, "publish-release");

    for needed in [
        "verify",
        "create-release",
        "build-binaries",
        "checksums",
        "dist-artifacts",
    ] {
        assert!(
            job.contains(needed),
            "PMAT-166 rule 5: publish-release's `needs` must include `{needed}`:\n{job}"
        );
    }
    assert!(
        job.contains("needs:"),
        "PMAT-166 rule 5: publish-release must declare `needs:`:\n{job}"
    );

    assert!(
        job.contains("always()"),
        "PMAT-166 rule 5: publish-release's `if` must contain `always()` \
         (it must still run to report state even when an asset job failed):\n{job}"
    );
    assert!(
        job.contains("!contains(needs.*.result, 'failure')"),
        "PMAT-166 rule 5: publish-release's `if` must contain \
         `!contains(needs.*.result, 'failure')`:\n{job}"
    );

    assert!(
        job.contains("gh release edit")
            && job.contains("--draft=false")
            && job.contains("--prerelease"),
        "PMAT-166 rule 5: publish-release must un-draft via \
         `gh release edit ... --draft=false --prerelease`:\n{job}"
    );
    assert!(
        job.contains("prerelease=true draft=false"),
        "PMAT-166 rule 5: publish-release must assert \
         `prerelease=true draft=false` after un-drafting:\n{job}"
    );
}

// ---------------------------------------------------------------------
// Rule 6: no workflow does its own crates.io publish or bypasses failures
// with continue-on-error, except mutation.yml's known, unrelated case.
// ---------------------------------------------------------------------
#[test]
fn rule6_no_stray_publish_or_swallowed_failure_outside_mutation_yml() {
    for path in all_workflow_files() {
        let file_name = path
            .file_name()
            .expect("workflow file has a name")
            .to_string_lossy()
            .to_string();
        let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));

        for line in non_comment_lines(&text) {
            assert!(
                !line.contains("cargo publish"),
                "PMAT-166 rule 6: {file_name} runs `cargo publish` outside a \
                 comment — crates.io publish is deliberately manual \
                 (release.yml's own comment explains why), no workflow may \
                 do it: {line}"
            );
            assert!(
                !line.contains("CARGO_REGISTRY_TOKEN"),
                "PMAT-166 rule 6: {file_name} references CARGO_REGISTRY_TOKEN \
                 — no workflow publishes to crates.io: {line}"
            );
            if line.contains("continue-on-error") && file_name != "mutation.yml" {
                panic!(
                    "PMAT-166 rule 6: {file_name} uses `continue-on-error` — \
                     the only known, accepted case is mutation.yml (its \
                     mutation-testing step is advisory and not \
                     release-relevant); every other workflow must fail \
                     closed: {line}"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------
// Rule 7: binary-release.yml's concurrency group is keyed on inputs.tag
// alone, and its backfill release is a prerelease too.
// ---------------------------------------------------------------------
#[test]
fn rule7_binary_release_concurrency_and_backfill_prerelease() {
    let text = read(".github/workflows/binary-release.yml");
    assert!(
        text.contains("group: binary-release-${{ inputs.tag }}"),
        "PMAT-166 rule 7: binary-release.yml's concurrency group must be \
         exactly `binary-release-${{{{ inputs.tag }}}}` (keyed on the \
         dispatch input alone, not github.event.release.tag_name or \
         github.ref_name — those no longer exist now that push/release \
         triggers are gone):\n{text}"
    );

    let ensure_release = job_block(&text, "ensure-release");
    assert!(
        ensure_release.contains("gh release create") && ensure_release.contains("--prerelease"),
        "PMAT-166 rule 7: binary-release.yml's backfill `gh release create` \
         must carry `--prerelease`:\n{ensure_release}"
    );
}

/// RULE 8 (PMAT-208): the dist job must not read the public download URL while
/// the release it describes is still a draft.
///
/// The 1.26.0 run uploaded all thirteen assets and then failed here:
///
/// ```text
/// error: cannot resolve checksums for release asset(s) forjar-1.26.0-x86_64-unknown-linux-gnu.tar.gz, ...
/// ```
///
/// `forjar dist` resolves checksums from
/// `https://github.com/<repo>/releases/download/<tag>/<asset>`, and GitHub does
/// not serve that path for a draft — which the release is until
/// `publish-release` un-drafts it, by the design RULE 3 pins. So the checksums
/// have to arrive over the authenticated API instead, and the dist step has to
/// be told to use them: `gh release download --pattern SHA256SUMS` before it,
/// `--checksums-file` on it. Both halves are asserted, and so is their order —
/// a fetch after the step that needs it would be no fetch at all.
#[test]
fn rule8_dist_artifacts_takes_its_checksums_from_the_api_not_the_public_url() {
    let wf = read(".github/workflows/release.yml");
    // Slice the job by LINES: a naive split on a two-space indent ends the job
    // at the first comment written at that indent, which is inside it.
    let mut job = String::new();
    let mut inside = false;
    for line in wf.lines() {
        if line.starts_with("  dist-artifacts:") {
            inside = true;
            continue;
        }
        if inside
            && line.len() > 2
            && line.starts_with("  ")
            && !line.starts_with("   ")
            && !line.trim_start().starts_with('#')
            && line.trim_end().ends_with(':')
        {
            break;
        }
        if inside {
            job.push_str(line);
            job.push('\n');
        }
    }
    assert!(
        !job.is_empty(),
        "RULE 8: release.yml has no dist-artifacts job"
    );
    let job = job.as_str();

    let fetch = job
        .find("gh release download")
        .expect("RULE 8: dist-artifacts never fetches the checksums over the API");
    assert!(
        job[fetch..fetch + 400].contains("--pattern SHA256SUMS"),
        "RULE 8: the fetch does not ask for SHA256SUMS: {}",
        &job[fetch..(fetch + 200).min(job.len())]
    );

    let dist = job
        .find("dist \\")
        .or_else(|| job.find("-- dist"))
        .expect("RULE 8: dist-artifacts no longer runs `forjar dist`");
    assert!(
        job.contains("--checksums-file"),
        "RULE 8: the dist step does not pass --checksums-file, so it reads the public \
         download URL and cannot see a draft release's assets"
    );
    assert!(
        fetch < dist,
        "RULE 8: the checksums are fetched AFTER the step that needs them"
    );
}
