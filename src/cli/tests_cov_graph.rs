//! Coverage tests for graph_extended, graph_impact, graph_visualization, graph_topology.

use super::graph_extended::*;
use super::graph_impact::*;
use super::graph_topology::*;
use super::graph_visualization::*;
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

    fn config_with_deps() -> String {
        "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  a:\n    type: file\n    machine: m\n    path: /tmp/a\n    content: a\n  b:\n    type: file\n    machine: m\n    path: /tmp/b\n    content: b\n    depends_on: [a]\n  c:\n    type: service\n    machine: m\n    name: svc\n    depends_on: [b]\n".to_string()
    }

    fn config_no_deps() -> String {
        "version: \"1.0\"\nname: t\nmachines:\n  m:\n    hostname: m\n    addr: 127.0.0.1\nresources:\n  x:\n    type: file\n    machine: m\n    path: /tmp/x\n    content: x\n  y:\n    type: file\n    machine: m\n    path: /tmp/y\n    content: y\n".to_string()
    }

    // ── graph_extended ──

    #[test]
    fn test_graph_depth_mermaid() {
        let f = write_temp_config(&config_with_deps());
        let _ = cmd_graph_depth(f.path(), "mermaid", 2);
    }

    #[test]
    fn test_graph_depth_dot() {
        let f = write_temp_config(&config_with_deps());
        let _ = cmd_graph_depth(f.path(), "dot", 1);
    }

    #[test]
    fn test_graph_cluster_mermaid() {
        let f = write_temp_config(&config_with_deps());
        let _ = cmd_graph_cluster(f.path(), "mermaid");
    }

    #[test]
    fn test_graph_cluster_dot() {
        let f = write_temp_config(&config_with_deps());
        let _ = cmd_graph_cluster(f.path(), "dot");
    }

    #[test]
    fn test_graph_orphans_with_deps() {
        let f = write_temp_config(&config_with_deps());
        let _ = cmd_graph_orphans(f.path());
    }

    #[test]
    fn test_graph_orphans_no_deps() {
        let f = write_temp_config(&config_no_deps());
        let _ = cmd_graph_orphans(f.path());
    }

    #[test]
    fn test_graph_stats_with_deps() {
        let f = write_temp_config(&config_with_deps());
        let _ = cmd_graph_stats(f.path());
    }

    #[test]
    fn test_graph_stats_no_deps() {
        let f = write_temp_config(&config_no_deps());
        let _ = cmd_graph_stats(f.path());
    }

    #[test]
    fn test_graph_json_with_deps() {
        let f = write_temp_config(&config_with_deps());
        let _ = cmd_graph_json(f.path());
    }

    #[test]
    fn test_graph_json_no_deps() {
        let f = write_temp_config(&config_no_deps());
        let _ = cmd_graph_json(f.path());
    }

    #[test]
    fn test_graph_highlight_dot() {
        let f = write_temp_config(&config_with_deps());
        let _ = cmd_graph_highlight(f.path(), "dot", "b");
    }

    #[test]
    fn test_graph_highlight_mermaid() {
        let f = write_temp_config(&config_with_deps());
        let _ = cmd_graph_highlight(f.path(), "mermaid", "c");
    }

    // ── graph_impact ──

    #[test]
    fn test_impact_radius_leaf() {
        let f = write_temp_config(&config_with_deps());
        let _ = cmd_graph_impact_radius(f.path(), "c");
    }

    #[test]
    fn test_impact_radius_root() {
        let f = write_temp_config(&config_with_deps());
        let _ = cmd_graph_impact_radius(f.path(), "a");
    }

    #[test]
    fn test_dependency_matrix_json() {
        let f = write_temp_config(&config_with_deps());
        let _ = cmd_graph_dependency_matrix(f.path(), true);
    }

    #[test]
    fn test_dependency_matrix_csv() {
        let f = write_temp_config(&config_with_deps());
        let _ = cmd_graph_dependency_matrix(f.path(), false);
    }

    #[test]
    fn test_hotspots_with_deps() {
        let f = write_temp_config(&config_with_deps());
        let _ = cmd_graph_hotspots(f.path());
    }

    #[test]
    fn test_hotspots_no_deps() {
        let f = write_temp_config(&config_no_deps());
        let _ = cmd_graph_hotspots(f.path());
    }

    #[test]
    fn test_timeline_with_deps() {
        let f = write_temp_config(&config_with_deps());
        let _ = cmd_graph_timeline(f.path());
    }

    #[test]
    fn test_timeline_no_deps() {
        let f = write_temp_config(&config_no_deps());
        let _ = cmd_graph_timeline(f.path());
    }

    #[test]
    fn test_what_if_root() {
        let f = write_temp_config(&config_with_deps());
        let _ = cmd_graph_what_if(f.path(), "a");
    }

    #[test]
    fn test_what_if_leaf() {
        let f = write_temp_config(&config_with_deps());
        let _ = cmd_graph_what_if(f.path(), "c");
    }

    #[test]
    fn test_blast_radius_json() {
        let f = write_temp_config(&config_with_deps());
        let _ = cmd_graph_blast_radius(f.path(), "a", true);
    }

    #[test]
    fn test_blast_radius_plain() {
        let f = write_temp_config(&config_with_deps());
        let _ = cmd_graph_blast_radius(f.path(), "c", false);
    }

    // ── graph_visualization ──

    #[test]
    fn test_prune_dot() {
        let f = write_temp_config(&config_with_deps());
        let _ = cmd_graph_prune(f.path(), "dot", "a");
    }

    #[test]
    fn test_prune_mermaid() {
        let f = write_temp_config(&config_with_deps());
        let _ = cmd_graph_prune(f.path(), "mermaid", "b");
    }

    #[test]
    fn test_layers_with_deps() {
        let f = write_temp_config(&config_with_deps());
        let _ = cmd_graph_layers(f.path());
    }

    #[test]
    fn test_layers_no_deps() {
        let f = write_temp_config(&config_no_deps());
        let _ = cmd_graph_layers(f.path());
    }

    #[test]
    fn test_critical_resources_with_deps() {
        let f = write_temp_config(&config_with_deps());
        let _ = cmd_graph_critical_resources(f.path());
    }

    #[test]
    fn test_critical_resources_no_deps() {
        let f = write_temp_config(&config_no_deps());
        let _ = cmd_graph_critical_resources(f.path());
    }

    #[test]
    fn test_weight_dot() {
        let f = write_temp_config(&config_with_deps());
        let _ = cmd_graph_weight(f.path(), "dot");
    }

    #[test]
    fn test_weight_mermaid() {
        let f = write_temp_config(&config_with_deps());
        let _ = cmd_graph_weight(f.path(), "mermaid");
    }

    #[test]
    fn test_subgraph_dot() {
        let f = write_temp_config(&config_with_deps());
        let _ = cmd_graph_subgraph(f.path(), "dot", "c");
    }

    #[test]
    fn test_subgraph_mermaid() {
        let f = write_temp_config(&config_with_deps());
        let _ = cmd_graph_subgraph(f.path(), "mermaid", "b");
    }

    // ── graph_topology ──

    #[test]
    fn test_topological_levels_json() {
        let f = write_temp_config(&config_with_deps());
        let _ = cmd_graph_topological_levels(f.path(), true);
    }

    #[test]
    fn test_topological_levels_plain() {
        let f = write_temp_config(&config_with_deps());
        let _ = cmd_graph_topological_levels(f.path(), false);
    }

    #[test]
    fn test_critical_chain_json() {
        let f = write_temp_config(&config_with_deps());
        let _ = cmd_graph_critical_chain(f.path(), true);
    }

    #[test]
    fn test_critical_chain_plain() {
        let f = write_temp_config(&config_no_deps());
        let _ = cmd_graph_critical_chain(f.path(), false);
    }

    #[test]
    fn test_parallel_groups_json() {
        let f = write_temp_config(&config_with_deps());
        let _ = cmd_graph_parallel_groups(f.path(), true);
    }

    #[test]
    fn test_parallel_groups_plain() {
        let f = write_temp_config(&config_no_deps());
        let _ = cmd_graph_parallel_groups(f.path(), false);
    }

    #[test]
    fn test_fan_out_json() {
        let f = write_temp_config(&config_with_deps());
        let _ = cmd_graph_fan_out(f.path(), true);
    }

    #[test]
    fn test_fan_out_plain() {
        let f = write_temp_config(&config_with_deps());
        let _ = cmd_graph_fan_out(f.path(), false);
    }

    #[test]
    fn test_depth_first_json() {
        let f = write_temp_config(&config_with_deps());
        let _ = cmd_graph_depth_first(f.path(), true);
    }

    #[test]
    fn test_depth_first_plain() {
        let f = write_temp_config(&config_with_deps());
        let _ = cmd_graph_depth_first(f.path(), false);
    }

    #[test]
    fn test_breadth_first_json() {
        let f = write_temp_config(&config_with_deps());
        let _ = cmd_graph_breadth_first(f.path(), true);
    }

    #[test]
    fn test_breadth_first_plain() {
        let f = write_temp_config(&config_with_deps());
        let _ = cmd_graph_breadth_first(f.path(), false);
    }

    #[test]
    fn test_dependency_count_json() {
        let f = write_temp_config(&config_with_deps());
        let _ = cmd_graph_dependency_count(f.path(), true);
    }

    #[test]
    fn test_dependency_count_plain() {
        let f = write_temp_config(&config_with_deps());
        let _ = cmd_graph_dependency_count(f.path(), false);
    }

    #[test]
    fn test_subgraph_stats_json() {
        let f = write_temp_config(&config_with_deps());
        let _ = cmd_graph_subgraph_stats(f.path(), true);
    }

    #[test]
    fn test_subgraph_stats_plain() {
        let f = write_temp_config(&config_no_deps());
        let _ = cmd_graph_subgraph_stats(f.path(), false);
    }
}
