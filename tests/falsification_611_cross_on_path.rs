//! Refs #611: an aarch64 Linux leg that installs `cross` must be able to RUN it.
//!
//! THE FLAW THIS CLOSES. `build-binaries` sets a private
//! `CARGO_HOME: ${{ github.workspace }}/../cargo-home-${{ github.job }}`
//! (infra#430). On a runner without `cross`, "Install target prerequisites
//! (Linux)" ran `cargo install cross`, which landed in `$CARGO_HOME/bin`, and
//! nothing put that directory on PATH. The next step died with
//! `cross: command not found` (exit 127), `checksums` and `publish-release`
//! skipped, and the release stayed a draft with 0 assets: v1.32.0, and
//! v1.33.0-rc.1 twice on yoga-build.
//!
//! AND AGAIN IN nightly.yml. This test used to read ONE step of ONE file, so
//! the same install in nightly.yml's `build` job (same private CARGO_HOME, no
//! `$GITHUB_PATH` line) stayed invisible to it: the aarch64 nightly legs were
//! green only while they landed on intel, which already had `cross`, and went
//! red with exit 127 the day they landed on yoga-build (2026-10-05). The test
//! now finds EVERY step in EVERY workflow that runs `cargo install cross`, and
//! refuses to pass if it finds fewer than the two known sites.
//!
//! WHAT THIS TEST MUST NOT BECOME. Grepping release.yml for `GITHUB_PATH`
//! stays green if the line writes the wrong directory or sits in a branch the
//! aarch64 legs never take. This test RUNS the step's own script, with a fake
//! `cargo` that installs `cross` where the real one does, then replays what
//! the runner does between steps (prepend each `$GITHUB_PATH` line to PATH)
//! and asks the next step's shell whether `cross` resolves.

use std::path::Path;
use std::process::Command;

/// Every `(file:job:step, run)` whose script runs `cargo install cross`.
fn cross_install_steps(target: &str) -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .expect("read .github/workflows")
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "yml" || x == "yaml"))
        .collect();
    files.sort();
    let mut sites = Vec::new();
    for p in files {
        let text = std::fs::read_to_string(&p).expect("read workflow");
        let doc: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text).expect("parse workflow");
        let Some(jobs) = doc["jobs"].as_mapping() else {
            continue;
        };
        for (job, body) in jobs {
            for step in body["steps"].as_sequence().into_iter().flatten() {
                let Some(run) = step["run"].as_str() else {
                    continue;
                };
                if run.contains("cargo install cross") {
                    let label = format!(
                        "{}:{}:{}",
                        p.file_name().unwrap().to_string_lossy(),
                        job.as_str().unwrap_or("?"),
                        step["name"].as_str().unwrap_or("?")
                    );
                    sites.push((label, run.replace("${{ matrix.target }}", target)));
                }
            }
        }
    }
    assert!(
        sites.len() >= 2,
        "expected release.yml and nightly.yml to install cross, found {sites:?}"
    );
    sites
}

fn write_exe(path: &Path, body: &str) {
    std::fs::write(path, format!("#!/bin/sh\n{body}\n")).expect("write fake");
    Command::new("chmod")
        .arg("+x")
        .arg(path)
        .status()
        .expect("chmod");
}

/// Runs one install step script on a runner without `cross`, then
/// returns whether the NEXT step's shell resolves `cross`, and the
/// `$GITHUB_PATH` file for the failure message.
fn cross_resolves_in_next_step(script: &str) -> (bool, String) {
    let dir = tempfile::tempdir().expect("tempdir");
    let fake = dir.path().join("fakebin");
    std::fs::create_dir(&fake).expect("fakebin");
    let cargo_home = dir.path().join("cargo-home-build-binaries");
    let gh_path = dir.path().join("github_path");
    std::fs::write(&gh_path, "").expect("github_path");
    // The real `cargo install` writes into $CARGO_HOME/bin and nowhere else.
    write_exe(
        &fake.join("cargo"),
        "[ \"$1 $2\" = 'install cross' ] || exit 0\nmkdir -p \"$CARGO_HOME/bin\"\n\
         printf '#!/bin/sh\\nexit 0\\n' > \"$CARGO_HOME/bin/cross\"\n\
         chmod +x \"$CARGO_HOME/bin/cross\"",
    );
    for name in ["rustup", "sudo", "apt-get"] {
        write_exe(&fake.join(name), "exit 0");
    }
    let base_path = format!("{}:/usr/bin:/bin", fake.display());
    let run = |script: &str, path: &str| {
        Command::new("bash")
            .args(["-e", "-c", script])
            .env_clear()
            .env("PATH", path)
            .env("HOME", dir.path())
            .env("CARGO_HOME", &cargo_home)
            .env("GITHUB_PATH", &gh_path)
            .status()
            .expect("spawn bash")
    };
    assert!(
        !run("command -v cross", &base_path).success(),
        "precondition: this runner must not already have cross"
    );
    assert!(
        run(script, &base_path).success(),
        "the install step itself failed"
    );
    // The runner prepends every $GITHUB_PATH line, last written first.
    let added = std::fs::read_to_string(&gh_path).expect("read github_path");
    let mut next_path: Vec<&str> = added.lines().filter(|l| !l.is_empty()).collect();
    next_path.reverse();
    next_path.push(&base_path);
    let ok = run("command -v cross", &next_path.join(":")).success();
    (ok, added)
}

fn every_install_step_puts_cross_on_path(target: &str) {
    let broken: Vec<String> = cross_install_steps(target)
        .into_iter()
        .filter_map(|(site, script)| {
            let (ok, added) = cross_resolves_in_next_step(&script);
            (!ok)
                .then(|| format!("{site}: cross installed but off PATH; GITHUB_PATH was {added:?}"))
        })
        .collect();
    assert!(broken.is_empty(), "{}", broken.join("\n"));
}

#[test]
fn an_aarch64_gnu_leg_that_installs_cross_can_run_it_in_the_next_step() {
    every_install_step_puts_cross_on_path("aarch64-unknown-linux-gnu");
}

#[test]
fn an_aarch64_musl_leg_that_installs_cross_can_run_it_in_the_next_step() {
    every_install_step_puts_cross_on_path("aarch64-unknown-linux-musl");
}
