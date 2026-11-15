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

/// True when a workflow line runs or installs Python: any `python*` word
/// (`python3`, `python`, `python3.12`, `setup-python`) or a `pip`/`pip3` word.
/// A YAML comment is prose, not a step, so everything from a `#` that starts a
/// line or follows whitespace is ignored.
fn uses_python(line: &str) -> bool {
    let mut code = line;
    if let Some(i) = line.find(" #") {
        code = &line[..i];
    }
    if code.trim_start().starts_with('#') {
        return false;
    }
    code.to_ascii_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .any(|w| w.starts_with("python") || w == "pip" || w == "pip3")
}

#[test]
fn proofs_workflow_has_no_python() {
    let text = read(".github/workflows/proofs.yml");
    for (i, line) in text.lines().enumerate() {
        assert!(
            !uses_python(line),
            "proofs.yml:{} uses Python: {line}",
            i + 1
        );
    }
}

#[test]
fn the_python_detector_catches_every_form_and_skips_comments() {
    for bad in [
        "        run: python3 -c 'print(1)'",
        "          python - <<'PY'",
        "          python <<'PY'",
        "          python script.py",
        "          python3.12 tool.py",
        "        uses: actions/setup-python@v5",
        "          pip install pyyaml",
        "          pip3 install x",
        "          PYTHON=python3 make",
    ] {
        assert!(uses_python(bad), "detector missed: {bad}");
    }
    for ok in [
        "      # This step used to be a Python heredoc",
        "          jq -r '.target_directory' # was python3 -c",
        "          cargo metadata --format-version 1 --no-deps",
    ] {
        assert!(!uses_python(ok), "detector false positive: {ok}");
    }
}
