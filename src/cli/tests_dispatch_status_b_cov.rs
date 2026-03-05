//! Coverage tests for dispatch_status_b.rs — all try_status_phases* routing.

#![allow(unused_imports)]
use super::dispatch_status_b::*;
use std::io::Write as IoWrite;

#[cfg(test)]
mod tests {
    use super::*;

    fn write_yaml(dir: &std::path::Path, name: &str, content: &str) {
        let p = dir.join(name);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&p, content).unwrap();
    }

    fn setup_state() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        write_yaml(dir.path(), "web1/state.lock.yaml", "resources:\n  nginx:\n    resource_type: Package\n    status: Converged\n    hash: abc123\n    applied_at: '2025-01-01T00:00:00Z'\n    duration_seconds: 2.5\n  mysql:\n    resource_type: Package\n    status: Failed\n    hash: def456\n");
        write_yaml(dir.path(), "web1/events.jsonl", "{\"ts\":\"2026-01-01T00:00:00Z\",\"event\":\"resource_started\",\"resource\":\"nginx\",\"machine\":\"web1\"}\n");
        dir
    }

    // try_status_phases_94_96: sd, machine, json, 9 bools
    #[test]
    fn test_p94_96_a1() {
        let d = setup_state();
        assert!(try_status_phases_94_96(d.path(), None, false, true, false, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_p94_96_c3() {
        let d = setup_state();
        assert!(try_status_phases_94_96(d.path(), None, false, false, false, false, false, false, false, false, true).is_some());
    }
    #[test]
    fn test_p94_96_none() {
        let d = setup_state();
        assert!(try_status_phases_94_96(d.path(), None, false, false, false, false, false, false, false, false, false).is_none());
    }

    // try_status_phases_97_99: sd, machine, json, 9 bools
    #[test]
    fn test_p97_99_d1() {
        let d = setup_state();
        assert!(try_status_phases_97_99(d.path(), None, false, true, false, false, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_p97_99_f3() {
        let d = setup_state();
        assert!(try_status_phases_97_99(d.path(), None, false, false, false, false, false, false, false, false, false, true).is_some());
    }
    #[test]
    fn test_p97_99_none() {
        let d = setup_state();
        assert!(try_status_phases_97_99(d.path(), None, false, false, false, false, false, false, false, false, false, false).is_none());
    }

    // try_status_phases_100_103: sd, machine, json, 12 bools
    #[test]
    fn test_p100_103_cadence() {
        let d = setup_state();
        assert!(try_status_phases_100_103(d.path(), None, false, true, false, false, false, false, false, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_p100_103_complexity() {
        let d = setup_state();
        assert!(try_status_phases_100_103(d.path(), None, false, false, false, false, false, false, false, false, false, false, false, false, true).is_some());
    }
    #[test]
    fn test_p100_103_none() {
        let d = setup_state();
        assert!(try_status_phases_100_103(d.path(), None, false, false, false, false, false, false, false, false, false, false, false, false, false).is_none());
    }

    // try_status_phases_104_107: sd, machine, json, 12 bools = 15 args
    #[test]
    fn test_p104_107_g1() {
        let d = setup_state();
        assert!(try_status_phases_104_107(d.path(), None, false, true, false, false, false, false, false, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_p104_107_j3() {
        let d = setup_state();
        assert!(try_status_phases_104_107(d.path(), None, false, false, false, false, false, false, false, false, false, false, false, false, true).is_some());
    }
    #[test]
    fn test_p104_107_none() {
        let d = setup_state();
        assert!(try_status_phases_104_107(d.path(), None, false, false, false, false, false, false, false, false, false, false, false, false, false).is_none());
    }

    // try_status_phases_87_92: sd, machine, json, 18 bools = 21 args
    #[test]
    fn test_p87_92_a1() {
        let d = setup_state();
        assert!(try_status_phases_87_92(d.path(), None, false, true, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_p87_92_f3() {
        let d = setup_state();
        assert!(try_status_phases_87_92(d.path(), None, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, true).is_some());
    }
    #[test]
    fn test_p87_92_none() {
        let d = setup_state();
        assert!(try_status_phases_87_92(d.path(), None, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false, false).is_none());
    }
}
