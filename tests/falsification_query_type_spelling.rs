//! `forjar query` names resource types in a spelling no document can write
//! (paiml/forjar#366, companion surface).
//!
//! `execute_query` reported `format!("{:?}", res.resource_type)` — the Rust
//! `Debug` spelling — and both filters (`--type`, `--pattern`) matched against
//! its lowercase. For the fifteen single-word variants that reads as the serde
//! name with a capital letter; for the six multi-word ones it is a string serde
//! REJECTS:
//!
//! ```text
//! $ forjar query --type github_release --json   ->  "total": 0
//! $ forjar query --type githubrelease  --json   ->  "total": 1
//! ```
//!
//! So the one spelling a `type:` key accepts found nothing, and the JSON
//! answered with `"resource_type": "GithubRelease"` — a value that round-trips
//! into no forjar surface.
//!
//! This is a deliberate OUTPUT-CONTRACT change and rides in its own commit: it
//! also renames `"Package"` to `"package"` and `pkg [Package]` to
//! `pkg [package]` for every variant, which anyone parsing `forjar query` will
//! see. `--type GithubRelease` stops matching; `--type github_release`,
//! `--type github` and `--type package` all work.
//!
//! Usage: cargo test --test falsification_query_type_spelling

use std::path::{Path, PathBuf};

const FIXTURE: &str = r#"
version: "1.0"
name: query-spelling
machines:
  local:
    hostname: localhost
    addr: localhost
resources:
  release:
    type: github_release
    machine: local
    repo: paiml/forjar
    binary: forjar
  tools:
    type: package
    machine: local
    provider: apt
    packages: [git]
"#;

fn fixture(dir: &Path) -> PathBuf {
    let cfg = dir.join("forjar.yaml");
    std::fs::write(&cfg, FIXTURE).unwrap();
    cfg
}

fn query(cfg: &Path, state: &Path, ty: &str) -> serde_json::Value {
    let run = std::process::Command::new(env!("CARGO_BIN_EXE_forjar"))
        .args(["query", "--file"])
        .arg(cfg)
        .arg("--state-dir")
        .arg(state)
        .args(["--type", ty, "--json"])
        .output()
        .expect("forjar query runs");
    assert!(
        run.status.success(),
        "forjar query --type {ty} failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    serde_json::from_slice(&run.stdout).unwrap_or_else(|e| {
        panic!(
            "--json did not print JSON ({e}): {}",
            String::from_utf8_lossy(&run.stdout)
        )
    })
}

#[test]
fn the_serde_spelling_of_a_multi_word_type_finds_the_resource() {
    let d = tempfile::tempdir().unwrap();
    let cfg = fixture(d.path());

    let out = query(&cfg, d.path(), "github_release");
    assert_eq!(
        out["total"], 1,
        "`--type github_release` matched nothing. That is the ONLY spelling the document's own \
         `type:` key accepts — serde rejects every other with `unknown variant` — so the filter \
         answers to a name the config cannot use (paiml/forjar#366): {out}"
    );
    assert_eq!(
        out["matches"][0]["resource_id"], "release",
        "the github_release resource is the one that matches: {out}"
    );
}

#[test]
fn the_reported_type_is_the_one_the_document_declared() {
    let d = tempfile::tempdir().unwrap();
    let cfg = fixture(d.path());

    let out = query(&cfg, d.path(), "github");
    assert_eq!(out["total"], 1, "substring matching still works: {out}");
    assert_eq!(
        out["matches"][0]["resource_type"], "github_release",
        "`forjar query --json` reported a resource type in a spelling no forjar surface accepts \
         — `type: GithubRelease` is an `unknown variant` error: {out}"
    );
}

/// Vacuity guard: the fifteen single-word types are the ones that already
/// worked, and they must keep working — through the filter and in the report.
#[test]
fn single_word_types_are_unaffected() {
    let d = tempfile::tempdir().unwrap();
    let cfg = fixture(d.path());

    let out = query(&cfg, d.path(), "package");
    assert_eq!(out["total"], 1, "{out}");
    assert_eq!(out["matches"][0]["resource_id"], "tools", "{out}");
    assert_eq!(out["matches"][0]["resource_type"], "package", "{out}");
}
