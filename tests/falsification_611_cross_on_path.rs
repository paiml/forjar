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

use std::path::Path;
use std::process::Command;

const STEP: &str = "Install target prerequisites (Linux)";

fn install_step_script(target: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/release.yml");
    let text = std::fs::read_to_string(&p).expect("read release.yml");
    let doc: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text).expect("parse release.yml");
    let steps = doc["jobs"]["build-binaries"]["steps"]
        .as_sequence()
        .expect("build-binaries.steps");
    let run = steps
        .iter()
        .find(|s| s["name"].as_str() == Some(STEP))
        .and_then(|s| s["run"].as_str())
        .unwrap_or_else(|| panic!("build-binaries has no step {STEP:?} with a run:"));
    run.replace("${{ matrix.target }}", target)
}

fn write_exe(path: &Path, body: &str) {
    std::fs::write(path, format!("#!/bin/sh\n{body}\n")).expect("write fake");
    Command::new("chmod")
        .arg("+x")
        .arg(path)
        .status()
        .expect("chmod");
}

/// Runs the install step for `target` on a runner without `cross`, then
/// returns whether the NEXT step's shell resolves `cross`, and the
/// `$GITHUB_PATH` file for the failure message.
fn cross_resolves_in_next_step(target: &str) -> (bool, String) {
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
        run(&install_step_script(target), &base_path).success(),
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
fn an_aarch64_gnu_leg_that_installs_cross_can_run_it_in_the_next_step() {
    let (ok, added) = cross_resolves_in_next_step("aarch64-unknown-linux-gnu");
    assert!(
        ok,
        "cross installed but off PATH; GITHUB_PATH was: {added:?}"
    );
}

#[test]
fn an_aarch64_musl_leg_that_installs_cross_can_run_it_in_the_next_step() {
    let (ok, added) = cross_resolves_in_next_step("aarch64-unknown-linux-musl");
    assert!(
        ok,
        "cross installed but off PATH; GITHUB_PATH was: {added:?}"
    );
}
