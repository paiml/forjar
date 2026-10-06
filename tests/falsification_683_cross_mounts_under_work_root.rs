//! Refs #683: every path `cross` bind-mounts must exist on the DOCKER HOST.
//!
//! THE FLAW THIS CLOSES. `cross` starts its build container through whatever
//! docker daemon the job can reach, and mounts the toolchain sysroot at
//! `/rust` — `cargo` inside is `/rust/bin/cargo`. Some clean-room runners are
//! themselves containers: they hold the host's docker socket, and only their
//! work root is bind-mounted, at the SAME path on the host. A path outside
//! that work root exists in the runner container and nowhere the daemon can
//! see, so the daemon mounts an empty directory instead.
//!
//! The runner image sets `RUSTUP_HOME=/home/runner/.rustup` (an image-layer
//! path). With no job-level `RUSTUP_HOME`, cross mounted it, got an empty
//! `/rust`, and died with `sh: 1: cargo: not found` (exit 127): nightly run
//! 37511323051, aarch64-gnu on a containerized runner. Every aarch64 leg in four nightlies
//! that landed on a containerized runner failed and every one on a native
//! runner passed, so the leg's colour was a function of scheduling.
//! `CARGO_HOME` never had the problem: an earlier change already put it under
//! `${{ github.workspace }}/..`.
//!
//! WHAT THIS TEST MUST NOT BECOME. A grep for `RUSTUP_HOME` stays green if the
//! value points somewhere the daemon cannot see, if it is set on a different
//! job, or if a fourth workflow starts calling cross. This test DISCOVERS every
//! job of every workflow whose steps run `cross build`, resolves the env that
//! job's `cross build` step actually sees (workflow, then job, then step; an
//! unset `RUSTUP_HOME` falls back to `$HOME/.rustup` as rustup does), expands
//! `${{ github.workspace }}` and `${{ github.job }}` the way the runner does,
//! and asks whether each mounted path lies under the work root — the one tree
//! a containerized runner shares with its host.
//!
//! It does not start docker: the mount is the host daemon's, and no test on a
//! native runner can observe it. What it checks is the precondition the
//! daemon needs, on the exact env the step runs with.

use std::path::{Component, Path, PathBuf};

/// The runner container's work root, identity-mounted on the host.
const WORK_ROOT: &str = "/srv/runner-work";
/// `$HOME` inside the runner container: NOT under the work root.
const RUNNER_HOME: &str = "/home/runner";

/// One job that runs `cross build`: `workflow:job`, and the env its
/// `cross build` step sees, before expression expansion.
struct CrossJob {
    id: String,
    job: String,
    env: std::collections::BTreeMap<String, String>,
}

fn env_of(v: &serde_yaml_ng::Value) -> Vec<(String, String)> {
    v["env"]
        .as_mapping()
        .into_iter()
        .flatten()
        .filter_map(|(k, v)| Some((k.as_str()?.to_string(), v.as_str()?.to_string())))
        .collect()
}

fn cross_jobs() -> Vec<CrossJob> {
    let wf = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows");
    let mut files: Vec<_> = std::fs::read_dir(&wf)
        .expect("read .github/workflows")
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "yml" || x == "yaml"))
        .collect();
    files.sort();
    let mut found = Vec::new();
    for p in files {
        let text = std::fs::read_to_string(&p).expect("read workflow");
        let doc: serde_yaml_ng::Value =
            serde_yaml_ng::from_str(&text).unwrap_or_else(|e| panic!("parse {}: {e}", p.display()));
        let file = p.file_name().unwrap().to_string_lossy().into_owned();
        let Some(jobs) = doc["jobs"].as_mapping() else {
            continue;
        };
        for (job, body) in jobs {
            let job = job.as_str().unwrap_or("?").to_string();
            for step in body["steps"].as_sequence().into_iter().flatten() {
                if !step["run"]
                    .as_str()
                    .is_some_and(|r| r.contains("cross build"))
                {
                    continue;
                }
                let mut env = std::collections::BTreeMap::new();
                for layer in [&doc, body, step] {
                    env.extend(env_of(layer));
                }
                found.push(CrossJob {
                    id: format!("{file}:{job}"),
                    job: job.clone(),
                    env,
                });
            }
        }
    }
    found
}

/// Expand the two expressions these values use, as the runner would for a
/// checkout of `forjar` under the work root.
fn expand(value: &str, job: &str) -> String {
    let workspace = format!("{WORK_ROOT}/_work/forjar/forjar");
    value
        .replace("${{ github.workspace }}", &workspace)
        .replace("${{ github.job }}", job)
}

/// `a/b/../c` → `a/c`, without touching the filesystem (none of it exists).
fn normalize(p: &str) -> PathBuf {
    let mut out = PathBuf::new();
    for c in Path::new(p).components() {
        match c {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other),
        }
    }
    out
}

/// The host paths `cross` bind-mounts that come from the job's env, resolved
/// as rustup and cargo resolve them when the variable is unset.
fn mounted_paths(j: &CrossJob) -> Vec<(&'static str, PathBuf)> {
    let rustup = j
        .env
        .get("RUSTUP_HOME")
        .map(|v| expand(v, &j.job))
        .unwrap_or_else(|| format!("{RUNNER_HOME}/.rustup"));
    let cargo = j
        .env
        .get("CARGO_HOME")
        .map(|v| expand(v, &j.job))
        .unwrap_or_else(|| format!("{RUNNER_HOME}/.cargo"));
    vec![
        ("RUSTUP_HOME (sysroot → /rust)", normalize(&rustup)),
        ("CARGO_HOME (→ /cargo)", normalize(&cargo)),
    ]
}

fn outside_work_root(j: &CrossJob) -> Vec<String> {
    mounted_paths(j)
        .into_iter()
        .filter(|(_, p)| !p.starts_with(WORK_ROOT))
        .map(|(what, p)| format!("{}: {what} = {}", j.id, p.display()))
        .collect()
}

#[test]
fn discovery_finds_every_known_cross_job() {
    let ids: Vec<String> = cross_jobs().into_iter().map(|j| j.id).collect();
    for known in [
        "binary-release.yml:build",
        "nightly.yml:build",
        "release.yml:build-binaries",
    ] {
        assert!(
            ids.iter().any(|i| i == known),
            "discovery lost {known}; found {ids:?}"
        );
    }
}

#[test]
fn every_path_cross_mounts_is_under_the_work_root() {
    let bad: Vec<String> = cross_jobs().iter().flat_map(outside_work_root).collect();
    assert!(
        bad.is_empty(),
        "#683: on a containerized runner the host daemon cannot see these, \
         mounts an empty dir, and cross dies with `cargo: not found`:\n  {}",
        bad.join("\n  ")
    );
}

/// The checker must be able to fail: the env every cross job had before #683.
#[test]
fn the_pre_683_env_is_caught() {
    let mut env = std::collections::BTreeMap::new();
    env.insert(
        "CARGO_HOME".to_string(),
        "${{ github.workspace }}/../cargo-home-${{ github.job }}".to_string(),
    );
    let j = CrossJob {
        id: "fixture:build".to_string(),
        job: "build".to_string(),
        env,
    };
    let bad = outside_work_root(&j);
    assert_eq!(bad.len(), 1, "{bad:?}");
    assert!(bad[0].contains("RUSTUP_HOME"), "{bad:?}");
    // And `..` cannot walk the path out unnoticed.
    let mut env = j.env.clone();
    env.insert(
        "RUSTUP_HOME".to_string(),
        "${{ github.workspace }}/../../../../../tmp/rustup".to_string(),
    );
    let escaped = CrossJob { env, ..j };
    assert_eq!(outside_work_root(&escaped).len(), 1);
}

// ---------------------------------------------------------------------------
// The other half of the fix: a per-job RUSTUP_HOME starts EMPTY.
//
// Moving RUSTUP_HOME under the work root also moves it away from the toolchain
// the runner image installed, so a job that sets it has no rustup toolchain
// until one of its own steps installs one. A leg that reaches `cargo`,
// `rustup` or `cross` first fails before it builds anything. The path test
// above cannot see that: the path is correct and the directory is empty. This
// checks, per matrix leg, that a toolchain install the leg actually runs comes
// before the first step that needs one.

/// Evaluate a step's `if:` for one leg. `Some(true)`/`Some(false)` when the
/// expression is one of the forms these workflows use, `None` when it is not.
fn step_runs(cond: Option<&str>, target: &str) -> Option<bool> {
    let Some(cond) = cond else {
        return Some(true);
    };
    let c = cond
        .trim()
        .trim_start_matches("${{")
        .trim_end_matches("}}")
        .trim();
    let quoted = |s: &str| s.trim().trim_matches('\'').to_string();
    if let Some(arg) = c
        .strip_prefix("contains(matrix.target,")
        .and_then(|r| r.strip_suffix(')'))
    {
        return Some(target.contains(&quoted(arg)));
    }
    if let Some(arg) = c.strip_prefix("matrix.target ==") {
        return Some(target == quoted(arg));
    }
    None
}

fn is_toolchain_install(step: &serde_yaml_ng::Value) -> bool {
    step["uses"]
        .as_str()
        .is_some_and(|u| u.starts_with("dtolnay/rust-toolchain"))
        || step["run"]
            .as_str()
            .is_some_and(|r| r.contains("sh.rustup.rs"))
}

/// Does this step's script invoke a command that needs a rustup toolchain?
fn needs_toolchain(step: &serde_yaml_ng::Value) -> bool {
    step["run"].as_str().is_some_and(|r| {
        r.split(|c: char| c.is_whitespace() || ";|&()`\"'".contains(c))
            .any(|t| matches!(t, "cargo" | "rustup" | "cross"))
    })
}

fn matrix_targets(job: &serde_yaml_ng::Value) -> Vec<String> {
    let m = &job["strategy"]["matrix"];
    let mut t: Vec<String> = m["include"]
        .as_sequence()
        .into_iter()
        .flatten()
        .filter_map(|leg| leg["target"].as_str().map(str::to_string))
        .chain(
            m["target"]
                .as_sequence()
                .into_iter()
                .flatten()
                .filter_map(|v| v.as_str().map(str::to_string)),
        )
        .collect();
    if t.is_empty() {
        t.push(String::new());
    }
    t
}

/// Every leg, of every job whose workflow- or job-level env sets RUSTUP_HOME,
/// that needs a toolchain before it has installed one.
fn toolchain_gaps(file: &str, doc: &serde_yaml_ng::Value) -> Vec<String> {
    let wf_sets = !doc["env"]["RUSTUP_HOME"].is_null();
    let mut gaps = Vec::new();
    for (name, job) in doc["jobs"].as_mapping().into_iter().flatten() {
        if !wf_sets && job["env"]["RUSTUP_HOME"].is_null() {
            continue;
        }
        let name = name.as_str().unwrap_or("?");
        for target in matrix_targets(job) {
            let mut installed = false;
            for step in job["steps"].as_sequence().into_iter().flatten() {
                let runs = step_runs(step["if"].as_str(), &target);
                if is_toolchain_install(step) {
                    installed |= runs == Some(true);
                    continue;
                }
                if !installed && runs != Some(false) && needs_toolchain(step) {
                    gaps.push(format!(
                        "{file}:{name} [{target}] step {:?} needs a toolchain \
                         and none is installed in the job's RUSTUP_HOME",
                        step["name"].as_str().unwrap_or("?")
                    ));
                    break;
                }
            }
        }
    }
    gaps
}

#[test]
fn every_job_that_sets_rustup_home_installs_a_toolchain_first() {
    let wf = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows");
    let mut checked = 0;
    let mut gaps = Vec::new();
    for e in std::fs::read_dir(&wf).expect("read .github/workflows") {
        let p = e.expect("dir entry").path();
        if !p.extension().is_some_and(|x| x == "yml" || x == "yaml") {
            continue;
        }
        let doc: serde_yaml_ng::Value =
            serde_yaml_ng::from_str(&std::fs::read_to_string(&p).expect("read workflow"))
                .unwrap_or_else(|e| panic!("parse {}: {e}", p.display()));
        let file = p.file_name().unwrap().to_string_lossy().into_owned();
        checked += doc["jobs"]
            .as_mapping()
            .into_iter()
            .flatten()
            .filter(|(_, j)| !j["env"]["RUSTUP_HOME"].is_null())
            .count();
        gaps.extend(toolchain_gaps(&file, &doc));
    }
    // Every cross job sets it, so fewer than three means discovery broke.
    assert!(checked >= 3, "only {checked} job(s) set RUSTUP_HOME");
    assert!(
        gaps.is_empty(),
        "#683: a per-job RUSTUP_HOME starts empty:\n  {}",
        gaps.join("\n  ")
    );
}

/// The checker must be able to fail: the shape `release.yml` had, where only
/// the macOS legs installed a toolchain.
#[test]
fn a_leg_with_no_toolchain_install_is_caught() {
    let doc: serde_yaml_ng::Value = serde_yaml_ng::from_str(
        r#"
jobs:
  build-binaries:
    strategy:
      matrix:
        include:
          - target: x86_64-unknown-linux-gnu
          - target: aarch64-apple-darwin
    env:
      RUSTUP_HOME: ${{ github.workspace }}/../rustup-home-${{ github.job }}
    steps:
      - uses: actions/checkout@v4
      - name: Install Rust toolchain (macOS)
        if: contains(matrix.target, 'darwin')
        run: curl -sSf https://sh.rustup.rs | sh -s -- -y
      - name: Discover binary names
        run: BINS=$(cargo metadata --no-deps --format-version 1)
"#,
    )
    .expect("fixture parses");
    let gaps = toolchain_gaps("fixture.yml", &doc);
    assert_eq!(gaps.len(), 1, "{gaps:?}");
    assert!(gaps[0].contains("x86_64-unknown-linux-gnu"), "{gaps:?}");
    assert!(gaps[0].contains("Discover binary names"), "{gaps:?}");
}
