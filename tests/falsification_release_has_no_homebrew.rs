//! PMAT-1014: FORJAR IS NOT DISTRIBUTED THROUGH HOMEBREW.
//!
//! Operator ruling: drop homebrew support. The `homebrew` job in
//! `release.yml` pushed a formula to `paiml/homebrew-tap` with
//! `secrets.HOMEBREW_TAP_TOKEN`, a secret that was never provisioned (#462,
//! closed not-planned). It failed on every tag from v1.30.0 to v1.34.0, and a
//! release run that is always red hides the job that is red for a real reason.
//!
//! These pins read the same bytes GitHub Actions and `forjar dist` read. They
//! do not touch the generic `forjar dist --homebrew` generator or the `brew`
//! package provider: those are features forjar offers to the projects it
//! manages, not the way forjar itself is shipped.

use std::fs;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(rel: &str) -> String {
    let p = root().join(rel);
    fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {p:?}: {e}"))
}

fn workflow_files() -> Vec<PathBuf> {
    let dir = root().join(".github/workflows");
    let mut out: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read_dir {dir:?}: {e}"))
        .map(|e| e.expect("dir entry").path())
        .filter(|p| matches!(p.extension().and_then(|x| x.to_str()), Some("yml" | "yaml")))
        .collect();
    out.sort();
    assert!(
        out.iter().any(|p| p.ends_with("release.yml")),
        "release.yml must exist, or every pin below passes for the wrong reason"
    );
    out
}

/// Lines that are not YAML/shell comments.
fn code_lines(text: &str) -> impl Iterator<Item = &str> {
    text.lines().filter(|l| !l.trim_start().starts_with('#'))
}

/// The keys of the top-level `jobs:` map, read by indentation.
fn job_names(text: &str) -> Vec<String> {
    let mut in_jobs = false;
    let mut indent: Option<usize> = None;
    let mut names = Vec::new();
    for line in code_lines(text) {
        if line.trim().is_empty() {
            continue;
        }
        let lead = line.len() - line.trim_start().len();
        if lead == 0 {
            in_jobs = line.trim_end() == "jobs:";
            continue;
        }
        if !in_jobs {
            continue;
        }
        let want = *indent.get_or_insert(lead);
        if lead == want {
            if let Some(key) = line.trim().strip_suffix(':') {
                names.push(key.to_string());
            }
        }
    }
    names
}

/// The text of one top-level job: from its `  <name>:` line to the next line
/// indented exactly two spaces (the next job key).
fn job_block<'a>(text: &'a str, job: &str) -> &'a str {
    let header = format!("\n  {job}:");
    let start = text
        .find(&header)
        .unwrap_or_else(|| panic!("release.yml has no {job} job"))
        + 1;
    let rest = &text[start..];
    let mut offset = rest.find('\n').map_or(rest.len(), |i| i + 1);
    for line in rest[offset..].split_inclusive('\n') {
        let lead = line.len() - line.trim_start_matches(' ').len();
        let body = line.trim();
        if lead == 2 && !body.is_empty() && !body.starts_with('#') {
            break;
        }
        offset += line.len();
    }
    &rest[..offset]
}

#[test]
fn release_workflow_has_no_homebrew_job() {
    let text = read(".github/workflows/release.yml");
    let jobs = job_names(&text);
    assert!(
        jobs.iter().any(|j| j == "dist-artifacts"),
        "job parser found no dist-artifacts job in release.yml ({jobs:?}); \
         a parser that finds nothing proves nothing"
    );
    assert!(
        !jobs.iter().any(|j| j.contains("brew")),
        "PMAT-1014: release.yml declares a homebrew job ({jobs:?}). Forjar is not \
         distributed through Homebrew; the job needs a tap token that does not exist \
         and turns every release run red."
    );
}

#[test]
fn no_workflow_reads_the_homebrew_tap_token_or_pushes_to_a_tap() {
    for path in workflow_files() {
        let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
        for line in code_lines(&text) {
            assert!(
                !line.contains("HOMEBREW_TAP_TOKEN"),
                "PMAT-1014: {path:?} reads HOMEBREW_TAP_TOKEN: {line}"
            );
            assert!(
                !line.contains("homebrew-tap"),
                "PMAT-1014: {path:?} still pushes to a Homebrew tap: {line}"
            );
        }
    }
}

#[test]
fn dist_artifacts_generates_no_homebrew_formula() {
    let text = read(".github/workflows/release.yml");
    let block_text = job_block(&text, "dist-artifacts");
    let block: Vec<&str> = code_lines(block_text).collect();
    let dist_at = block
        .iter()
        .position(|l| l.contains("-- dist"))
        .expect("dist-artifacts runs `cargo run --release -- dist`");
    let mut cmd = String::new();
    for l in &block[dist_at..] {
        cmd.push_str(l.trim().trim_end_matches('\\'));
        cmd.push(' ');
        if !l.trim_end().ends_with('\\') {
            break;
        }
    }
    let toks: Vec<&str> = cmd.split_whitespace().collect();
    assert!(
        toks.contains(&"--installer"),
        "dist-artifacts must still generate the installer: {cmd}"
    );
    for banned in ["--all", "--homebrew"] {
        assert!(
            !toks.contains(&banned),
            "PMAT-1014: dist-artifacts passes {banned}, which generates a Homebrew \
             formula for a distribution channel forjar no longer has: {cmd}"
        );
    }
}

#[test]
fn forjars_own_dist_config_declares_no_homebrew_tap() {
    let yaml = read("dist-forjar.yaml");
    let config: forjar::core::types::ForjarConfig =
        serde_yaml_ng::from_str(&yaml).expect("dist-forjar.yaml parses");
    let dist = config
        .dist
        .expect("dist-forjar.yaml must have a dist: block");
    assert!(
        dist.homebrew.is_none(),
        "PMAT-1014: dist-forjar.yaml still declares a homebrew tap for forjar itself"
    );
}
