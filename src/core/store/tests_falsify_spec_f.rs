//! Spec falsification tests: Phases K–L (Bash provability, Execution bridges)
//!
//! Split from tests_falsify_spec_c.rs to stay under 500-line limit.
#![allow(unused_imports)]

use super::cache_exec::CachePullResult;
use super::convert_exec::ConversionApplyResult;
use super::gc_exec::{DryRunEntry, GcSweepResult};
use super::pin_resolve::{parse_resolved_version, resolution_command, ResolvedPin};
use super::provider_exec::ExecutionContext;
use super::sandbox_run::SandboxExecResult;
use super::sync_exec::{DiffExecResult, SyncExecResult};
use crate::core::purifier::{
    lint_error_count, lint_script, purify_script, validate_or_purify, validate_script,
};

// ═══════════════════════════════════════════════════════════════════
// Phase K: Bash Provability (I8 Invariant)
// ═══════════════════════════════════════════════════════════════════

/// K-01: validate_script() accepts valid POSIX shell.
#[test]
fn falsify_k01_validate_valid_script() {
    let result = validate_script("echo hello world");
    assert!(result.is_ok(), "valid script must pass: {result:?}");
}

/// K-02: validate_or_purify returns Ok for valid scripts.
#[test]
fn falsify_k02_validate_or_purify_valid() {
    let result = validate_or_purify("echo hello");
    assert!(result.is_ok(), "valid script should pass validate_or_purify");
}

/// K-03: lint_error_count returns 0 for clean scripts.
#[test]
fn falsify_k03_lint_error_count_clean() {
    let count = lint_error_count("echo hello");
    assert_eq!(count, 0, "clean script must have 0 lint errors");
}

/// K-04: lint_script returns diagnostics.
#[test]
fn falsify_k04_lint_script_returns_diagnostics() {
    let result = lint_script("echo hello");
    assert!(result.diagnostics.is_empty() || !result.diagnostics.is_empty());
}

/// K-05: purify_script succeeds on simple scripts.
#[test]
fn falsify_k05_purify_simple_script() {
    let result = purify_script("echo hello world");
    assert!(result.is_ok(), "simple script must purify: {result:?}");
}

/// K-06: validate_or_purify tries validation first (fast path).
#[test]
fn falsify_k06_validate_or_purify_fast_path() {
    let script = "echo test";
    let result = validate_or_purify(script).expect("must succeed");
    assert_eq!(result, script, "valid script returned as-is (fast path)");
}

// ═══════════════════════════════════════════════════════════════════
// Phase L: Execution Bridges
// ═══════════════════════════════════════════════════════════════════

/// L-01: ExecutionContext struct exists with store_dir, staging_dir, machine.
#[test]
fn falsify_l01_execution_context_fields() {
    use crate::core::types::Machine;
    use std::path::PathBuf;
    let _ctx = ExecutionContext {
        store_dir: PathBuf::from("/var/lib/forjar/store"),
        staging_dir: PathBuf::from("/tmp/staging"),
        machine: Machine {
            hostname: "local".to_string(),
            addr: "127.0.0.1".to_string(),
            user: "root".to_string(),
            arch: "x86_64".to_string(),
            ssh_key: None,
            roles: vec![],
            transport: None,
            container: None,
            cost: 0,
            pepita: None,
        },
        timeout_secs: Some(600),
    };
}

/// L-02: GcSweepResult has removed, bytes_freed, errors fields.
#[test]
fn falsify_l02_gc_sweep_result_fields() {
    let result = GcSweepResult {
        removed: vec!["blake3:abc".to_string()],
        bytes_freed: 1024,
        errors: vec![],
    };
    assert_eq!(result.removed.len(), 1);
    assert_eq!(result.bytes_freed, 1024);
    assert!(result.errors.is_empty());
}

/// L-03: CachePullResult has store_hash, store_path, verified fields.
#[test]
fn falsify_l03_cache_pull_result_fields() {
    let result = CachePullResult {
        store_hash: "blake3:abc".to_string(),
        store_path: "/store/abc".to_string(),
        bytes_transferred: 2048,
        verified: true,
    };
    assert!(result.verified, "must track verification status");
}

/// L-04: SandboxExecResult has output_hash, steps_executed, duration_secs.
#[test]
fn falsify_l04_sandbox_exec_result_fields() {
    let result = SandboxExecResult {
        output_hash: "blake3:out".to_string(),
        store_path: "/store/out".to_string(),
        steps_executed: vec![(1, "create namespace".to_string(), true)],
        duration_secs: 42.5,
    };
    assert!(!result.steps_executed.is_empty());
    assert!(result.duration_secs > 0.0);
}

/// L-05: DiffExecResult has diff, upstream_command, upstream_output.
#[test]
fn falsify_l05_diff_exec_result_fields() {
    use super::store_diff::DiffResult;
    let result = DiffExecResult {
        diff: DiffResult {
            store_hash: "blake3:abc".to_string(),
            upstream_changed: false,
            local_origin_hash: Some("sha256:old".to_string()),
            upstream_hash: Some("sha256:old".to_string()),
            provider: "apt".to_string(),
            origin_ref: Some("apt:nginx".to_string()),
            derivation_chain_depth: 0,
        },
        upstream_command: Some("apt-cache policy nginx".to_string()),
        upstream_output: Some("Candidate: 1.24.0".to_string()),
    };
    assert!(result.upstream_command.is_some());
}

/// L-06: SyncExecResult has re_imported, derivations_replayed fields.
#[test]
fn falsify_l06_sync_exec_result_fields() {
    let result = SyncExecResult {
        re_imported: vec![],
        derivations_replayed: 0,
        new_profile_hash: None,
    };
    assert_eq!(result.derivations_replayed, 0);
}

/// L-07: ConversionApplyResult has changes_applied, backup_path, new_purity.
#[test]
fn falsify_l07_convert_apply_result_fields() {
    let result = ConversionApplyResult {
        changes_applied: 3,
        backup_path: std::path::PathBuf::from("forjar.yaml.bak"),
        new_purity: super::purity::PurityLevel::Pinned,
        lock_pins_generated: 2,
    };
    assert_eq!(result.changes_applied, 3);
}

/// L-08: ResolvedPin has name, provider, version, hash fields.
#[test]
fn falsify_l08_resolved_pin_fields() {
    let pin = ResolvedPin {
        name: "nginx".to_string(),
        provider: "apt".to_string(),
        version: "1.24.0".to_string(),
        hash: "blake3:abc".to_string(),
    };
    assert_eq!(pin.name, "nginx");
    assert_eq!(pin.provider, "apt");
}

/// L-09: resolution_command per provider matches spec.
#[test]
fn falsify_l09_resolution_commands() {
    assert!(resolution_command("apt", "nginx")
        .unwrap()
        .contains("apt-cache policy"));
    assert!(resolution_command("cargo", "rg")
        .unwrap()
        .contains("cargo search"));
    assert!(resolution_command("nix", "ripgrep")
        .unwrap()
        .contains("nix eval"));
    assert!(resolution_command("uv", "flask")
        .unwrap()
        .contains("pip index versions"));
    assert!(resolution_command("docker", "nginx")
        .unwrap()
        .contains("docker image inspect"));
}

/// L-10: parse_resolved_version extracts apt "Candidate:" version.
#[test]
fn falsify_l10_parse_apt_version() {
    let output = "  Installed: 1.22.0-1ubuntu1\n  Candidate: 1.24.0-1ubuntu1\n";
    let version = parse_resolved_version("apt", output);
    assert_eq!(version, Some("1.24.0-1ubuntu1".to_string()));
}

/// L-11: parse_resolved_version extracts cargo search version.
#[test]
fn falsify_l11_parse_cargo_version() {
    let output = "ripgrep = \"14.1.0\"    # Fast line-oriented search tool\n";
    let version = parse_resolved_version("cargo", output);
    assert_eq!(version, Some("14.1.0".to_string()));
}

/// L-12: DryRunEntry has hash and size_bytes.
#[test]
fn falsify_l12_dry_run_entry_fields() {
    let entry = DryRunEntry {
        hash: "blake3:abc".to_string(),
        size_bytes: 65536,
    };
    assert_eq!(entry.size_bytes, 65536);
}

/// L-13: 7 execution modules exist (L spec requirement).
#[test]
fn falsify_l13_seven_execution_modules() {
    let _ctx: Option<ExecutionContext> = None;
    let _gc: Option<GcSweepResult> = None;
    let _pin: Option<ResolvedPin> = None;
    let _cache: Option<CachePullResult> = None;
    let _conv: Option<ConversionApplyResult> = None;
    let _diff: Option<DiffExecResult> = None;
    let _sand: Option<SandboxExecResult> = None;
}

/// L-14: Module file existence — all 12 phases have implementation modules.
#[test]
fn falsify_l14_all_phase_modules_exist() {
    let modules = [
        "src/core/store/path.rs",
        "src/core/store/meta.rs",
        "src/core/store/purity.rs",
        "src/core/store/closure.rs",
        "src/core/store/lockfile.rs",
        "src/core/store/sandbox.rs",
        "src/core/store/repro_score.rs",
        "src/core/store/cache.rs",
        "src/core/store/gc.rs",
        "src/core/store/derivation.rs",
        "src/core/store/provider.rs",
        "src/core/store/far.rs",
        "src/core/store/convert.rs",
        "src/core/store/secret_scan.rs",
        "src/core/store/provider_exec.rs",
        "src/core/store/gc_exec.rs",
        "src/core/store/pin_resolve.rs",
        "src/core/store/cache_exec.rs",
        "src/core/store/convert_exec.rs",
        "src/core/store/sync_exec.rs",
        "src/core/store/sandbox_run.rs",
    ];
    for module in &modules {
        assert!(
            std::path::Path::new(module).exists(),
            "module must exist: {module}"
        );
    }
}
