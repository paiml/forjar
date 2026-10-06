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
//! WHAT THIS TEST MUST NOT BECOME. Grepping release.yml for `GITHUB_PATH`
//! stays green if the line writes the wrong directory or sits in a branch the
//! aarch64 legs never take. This test RUNS the step's own script, with a fake
//! `cargo` that installs `cross` where the real one does, then replays what
//! the runner does between steps (prepend each `$GITHUB_PATH` line to PATH)
//! and asks the next step's shell whether `cross` resolves.

//!
//! EVERY STEP, NOT ONE NAMED STEP (#671). The first version of this test read
//! the one step above by name, so the same defect in nightly.yml's "Install
//! cross (aarch64 legs only)" — same private CARGO_HOME, no `$GITHUB_PATH`
//! line — stayed red in the nightly and green here. The steps are now
//! DISCOVERED: every step of every workflow whose `run:` says
//! `cargo install cross`, and the discovery itself must find both known ones.

use std::path::Path;
use std::process::Command;

/// `(workflow:job:step, run script)` for every step that installs cross.
fn cross_install_steps() -> Vec<(String, String)> {
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
            let job = job.as_str().unwrap_or("?");
            for step in body["steps"].as_sequence().into_iter().flatten() {
                let Some(run) = step["run"].as_str() else {
                    continue;
                };
                if run.contains("cargo install cross") {
                    let name = step["name"].as_str().unwrap_or("<unnamed>");
                    found.push((format!("{file}:{job}:{name}"), run.to_string()));
                }
            }
        }
    }
    found
}
fn write_exe(path: &Path, body: &str) {
    std::fs::write(path, format!("#!/bin/sh\n{body}\n")).expect("write fake");
    Command::new("chmod")
        .arg("+x")
        .arg(path)
        .status()
        .expect("chmod");
}

/// Runs one install step for `target` on a runner without `cross`, then
/// returns whether the NEXT step's shell resolves `cross`, and the
/// `$GITHUB_PATH` file for the failure message.
fn cross_resolves_in_next_step(step_run: &str, target: &str) -> (bool, String) {
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
        run(
            &step_run.replace("${{ matrix.target }}", target),
            &base_path
        )
        .success(),
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

#[test]
fn discovery_finds_the_release_and_nightly_cross_installs() {
    let labels: Vec<String> = cross_install_steps().into_iter().map(|(l, _)| l).collect();
    for want in [
        "release.yml:build-binaries:Install target prerequisites (Linux)",
        "nightly.yml:build:Install cross (aarch64 legs only)",
    ] {
        assert!(
            labels.iter().any(|l| l == want),
            "discovery lost {want:?}; found {labels:?}"
        );
    }
}

fn every_cross_install_puts_cross_on_path(target: &str) {
    let steps = cross_install_steps();
    assert!(
        !steps.is_empty(),
        "no step installs cross: nothing measured"
    );
    let off_path: Vec<String> = steps
        .iter()
        .filter_map(|(label, run)| {
            let (ok, added) = cross_resolves_in_next_step(run, target);
            (!ok).then(|| format!("{label}: GITHUB_PATH was {added:?}"))
        })
        .collect();
    assert!(
        off_path.is_empty(),
        "{target}: cross installed but off PATH in the next step of {} of {} step(s):\n{}",
        off_path.len(),
        steps.len(),
        off_path.join("\n")
    );
}

#[test]
fn an_aarch64_gnu_leg_that_installs_cross_can_run_it_in_the_next_step() {
    every_cross_install_puts_cross_on_path("aarch64-unknown-linux-gnu");
}

#[test]
fn an_aarch64_musl_leg_that_installs_cross_can_run_it_in_the_next_step() {
    every_cross_install_puts_cross_on_path("aarch64-unknown-linux-musl");
}
