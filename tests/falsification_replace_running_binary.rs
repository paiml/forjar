//! forjar must be able to replace a binary that is CURRENTLY RUNNING.
//!
//! On Linux, `ETXTBSY` ("Text file busy") is raised by `open(2)` for write on a
//! file that is currently being executed. It is raised for the INODE, not for
//! the directory entry — so `unlink` + create, or `rename(2)` over the target,
//! both succeed while the running process keeps its old inode. Every
//! self-updating tool works this way.
//!
//! `cp` does neither: it opens the destination `O_WRONLY|O_TRUNC` in place, so
//! it takes ETXTBSY and refuses. `install(1)` unlinks the destination first
//! (GNU) or writes a sibling temp and `rename`s (BSD/macOS), so it succeeds.
//!
//! WHY THIS COSTS SOMETHING. paiml/infra's lambda-labs box is the machine that
//! RUNS `forjar apply` against itself, and its `forjar.yaml` carries two
//! comments refusing to declare tools because of this:
//!
//!   :155  "forjar is still intentionally absent from this machine: the box
//!          self-applies, and re-installing the running forjar binary mid-apply
//!          fails 'Text file busy'."
//!   :207  "a cargo resource can't re-cargo-install the running `forjar` binary
//!          ('Text file busy') during a self-apply"
//!
//! An undeclarable machine drifts. That box sat on forjar 1.20.1 while the
//! fleet ran 1.21.x, which made its YAML guard NO-GO.
//!
//! It is not only forjar-updating-forjar. `github_release` installs `rclone`,
//! `age` and `sops` into /usr/local/bin on that same box and `apr`, `batuta`,
//! `renacer` into ~/.cargo/bin on mini and jetson — a running `rclone` sync is
//! enough to make the resource fail.
//!
//! THE SAME `cp` REFUSES A DANGLING SYMLINK. "cp: not writing through dangling
//! symlink" is the other half of this defect and shares its root cause: an
//! in-place open of the destination path. It is exactly the wreckage a repair
//! has to fix — a CI cache-prune deletes the real files in a shared
//! `~/.cargo/bin` and leaves the symlinks. The cargo provider already learned
//! this and moved to `install`; `github_release` and the generated `install.sh`
//! did not.
//!
//! THESE TESTS EXECUTE THE GENERATED SHELL. Asserting on script text is how
//! `forjar check` passed everything for five months while dozens of
//! `script.contains(...)` tests were green.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};

#[path = "common/running_binary_probe.rs"]
mod running_binary_probe;
use running_binary_probe::{curl_stub, hold_running, sandbox, RunningBinary};

/// A real executable we can copy, run, and later overwrite. Any ELF/Mach-O
/// will do; `sleep` is on every host in the fleet and blocks without spinning.
const LIVE_BIN: &str = "/bin/sleep";
/// A DIFFERENT real executable, so "did the replacement land" is answerable by
/// comparing bytes rather than by trusting an exit code.
const NEW_BIN: &str = "/bin/cat";

fn run_bash(script: &str, envs: &[(&str, &str)]) -> std::process::Output {
    let mut c = Command::new("bash");
    c.arg("-c").arg(script);
    for (k, v) in envs {
        c.env(k, v);
    }
    c.output().expect("run generated script")
}

fn size_of(p: &Path) -> u64 {
    fs::metadata(p).map(|m| m.len()).unwrap_or(0)
}

/// The installed file must actually be runnable. Placement and mode are one
/// step now, so a regression that lands the bytes without the mode bit would
/// otherwise pass every size assertion here.
fn assert_executable(p: &Path, ctx: &str) {
    use std::os::unix::fs::PermissionsExt;
    let mode = fs::metadata(p)
        .unwrap_or_else(|e| panic!("{ctx}: cannot stat {}: {e}", p.display()))
        .permissions()
        .mode();
    assert!(
        mode & 0o111 != 0,
        "{ctx}: installed {} is not executable (mode {:o})",
        p.display(),
        mode & 0o777
    );
}

fn describe(out: &std::process::Output) -> String {
    format!(
        "exit={:?}\n--- stdout ---\n{}\n--- stderr ---\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

// ────────────────────────────────────────────────────────────────────────────
// The cargo provider — already fixed, and pinned here so it stays fixed.
// ────────────────────────────────────────────────────────────────────────────

fn cargo_script(pkg: &str, version: &str) -> String {
    let r = forjar::core::types::Resource {
        resource_type: forjar::core::types::ResourceType::Package,
        provider: Some("cargo".to_string()),
        packages: vec![pkg.to_string()],
        version: Some(version.to_string()),
        ..Default::default()
    };
    forjar::resources::package::apply_script(&r)
}

fn arch() -> String {
    let out = Command::new("uname").arg("-m").output().expect("uname -m");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// The claim in paiml/infra's lambda-labs forjar.yaml, executed.
///
/// A cargo resource landing a crate whose binary is RUNNING must converge.
/// The provider takes the cache-hit arm here, which is the arm that actually
/// writes into `$CARGO_HOME/bin` without a network fetch.
#[test]
fn cargo_provider_replaces_a_running_binary() {
    let dir = sandbox("cargo-running");
    let cargo_home = dir.join("home/.cargo");
    let bin = cargo_home.join("bin");
    fs::create_dir_all(&bin).expect("cargo bin");

    let cache_key = format!("toolx-1.0.0-{}", arch());
    let cache_bin = dir.join("cache").join(&cache_key).join("bin");
    fs::create_dir_all(&cache_bin).expect("cache bin");
    fs::copy(NEW_BIN, cache_bin.join("toolx")).expect("stage cache binary");
    fs::write(
        dir.join("cache").join(&cache_key).join(".crates.toml"),
        "[v1]\n\"toolx 1.0.0 (registry+https://github.com/rust-lang/crates.io-index)\" = [\"toolx\"]\n",
    )
    .expect("cache crates.toml");

    let dest = bin.join("toolx");
    let _live = hold_running(LIVE_BIN, &dest);

    let out = run_bash(
        &cargo_script("toolx", "1.0.0"),
        &[
            ("HOME", dir.join("home").to_str().unwrap()),
            ("CARGO_HOME", cargo_home.to_str().unwrap()),
            ("FORJAR_CACHE_DIR", dir.join("cache").to_str().unwrap()),
        ],
    );

    assert!(
        out.status.success(),
        "cargo provider could not replace a RUNNING binary.\n{}",
        describe(&out)
    );
    assert_eq!(
        size_of(&dest),
        size_of(Path::new(NEW_BIN)),
        "the resource reported success without replacing the binary.\n{}",
        describe(&out)
    );
    assert_executable(&dest, "after replace");
}

// ────────────────────────────────────────────────────────────────────────────
// The github_release provider — live on lambda-labs, mini and jetson.
// ────────────────────────────────────────────────────────────────────────────

fn github_release_script(install_dir: &Path) -> String {
    let r = forjar::core::types::Resource {
        resource_type: forjar::core::types::ResourceType::GithubRelease,
        repo: Some("acme/tool".to_string()),
        tag: Some("v1.0.0".to_string()),
        asset_pattern: Some("*linux*".to_string()),
        binary: Some("toolx".to_string()),
        install_dir: Some(install_dir.to_string_lossy().to_string()),
        ..Default::default()
    };
    forjar::resources::github_release::apply_script(&r)
}

/// Run the github_release script with `curl` stubbed to serve a local asset,
/// so the whole generated artifact executes offline.
fn run_github_release(dir: &Path, install_dir: &Path) -> std::process::Output {
    let asset = dir.join("asset/toolx-linux");
    fs::create_dir_all(asset.parent().unwrap()).expect("asset dir");
    fs::copy(NEW_BIN, &asset).expect("stage asset");
    let stub_dir = curl_stub(dir, &asset, "https://example.invalid/toolx-linux");

    let path = format!(
        "{}:{}",
        stub_dir.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    run_bash(&github_release_script(install_dir), &[("PATH", &path)])
}

/// THE DEFECT, executed. `rclone` is installed by a `github_release` resource
/// into /usr/local/bin on a box that also RUNS rclone.
#[test]
fn github_release_replaces_a_running_binary() {
    let dir = sandbox("ghrel-running");
    let install_dir = dir.join("bin");
    fs::create_dir_all(&install_dir).expect("install dir");

    let dest = install_dir.join("toolx");
    let _live = hold_running(LIVE_BIN, &dest);

    let out = run_github_release(&dir, &install_dir);

    assert!(
        out.status.success(),
        "github_release could not replace a RUNNING binary.\n{}",
        describe(&out)
    );
    assert_eq!(
        size_of(&dest),
        size_of(Path::new(NEW_BIN)),
        "the resource reported success without replacing the binary.\n{}",
        describe(&out)
    );
    assert_executable(&dest, "after replace");
}

/// The adjacent defect, same root cause: an in-place open of the destination
/// PATH. A shared `~/.cargo/bin` pruned by CI leaves symlinks pointing at
/// nothing, and repairing that is the whole job of re-applying the resource.
#[test]
fn github_release_replaces_a_dangling_symlink() {
    let dir = sandbox("ghrel-dangling");
    let install_dir = dir.join("bin");
    fs::create_dir_all(&install_dir).expect("install dir");

    let dest = install_dir.join("toolx");
    std::os::unix::fs::symlink(dir.join("gone/toolx"), &dest).expect("dangling symlink");
    assert!(
        fs::symlink_metadata(&dest).is_ok() && fs::metadata(&dest).is_err(),
        "fixture is not actually dangling"
    );

    let out = run_github_release(&dir, &install_dir);

    assert!(
        out.status.success(),
        "github_release could not repair a DANGLING SYMLINK.\n{}",
        describe(&out)
    );
    assert_eq!(
        size_of(&dest),
        size_of(Path::new(NEW_BIN)),
        "the resource reported success without repairing the symlink.\n{}",
        describe(&out)
    );
    assert_executable(&dest, "after replace");
}

// ────────────────────────────────────────────────────────────────────────────
// The generated install.sh — the `curl | sh` self-update path for every
// sovereign tool built with `forjar dist`, forjar's own included.
// ────────────────────────────────────────────────────────────────────────────

fn sample_dist() -> forjar::core::types::DistConfig {
    forjar::core::types::DistConfig {
        source: "github_release".into(),
        repo: "acme/tool".into(),
        binary: "toolx".into(),
        targets: vec![forjar::core::types::DistBinaryTarget {
            os: "linux".into(),
            arch: arch(),
            asset: "toolx-{version}-linux.tar.gz".into(),
            libc: None,
        }],
        install_dir: "/usr/local/bin".into(),
        install_dir_fallback: "~/.local/bin".into(),
        checksums: None,
        checksum_algo: "sha256".into(),
        description: "A test tool".into(),
        homepage: "https://example.invalid".into(),
        license: "MIT".into(),
        maintainer: "Test".into(),
        version_cmd: None,
        latest_tag: true,
        post_install: None,
        homebrew: None,
        nix: None,
    }
}

/// Build the staged `toolx-1.0.0-linux.tar.gz` the installer will extract.
fn staged_archive(dir: &Path) -> PathBuf {
    let stage = dir.join("stage/toolx-1.0.0-linux");
    fs::create_dir_all(&stage).expect("stage dir");
    fs::copy(NEW_BIN, stage.join("toolx")).expect("stage binary");
    let archive = dir.join("stage/toolx-1.0.0-linux.tar.gz");
    let ok = Command::new("tar")
        .args(["czf", archive.to_str().unwrap(), "toolx-1.0.0-linux"])
        .current_dir(dir.join("stage"))
        .status()
        .expect("tar");
    assert!(ok.success(), "could not build staged archive");
    archive
}

/// Run the generated installer offline: pin the tag, serve the staged archive
/// instead of github.com, and force an overwrite into the sandbox prefix.
fn run_installer(dir: &Path, prefix: &Path) -> std::process::Output {
    let body = forjar::cli::dist_generators::generate_installer(&sample_dist());
    let body = body.trim_end().strip_suffix("\nmain").unwrap_or(&body);
    let archive = staged_archive(dir);
    let harness = format!(
        "{body}\n\
         TAG=\"v1.0.0\"\n\
         FORCE=1\n\
         PREFIX=\"{prefix}\"\n\
         resolve_version() {{ TAG=\"v1.0.0\"; }}\n\
         verify_checksum() {{ :; }}\n\
         download() {{ cat \"{archive}\"; }}\n\
         download_file() {{ cp \"{archive}\" \"$2\"; }}\n\
         main\n",
        body = body,
        prefix = prefix.display(),
        archive = archive.display(),
    );
    let runner = dir.join("run-installer.sh");
    fs::write(&runner, harness).expect("write harness");
    Command::new("sh")
        .arg(&runner)
        .output()
        .expect("run installer")
}

/// THE DEFECT, executed. `curl -sSf .../install.sh | sh` is how forjar tells
/// people to install and upgrade forjar.
#[test]
fn generated_installer_replaces_a_running_binary() {
    let dir = sandbox("installer-running");
    let prefix = dir.join("bin");
    fs::create_dir_all(&prefix).expect("prefix");

    let dest = prefix.join("toolx");
    let _live = hold_running(LIVE_BIN, &dest);

    let out = run_installer(&dir, &prefix);

    assert!(
        out.status.success(),
        "the generated installer could not replace a RUNNING binary.\n{}",
        describe(&out)
    );
    assert_eq!(
        size_of(&dest),
        size_of(Path::new(NEW_BIN)),
        "the installer reported success without replacing the binary.\n{}",
        describe(&out)
    );
    assert_executable(&dest, "after replace");
}

/// The probe must be capable of failing. If `cp` over a running binary were
/// legal on this host, every assertion above would pass for the wrong reason
/// and the guard would be worthless.
#[test]
fn the_probe_can_fail() {
    let dir = sandbox("probe-discriminates");
    fs::create_dir_all(&dir).expect("sandbox");
    let dest = dir.join("toolx");
    let live: RunningBinary = hold_running(LIVE_BIN, &dest);

    let out = run_bash(
        &format!("cp {} {}", NEW_BIN, dest.display()),
        &[("LC_ALL", "C")],
    );
    assert!(
        !out.status.success(),
        "`cp` over a running binary SUCCEEDED on this host, so these tests \
         cannot discriminate. Every ETXTBSY assertion above is vacuous.\n{}",
        describe(&out)
    );
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("Text file busy"),
        "expected ETXTBSY, got something else — the fixture is not measuring \
         what it claims.\n{}",
        describe(&out)
    );
    drop(live);
}

// ────────────────────────────────────────────────────────────────────────────
// Atomicity — the property `install(1)` does NOT have.
// ────────────────────────────────────────────────────────────────────────────

/// Replace a path N times and report how often a concurrent observer found it
/// ABSENT. Returns `(absent, observations)`.
fn window_observed(dest: &Path, replace_loop: &str) -> (u64, u64) {
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(replace_loop)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn replace loop");

    let (mut absent, mut seen) = (0u64, 0u64);
    while child.try_wait().expect("try_wait").is_none() {
        seen += 1;
        if fs::symlink_metadata(dest).is_err() {
            absent += 1;
        }
    }
    (absent, seen)
}

/// The destination must never be observably absent.
///
/// `install(1)` — which this code used until now, and which clears both
/// refusals above — unlinks the destination and then creates it. On the host
/// this matters for, sixteen CI runners share one `$CARGO_HOME/bin`, and an
/// `exec` landing in that window fails ENOENT. `rename(2)` has no window.
///
/// The two loops below are the SAME work by the two mechanisms, so the
/// comparison is the measurement: if `install` shows no window on this host
/// either, the test says so rather than claiming a property it did not see.
#[test]
fn replacement_is_atomic_with_no_absent_window() {
    let dir = sandbox("atomicity");
    let bin = dir.join("bin");
    fs::create_dir_all(&bin).expect("bin");
    let dest = bin.join("toolx");
    fs::copy(LIVE_BIN, &dest).expect("seed");

    let reps = 400;
    let (install_absent, install_seen) = window_observed(
        &dest,
        &format!(
            "i=0; while [ $i -lt {reps} ]; do install -m 755 {src} {dst}; i=$((i+1)); done",
            src = LIVE_BIN,
            dst = dest.display()
        ),
    );

    let helper = forjar::core::shell_install::atomic_install_fn();
    let (rename_absent, rename_seen) = window_observed(
        &dest,
        &format!(
            "{helper}\ni=0; while [ $i -lt {reps} ]; do _fj_install_bin {src} {dst}; i=$((i+1)); done",
            src = LIVE_BIN,
            dst = dest.display()
        ),
    );

    // Discrimination first: a probe that cannot see the window it is named
    // after would pass for any implementation at all.
    assert!(
        install_absent > 0,
        "the probe saw NO absent window even for install(1) ({install_absent} of \
         {install_seen}); it cannot discriminate, so its verdict on rename is \
         worthless"
    );
    assert_eq!(
        rename_absent, 0,
        "rename(2) left the destination absent {rename_absent} of {rename_seen} \
         observations — replacement is not atomic (install(1) baseline: \
         {install_absent} of {install_seen})"
    );
}

/// Keep the unused-import lint honest about the guard type.
fn _assert_guard_is_a_child(_c: &Child) {}
