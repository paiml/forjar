//! Coverage tests for graph_intelligence_ext*.rs, graph_impact.rs, graph_compliance.rs,
//! graph_analysis.rs, graph_cross.rs — exercises uncovered graph functions.

#![allow(unused_imports)]
use super::graph_intelligence_ext::*;
use super::graph_intelligence_ext_b::*;
use super::graph_impact::*;
use super::graph_compliance::*;
use super::graph_analysis::*;
use super::graph_cross::*;

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

    const CFG: &str = "version: '1'\nname: test\nmachines:\n  m1:\n    hostname: m1\n    addr: 1.2.3.4\n  m2:\n    hostname: m2\n    addr: 5.6.7.8\nresources:\n  pkg:\n    machine: m1\n    type: package\n    name: nginx\n    tags:\n      - web\n  cfg:\n    machine: m1\n    type: file\n    path: /etc/nginx.conf\n    content: hi\n    state: present\n    depends_on:\n      - pkg\n    tags:\n      - web\n  svc:\n    machine: m1\n    type: service\n    name: nginx\n    depends_on:\n      - cfg\n  remote:\n    machine: m2\n    type: package\n    name: redis\n    depends_on:\n      - pkg\n";

    // graph_intelligence_ext
    #[test]
    fn test_dep_fan_out() {
        let f = write_cfg(CFG);
        assert!(cmd_graph_resource_dependency_fan_out(f.path(), false).is_ok());
    }
    #[test]
    fn test_dep_fan_out_json() {
        let f = write_cfg(CFG);
        assert!(cmd_graph_resource_dependency_fan_out(f.path(), true).is_ok());
    }
    #[test]
    fn test_dep_fan_in() {
        let f = write_cfg(CFG);
        assert!(cmd_graph_resource_dependency_fan_in(f.path(), false).is_ok());
    }
    #[test]
    fn test_dep_fan_in_json() {
        let f = write_cfg(CFG);
        assert!(cmd_graph_resource_dependency_fan_in(f.path(), true).is_ok());
    }
    #[test]
    fn test_dep_path_count() {
        let f = write_cfg(CFG);
        assert!(cmd_graph_resource_dependency_path_count(f.path(), false).is_ok());
    }
    #[test]
    fn test_dep_articulation() {
        let f = write_cfg(CFG);
        assert!(cmd_graph_resource_dependency_articulation_points(f.path(), false).is_ok());
    }
    #[test]
    fn test_dep_longest_path() {
        let f = write_cfg(CFG);
        assert!(cmd_graph_resource_dependency_longest_path(f.path(), false).is_ok());
    }
    #[test]
    fn test_dep_strongly_connected() {
        let f = write_cfg(CFG);
        assert!(cmd_graph_resource_dependency_strongly_connected(f.path(), false).is_ok());
    }

    // graph_intelligence_ext_b
    #[test]
    fn test_dep_topo_depth() {
        let f = write_cfg(CFG);
        assert!(cmd_graph_resource_dependency_topological_depth(f.path(), false).is_ok());
    }
    #[test]
    fn test_dep_topo_depth_json() {
        let f = write_cfg(CFG);
        assert!(cmd_graph_resource_dependency_topological_depth(f.path(), true).is_ok());
    }
    #[test]
    fn test_dep_weak_links() {
        let f = write_cfg(CFG);
        assert!(cmd_graph_resource_dependency_weak_links(f.path(), false).is_ok());
    }
    #[test]
    fn test_dep_minimum_cut() {
        let f = write_cfg(CFG);
        assert!(cmd_graph_resource_dependency_minimum_cut(f.path(), false).is_ok());
    }
    #[test]
    fn test_dep_dominator_tree() {
        let f = write_cfg(CFG);
        assert!(cmd_graph_resource_dependency_dominator_tree(f.path(), false).is_ok());
    }

    // graph_impact
    #[test]
    fn test_impact_radius() {
        let f = write_cfg(CFG);
        let _ = cmd_graph_impact_radius(f.path(), "pkg");
    }
    #[test]
    fn test_dep_matrix() {
        let f = write_cfg(CFG);
        let _ = cmd_graph_dependency_matrix(f.path(), false);
    }
    #[test]
    fn test_dep_matrix_json() {
        let f = write_cfg(CFG);
        let _ = cmd_graph_dependency_matrix(f.path(), true);
    }
    #[test]
    fn test_hotspots() {
        let f = write_cfg(CFG);
        let _ = cmd_graph_hotspots(f.path());
    }
    #[test]
    fn test_timeline() {
        let f = write_cfg(CFG);
        let _ = cmd_graph_timeline(f.path());
    }
    #[test]
    fn test_what_if() {
        let f = write_cfg(CFG);
        let _ = cmd_graph_what_if(f.path(), "pkg");
    }
    #[test]
    fn test_blast_radius() {
        let f = write_cfg(CFG);
        let _ = cmd_graph_blast_radius(f.path(), "pkg", false);
    }
    #[test]
    fn test_blast_radius_json() {
        let f = write_cfg(CFG);
        let _ = cmd_graph_blast_radius(f.path(), "pkg", true);
    }

    // graph_compliance
    #[test]
    fn test_risk_score() {
        let f = write_cfg(CFG);
        assert!(cmd_graph_resource_dependency_risk_score(f.path(), false).is_ok());
    }
    #[test]
    fn test_risk_score_json() {
        let f = write_cfg(CFG);
        assert!(cmd_graph_resource_dependency_risk_score(f.path(), true).is_ok());
    }
    #[test]
    fn test_layering() {
        let f = write_cfg(CFG);
        assert!(cmd_graph_resource_dependency_layering(f.path(), false).is_ok());
    }
    #[test]
    fn test_layering_json() {
        let f = write_cfg(CFG);
        assert!(cmd_graph_resource_dependency_layering(f.path(), true).is_ok());
    }

    // graph_analysis
    #[test]
    fn test_resource_clusters() {
        let f = write_cfg(CFG);
        assert!(cmd_graph_resource_clusters(f.path(), false).is_ok());
    }
    #[test]
    fn test_resource_types() {
        let f = write_cfg(CFG);
        let _ = cmd_graph_resource_types(f.path(), false);
    }
    #[test]
    fn test_resource_age() {
        let f = write_cfg(CFG);
        let _ = cmd_graph_resource_age(f.path(), false);
    }
    #[test]
    fn test_orphan_detection() {
        let f = write_cfg(CFG);
        assert!(cmd_graph_orphan_detection(f.path(), false).is_ok());
    }
    #[test]
    fn test_dep_depth() {
        let f = write_cfg(CFG);
        assert!(cmd_graph_dependency_depth(f.path(), false).is_ok());
    }

    // graph_cross
    #[test]
    fn test_cross_machine_deps() {
        let f = write_cfg(CFG);
        assert!(cmd_graph_cross_machine_deps(f.path(), false).is_ok());
    }
    #[test]
    fn test_cross_machine_deps_json() {
        let f = write_cfg(CFG);
        assert!(cmd_graph_cross_machine_deps(f.path(), true).is_ok());
    }
    #[test]
    fn test_machine_groups() {
        let f = write_cfg(CFG);
        assert!(cmd_graph_machine_groups(f.path(), false).is_ok());
    }
    #[test]
    fn test_change_impact() {
        let f = write_cfg(CFG);
        let _ = cmd_graph_change_impact(f.path(), "pkg", false);
    }
    #[test]
    fn test_change_impact_json() {
        let f = write_cfg(CFG);
        let _ = cmd_graph_change_impact(f.path(), "pkg", true);
    }
    #[test]
    fn test_security_boundaries() {
        let f = write_cfg(CFG);
        let _ = cmd_graph_security_boundaries(f.path(), false);
    }
    #[test]
    fn test_reverse_deps() {
        let f = write_cfg(CFG);
        let _ = cmd_graph_reverse_deps(f.path(), false);
    }
    #[test]
    fn test_leaf_resources() {
        let f = write_cfg(CFG);
        let _ = cmd_graph_leaf_resources(f.path(), false);
    }
}
