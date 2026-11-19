//! Spot (burst) labels, judged on the PARSED workflow documents, never on raw lines.
//!
//! Every `.yml` and `.yaml` under `.github/workflows/` is parsed with the crate's own YAML
//! dependency (serde_yaml_ng), so quoted keys, `\x73` escapes, flow style, explicit `? key`
//! entries and anchors all resolve before anything is judged. Rules:
//!  (a) a job's resolved `runs-on` (string, list, or labels map) may name 'burst' or
//!      'pool-canary', or hold a `${{` expression, ONLY for an allowlisted ci.yml job id and ONLY
//!      when it equals that job's one canonical value. A matrix reference is allowed only when
//!      every matrix value is a plain literal without those names. A reusable-workflow call must
//!      not pass such a value in `with`.
//!  (b) an allowlisted job has resolved permissions exactly {contents: read}, no scalar naming
//!      'secrets' or 'id-token', and ci.yml has no workflow-level scalar naming 'secrets' and no
//!      workflow permissions wider than contents: read.
//!  (c) pool-canary.yml parses equal to tests/pool-canary.expected.yml and is dispatch-only.
//!  (e) EVERY job of EVERY workflow, whatever its trigger: its labels must include a pool this
//!      repository already uses, or a GitHub-hosted runner label; runner groups are refused.
//!  (f) a `uses:` call: local files are judged with the caller context (and may pass no secrets);
//!      an external call must match an allowlist entry on its call site, exact pinned `uses`
//!      string, `secrets` form and inputs (required values exact, anything unlisted red).
//!  (d) cannot judge = red: parse failure, duplicate keys, a merge key or tag, a non-mapping
//!      document or jobs section, an unmodeled runs-on shape, an empty or missing directory.

mod judge;
mod model;
mod routing;
mod t_calls;
mod t_class;
mod t_core;
mod t_labels;
mod t_yaml;
mod wiring;

pub(crate) use judge::*;
pub(crate) use model::*;
pub(crate) use routing::*;
pub(crate) use serde_yaml_ng::Value;
pub(crate) use std::fs;
pub(crate) use std::path::{Path, PathBuf};
pub(crate) use t_core::*;
pub(crate) use t_labels::*;

pub(crate) fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub(crate) fn expected_canary() -> String {
    fs::read_to_string(repo().join("tests/pool-canary.expected.yml")).unwrap()
}

pub(crate) fn guard(dir: &Path) -> bool {
    match judge(dir, &expected_canary()) {
        Ok(_) => true,
        Err(w) => {
            eprintln!("guard refused: {}", w.join(" | "));
            false
        }
    }
}

/// One directory per call, never shared between tests that run on parallel threads (the r5 flake:
/// two tests used one name per process and one deleted the other's tree). Lives under the cargo
/// target tmp dir, not the system temp dir, so nothing leaks outside the build tree.
pub(crate) fn unique_dir(name: &str) -> PathBuf {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let i = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("forjar-spot-{name}-{i}"))
}

pub(crate) fn scratch(name: &str, ci: &str) -> PathBuf {
    let d = unique_dir(name);
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    fs::write(d.join("ci.yml"), ci).unwrap();
    d
}

pub(crate) fn red(name: &str, ci: &str) {
    assert!(!guard(&scratch(name, ci)), "mutant {name} was accepted");
}

/// Red for the NAMED reason, so a mutant cannot pass by tripping some other rule.
pub(crate) fn red_why(name: &str, ci: &str, needle: &str) {
    match judge(&scratch(name, ci), &expected_canary()) {
        Ok(_) => panic!("mutant {name} was accepted"),
        Err(w) => assert!(
            w.iter().any(|x| x.contains(needle)),
            "mutant {name} refused for the wrong reason (want `{needle}`): {w:?}"
        ),
    }
}

pub(crate) fn ci_text() -> String {
    fs::read_to_string(repo().join(".github/workflows/ci.yml")).unwrap()
}

/// Replace the first `from` that sits inside job `job` of the real ci.yml.
pub(crate) fn job_mut(job: &str, from: &str, to: &str) -> String {
    let ci = ci_text();
    let at = ci.find(&format!("\n  {job}:\n")).expect("job present");
    let (a, b) = ci.split_at(at);
    let m = b.replacen(from, to, 1);
    assert_ne!(b, m, "mutation must apply in {job}: {from}");
    format!("{a}{m}")
}

pub(crate) fn real_mut(from: &str, to: &str) -> String {
    let ci = ci_text();
    let m = ci.replacen(from, to, 1);
    assert_ne!(ci, m, "mutation must apply: {from}");
    m
}
