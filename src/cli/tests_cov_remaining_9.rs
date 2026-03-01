//! Tests: Coverage for validate_compliance, validate_structural, lock (part 9).

#![allow(unused_imports)]
use super::lock_security::*;
use super::lock_ops::*;
use super::lock_core::*;
use super::destroy::*;
use super::observe::*;
use super::validate_compliance::*;
use super::validate_structural::*;
use std::io::Write;

#[cfg(test)]
mod tests {
    use super::*;

    fn write_temp_config(yaml: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(yaml.as_bytes()).unwrap();
        f.flush().unwrap();
        f
    }

    fn empty_config() -> String {
        concat!(
            "version: \"1.0\"\n",
            "name: t\n",
            "machines:\n",
            "  m:\n",
            "    hostname: m\n",
            "    addr: 127.0.0.1\n",
            "resources: {}\n",
        )
        .to_string()
    }

    fn basic_config() -> String {
        concat!(
            "version: \"1.0\"\n",
            "name: test-project\n",
            "machines:\n",
            "  web:\n",
            "    hostname: web\n",
            "    addr: 127.0.0.1\n",
            "resources:\n",
            "  app-config:\n",
            "    type: file\n",
            "    machine: web\n",
            "    path: /etc/app.conf\n",
            "    content: \"port=8080\"\n",
            "    owner: root\n",
            "    group: root\n",
            "    mode: \"0644\"\n",
            "  web-svc:\n",
            "    type: service\n",
            "    machine: web\n",
            "    name: nginx\n",
            "    depends_on: [app-config]\n",
        )
        .to_string()
    }

    #[test]
    fn test_cov_compliance_hipaa_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_compliance(f.path(), "HIPAA", false).is_ok());
    }

    /// HIPAA: mode with nonzero 'other' bits triggers violation.
    #[test]
    fn test_cov_compliance_hipaa_nonzero_other() {
        let yaml = concat!(
            "version: \"1.0\"\n",
            "name: t\n",
            "machines:\n",
            "  m:\n",
            "    hostname: m\n",
            "    addr: 127.0.0.1\n",
            "resources:\n",
            "  phi:\n",
            "    type: file\n",
            "    machine: m\n",
            "    path: /etc/phi.dat\n",
            "    content: patient-data\n",
            "    mode: \"0644\"\n",
        );
        let f = write_temp_config(yaml);
        assert!(cmd_validate_check_compliance(f.path(), "HIPAA", false).is_ok());
    }

    #[test]
    fn test_cov_compliance_hipaa_json() {
        let yaml = concat!(
            "version: \"1.0\"\n",
            "name: t\n",
            "machines:\n",
            "  m:\n",
            "    hostname: m\n",
            "    addr: 127.0.0.1\n",
            "resources:\n",
            "  phi:\n",
            "    type: file\n",
            "    machine: m\n",
            "    path: /etc/phi.dat\n",
            "    content: patient-data\n",
            "    mode: \"0644\"\n",
        );
        let f = write_temp_config(yaml);
        assert!(cmd_validate_check_compliance(f.path(), "HIPAA", true).is_ok());
    }

    /// Unknown policy returns error.
    #[test]
    fn test_cov_compliance_unknown_policy() {
        let f = write_temp_config(&basic_config());
        let result = cmd_validate_check_compliance(f.path(), "NIST", false);
        assert!(result.is_err());
    }

    // ========================================================================
    // 45. validate_compliance: portability
    // ========================================================================

    #[test]
    fn test_cov_portability_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_portability(f.path(), false).is_ok());
    }

    #[test]
    fn test_cov_portability_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_portability(f.path(), false).is_ok());
    }

    #[test]
    fn test_cov_portability_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_portability(f.path(), true).is_ok());
    }

    /// /sys path triggers portability warning.
    #[test]
    fn test_cov_portability_sys_path() {
        let yaml = concat!(
            "version: \"1.0\"\n",
            "name: t\n",
            "machines:\n",
            "  m:\n",
            "    hostname: m\n",
            "    addr: 127.0.0.1\n",
            "resources:\n",
            "  kern:\n",
            "    type: file\n",
            "    machine: m\n",
            "    path: /sys/kernel/param\n",
            "    content: 1\n",
        );
        let f = write_temp_config(yaml);
        assert!(cmd_validate_check_portability(f.path(), false).is_ok());
    }

    /// /proc path triggers portability warning.
    #[test]
    fn test_cov_portability_proc_path() {
        let yaml = concat!(
            "version: \"1.0\"\n",
            "name: t\n",
            "machines:\n",
            "  m:\n",
            "    hostname: m\n",
            "    addr: 127.0.0.1\n",
            "resources:\n",
            "  proc:\n",
            "    type: file\n",
            "    machine: m\n",
            "    path: /proc/sys/net/ipv4\n",
            "    content: 1\n",
        );
        let f = write_temp_config(yaml);
        assert!(cmd_validate_check_portability(f.path(), true).is_ok());
    }

    /// apt provider triggers portability warning.
    #[test]
    fn test_cov_portability_apt() {
        let yaml = concat!(
            "version: \"1.0\"\n",
            "name: t\n",
            "machines:\n",
            "  m:\n",
            "    hostname: m\n",
            "    addr: 127.0.0.1\n",
            "resources:\n",
            "  pkg:\n",
            "    type: package\n",
            "    machine: m\n",
            "    provider: apt\n",
            "    packages: [curl]\n",
        );
        let f = write_temp_config(yaml);
        assert!(cmd_validate_check_portability(f.path(), false).is_ok());
    }

    // ========================================================================
    // 46. validate_compliance: idempotency_deep
    // ========================================================================

    #[test]
    fn test_cov_idempotency_deep_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_idempotency_deep(f.path(), false).is_ok());
    }

    #[test]
    fn test_cov_idempotency_deep_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_idempotency_deep(f.path(), false).is_ok());
    }

    #[test]
    fn test_cov_idempotency_deep_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_idempotency_deep(f.path(), true).is_ok());
    }

    /// Dynamic shell expansion triggers idempotency suspect.
    #[test]
    fn test_cov_idempotency_deep_hostname() {
        let yaml = concat!(
            "version: \"1.0\"\n",
            "name: t\n",
            "machines:\n",
            "  m:\n",
            "    hostname: m\n",
            "    addr: 127.0.0.1\n",
            "resources:\n",
            "  dyn:\n",
            "    type: file\n",
            "    machine: m\n",
            "    path: /tmp/dyn\n",
            "    content: \"host=$(hostname)\"\n",
        );
        let f = write_temp_config(yaml);
        assert!(cmd_validate_check_idempotency_deep(f.path(), false).is_ok());
    }

    /// $RANDOM in content triggers idempotency suspect.
    #[test]
    fn test_cov_idempotency_deep_random() {
        let yaml = concat!(
            "version: \"1.0\"\n",
            "name: t\n",
            "machines:\n",
            "  m:\n",
            "    hostname: m\n",
            "    addr: 127.0.0.1\n",
            "resources:\n",
            "  rng:\n",
            "    type: file\n",
            "    machine: m\n",
            "    path: /tmp/rng\n",
            "    content: \"seed=$RANDOM\"\n",
        );
        let f = write_temp_config(yaml);
        assert!(cmd_validate_check_idempotency_deep(f.path(), true).is_ok());
    }

    /// File content without mode triggers idempotency suspect.
    #[test]
    fn test_cov_idempotency_deep_no_mode() {
        let yaml = concat!(
            "version: \"1.0\"\n",
            "name: t\n",
            "machines:\n",
            "  m:\n",
            "    hostname: m\n",
            "    addr: 127.0.0.1\n",
            "resources:\n",
            "  nomode:\n",
            "    type: file\n",
            "    machine: m\n",
            "    path: /tmp/nomode\n",
            "    content: data\n",
        );
        let f = write_temp_config(yaml);
        assert!(cmd_validate_check_idempotency_deep(f.path(), false).is_ok());
    }

    // ========================================================================
    // 47. validate_structural: cmd_validate_check_cycles_deep
    // ========================================================================

    #[test]
    fn test_cov_cycles_deep_empty() {
        let f = write_temp_config(&empty_config());
        assert!(cmd_validate_check_cycles_deep(f.path(), false).is_ok());
    }

    #[test]
    fn test_cov_cycles_deep_basic() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_cycles_deep(f.path(), false).is_ok());
    }

    #[test]
    fn test_cov_cycles_deep_json() {
        let f = write_temp_config(&basic_config());
        assert!(cmd_validate_check_cycles_deep(f.path(), true).is_ok());
    }

    // ========================================================================
    // 48. validate_structural: CIS with root in /tmp
    // ========================================================================

    #[test]
    fn test_cov_cis_root_tmp() {
        let yaml = concat!(
            "version: \"1.0\"\n",
            "name: t\n",
            "machines:\n",
            "  m:\n",
            "    hostname: m\n",
            "    addr: 127.0.0.1\n",
            "resources:\n",
            "  tmp-root:\n",
            "    type: file\n",
            "    machine: m\n",
            "    path: /tmp/sensitive\n",
            "    content: data\n",
            "    owner: root\n",
            "    mode: \"0644\"\n",
        );
        let f = write_temp_config(yaml);
        assert!(cmd_validate_check_compliance(f.path(), "CIS", false).is_ok());
    }

    /// CIS with mode ending in 6.
    #[test]
    fn test_cov_cis_mode_ending_6() {
        let yaml = concat!(
            "version: \"1.0\"\n",
            "name: t\n",
            "machines:\n",
            "  m:\n",
            "    hostname: m\n",
            "    addr: 127.0.0.1\n",
            "resources:\n",
            "  readable:\n",
            "    type: file\n",
            "    machine: m\n",
            "    path: /etc/readable\n",
            "    content: data\n",
            "    mode: \"0646\"\n",
        );
        let f = write_temp_config(yaml);
        assert!(cmd_validate_check_compliance(f.path(), "CIS", true).is_ok());
    }

    // ========================================================================
    // 49. Lock validate with bad schema version
    // ========================================================================

    #[test]
    fn test_cov_lock_validate_bad_schema() {
        let td = tempfile::tempdir().unwrap();
        let mdir = td.path().join("web");
        std::fs::create_dir_all(&mdir).unwrap();
        std::fs::write(
            mdir.join("state.lock.yaml"),
            "schema: \"1\"\nmachine: web\nhostname: web\nresources: {}\n",
        )
        .unwrap();
        // Write flat lock with bad schema
        std::fs::write(
            td.path().join("web.lock.yaml"),
            "schema: \"99\"\nmachine: web\nhostname: web\ngenerated_at: now\ngenerator: test\nresources: {}\n",
        )
        .unwrap();
        assert!(cmd_lock_validate(td.path(), false).is_ok());
    }

    #[test]
    fn test_cov_lock_validate_bad_schema_json() {
        let td = tempfile::tempdir().unwrap();
        let mdir = td.path().join("web");
        std::fs::create_dir_all(&mdir).unwrap();
        std::fs::write(
            mdir.join("state.lock.yaml"),
            "schema: \"1\"\nmachine: web\nhostname: web\nresources: {}\n",
        )
        .unwrap();
        std::fs::write(
            td.path().join("web.lock.yaml"),
            "schema: \"99\"\nmachine: web\nhostname: web\ngenerated_at: now\ngenerator: test\nresources: {}\n",
        )
        .unwrap();
        assert!(cmd_lock_validate(td.path(), true).is_ok());
    }

    // ========================================================================
    // 50. Lock integrity with bad schema
    // ========================================================================

    #[test]
    fn test_cov_lock_integrity_bad_schema() {
        let td = tempfile::tempdir().unwrap();
        let mdir = td.path().join("web");
        std::fs::create_dir_all(&mdir).unwrap();
        std::fs::write(
            mdir.join("state.lock.yaml"),
            "schema: \"1\"\nmachine: web\nhostname: web\nresources: {}\n",
        )
        .unwrap();
        std::fs::write(
            td.path().join("web.lock.yaml"),
            "schema: \"2\"\nmachine: web\nhostname: web\ngenerated_at: now\ngenerator: test\nresources: {}\n",
        )
        .unwrap();
        assert!(cmd_lock_integrity(td.path(), false).is_ok());
    }

    // ========================================================================
    // 51. Consecutive hyphens in naming conventions
    // ========================================================================

    #[test]
    fn test_cov_naming_conventions_consecutive_hyphens() {
        let yaml = concat!(
            "version: \"1.0\"\n",
            "name: t\n",
            "machines:\n",
            "  m:\n",
            "    hostname: m\n",
            "    addr: 127.0.0.1\n",
            "resources:\n",
            "  bad--name:\n",
            "    type: file\n",
            "    machine: m\n",
            "    path: /tmp/x\n",
            "    content: hi\n",
        );
        let f = write_temp_config(yaml);
        let _ = cmd_validate_check_naming_conventions(f.path(), false);
    }

    #[test]
    fn test_cov_naming_conventions_trailing_hyphen() {
        let yaml = concat!(
            "version: \"1.0\"\n",
            "name: t\n",
            "machines:\n",
            "  m:\n",
            "    hostname: m\n",
            "    addr: 127.0.0.1\n",
            "resources:\n",
            "  trailing-:\n",
            "    type: file\n",
            "    machine: m\n",
            "    path: /tmp/x\n",
            "    content: hi\n",
        );
        let f = write_temp_config(yaml);
        let _ = cmd_validate_check_naming_conventions(f.path(), true);
    }
}
