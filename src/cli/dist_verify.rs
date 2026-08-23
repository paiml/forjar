//! PMAT-082 / FJ-3607: `forjar dist --verify` — Tier 1 verification.
//!
//! Generates the requested artifacts into a temp dir, then verifies the
//! installer without containers:
//! - `sh -n` parses the generated script (spawned, POSIX syntax)
//! - the script actually RUNS (`dist_verify_exec`): `sh install.sh
//!   --help` exits 0 and prints usage, and an unknown flag reaches
//!   `die()`. Static checks cannot see a forward call — using a function
//!   above its definition is valid POSIX syntax that fails only at
//!   runtime — so that check executes the script inside a PATH sandbox
//!   where curl/wget/sudo/tar/install/cp refuse to run.
//! - bashrs lint reports zero errors (in-process library API via
//!   `crate::core::purifier` — no binary spawn needed)
//! - checksum-verification and platform-detection snippets are present
//!   (`verify_checksum`, `detect_arch`, `detect_libc`)
//! - every download URL matches the
//!   `https://github.com/<org>/<repo>/releases/download/<tag>/<asset>` shape
//!
//! Tier 2 (container-based execution per the spec's FJ-3607 table —
//! alpine/ubuntu installer runs, brew/nix builds) is a follow-up and
//! intentionally out of scope here.
//!
//! Also home to PMAT-081's `validate_dist_source` — the fail-fast guard
//! that keeps `forjar dist` from generating artifacts with broken
//! download URLs for unsupported sources.

use crate::core::types::DistConfig;
use std::path::Path;

/// Snippets every generated installer must contain (spec FJ-3601).
const REQUIRED_SNIPPETS: [&str; 3] = ["verify_checksum", "detect_arch", "detect_libc"];

/// PMAT-081: only `github_release` produces working artifacts today.
///
/// `local`, `url`, and `s3` are documented in the spec but not yet
/// implemented — generating artifacts for them yields broken empty-repo
/// github.com URLs, so fail fast with a clear message instead.
pub fn validate_dist_source(dist: &DistConfig) -> Result<(), String> {
    if dist.source == "github_release" {
        Ok(())
    } else {
        Err(format!(
            "dist.source \"{}\" is not yet supported (only github_release)",
            dist.source
        ))
    }
}

/// FJ-3607 Tier 1: generate artifacts to a temp dir, then verify them
/// statically and by executing the installer's `--help` in a sandbox.
pub(crate) fn run_verify(
    dist: &DistConfig,
    args: &super::commands::DistArgs,
) -> Result<(), String> {
    // PID alone collides across threads in one process — a concurrent
    // caller's cleanup would delete this dir mid-verify (same race as
    // convergence_runner, GH-141/#147). Atomic counter guarantees
    // in-process uniqueness.
    static VERIFY_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let seq = VERIFY_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!("forjar-dist-verify-{}-{seq}", std::process::id()));
    std::fs::create_dir_all(&tmp).map_err(|e| format!("cannot create {}: {e}", tmp.display()))?;
    let result = verify_in_dir(dist, args, &tmp);
    let _ = std::fs::remove_dir_all(&tmp);
    result
}

/// Generate + verify inside `dir` (separated so cleanup always runs).
fn verify_in_dir(
    dist: &DistConfig,
    args: &super::commands::DistArgs,
    dir: &Path,
) -> Result<(), String> {
    let script = super::dist_generators::generate_installer(dist);
    let script_path = dir.join("install.sh");
    std::fs::write(&script_path, &script)
        .map_err(|e| format!("write {}: {e}", script_path.display()))?;
    write_extra_artifacts(dist, args, dir)?;

    let mut failures: Vec<String> = Vec::new();
    let checks: [(&str, Result<(), String>); 5] = [
        ("sh -n (POSIX syntax)", check_sh_syntax(&script_path)),
        (
            "--help executes",
            super::dist_verify_exec::check_help_runs(&script_path, dir),
        ),
        ("bashrs lint (0 errors)", check_bashrs_lint(&script)),
        ("required snippets", check_required_snippets(&script)),
        ("asset URL structure", check_asset_urls(dist)),
    ];
    for (name, result) in checks {
        match result {
            Ok(()) => println!("  ok: {name}"),
            Err(e) => failures.push(format!("  FAIL {name}: {e}")),
        }
    }

    if failures.is_empty() {
        println!(
            "verify: PASS — Tier 1 checks on generated installer (static + sandboxed --help run)"
        );
        Ok(())
    } else {
        Err(format!("verify: FAIL\n{}", failures.join("\n")))
    }
}

/// Generate the other requested offline artifacts into the verify dir.
///
/// `--homebrew`/`--nix` are skipped: they need release checksum
/// resolution (network or `--checksums-file`) and their execution is a
/// Tier 2 container concern.
fn write_extra_artifacts(
    dist: &DistConfig,
    args: &super::commands::DistArgs,
    dir: &Path,
) -> Result<(), String> {
    use super::dist_generators::generate_binstall;
    use super::dist_generators_b::{
        generate_deb, generate_github_action, generate_rpm, write_artifact,
    };

    if args.binstall || args.all {
        write_artifact(&dir.join("binstall.toml"), &generate_binstall(dist))?;
    }
    if args.github_action || args.all {
        write_artifact(&dir.join("action.yml"), &generate_github_action(dist))?;
    }
    if args.deb || args.all {
        generate_deb(dist, &dir.join("debian"))?;
    }
    if args.rpm || args.all {
        write_artifact(
            &dir.join(format!("{}.spec", dist.binary)),
            &generate_rpm(dist),
        )?;
    }
    Ok(())
}

/// Spawn `sh -n` to syntax-check the generated script.
fn check_sh_syntax(path: &Path) -> Result<(), String> {
    let out = std::process::Command::new("sh")
        .arg("-n")
        .arg(path)
        .output()
        .map_err(|e| format!("cannot spawn sh -n: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!(
            "sh -n rejected the script: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}

/// In-process bashrs lint (library API — bashrs is a dependency, no
/// binary spawn). Error-severity diagnostics fail; warnings pass.
fn check_bashrs_lint(script: &str) -> Result<(), String> {
    crate::core::purifier::validate_script(script)
}

/// The installer must carry checksum-verification and platform-detection
/// snippets — their absence means a silently weaker install path.
fn check_required_snippets(script: &str) -> Result<(), String> {
    let missing: Vec<&str> = REQUIRED_SNIPPETS
        .iter()
        .filter(|s| !script.contains(**s))
        .copied()
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "installer is missing required snippets: {}",
            missing.join(", ")
        ))
    }
}

/// F-3609: every download URL the artifacts can construct must match
/// `https://github.com/<org>/<repo>/releases/download/<tag>/<asset>`.
fn check_asset_urls(dist: &DistConfig) -> Result<(), String> {
    validate_repo_shape(&dist.repo)?;
    if dist.targets.is_empty() {
        return Err("dist.targets is empty — installer cannot resolve any asset".to_string());
    }
    for t in &dist.targets {
        validate_asset_shape(&t.asset)?;
        validate_release_url(&build_release_url(&dist.repo, &t.asset))?;
    }
    Ok(())
}

/// A repo must be a bare `<org>/<repo>` slug — schemes or extra path
/// segments produce malformed download URLs.
fn validate_repo_shape(repo: &str) -> Result<(), String> {
    match repo.split_once('/') {
        Some((org, name)) if is_valid_slug_part(org) && is_valid_slug_part(name) => Ok(()),
        _ => Err(format!(
            "dist.repo \"{repo}\" is not a valid <org>/<repo> slug"
        )),
    }
}

fn is_valid_slug_part(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// An asset template must carry `{version}` (pinned installs 404
/// otherwise) and must be a single path segment.
fn validate_asset_shape(asset: &str) -> Result<(), String> {
    if !asset.contains("{version}") {
        return Err(format!(
            "asset \"{asset}\" lacks the {{version}} placeholder — pinned installs would 404"
        ));
    }
    if asset
        .chars()
        .any(|c| c == '/' || c == ':' || c.is_whitespace())
    {
        return Err(format!(
            "asset \"{asset}\" contains '/', ':', or whitespace — not a valid release asset name"
        ));
    }
    Ok(())
}

/// Mirror the runtime URL the installer builds (`$ASSET_URL`), with the
/// tag still variable.
fn build_release_url(repo: &str, asset: &str) -> String {
    format!("https://github.com/{repo}/releases/download/${{TAG}}/{asset}")
}

/// Structural check on a constructed download URL.
fn validate_release_url(url: &str) -> Result<(), String> {
    let shape = "https://github.com/<org>/<repo>/releases/download/<tag>/<asset>";
    let rest = url
        .strip_prefix("https://github.com/")
        .ok_or_else(|| format!("download URL \"{url}\" does not match {shape}"))?;
    let segments: Vec<&str> = rest.split('/').collect();
    let ok = segments.len() == 6
        && segments.iter().all(|s| !s.is_empty())
        && segments[2] == "releases"
        && segments[3] == "download";
    if ok {
        Ok(())
    } else {
        Err(format!("download URL \"{url}\" does not match {shape}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::DistBinaryTarget;

    fn sample_dist() -> DistConfig {
        DistConfig {
            source: "github_release".into(),
            repo: "acme/tool".into(),
            binary: "mytool".into(),
            targets: vec![DistBinaryTarget {
                os: "linux".into(),
                arch: "x86_64".into(),
                asset: "mytool-{version}-x86_64-unknown-linux-gnu.tar.gz".into(),
                libc: Some("gnu".into()),
            }],
            install_dir: "/usr/local/bin".into(),
            install_dir_fallback: "~/.local/bin".into(),
            checksums: Some("SHA256SUMS".into()),
            checksum_algo: "sha256".into(),
            description: "A test tool".into(),
            homepage: "https://example.com".into(),
            license: "MIT".into(),
            maintainer: "Test".into(),
            version_cmd: Some("mytool --version".into()),
            latest_tag: true,
            post_install: None,
            homebrew: None,
            nix: None,
        }
    }

    fn verify_args() -> crate::cli::commands::DistArgs {
        crate::cli::commands::DistArgs {
            file: "forjar.yaml".into(),
            installer: false,
            homebrew: false,
            binstall: false,
            nix: false,
            github_action: false,
            deb: false,
            rpm: false,
            all: false,
            verify: true,
            verify_containers: false,
            version: None,
            checksums_file: None,
            output: None,
            output_dir: None,
            json: false,
        }
    }

    // ── PMAT-081: validate_dist_source ──

    #[test]
    fn source_github_release_is_supported() {
        assert!(validate_dist_source(&sample_dist()).is_ok());
    }

    #[test]
    fn source_local_url_s3_are_rejected_with_clear_error() {
        for source in ["local", "url", "s3"] {
            let mut dist = sample_dist();
            dist.source = source.into();
            let err = validate_dist_source(&dist).unwrap_err();
            assert_eq!(
                err,
                format!("dist.source \"{source}\" is not yet supported (only github_release)")
            );
        }
    }

    // ── sh -n ──

    #[test]
    fn sh_syntax_passes_on_generated_installer() {
        let script = crate::cli::dist_generators::generate_installer(&sample_dist());
        let dir = std::env::temp_dir().join(format!("fj-verify-shn-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("install.sh");
        std::fs::write(&path, script).unwrap();
        let result = check_sh_syntax(&path);
        let _ = std::fs::remove_dir_all(&dir);
        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn sh_syntax_fails_on_broken_script() {
        let dir = std::env::temp_dir().join(format!("fj-verify-shn-bad-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("broken.sh");
        std::fs::write(&path, "#!/bin/sh\nif then fi (\n").unwrap();
        let result = check_sh_syntax(&path);
        let _ = std::fs::remove_dir_all(&dir);
        assert!(result.unwrap_err().contains("sh -n rejected"));
    }

    // ── FJ-3607: --help must EXECUTE (not just parse) ──

    /// The generated installer must actually run.
    #[test]
    fn help_run_passes_on_generated_installer() {
        let dir = std::env::temp_dir().join(format!("fj-verify-help-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("install.sh");
        std::fs::write(
            &path,
            crate::cli::dist_generators::generate_installer(&sample_dist()),
        )
        .unwrap();
        let result = super::super::dist_verify_exec::check_help_runs(&path, &dir);
        let _ = std::fs::remove_dir_all(&dir);
        assert!(result.is_ok(), "{result:?}");
    }

    // ── bashrs lint (in-process library API) ──

    #[test]
    fn bashrs_lint_passes_on_generated_installer() {
        let script = crate::cli::dist_generators::generate_installer(&sample_dist());
        assert!(check_bashrs_lint(&script).is_ok());
    }

    // ── required snippets ──

    #[test]
    fn snippets_present_in_generated_installer() {
        let script = crate::cli::dist_generators::generate_installer(&sample_dist());
        assert!(check_required_snippets(&script).is_ok());
    }

    #[test]
    fn snippets_missing_reports_each_name() {
        let err = check_required_snippets("#!/bin/sh\necho hi\n").unwrap_err();
        assert!(err.contains("verify_checksum"));
        assert!(err.contains("detect_arch"));
        assert!(err.contains("detect_libc"));
    }

    // ── asset URL structure ──

    #[test]
    fn asset_urls_valid_for_sample_dist() {
        assert!(check_asset_urls(&sample_dist()).is_ok());
    }

    #[test]
    fn asset_urls_reject_empty_targets() {
        let mut dist = sample_dist();
        dist.targets.clear();
        assert!(check_asset_urls(&dist)
            .unwrap_err()
            .contains("targets is empty"));
    }

    #[test]
    fn repo_slug_valid() {
        assert!(validate_repo_shape("paiml/forjar").is_ok());
    }

    #[test]
    fn repo_with_scheme_is_rejected() {
        let err = validate_repo_shape("https://github.com/acme/tool").unwrap_err();
        assert!(err.contains("not a valid <org>/<repo> slug"));
    }

    #[test]
    fn repo_without_slash_is_rejected() {
        assert!(validate_repo_shape("acme").is_err());
    }

    #[test]
    fn repo_with_extra_segment_is_rejected() {
        assert!(validate_repo_shape("acme/tool/extra").is_err());
    }

    #[test]
    fn asset_without_version_placeholder_is_rejected() {
        let err = validate_asset_shape("mytool-x86_64.tar.gz").unwrap_err();
        assert!(err.contains("{version} placeholder"));
    }

    #[test]
    fn asset_with_slash_is_rejected() {
        let err = validate_asset_shape("dir/mytool-{version}.tar.gz").unwrap_err();
        assert!(err.contains("not a valid release asset name"));
    }

    #[test]
    fn asset_valid_shape_passes() {
        assert!(validate_asset_shape("mytool-{version}-x86_64.tar.gz").is_ok());
    }

    #[test]
    fn release_url_shape_valid() {
        let url = build_release_url("acme/tool", "mytool-{version}.tar.gz");
        assert!(validate_release_url(&url).is_ok());
    }

    #[test]
    fn release_url_wrong_host_is_rejected() {
        let err =
            validate_release_url("http://example.com/a/b/releases/download/v1/x").unwrap_err();
        assert!(err.contains("does not match"));
    }

    #[test]
    fn release_url_wrong_path_is_rejected() {
        let err =
            validate_release_url("https://github.com/acme/tool/archive/v1/x.tar.gz").unwrap_err();
        assert!(err.contains("does not match"));
    }

    // ── run_verify end-to-end (in-process) ──

    #[test]
    fn run_verify_passes_on_valid_dist() {
        let result = run_verify(&sample_dist(), &verify_args());
        assert!(result.is_ok(), "{result:?}");
    }

    /// F-3609: a deliberately broken asset URL must fail --verify with a
    /// useful message.
    #[test]
    fn f3609_run_verify_fails_on_broken_asset_url() {
        let mut dist = sample_dist();
        dist.targets[0].asset = "mytool-x86_64.tar.gz".into(); // no {version}
        let err = run_verify(&dist, &verify_args()).unwrap_err();
        assert!(err.contains("verify: FAIL"), "got: {err}");
        assert!(err.contains("{version} placeholder"), "got: {err}");
    }

    #[test]
    fn run_verify_generates_extra_artifacts_when_requested() {
        let mut args = verify_args();
        args.binstall = true;
        args.rpm = true;
        assert!(run_verify(&sample_dist(), &args).is_ok());
    }
}
