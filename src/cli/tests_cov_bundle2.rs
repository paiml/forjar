//! Coverage tests for bundle.rs, sbom.rs, doctor.rs, secrets.rs, observe.rs, drift.rs.

#![allow(unused_imports)]
use super::bundle::*;
use super::sbom::*;
use super::doctor::*;
use super::secrets::*;
use super::observe::*;
use super::drift::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_cfg(yaml: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(yaml.as_bytes()).unwrap();
        f.flush().unwrap();
        f
    }

    fn write_yaml(dir: &std::path::Path, name: &str, content: &str) {
        let p = dir.join(name);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&p, content).unwrap();
    }

    const CFG: &str = "version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n  cfg:\n    machine: m1\n    type: file\n    path: /etc/nginx.conf\n    content: hi\n    state: present\n    depends_on:\n      - pkg\n";

    fn setup_state() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "m1/state.lock.yaml", "resources:\n  pkg:\n    resource_type: Package\n    status: Converged\n    hash: abc123\n    applied_at: '2025-01-01T00:00:00Z'\n    duration_seconds: 1.0\n");
        write_yaml(dir.path(), "m1/events.jsonl", "{\"ts\":\"2026-01-01T00:00:00Z\",\"event\":\"resource_started\",\"resource\":\"pkg\",\"machine\":\"m1\"}\n{\"ts\":\"2026-01-01T00:00:01Z\",\"event\":\"resource_converged\",\"resource\":\"pkg\",\"machine\":\"m1\"}\n");
        dir
    }

    // bundle.rs
    #[test]
    fn test_bundle_basic() {
        let f = write_cfg(CFG);
        let out = tempfile::tempdir().unwrap();
        let bundle_path = out.path().join("test.bundle");
        let _ = cmd_bundle(f.path(), Some(bundle_path.as_path()), false);
    }
    #[test]
    fn test_bundle_include_state() {
        let f = write_cfg(CFG);
        let out = tempfile::tempdir().unwrap();
        let bundle_path = out.path().join("test.bundle");
        let _ = cmd_bundle(f.path(), Some(bundle_path.as_path()), true);
    }
    #[test]
    fn test_bundle_no_output() {
        let f = write_cfg(CFG);
        let _ = cmd_bundle(f.path(), None, false);
    }
    #[test]
    fn test_bundle_verify() {
        let f = write_cfg(CFG);
        let _ = cmd_bundle_verify(f.path());
    }

    // sbom.rs
    #[test]
    fn test_sbom() {
        let f = write_cfg(CFG);
        let d = setup_state();
        let _ = cmd_sbom(f.path(), d.path(), false);
    }
    #[test]
    fn test_sbom_json() {
        let f = write_cfg(CFG);
        let d = setup_state();
        let _ = cmd_sbom(f.path(), d.path(), true);
    }

    // doctor.rs
    #[test]
    fn test_doctor_no_file() {
        let _ = cmd_doctor(None, false, false);
    }
    #[test]
    fn test_doctor_with_file() {
        let f = write_cfg(CFG);
        let _ = cmd_doctor(Some(f.path()), false, false);
    }
    #[test]
    fn test_doctor_json() {
        let f = write_cfg(CFG);
        let _ = cmd_doctor(Some(f.path()), true, false);
    }
    #[test]
    fn test_doctor_fix() {
        let f = write_cfg(CFG);
        let _ = cmd_doctor(Some(f.path()), false, true);
    }
    #[test]
    fn test_doctor_network() {
        let _ = cmd_doctor_network(None, false);
    }
    #[test]
    fn test_doctor_network_json() {
        let f = write_cfg(CFG);
        let _ = cmd_doctor_network(Some(f.path()), true);
    }

    // secrets.rs
    #[test]
    fn test_secrets_find_enc_markers_none() {
        assert!(find_enc_markers("hello world").is_empty());
    }
    #[test]
    fn test_secrets_find_enc_markers_found() {
        let s = "ENC[age,abc123]";
        let markers = find_enc_markers(s);
        assert!(!markers.is_empty());
    }
    #[test]
    fn test_secrets_keygen() {
        let _ = cmd_secrets_keygen();
    }
    #[test]
    fn test_secrets_encrypt() {
        let _ = cmd_secrets_encrypt("test-value", &["age1test".to_string()]);
    }
    #[test]
    fn test_secrets_decrypt() {
        let _ = cmd_secrets_decrypt("ENC[age:abc]", None);
    }
    #[test]
    fn test_secrets_view() {
        let f = write_cfg(CFG);
        let _ = cmd_secrets_view(f.path(), None);
    }
    #[test]
    fn test_secrets_rekey() {
        let f = write_cfg(CFG);
        let _ = cmd_secrets_rekey(f.path(), None, &["age1new".to_string()]);
    }
    #[test]
    fn test_secrets_rotate() {
        let f = write_cfg(CFG);
        let d = setup_state();
        let _ = cmd_secrets_rotate(f.path(), None, &["age1new".to_string()], false, d.path());
    }

    // observe.rs
    #[test]
    fn test_anomaly() {
        let d = setup_state();
        let _ = cmd_anomaly(d.path(), None, 1, false);
    }
    #[test]
    fn test_anomaly_json() {
        let d = setup_state();
        let _ = cmd_anomaly(d.path(), None, 1, true);
    }
    #[test]
    fn test_anomaly_machine_filter() {
        let d = setup_state();
        let _ = cmd_anomaly(d.path(), Some("m1"), 1, false);
    }
    #[test]
    fn test_trace() {
        let d = setup_state();
        let _ = cmd_trace(d.path(), None, false);
    }
    #[test]
    fn test_trace_json() {
        let d = setup_state();
        let _ = cmd_trace(d.path(), None, true);
    }
    #[test]
    fn test_trace_machine_filter() {
        let d = setup_state();
        let _ = cmd_trace(d.path(), Some("m1"), false);
    }

    // drift.rs
    #[test]
    fn test_drift_dry_run() {
        let d = setup_state();
        let _ = cmd_drift_dry_run(d.path(), None, false);
    }
    #[test]
    fn test_drift_dry_run_json() {
        let d = setup_state();
        let _ = cmd_drift_dry_run(d.path(), None, true);
    }
    #[test]
    fn test_drift_dry_run_machine() {
        let d = setup_state();
        let _ = cmd_drift_dry_run(d.path(), Some("m1"), false);
    }
    #[test]
    fn test_drift_full() {
        let f = write_cfg(CFG);
        let d = setup_state();
        let _ = cmd_drift(f.path(), d.path(), None, false, None, false, true, false, false, None);
    }
    #[test]
    fn test_drift_json() {
        let f = write_cfg(CFG);
        let d = setup_state();
        let _ = cmd_drift(f.path(), d.path(), None, false, None, false, true, true, false, None);
    }
    #[test]
    fn test_drift_verbose() {
        let f = write_cfg(CFG);
        let d = setup_state();
        let _ = cmd_drift(f.path(), d.path(), None, false, None, false, true, false, true, None);
    }
    #[test]
    fn test_drift_tripwire() {
        let f = write_cfg(CFG);
        let d = setup_state();
        let _ = cmd_drift(f.path(), d.path(), None, true, None, false, true, false, false, None);
    }
}
