//! paiml/infra#775: `Swatinem/rust-cache`'s SAVE step deletes the registry
//! SOURCES, and on a self-hosted runner those sources are shared.
//!
//! `cache-bin: "false"` is the documented mitigation and it is not the fix. It
//! skips `cleanBin` and nothing else; the same save step runs `cleanRegistry`
//! unconditionally and removes `${CARGO_HOME}/registry/src` — the extracted
//! crate sources every OTHER job on the box is compiling from at that instant.
//! All sixteen `intel-clean-room-*` runner services share `HOME=/home/noah`.
//!
//! Measured on intel 2026-09-19: eight save steps in 100 minutes, every crate
//! directory under the shared registry carrying a birth time pinned to the
//! second one of them finished, and an infra `cargo kani` build killed
//! mid-crate with `could not parse/generate dep info at .../serde_core-<hash>.d:
//! No such file or directory`.
//!
//! THE ONLY EXONERATION IS A JOB-PRIVATE `CARGO_HOME`, and that is the whole
//! content of the rule below. `--target-dir`, `cargo install --root` and a
//! fresh `$HOME` all leave cargo reading sources from the shared
//! `$CARGO_HOME`, so none of them moves the thing the save step deletes.
//!
//! ONE IMPLEMENTATION, TWO CONSUMERS. This test used to re-implement the rule
//! in Rust, "deliberately redundant" with `scripts/lint-rust-cache-guard.sh`.
//! Review round 1 on forjar#589 showed what redundancy buys: the two disagreed
//! — the shell lint decided per FILE (one job's private CARGO_HOME exonerated
//! a second self-hosted job; a hosted job was flagged for sharing a file), the
//! Rust copy let a COMMENT mentioning `CARGO_HOME:` exonerate a job. Two
//! parsers of one rule are two lists with nothing tying them together
//! (bashrs#266's root cause). So the rule lives once, in the lint, which now
//! decides per job and carries a fixture per shape; this test RUNS it — its
//! self-test first, then the tree — so `cargo test` still fails if the lint is
//! unwired from `make lint`, deleted, or made vacuous. The quorum gate reverts
//! and observes THIS target.

use std::path::Path;
use std::process::Command;

fn lint(args: &[&str]) -> (i32, String) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let script = root.join("scripts/lint-rust-cache-guard.sh");
    assert!(
        script.is_file(),
        "{} is missing: the rule this test enforces lives there",
        script.display()
    );
    let out = Command::new("bash")
        .arg(&script)
        .args(args)
        .current_dir(root)
        .output()
        .expect("bash must run");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.code().unwrap_or(-1), text)
}

/// The number before `label` in the lint's summary line, e.g. `46 self-hosted job(s)`.
fn count_before(text: &str, label: &str) -> usize {
    let at = text
        .find(label)
        .unwrap_or_else(|| panic!("summary has no `{label}`:\n{text}"));
    text[..at]
        .split_whitespace()
        .last()
        .and_then(|w| w.trim_start_matches('(').parse().ok())
        .unwrap_or_else(|| panic!("no count before `{label}`:\n{text}"))
}

#[test]
fn the_lint_can_still_reject() {
    // A lint nobody has seen fail is evidence of nothing: its fixtures include
    // every shape review found it wrong on, each with the verdict it must give.
    let (rc, text) = lint(&["--selftest"]);
    assert_eq!(rc, 0, "the lint's self-test failed:\n{text}");
    let cases = count_before(&text, "cases)");
    assert!(
        cases >= 12,
        "self-test ran {cases} case(s); the shapes need at least 12:\n{text}"
    );
}

#[test]
fn a_self_hosted_job_caching_cargo_must_own_its_cargo_home() {
    let (rc, text) = lint(&[]);
    assert_eq!(
        rc, 0,
        "a self-hosted job caches the SHARED CARGO_HOME. Its save step deletes \
         ${{CARGO_HOME}}/registry/src — and cleanBin empties ~/.cargo/bin — for every \
         other job on the box (paiml/infra#775). `cache-bin: \"false\"` does not prevent \
         it. The lint says:\n{text}"
    );
    // ANTI-VACUITY: an OK over nothing is the shape this fleet keeps finding.
    let jobs = count_before(&text, "self-hosted job(s)");
    let workflows = count_before(&text, "workflow(s)");
    let uses = count_before(&text, "$CARGO_HOME cache use(s)");
    assert!(
        workflows >= 10,
        "lint scanned {workflows} workflow(s):\n{text}"
    );
    assert!(
        jobs >= 10,
        "lint classified {jobs} self-hosted job(s):\n{text}"
    );
    assert!(
        uses >= 1,
        "no cache use found at all — quorum.yml's is expected, so the matcher is blind:\n{text}"
    );
}

/// The negative control, on the REAL workflow rather than a fixture: copy
/// every workflow, delete `quorum.yml`'s private CARGO_HOME, and the lint must
/// refuse exactly that job by name. Without this the green test above could
/// be green for a reason other than the fix (a matcher that stopped seeing
/// quorum.yml, say) — a control has to fail for the reason it names.
#[test]
fn quorum_without_its_private_cargo_home_is_refused_by_name() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let src = root.join(".github/workflows");
    let tmp = std::env::temp_dir().join(format!(
        "forjar-588-negative-control-{}",
        std::process::id()
    ));
    let wf = tmp.join(".github/workflows");
    std::fs::create_dir_all(&wf).expect("temp workflows dir");
    let mut removed = 0;
    for entry in std::fs::read_dir(&src).expect("read workflows") {
        let path = entry.expect("dir entry").path();
        let name = path.file_name().expect("file name").to_owned();
        let mut text = std::fs::read_to_string(&path).expect("read workflow");
        if name == "quorum.yml" {
            let kept: Vec<&str> = text
                .lines()
                .filter(|l| {
                    let drop = l.trim_start().starts_with("CARGO_HOME:");
                    if drop {
                        removed += 1;
                    }
                    !drop
                })
                .collect();
            text = kept.join("\n") + "\n";
        }
        std::fs::write(wf.join(&name), text).expect("write workflow copy");
    }
    let script = root.join("scripts/lint-rust-cache-guard.sh");
    let out = Command::new("bash")
        .arg(&script)
        .current_dir(&tmp)
        .output()
        .expect("bash must run");
    let _ = std::fs::remove_dir_all(&tmp);
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        removed, 1,
        "expected exactly one CARGO_HOME declaration in quorum.yml to remove"
    );
    assert_ne!(
        out.status.code(),
        Some(0),
        "with quorum.yml's private CARGO_HOME removed the lint still passed:\n{text}"
    );
    assert!(
        text.contains("quorum.yml") && text.contains("(job `receipt`)"),
        "the lint failed, but not naming quorum.yml job `receipt`:\n{text}"
    );
}
