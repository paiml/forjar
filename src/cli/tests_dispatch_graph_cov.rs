//! Coverage tests for dispatch_graph.rs — routing tests for every graph flag.

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

    // try_traversal: file, json, 8 bools = 10 args
    #[test]
    fn test_traversal_depth_first() {
        let f = write_cfg(CFG);
        assert!(try_traversal(f.path(), false, true, false, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_traversal_reverse_deps() {
        let f = write_cfg(CFG);
        assert!(try_traversal(f.path(), false, false, true, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_traversal_leaf_resources() {
        let f = write_cfg(CFG);
        assert!(try_traversal(f.path(), false, false, false, true, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_traversal_fan_out() {
        let f = write_cfg(CFG);
        assert!(try_traversal(f.path(), false, false, false, false, true, false, false, false, false).is_some());
    }
    #[test]
    fn test_traversal_resource_clusters() {
        let f = write_cfg(CFG);
        assert!(try_traversal(f.path(), false, false, false, false, false, true, false, false, false).is_some());
    }
    #[test]
    fn test_traversal_machine_groups() {
        let f = write_cfg(CFG);
        assert!(try_traversal(f.path(), false, false, false, false, false, false, true, false, false).is_some());
    }
    #[test]
    fn test_traversal_cross_machine_deps() {
        let f = write_cfg(CFG);
        assert!(try_traversal(f.path(), false, false, false, false, false, false, false, true, false).is_some());
    }
    #[test]
    fn test_traversal_orphan_detection() {
        let f = write_cfg(CFG);
        assert!(try_traversal(f.path(), false, false, false, false, false, false, false, false, true).is_some());
    }
    #[test]
    fn test_traversal_none() {
        let f = write_cfg(CFG);
        assert!(try_traversal(f.path(), false, false, false, false, false, false, false, false, false).is_none());
    }

    // try_topology: file, json, 8 bools = 10 args
    #[test]
    fn test_topology_dependency_depth() {
        let f = write_cfg(CFG);
        assert!(try_topology(f.path(), false, true, false, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_topology_critical_chain() {
        let f = write_cfg(CFG);
        assert!(try_topology(f.path(), false, false, true, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_topology_parallel_groups() {
        let f = write_cfg(CFG);
        assert!(try_topology(f.path(), false, false, false, true, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_topology_none() {
        let f = write_cfg(CFG);
        assert!(try_topology(f.path(), false, false, false, false, false, false, false, false, false).is_none());
    }

    // try_impact: file, format: &str, json, change_impact: &Option<String>,
    //   blast_radius: &Option<String>, what_if: &Option<String>, timeline_graph,
    //   hotspots, dependency_matrix, impact_radius: &Option<String>,
    //   subgraph: &Option<String>, weight = 12 args
    #[test]
    fn test_impact_blast_radius() {
        let f = write_cfg(CFG);
        let br = Some("pkg".to_string());
        assert!(try_impact(f.path(), "text", false, &None, &br, &None, false, false, false, &None, &None, false).is_some());
    }
    #[test]
    fn test_impact_what_if() {
        let f = write_cfg(CFG);
        let wi = Some("pkg".to_string());
        assert!(try_impact(f.path(), "text", false, &None, &None, &wi, false, false, false, &None, &None, false).is_some());
    }
    #[test]
    fn test_impact_hotspots() {
        let f = write_cfg(CFG);
        assert!(try_impact(f.path(), "text", false, &None, &None, &None, false, true, false, &None, &None, false).is_some());
    }
    #[test]
    fn test_impact_none() {
        let f = write_cfg(CFG);
        assert!(try_impact(f.path(), "text", false, &None, &None, &None, false, false, false, &None, &None, false).is_none());
    }

    // try_visualization: file, format: &str, json, critical_resources, layers,
    //   prune: &Option<String>, highlight: &Option<String>, stats, orphans,
    //   cluster, reverse, critical_path, affected: &Option<String>,
    //   depth: Option<usize> = 14 args
    #[test]
    fn test_viz_critical_resources() {
        let f = write_cfg(CFG);
        assert!(try_visualization(f.path(), "text", false, true, false, &None, &None, false, false, false, false, false, &None, None).is_some());
    }
    #[test]
    fn test_viz_layers() {
        let f = write_cfg(CFG);
        assert!(try_visualization(f.path(), "text", false, false, true, &None, &None, false, false, false, false, false, &None, None).is_some());
    }
    #[test]
    fn test_viz_stats() {
        let f = write_cfg(CFG);
        assert!(try_visualization(f.path(), "text", false, false, false, &None, &None, true, false, false, false, false, &None, None).is_some());
    }
    #[test]
    fn test_viz_orphans() {
        let f = write_cfg(CFG);
        assert!(try_visualization(f.path(), "text", false, false, false, &None, &None, false, true, false, false, false, &None, None).is_some());
    }
    #[test]
    fn test_viz_none() {
        let f = write_cfg(CFG);
        assert!(try_visualization(f.path(), "text", false, false, false, &None, &None, false, false, false, false, false, &None, None).is_none());
    }

    // try_graph_paths: file, json, resource_dependency_chain: &Option<String>, 9 bools = 12 args
    #[test]
    fn test_paths_bottleneck() {
        let f = write_cfg(CFG);
        assert!(try_graph_paths(f.path(), false, &None, true, false, false, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_paths_chain() {
        let f = write_cfg(CFG);
        let chain = Some("pkg".to_string());
        assert!(try_graph_paths(f.path(), false, &chain, false, false, false, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_paths_none() {
        let f = write_cfg(CFG);
        assert!(try_graph_paths(f.path(), false, &None, false, false, false, false, false, false, false, false, false).is_none());
    }

    // try_graph_analysis: file, json, 8 bools = 10 args
    #[test]
    fn test_analysis_resource_weight() {
        let f = write_cfg(CFG);
        assert!(try_graph_analysis(f.path(), false, true, false, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_analysis_none() {
        let f = write_cfg(CFG);
        assert!(try_graph_analysis(f.path(), false, false, false, false, false, false, false, false, false).is_none());
    }

    // try_graph_export_a: file, json, 7 bools = 9 args
    #[test]
    fn test_export_a_breadth_first() {
        let f = write_cfg(CFG);
        assert!(try_graph_export_a(f.path(), false, true, false, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_export_a_none() {
        let f = write_cfg(CFG);
        assert!(try_graph_export_a(f.path(), false, false, false, false, false, false, false, false).is_none());
    }

    // try_graph_export_b: file, json, 6 bools = 8 args
    #[test]
    fn test_export_b_longest_path() {
        let f = write_cfg(CFG);
        assert!(try_graph_export_b(f.path(), false, true, false, false, false, false, false).is_some());
    }
    #[test]
    fn test_export_b_none() {
        let f = write_cfg(CFG);
        assert!(try_graph_export_b(f.path(), false, false, false, false, false, false, false).is_none());
    }
}
