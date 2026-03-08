//! Coverage tests for dispatch_graph.rs — remaining untested branches.

#![allow(unused_imports)]
use super::dispatch_graph::*;
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

    // ── try_topology missing branches ──
    #[test]
    fn topology_resource_age() {
        let f = write_cfg(CFG);
        assert!(try_topology(f.path(), false, false, false, false, true, false, false, false, false).is_some());
    }
    #[test]
    fn topology_security_boundaries() {
        let f = write_cfg(CFG);
        assert!(try_topology(f.path(), false, false, false, false, false, true, false, false, false).is_some());
    }
    #[test]
    fn topology_execution_order() {
        let f = write_cfg(CFG);
        assert!(try_topology(f.path(), false, false, false, false, false, false, true, false, false).is_some());
    }
    #[test]
    fn topology_topological_levels() {
        let f = write_cfg(CFG);
        assert!(try_topology(f.path(), false, false, false, false, false, false, false, true, false).is_some());
    }
    #[test]
    fn topology_resource_types() {
        let f = write_cfg(CFG);
        assert!(try_topology(f.path(), false, false, false, false, false, false, false, false, true).is_some());
    }

    // ── try_impact missing branches ──
    #[test]
    fn impact_change_impact() {
        let f = write_cfg(CFG);
        let ci = Some("pkg".to_string());
        assert!(try_impact(f.path(), "text", false, &ci, &None, &None, false, false, false, &None, &None, false).is_some());
    }
    #[test]
    fn impact_timeline_graph() {
        let f = write_cfg(CFG);
        assert!(try_impact(f.path(), "text", false, &None, &None, &None, true, false, false, &None, &None, false).is_some());
    }
    #[test]
    fn impact_dependency_matrix() {
        let f = write_cfg(CFG);
        assert!(try_impact(f.path(), "text", false, &None, &None, &None, false, false, true, &None, &None, false).is_some());
    }
    #[test]
    fn impact_impact_radius() {
        let f = write_cfg(CFG);
        let ir = Some("pkg".to_string());
        assert!(try_impact(f.path(), "text", false, &None, &None, &None, false, false, false, &ir, &None, false).is_some());
    }
    #[test]
    fn impact_subgraph() {
        let f = write_cfg(CFG);
        let sg = Some("pkg".to_string());
        assert!(try_impact(f.path(), "text", false, &None, &None, &None, false, false, false, &None, &sg, false).is_some());
    }
    #[test]
    fn impact_weight() {
        let f = write_cfg(CFG);
        assert!(try_impact(f.path(), "text", false, &None, &None, &None, false, false, false, &None, &None, true).is_some());
    }

    // ── try_visualization missing branches ──
    #[test]
    fn viz_prune() {
        let f = write_cfg(CFG);
        let p = Some("pkg".to_string());
        assert!(try_visualization(f.path(), "text", false, false, false, &p, &None, false, false, false, false, false, &None, None).is_some());
    }
    #[test]
    fn viz_highlight() {
        let f = write_cfg(CFG);
        let h = Some("pkg".to_string());
        assert!(try_visualization(f.path(), "text", false, false, false, &None, &h, false, false, false, false, false, &None, None).is_some());
    }
    #[test]
    fn viz_json() {
        let f = write_cfg(CFG);
        assert!(try_visualization(f.path(), "text", true, false, false, &None, &None, false, false, false, false, false, &None, None).is_some());
    }
    #[test]
    fn viz_cluster() {
        let f = write_cfg(CFG);
        assert!(try_visualization(f.path(), "text", false, false, false, &None, &None, false, false, true, false, false, &None, None).is_some());
    }
    #[test]
    fn viz_reverse() {
        let f = write_cfg(CFG);
        assert!(try_visualization(f.path(), "text", false, false, false, &None, &None, false, false, false, true, false, &None, None).is_some());
    }
    #[test]
    fn viz_critical_path() {
        let f = write_cfg(CFG);
        assert!(try_visualization(f.path(), "text", false, false, false, &None, &None, false, false, false, false, true, &None, None).is_some());
    }
    #[test]
    fn viz_affected() {
        let f = write_cfg(CFG);
        let a = Some("pkg".to_string());
        assert!(try_visualization(f.path(), "text", false, false, false, &None, &None, false, false, false, false, false, &a, None).is_some());
    }
    #[test]
    fn viz_depth() {
        let f = write_cfg(CFG);
        assert!(try_visualization(f.path(), "text", false, false, false, &None, &None, false, false, false, false, false, &None, Some(2)).is_some());
    }

    // ── try_graph_paths missing branches ──
    #[test]
    fn paths_critical_dependency_path() {
        let f = write_cfg(CFG);
        assert!(try_graph_paths(f.path(), false, &None, false, true, false, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn paths_resource_depth_histogram() {
        let f = write_cfg(CFG);
        assert!(try_graph_paths(f.path(), false, &None, false, false, true, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn paths_resource_coupling_score() {
        let f = write_cfg(CFG);
        assert!(try_graph_paths(f.path(), false, &None, false, false, false, true, false, false, false, false, false).is_some());
    }
    #[test]
    fn paths_resource_change_frequency() {
        let f = write_cfg(CFG);
        assert!(try_graph_paths(f.path(), false, &None, false, false, false, false, true, false, false, false, false).is_some());
    }
    #[test]
    fn paths_resource_impact_score() {
        let f = write_cfg(CFG);
        assert!(try_graph_paths(f.path(), false, &None, false, false, false, false, false, true, false, false, false).is_some());
    }
    #[test]
    fn paths_resource_stability_score() {
        let f = write_cfg(CFG);
        assert!(try_graph_paths(f.path(), false, &None, false, false, false, false, false, false, true, false, false).is_some());
    }
    #[test]
    fn paths_resource_dependency_fanout() {
        let f = write_cfg(CFG);
        assert!(try_graph_paths(f.path(), false, &None, false, false, false, false, false, false, false, true, false).is_some());
    }
    #[test]
    fn paths_resource_dependency_weight() {
        let f = write_cfg(CFG);
        assert!(try_graph_paths(f.path(), false, &None, false, false, false, false, false, false, false, false, true).is_some());
    }

    // ── try_graph_analysis missing branches ──
    #[test]
    fn analysis_dependency_depth_per_resource() {
        let f = write_cfg(CFG);
        assert!(try_graph_analysis(f.path(), false, false, true, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn analysis_resource_fanin() {
        let f = write_cfg(CFG);
        assert!(try_graph_analysis(f.path(), false, false, false, true, false, false, false, false, false).is_some());
    }
    #[test]
    fn analysis_isolated_subgraphs() {
        let f = write_cfg(CFG);
        assert!(try_graph_analysis(f.path(), false, false, false, false, true, false, false, false, false).is_some());
    }
    #[test]
    fn analysis_dependency_matrix_csv() {
        let f = write_cfg(CFG);
        assert!(try_graph_analysis(f.path(), false, false, false, false, false, true, false, false, false).is_some());
    }
    #[test]
    fn analysis_strongly_connected() {
        let f = write_cfg(CFG);
        assert!(try_graph_analysis(f.path(), false, false, false, false, false, false, true, false, false).is_some());
    }
    #[test]
    fn analysis_bipartite_check() {
        let f = write_cfg(CFG);
        assert!(try_graph_analysis(f.path(), false, false, false, false, false, false, false, true, false).is_some());
    }
    #[test]
    fn analysis_sink_resources() {
        let f = write_cfg(CFG);
        assert!(try_graph_analysis(f.path(), false, false, false, false, false, false, false, false, true).is_some());
    }

    // ── try_graph_export_a missing branches ──
    #[test]
    fn export_a_subgraph_stats() {
        let f = write_cfg(CFG);
        assert!(try_graph_export_a(f.path(), false, false, true, false, false, false, false, false).is_some());
    }
    #[test]
    fn export_a_dependency_count() {
        let f = write_cfg(CFG);
        assert!(try_graph_export_a(f.path(), false, false, false, true, false, false, false, false).is_some());
    }
    #[test]
    fn export_a_root_resources() {
        let f = write_cfg(CFG);
        assert!(try_graph_export_a(f.path(), false, false, false, false, true, false, false, false).is_some());
    }
    #[test]
    fn export_a_edge_list() {
        let f = write_cfg(CFG);
        assert!(try_graph_export_a(f.path(), false, false, false, false, false, true, false, false).is_some());
    }
    #[test]
    fn export_a_connected_components() {
        let f = write_cfg(CFG);
        assert!(try_graph_export_a(f.path(), false, false, false, false, false, false, true, false).is_some());
    }
    #[test]
    fn export_a_adjacency_matrix() {
        let f = write_cfg(CFG);
        assert!(try_graph_export_a(f.path(), false, false, false, false, false, false, false, true).is_some());
    }

    // ── try_graph_export_b missing branches ──
    #[test]
    fn export_b_in_degree() {
        let f = write_cfg(CFG);
        assert!(try_graph_export_b(f.path(), false, false, true, false, false, false, false).is_some());
    }
    #[test]
    fn export_b_out_degree() {
        let f = write_cfg(CFG);
        assert!(try_graph_export_b(f.path(), false, false, false, true, false, false, false).is_some());
    }
    #[test]
    fn export_b_density() {
        let f = write_cfg(CFG);
        assert!(try_graph_export_b(f.path(), false, false, false, false, true, false, false).is_some());
    }
    #[test]
    fn export_b_topological_sort() {
        let f = write_cfg(CFG);
        assert!(try_graph_export_b(f.path(), false, false, false, false, false, true, false).is_some());
    }
    #[test]
    fn export_b_critical_path_resources() {
        let f = write_cfg(CFG);
        assert!(try_graph_export_b(f.path(), false, false, false, false, false, false, true).is_some());
    }
}
