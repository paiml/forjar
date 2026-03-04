//! Coverage tests for dispatch_graph_b.rs — all try_graph_* routing.

#![allow(unused_imports)]
use super::dispatch_graph_b::*;
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

    // try_graph_scoring_inline: file, json, 12 bools
    #[test]
    fn test_scoring_bottleneck() {
        let f = write_cfg(CFG);
        assert!(try_graph_scoring_inline(f.path(), false, true, false, false, false, false, false, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_scoring_redundancy() {
        let f = write_cfg(CFG);
        assert!(try_graph_scoring_inline(f.path(), false, false, false, false, false, false, false, false, false, false, false, false, true).is_some());
    }
    #[test]
    fn test_scoring_none() {
        let f = write_cfg(CFG);
        assert!(try_graph_scoring_inline(f.path(), false, false, false, false, false, false, false, false, false, false, false, false, false).is_none());
    }

    // try_graph_scoring_phase81: file, json, 12 bools
    #[test]
    fn test_p81_centrality() {
        let f = write_cfg(CFG);
        assert!(try_graph_scoring_phase81(f.path(), false, true, false, false, false, false, false, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_p81_articulation() {
        let f = write_cfg(CFG);
        assert!(try_graph_scoring_phase81(f.path(), false, false, false, false, false, false, false, false, false, false, false, false, true).is_some());
    }
    #[test]
    fn test_p81_none() {
        let f = write_cfg(CFG);
        assert!(try_graph_scoring_phase81(f.path(), false, false, false, false, false, false, false, false, false, false, false, false, false).is_none());
    }

    // try_graph_phase87: file, json, 12 bools
    #[test]
    fn test_p87_longest_path() {
        let f = write_cfg(CFG);
        assert!(try_graph_phase87(f.path(), false, true, false, false, false, false, false, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_p87_diameter_path() {
        let f = write_cfg(CFG);
        assert!(try_graph_phase87(f.path(), false, false, false, false, false, false, false, false, false, false, false, false, true).is_some());
    }
    #[test]
    fn test_p87_none() {
        let f = write_cfg(CFG);
        assert!(try_graph_phase87(f.path(), false, false, false, false, false, false, false, false, false, false, false, false, false).is_none());
    }

    // try_graph_phase94: file, json, 2 bools
    #[test]
    fn test_p94_bridge() {
        let f = write_cfg(CFG);
        assert!(try_graph_phase94(f.path(), false, true, false).is_some());
    }
    #[test]
    fn test_p94_none() {
        let f = write_cfg(CFG);
        assert!(try_graph_phase94(f.path(), false, false, false).is_none());
    }

    // try_graph_phase95: file, json, 2 bools
    #[test]
    fn test_p95_parallel() {
        let f = write_cfg(CFG);
        assert!(try_graph_phase95(f.path(), false, true, false).is_some());
    }
    #[test]
    fn test_p95_none() {
        let f = write_cfg(CFG);
        assert!(try_graph_phase95(f.path(), false, false, false).is_none());
    }

    // try_graph_phase96: file, json, 2 bools
    #[test]
    fn test_p96_recipe() {
        let f = write_cfg(CFG);
        assert!(try_graph_phase96(f.path(), false, true, false).is_some());
    }
    #[test]
    fn test_p96_none() {
        let f = write_cfg(CFG);
        assert!(try_graph_phase96(f.path(), false, false, false).is_none());
    }

    // try_graph_phase97: file, json, 2 bools
    #[test]
    fn test_p97_apply_order() {
        let f = write_cfg(CFG);
        assert!(try_graph_phase97(f.path(), false, true, false).is_some());
    }
    #[test]
    fn test_p97_none() {
        let f = write_cfg(CFG);
        assert!(try_graph_phase97(f.path(), false, false, false).is_none());
    }

    // try_graph_phase98: file, json, 2 bools
    #[test]
    fn test_p98_risk() {
        let f = write_cfg(CFG);
        assert!(try_graph_phase98(f.path(), false, true, false).is_some());
    }
    #[test]
    fn test_p98_none() {
        let f = write_cfg(CFG);
        assert!(try_graph_phase98(f.path(), false, false, false).is_none());
    }

    // try_graph_phase99: file, json, 2 bools
    #[test]
    fn test_p99_lifecycle() {
        let f = write_cfg(CFG);
        assert!(try_graph_phase99(f.path(), false, true, false).is_some());
    }
    #[test]
    fn test_p99_none() {
        let f = write_cfg(CFG);
        assert!(try_graph_phase99(f.path(), false, false, false).is_none());
    }

    // try_graph_phase100: file, json, 2 bools
    #[test]
    fn test_p100_health_overlay() {
        let f = write_cfg(CFG);
        assert!(try_graph_phase100(f.path(), false, true, false).is_some());
    }
    #[test]
    fn test_p100_none() {
        let f = write_cfg(CFG);
        assert!(try_graph_phase100(f.path(), false, false, false).is_none());
    }

    // try_graph_phase101: file, json, 2 bools
    #[test]
    fn test_p101_critical_path() {
        let f = write_cfg(CFG);
        assert!(try_graph_phase101(f.path(), false, true, false).is_some());
    }
    #[test]
    fn test_p101_none() {
        let f = write_cfg(CFG);
        assert!(try_graph_phase101(f.path(), false, false, false).is_none());
    }

    // try_graph_phase102: file, json, 2 bools
    #[test]
    fn test_p102_cluster() {
        let f = write_cfg(CFG);
        assert!(try_graph_phase102(f.path(), false, true, false).is_some());
    }
    #[test]
    fn test_p102_none() {
        let f = write_cfg(CFG);
        assert!(try_graph_phase102(f.path(), false, false, false).is_none());
    }

    // try_graph_phase103: file, json, 2 bools
    #[test]
    fn test_p103_depth_histogram() {
        let f = write_cfg(CFG);
        assert!(try_graph_phase103(f.path(), false, true, false).is_some());
    }
    #[test]
    fn test_p103_none() {
        let f = write_cfg(CFG);
        assert!(try_graph_phase103(f.path(), false, false, false).is_none());
    }

    // try_graph_phases_104_106: file, json, 6 bools
    #[test]
    fn test_p104_106_a1() {
        let f = write_cfg(CFG);
        assert!(try_graph_phases_104_106(f.path(), false, true, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_p104_106_c2() {
        let f = write_cfg(CFG);
        assert!(try_graph_phases_104_106(f.path(), false, false, false, false, false, false, true).is_some());
    }
    #[test]
    fn test_p104_106_none() {
        let f = write_cfg(CFG);
        assert!(try_graph_phases_104_106(f.path(), false, false, false, false, false, false, false).is_none());
    }
}
