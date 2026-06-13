//! FJ-3607 Tier 2: container-based execution of the generated installer.
//!
//! Tier 1 (`src/cli/dist_verify.rs`) statically checks the installer
//! (`sh -n`, bashrs lint, snippet presence, URL structure) without ever
//! running it. Tier 2 goes further: it actually RUNS the generated
//! `install.sh` inside ubuntu (a glibc/gnu host) and alpine (a musl host)
//! containers and confirms the binary lands in `install_dir` and that
//! `version_cmd` succeeds.
//!
//! ## What Tier 2 verifies vs. defers
//!
//! Reaching `github.com` from inside the clean-room CI containers is
//! unreliable (the HTTP proxy stalls loopback/release downloads — the
//! same reason PR #153's OCI tests are `#[ignore]`d). So Tier 2 verifies
//! the installer **offline against a locally-staged tarball** rather than
//! a live GitHub release:
//!
//! - VERIFIED: `detect_os`/`detect_arch`/`detect_libc` pick the right
//!   target on each container's real OS+libc; `resolve_asset` expands the
//!   `{version}` placeholder; `verify_checksum` passes against a real
//!   sha256; `tar xzf` extracts the archive directory layout; the binary
//!   is installed to `install_dir`; `version_cmd` runs the installed
//!   binary successfully.
//! - DEFERRED (to release-workflow integration): the live network fetch
//!   from `github.com` and the GitHub `releases/latest` tag resolution.
//!   Tier 1's URL-structure check already guards the URL shape; the live
//!   fetch is exercised by the real release pipeline, not unit CI.
//!
//! ## CI-safety
//!
//! Container/network tests HANG the proxied clean-room CI, so every test
//! here that spawns a container is `#[ignore]`d (runnable locally with
//! `cargo test -- --ignored`). The ORCHESTRATION logic — target
//! selection, harness construction, result parsing, docker-absent
//! degradation — is unit-tested hermetically WITHOUT spawning anything.

use crate::core::store::convergence_container::detect_container_runtime;
use crate::core::types::DistConfig;

/// One ubuntu/alpine container run plan: the image to boot and the
/// rust-target libc it exercises.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Tier2Target {
    /// Container image (e.g. `ubuntu:latest`).
    pub image: &'static str,
    /// libc this image presents (`gnu` for ubuntu, `musl` for alpine).
    pub libc: &'static str,
    /// Short label for log lines.
    pub label: &'static str,
}

/// The two libc surfaces the spec's F-3601 falsification cares about:
/// ubuntu (glibc/gnu) and alpine (musl, no bash by default).
pub(crate) const TIER2_TARGETS: [Tier2Target; 2] = [
    Tier2Target {
        image: "ubuntu:latest",
        libc: "gnu",
        label: "ubuntu",
    },
    Tier2Target {
        image: "alpine:latest",
        libc: "musl",
        label: "alpine",
    },
];

/// Outcome of a single container installer run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Tier2Outcome {
    pub label: String,
    pub passed: bool,
    pub detail: String,
}

/// FJ-3607 Tier 2 entry point. Runs the generated installer in each
/// `TIER2_TARGETS` container; degrades to a clean skip (Ok) when no
/// container runtime is available so it is safe in Docker-less CI.
pub(crate) fn run_verify_containers(
    dist: &DistConfig,
    args: &super::commands::DistArgs,
) -> Result<(), String> {
    // Tier 2 implies Tier 1 — run the static checks first so a broken
    // generator fails fast before we pay container-startup cost.
    super::dist_verify::run_verify(dist, args)?;

    let runtime = match detect_container_runtime() {
        Some(rt) => rt,
        None => {
            println!("verify-containers: Docker not available — Tier 2 skipped");
            return Ok(());
        }
    };
    println!("verify-containers: using {runtime} runtime");

    let outcomes = run_all_targets(&runtime, dist);
    report_outcomes(&outcomes)
}

/// Run the installer against every Tier-2 target, collecting outcomes.
/// Separated from reporting so the loop is testable independently of the
/// process spawn (each `run_one_target` is the only spawning surface).
pub(crate) fn run_all_targets(runtime: &str, dist: &DistConfig) -> Vec<Tier2Outcome> {
    TIER2_TARGETS
        .iter()
        .map(|t| run_one_target(runtime, dist, t))
        .collect()
}

/// Turn a set of per-target outcomes into a pass/fail result + summary.
pub(crate) fn report_outcomes(outcomes: &[Tier2Outcome]) -> Result<(), String> {
    let mut failures = Vec::new();
    for o in outcomes {
        if o.passed {
            println!("  ok: {} installer run — {}", o.label, o.detail);
        } else {
            failures.push(format!("  FAIL {}: {}", o.label, o.detail));
        }
    }
    if failures.is_empty() {
        println!("verify-containers: PASS — Tier 2 installer runs (ubuntu+alpine, offline)");
        Ok(())
    } else {
        Err(format!("verify-containers: FAIL\n{}", failures.join("\n")))
    }
}

/// Pick the asset template whose libc matches this container target.
/// ubuntu wants a `gnu` linux/x86_64 asset; alpine wants `musl`. Returns
/// the resolved-for-x86_64 asset name (with `{version}` still in place).
pub(crate) fn select_asset_for_target<'a>(
    dist: &'a DistConfig,
    target: &Tier2Target,
) -> Result<&'a str, String> {
    dist.targets
        .iter()
        .find(|t| t.os == "linux" && t.arch == "x86_64" && t.libc.as_deref() == Some(target.libc))
        .map(|t| t.asset.as_str())
        .ok_or_else(|| {
            format!(
                "no linux/x86_64 {} asset in dist.targets for {} container",
                target.libc, target.label
            )
        })
}

/// Build the in-container harness that runs the generated installer
/// OFFLINE: it sources the installer body (with the trailing `main` call
/// stripped), redefines the network surfaces (`resolve_version`,
/// `download_file`, `download`) to use a locally-staged tarball + a
/// staged SHA256SUMS line, then invokes `main`. Finally it runs the
/// installed binary's version_cmd.
///
/// `staged_archive` is the in-container path of the staged `.tar.gz`;
/// `sums_line` is a real `<sha256>  <asset>` line so the installer's
/// `verify_checksum` actually verifies (rather than warn-skipping);
/// `sums_file` is the CONFIGURED checksums basename (`dist.checksums`, e.g.
/// `SHA256SUMS` or a custom `CHECKSUMS.txt`) that the installer builds its
/// `SUMS_URL` from; `tag` is the pinned version; `version_cmd` is the
/// post-install check.
///
/// The `download` override is URL-aware: the configured-sums URL (or the
/// `${{ASSET}}.sha256` per-asset fallback) returns the staged sums line, and
/// anything else returns the tarball bytes. Matching the ACTUAL configured
/// sums filename (#165) — not a hardcoded `*SHA256SUMS*` glob — is what makes
/// `verify_checksum` actually run for ANY `dist.checksums` name; a hardcoded
/// glob would let a custom filename fall through to the tarball arm and the
/// installer would silently warn-skip verification.
pub(crate) fn build_install_harness(
    installer_body: &str,
    staged_archive: &str,
    sums_line: &str,
    sums_file: &str,
    tag: &str,
    version_cmd: &str,
) -> String {
    // Strip the trailing top-level `main` invocation so we can redefine
    // network functions BEFORE main runs. The generator always ends the
    // script with a lone `main` line.
    let body = strip_trailing_main(installer_body);
    let sums_glob = sums_url_glob(sums_file);
    format!(
        r#"set -eu
{body}

# ── Tier 2 offline overrides ──
# Pin the tag so no GitHub /releases/latest call is made.
TAG="{tag}"
resolve_version() {{ TAG="{tag}"; }}
# Serve the staged tarball / checksums instead of fetching from github.com.
# The sums case arm matches the CONFIGURED checksums filename ({sums_file})
# plus the *.sha256 per-asset fallback, so verify_checksum actually verifies.
download() {{
  case "$1" in
    {sums_glob}) printf '%s\n' '{sums_line}' ;;
    *) cat "{staged}" ;;
  esac
}}
download_file() {{ cp "{staged}" "$2"; }}

main
echo "TIER2_INSTALL_OK"
{version_cmd}
echo "TIER2_VERSION_OK"
"#,
        body = body,
        tag = tag,
        staged = staged_archive,
        sums_line = sums_line,
        sums_file = sums_file,
        sums_glob = sums_glob,
        version_cmd = version_cmd,
    )
}

/// Build the `case` pattern that matches the installer's SUMS download URLs.
///
/// The installer (`dist_generators::build_checksum_snippet`) fetches the
/// configured `SUMS_URL` (ending in `dist.checksums`, default `SHA256SUMS`)
/// and falls back to a per-asset `${{ASSET}}.sha256`. We match the configured
/// basename anywhere in the URL plus the `.sha256` fallback. A blank/whitespace
/// filename degrades to the historical `*SHA256SUMS*` default so we never emit
/// an empty `**` glob that would swallow the tarball URL too.
fn sums_url_glob(sums_file: &str) -> String {
    let name = sums_file.trim();
    let basename = if name.is_empty() { "SHA256SUMS" } else { name };
    format!("*{basename}*|*.sha256")
}

/// Remove the final top-level `main` invocation the generator appends, so
/// the harness can redefine functions before calling `main` itself.
pub(crate) fn strip_trailing_main(script: &str) -> String {
    let trimmed = script.trim_end();
    match trimmed.strip_suffix("\nmain") {
        Some(rest) => rest.to_string(),
        None => trimmed.to_string(),
    }
}

/// Parse the harness stdout: both markers must be present for a pass.
pub(crate) fn parse_harness_output(stdout: &str) -> Tier2RunStatus {
    let installed = stdout.contains("TIER2_INSTALL_OK");
    let versioned = stdout.contains("TIER2_VERSION_OK");
    if installed && versioned {
        Tier2RunStatus::Passed
    } else if installed {
        Tier2RunStatus::InstalledButVersionFailed
    } else {
        Tier2RunStatus::InstallFailed
    }
}

/// Distinct failure modes so the operator sees exactly which stage broke.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Tier2RunStatus {
    Passed,
    InstalledButVersionFailed,
    InstallFailed,
}

impl Tier2RunStatus {
    pub(crate) fn detail(&self) -> &'static str {
        match self {
            Tier2RunStatus::Passed => "installed and version_cmd OK",
            Tier2RunStatus::InstalledButVersionFailed => {
                "binary installed but version_cmd did not succeed"
            }
            Tier2RunStatus::InstallFailed => "installer did not reach a completed install",
        }
    }
    pub(crate) fn passed(&self) -> bool {
        matches!(self, Tier2RunStatus::Passed)
    }
}

/// Build the `docker run` argv that boots an ephemeral container, copies
/// in nothing (the harness + staged tarball are piped via a heredoc-free
/// stdin model used by `run_one_target`), and sleeps. Pure, so the argv
/// is testable without spawning.
pub(crate) fn build_run_argv<'a>(image: &'a str, container_name: &'a str) -> Vec<&'a str> {
    vec![
        "run",
        "-d",
        "--rm",
        "--name",
        container_name,
        image,
        "sleep",
        "300",
    ]
}

/// The single container-spawning surface. Everything above is pure and
/// unit-tested; this is the part the `#[ignore]`d tests exercise locally.
fn run_one_target(runtime: &str, dist: &DistConfig, target: &Tier2Target) -> Tier2Outcome {
    match try_run_one_target(runtime, dist, target) {
        Ok(status) => Tier2Outcome {
            label: target.label.to_string(),
            passed: status.passed(),
            detail: status.detail().to_string(),
        },
        Err(e) => Tier2Outcome {
            label: target.label.to_string(),
            passed: false,
            detail: e,
        },
    }
}

/// Boot a container, stage a tarball, run the installer harness, tear down.
fn try_run_one_target(
    runtime: &str,
    dist: &DistConfig,
    target: &Tier2Target,
) -> Result<Tier2RunStatus, String> {
    use std::process::Command;

    let asset_tpl = select_asset_for_target(dist, target)?;
    // The installer strips a leading 'v' (VERSION_NUM="${TAG#v}") before
    // expanding {version}, so the tag's numeric form is what lands in the
    // asset name.
    let tag = "v0.0.0";
    let version = tag.trim_start_matches('v');
    let asset = asset_tpl.replace("{version}", version);

    let (staged, sums_line) =
        super::dist_verify_tier2_stage::stage_with_sums(&dist.binary, &asset)?;
    let staged_in_container = format!("/tmp/{asset}");

    let container_name = format!("forjar-dist-t2-{}-{}", target.label, std::process::id());
    boot_container(runtime, target.image, &container_name)?;

    let result = run_inside(
        runtime,
        &container_name,
        dist,
        target,
        &staged,
        &staged_in_container,
        &sums_line,
        tag,
    );

    let _ = Command::new(runtime)
        .args(["rm", "-f", &container_name])
        .output();
    let _ = std::fs::remove_file(&staged);
    result
}

/// Start the ephemeral container; map a failed start to a clear error.
fn boot_container(runtime: &str, image: &str, name: &str) -> Result<(), String> {
    use std::process::Command;
    let argv = build_run_argv(image, name);
    let out = Command::new(runtime)
        .args(&argv)
        .output()
        .map_err(|e| format!("container start ({image}): {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!(
            "container start failed ({image}): {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}

/// Copy the staged tarball in, run the harness, parse markers.
#[allow(clippy::too_many_arguments)]
fn run_inside(
    runtime: &str,
    name: &str,
    dist: &DistConfig,
    target: &Tier2Target,
    staged_host: &str,
    staged_in_container: &str,
    sums_line: &str,
    tag: &str,
) -> Result<Tier2RunStatus, String> {
    use std::process::Command;

    // Copy the staged tarball into the container.
    let cp = Command::new(runtime)
        .args(["cp", staged_host, &format!("{name}:{staged_in_container}")])
        .output()
        .map_err(|e| format!("docker cp: {e}"))?;
    if !cp.status.success() {
        return Err(format!(
            "docker cp failed: {}",
            String::from_utf8_lossy(&cp.stderr).trim()
        ));
    }

    let installer = super::dist_generators::generate_installer(dist);
    let version_cmd = dist
        .version_cmd
        .clone()
        .unwrap_or_else(|| format!("{} --version", dist.binary));
    // The configured checksums basename the installer's SUMS_URL is built
    // from (default SHA256SUMS); the harness must match THIS so verify_checksum
    // runs for any custom dist.checksums name (#165).
    let sums_file = dist.checksums.as_deref().unwrap_or("SHA256SUMS");
    // The installer is POSIX sh; run it through sh inside the container so
    // alpine (no bash) works the same as ubuntu.
    let harness = build_install_harness(
        &installer,
        staged_in_container,
        sums_line,
        sums_file,
        tag,
        &version_cmd,
    );
    let stdout = exec_sh(runtime, name, &harness)
        .map_err(|e| format!("{} installer run: {e}", target.label))?;
    Ok(parse_harness_output(&stdout))
}

/// Execute a POSIX-sh script inside the container via `exec -i ... sh`.
fn exec_sh(runtime: &str, name: &str, script: &str) -> Result<String, String> {
    use std::process::{Command, Stdio};
    let mut child = Command::new(runtime)
        .args(["exec", "-i", name, "sh"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("container exec failed: {e}"))?;
    crate::transport::write_stdin_or_reap(&mut child, script)?;
    let output = child.wait_with_output().map_err(|e| format!("wait: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "exit {}: {}",
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
