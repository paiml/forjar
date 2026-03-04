//! Coverage tests for dispatch_validate_c.rs — all try_validate_* routing.

#![allow(unused_imports)]
use super::dispatch_validate_c::*;
use std::io::Write;

#[cfg(test)]
mod tests {
    use super::*;

    fn write_cfg(yaml: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(yaml.as_bytes()).unwrap();
        f.flush().unwrap();
        f
    }

    const CFG: &str = "version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    tags:\n      - web\n  cfg:\n    machine: m1\n    type: file\n    path: /etc/nginx.conf\n    content: hi\n    state: present\n    depends_on:\n      - pkg\n    tags:\n      - web\n";

    // try_validate_phases_94_96: file, json, 8 bools
    #[test]
    fn test_p94_96_x1() {
        let f = write_cfg(CFG);
        assert!(try_validate_phases_94_96(f.path(), false, true, false, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_p94_96_b3() {
        let f = write_cfg(CFG);
        assert!(try_validate_phases_94_96(f.path(), false, false, false, false, false, false, false, false, true).is_some());
    }
    #[test]
    fn test_p94_96_none() {
        let f = write_cfg(CFG);
        assert!(try_validate_phases_94_96(f.path(), false, false, false, false, false, false, false, false, false).is_none());
    }

    // try_validate_checks_early_a: file, json, 4 bools, Option<&str>, Option<usize>, 4 bools
    #[test]
    fn test_checks_a_cron() {
        let f = write_cfg(CFG);
        assert!(try_validate_checks_early_a(f.path(), false, true, false, None, None, false, false, false, false).is_some());
    }
    #[test]
    fn test_checks_a_names() {
        let f = write_cfg(CFG);
        assert!(try_validate_checks_early_a(f.path(), false, false, false, Some("^[a-z]"), None, false, false, false, false).is_some());
    }
    #[test]
    fn test_checks_a_count() {
        let f = write_cfg(CFG);
        assert!(try_validate_checks_early_a(f.path(), false, false, false, None, Some(100), false, false, false, false).is_some());
    }
    #[test]
    fn test_checks_a_none() {
        let f = write_cfg(CFG);
        assert!(try_validate_checks_early_a(f.path(), false, false, false, None, None, false, false, false, false).is_none());
    }

    // try_validate_checks_early_b: file, json, 7 bools
    #[test]
    fn test_checks_b_state_values() {
        let f = write_cfg(CFG);
        assert!(try_validate_checks_early_b(f.path(), false, true, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_checks_b_resource_groups() {
        let f = write_cfg(CFG);
        assert!(try_validate_checks_early_b(f.path(), false, false, false, false, false, false, false, true).is_some());
    }
    #[test]
    fn test_checks_b_none() {
        let f = write_cfg(CFG);
        assert!(try_validate_checks_early_b(f.path(), false, false, false, false, false, false, false, false).is_none());
    }

    // try_validate_phases_97_100: file, json, 12 bools
    #[test]
    fn test_p97_100_a1() {
        let f = write_cfg(CFG);
        assert!(try_validate_phases_97_100(f.path(), false, true, false, false, false, false, false, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_p97_100_d3() {
        let f = write_cfg(CFG);
        assert!(try_validate_phases_97_100(f.path(), false, false, false, false, false, false, false, false, false, false, false, false, true).is_some());
    }
    #[test]
    fn test_p97_100_none() {
        let f = write_cfg(CFG);
        assert!(try_validate_phases_97_100(f.path(), false, false, false, false, false, false, false, false, false, false, false, false, false).is_none());
    }

    // try_validate_phases_101_103: file, json, 9 bools
    #[test]
    fn test_p101_103_e1() {
        let f = write_cfg(CFG);
        assert!(try_validate_phases_101_103(f.path(), false, true, false, false, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_p101_103_g3() {
        let f = write_cfg(CFG);
        assert!(try_validate_phases_101_103(f.path(), false, false, false, false, false, false, false, false, false, true).is_some());
    }
    #[test]
    fn test_p101_103_none() {
        let f = write_cfg(CFG);
        assert!(try_validate_phases_101_103(f.path(), false, false, false, false, false, false, false, false, false, false).is_none());
    }

    // try_validate_phases_104_106: file, json, 9 bools
    #[test]
    fn test_p104_106_h1() {
        let f = write_cfg(CFG);
        assert!(try_validate_phases_104_106(f.path(), false, true, false, false, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_p104_106_none() {
        let f = write_cfg(CFG);
        assert!(try_validate_phases_104_106(f.path(), false, false, false, false, false, false, false, false, false, false).is_none());
    }

    // try_validate_phase107: file, json, 3 bools
    #[test]
    fn test_p107_k1() {
        let f = write_cfg(CFG);
        assert!(try_validate_phase107(f.path(), false, true, false, false).is_some());
    }
    #[test]
    fn test_p107_none() {
        let f = write_cfg(CFG);
        assert!(try_validate_phase107(f.path(), false, false, false, false).is_none());
    }

    // try_validate_store: file, json, 2 bools
    #[test]
    fn test_store_purity() {
        let f = write_cfg(CFG);
        assert!(try_validate_store(f.path(), false, true, false).is_some());
    }
    #[test]
    fn test_store_repro() {
        let f = write_cfg(CFG);
        assert!(try_validate_store(f.path(), false, false, true).is_some());
    }
    #[test]
    fn test_store_none() {
        let f = write_cfg(CFG);
        assert!(try_validate_store(f.path(), false, false, false).is_none());
    }
}
