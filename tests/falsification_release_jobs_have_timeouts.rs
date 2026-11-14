//! PMAT-1027: every release.yml job bounds its runtime, and proofs.yml
//! parses `cargo metadata` with jq, not an inline Python one-liner.
//!
//! A job with no `timeout-minutes` holds a clean-room runner for GitHub's
//! 360 minute default when it hangs. The bounds below are about 2x the
//! durations measured on a real tag run.

use std::fs;
use std::path::Path;

fn read(rel: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

#[test]
fn every_release_job_declares_a_timeout() {
    let doc: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&read(".github/workflows/release.yml")).expect("parse release.yml");
    let jobs = doc
        .get("jobs")
        .and_then(|j| j.as_mapping())
        .expect("jobs map");
    let expected = [
        ("verify", 15),
        ("create-release", 10),
        ("build-binaries", 70),
        ("checksums", 10),
        ("dist-artifacts", 15),
        ("publish-release", 10),
    ];
    for (name, mins) in expected {
        let job = jobs
            .get(name)
            .unwrap_or_else(|| panic!("release.yml has no job `{name}`"));
        let got = job.get("timeout-minutes").and_then(|v| v.as_u64());
        assert_eq!(got, Some(mins), "release.yml job `{name}` timeout-minutes");
    }
    for (k, _) in jobs {
        let k = k.as_str().unwrap_or_default();
        assert!(
            expected.iter().any(|(n, _)| *n == k),
            "release.yml job `{k}` is not covered by this test; add its timeout here"
        );
    }
}

#[test]
fn proofs_workflow_has_no_python() {
    let text = read(".github/workflows/proofs.yml");
    for (i, line) in text.lines().enumerate() {
        assert!(
            !line.contains("python3") && !line.contains("python -"),
            "proofs.yml:{} uses Python: {line}",
            i + 1
        );
    }
}
