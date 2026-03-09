//! FJ-1351/1352/1365/E14: Contract coverage, scaffolding, GC sweep, registry push.
//! Usage: cargo test --test falsification_contract_gc_registry

use forjar::core::store::contract_coverage::{
    coverage_report, scan_contracts_dir, BindingEntry, BindingRegistry, ContractStatus,
};
use forjar::core::store::contract_scaffold::{scaffold_contracts, write_stubs};
use forjar::core::store::gc::{mark_and_sweep, GcReport};
use forjar::core::store::gc_exec::{dir_size, sweep, sweep_dry_run};
use forjar::core::store::hf_config::KernelRequirement;
use forjar::core::store::meta::{new_meta, write_meta};
use forjar::core::store::mutation_runner::{applicable_operators, mutation_script};
use forjar::core::store::registry_push::{
    format_push_summary, head_check_command, manifest_put_command, upload_complete_command,
    upload_initiate_command, validate_push_config, RegistryPushConfig,
};
use forjar::core::types::{MutationOperator, PushKind, PushResult};
use std::collections::BTreeSet;

// ── FJ-1351: contract coverage ──

fn binding_registry(bindings: &[(&str, &str, &str)]) -> BindingRegistry {
    BindingRegistry {
        version: "1.0".into(),
        target_crate: "forjar-kernels".into(),
        bindings: bindings
            .iter()
            .map(|(contract, eq, status)| BindingEntry {
                contract: contract.to_string(),
                equation: eq.to_string(),
                status: status.to_string(),
            })
            .collect(),
    }
}

fn reqs(ops: &[(&str, &str)]) -> Vec<KernelRequirement> {
    ops.iter()
        .map(|(op, contract)| KernelRequirement {
            op: op.to_string(),
            contract: contract.to_string(),
        })
        .collect()
}

#[test]
fn coverage_all_implemented() {
    let registry = binding_registry(&[
        ("softmax-v1", "EQ-1", "implemented"),
        ("matmul-v1", "EQ-2", "implemented"),
    ]);
    let required = reqs(&[("softmax", "softmax-v1"), ("matmul", "matmul-v1")]);
    let available = vec!["softmax-v1".into(), "matmul-v1".into()];
    let report = coverage_report("llama", &required, &registry, &available);
    assert_eq!(report.covered, 2);
    assert_eq!(report.missing, 0);
    assert!((report.coverage_pct - 100.0).abs() < 0.1);
}

#[test]
fn coverage_partial_contract() {
    let registry = binding_registry(&[("softmax-v1", "EQ-1", "partial")]);
    let required = reqs(&[("softmax", "softmax-v1")]);
    let report = coverage_report("llama", &required, &registry, &[]);
    assert_eq!(report.contracts["softmax-v1"], ContractStatus::Partial);
    assert_eq!(report.covered, 0);
}

#[test]
fn coverage_missing_contract() {
    let registry = binding_registry(&[]);
    let required = reqs(&[("softmax", "softmax-v1")]);
    let report = coverage_report("llama", &required, &registry, &[]);
    assert_eq!(report.contracts["softmax-v1"], ContractStatus::Missing);
}

#[test]
fn coverage_empty_requirements() {
    let registry = binding_registry(&[]);
    let report = coverage_report("llama", &[], &registry, &[]);
    assert_eq!(report.total_required, 0);
    assert!((report.coverage_pct - 100.0).abs() < 0.1);
}

#[test]
fn coverage_implemented_but_not_available() {
    let registry = binding_registry(&[("softmax-v1", "EQ-1", "implemented")]);
    let required = reqs(&[("softmax", "softmax-v1")]);
    // Available is empty — file doesn't exist on disk
    let report = coverage_report("llama", &required, &registry, &[]);
    assert_eq!(report.contracts["softmax-v1"], ContractStatus::Missing);
}

#[test]
fn scan_contracts_dir_finds_yaml() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("softmax-v1.yaml"), "name: softmax").unwrap();
    std::fs::write(tmp.path().join("matmul-v1.yaml"), "name: matmul").unwrap();
    std::fs::write(tmp.path().join("notes.txt"), "not a contract").unwrap();
    let names = scan_contracts_dir(tmp.path()).unwrap();
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"softmax-v1".to_string()));
    assert!(names.contains(&"matmul-v1".to_string()));
}

#[test]
fn scan_contracts_dir_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let names = scan_contracts_dir(tmp.path()).unwrap();
    assert!(names.is_empty());
}

#[test]
fn scan_contracts_dir_missing() {
    assert!(
        scan_contracts_dir(std::path::Path::new("/tmp/forjar_nonexistent_dir_test_xyz")).is_err()
    );
}

// ── FJ-1352: contract scaffold ──

#[test]
fn scaffold_generates_stubs() {
    let missing = reqs(&[("softmax", "softmax-v1"), ("matmul", "matmul-v1")]);
    let stubs = scaffold_contracts(&missing, "test-author");
    assert_eq!(stubs.len(), 2);
    assert_eq!(stubs[0].filename, "softmax-v1.yaml");
    assert!(stubs[0].yaml_content.contains("softmax"));
    assert!(stubs[0].yaml_content.contains("test-author"));
    assert!(stubs[0].yaml_content.contains("SOFTMAX"));
}

#[test]
fn scaffold_empty_missing() {
    let stubs = scaffold_contracts(&[], "author");
    assert!(stubs.is_empty());
}

#[test]
fn write_stubs_creates_files() {
    let tmp = tempfile::tempdir().unwrap();
    let missing = reqs(&[("softmax", "softmax-v1")]);
    let stubs = scaffold_contracts(&missing, "test");
    let written = write_stubs(&stubs, tmp.path()).unwrap();
    assert_eq!(written.len(), 1);
    assert!(tmp.path().join("softmax-v1.yaml").exists());
}

#[test]
fn write_stubs_no_overwrite() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("softmax-v1.yaml"), "existing").unwrap();
    let missing = reqs(&[("softmax", "softmax-v1")]);
    let stubs = scaffold_contracts(&missing, "test");
    let written = write_stubs(&stubs, tmp.path()).unwrap();
    assert!(written.is_empty(), "should not overwrite existing files");
    let content = std::fs::read_to_string(tmp.path().join("softmax-v1.yaml")).unwrap();
    assert_eq!(content, "existing");
}

// ── FJ-1365: GC sweep execution ──

#[test]
fn gc_sweep_removes_dead() {
    let tmp = tempfile::tempdir().unwrap();
    let store = tmp.path();
    let live_hash = "a".repeat(64);
    let dead_hash = "b".repeat(64);
    for h in [&live_hash, &dead_hash] {
        std::fs::create_dir_all(store.join(h)).unwrap();
        write_meta(
            &store.join(h),
            &new_meta(&format!("blake3:{h}"), "blake3:r", &[], "x86_64", "apt"),
        )
        .unwrap();
    }

    let roots: BTreeSet<String> = [format!("blake3:{live_hash}")].into();
    let gc_report = mark_and_sweep(&roots, store).unwrap();
    let result = sweep(&gc_report, store).unwrap();
    assert_eq!(result.removed.len(), 1);
    assert!(result.bytes_freed > 0);
    assert!(!store.join(&dead_hash).exists());
    assert!(store.join(&live_hash).exists());
}

#[test]
fn gc_sweep_empty_report() {
    let tmp = tempfile::tempdir().unwrap();
    let report = GcReport {
        live: BTreeSet::new(),
        dead: BTreeSet::new(),
        total: 0,
    };
    let result = sweep(&report, tmp.path()).unwrap();
    assert!(result.removed.is_empty());
    assert_eq!(result.bytes_freed, 0);
}

#[test]
fn gc_dry_run_reports_sizes() {
    let tmp = tempfile::tempdir().unwrap();
    let store = tmp.path();
    let hash = "c".repeat(64);
    let dir = store.join(&hash);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("data.bin"), vec![0u8; 1024]).unwrap();

    let report = GcReport {
        live: BTreeSet::new(),
        dead: [format!("blake3:{hash}")].into(),
        total: 1,
    };
    let dry = sweep_dry_run(&report, store);
    assert_eq!(dry.len(), 1);
    assert!(dry[0].size_bytes >= 1024);
    // Verify nothing was deleted
    assert!(dir.exists());
}

#[test]
fn dir_size_empty() {
    let tmp = tempfile::tempdir().unwrap();
    assert_eq!(dir_size(tmp.path()), 0);
}

#[test]
fn dir_size_with_files() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("a"), vec![0u8; 100]).unwrap();
    std::fs::write(tmp.path().join("b"), vec![0u8; 200]).unwrap();
    assert!(dir_size(tmp.path()) >= 300);
}

#[test]
fn dir_size_nonexistent() {
    assert_eq!(
        dir_size(std::path::Path::new("/tmp/forjar_nonexistent_dir_xyz")),
        0
    );
}

// ── E14: Registry push commands ──

#[test]
fn head_check_command_format() {
    let cmd = head_check_command("ghcr.io", "myorg/app", "sha256:abc");
    assert!(cmd.contains("ghcr.io"));
    assert!(cmd.contains("myorg/app"));
    assert!(cmd.contains("sha256:abc"));
    assert!(cmd.contains("--head"));
}

#[test]
fn upload_initiate_command_format() {
    let cmd = upload_initiate_command("ghcr.io", "myorg/app");
    assert!(cmd.contains("POST"));
    assert!(cmd.contains("blobs/uploads/"));
}

#[test]
fn upload_complete_command_format() {
    let cmd = upload_complete_command("https://ghcr.io/upload/123", "sha256:abc", "/tmp/blob");
    assert!(cmd.contains("PUT"));
    assert!(cmd.contains("sha256:abc"));
    assert!(cmd.contains("/tmp/blob"));
}

#[test]
fn manifest_put_command_format() {
    let cmd = manifest_put_command("ghcr.io", "myorg/app", "v1.0", "/tmp/manifest.json");
    assert!(cmd.contains("PUT"));
    assert!(cmd.contains("manifests/v1.0"));
    assert!(cmd.contains("oci.image.manifest"));
}

#[test]
fn validate_push_config_good() {
    let cfg = RegistryPushConfig {
        registry: "ghcr.io".into(),
        name: "myorg/app".into(),
        tag: "v1.0".into(),
        check_existing: true,
    };
    assert!(validate_push_config(&cfg).is_empty());
}

#[test]
fn validate_push_config_empty_fields() {
    let cfg = RegistryPushConfig {
        registry: String::new(),
        name: String::new(),
        tag: String::new(),
        check_existing: true,
    };
    let errors = validate_push_config(&cfg);
    assert!(errors.len() >= 3);
}

#[test]
fn format_push_summary_mixed() {
    let results = vec![
        PushResult {
            kind: PushKind::Layer,
            digest: "sha256:aaa".into(),
            size: 1024,
            existed: false,
            duration_secs: 1.5,
        },
        PushResult {
            kind: PushKind::Config,
            digest: "sha256:bbb".into(),
            size: 512,
            existed: true,
            duration_secs: 0.0,
        },
    ];
    let summary = format_push_summary(&results);
    assert!(summary.contains("1024") || summary.contains("upload"));
}

// ── Mutation scripts ──

#[test]
fn mutation_script_delete_file() {
    let script = mutation_script(MutationOperator::DeleteFile, "nginx-conf");
    assert!(script.contains("nginx-conf"));
    assert!(script.contains("rm"));
}

#[test]
fn mutation_script_stop_service() {
    let script = mutation_script(MutationOperator::StopService, "nginx");
    assert!(script.contains("nginx"));
    assert!(script.contains("stop") || script.contains("systemctl"));
}

#[test]
fn applicable_operators_file() {
    let ops = applicable_operators("file");
    assert!(ops.contains(&MutationOperator::DeleteFile));
    assert!(ops.contains(&MutationOperator::ModifyContent));
    assert!(ops.contains(&MutationOperator::ChangePermissions));
}

#[test]
fn applicable_operators_service() {
    let ops = applicable_operators("service");
    assert!(ops.contains(&MutationOperator::StopService));
}

#[test]
fn applicable_operators_package() {
    let ops = applicable_operators("package");
    assert!(ops.contains(&MutationOperator::RemovePackage));
}

#[test]
fn applicable_operators_unknown() {
    let ops = applicable_operators("unknown_type");
    assert!(ops.is_empty());
}
