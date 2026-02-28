//! CLI Args structs for graph-related commands.

use std::path::PathBuf;


#[derive(clap::Args, Debug)]
pub struct GraphArgs {
    /// Path to forjar.yaml
    #[arg(short, long, default_value = "forjar.yaml")]
    pub file: PathBuf,

    /// Output format: mermaid (default) or dot
    #[arg(long, default_value = "mermaid")]
    pub format: String,

    /// Filter to specific machine
    #[arg(short, long)]
    pub machine: Option<String>,

    /// Filter to specific resource group
    #[arg(short, long)]
    pub group: Option<String>,

    /// FJ-354: Show transitive dependents of a resource (impact analysis)
    #[arg(long)]
    pub affected: Option<String>,

    /// FJ-375: Highlight the longest dependency chain
    #[arg(long)]
    pub critical_path: bool,

    /// FJ-385: Show reverse dependency graph
    #[arg(long)]
    pub reverse: bool,

    /// FJ-394: Limit graph traversal depth
    #[arg(long)]
    pub depth: Option<usize>,

    /// FJ-404: Group resources by machine in graph output
    #[arg(long)]
    pub cluster: bool,

    /// FJ-414: Show resources with no dependencies and no dependents
    #[arg(long)]
    pub orphans: bool,

    /// FJ-424: Show graph statistics (nodes, edges, depth, width)
    #[arg(long)]
    pub stats: bool,

    /// FJ-434: Output graph as JSON adjacency list
    #[arg(long, name = "json")]
    pub json_output: bool,

    /// FJ-444: Highlight a resource and its transitive deps in graph output
    #[arg(long)]
    pub highlight: Option<String>,

    /// FJ-454: Show graph with a resource and its subtree removed
    #[arg(long)]
    pub prune: Option<String>,

    /// FJ-464: Show graph organized by dependency layers (depth levels)
    #[arg(long)]
    pub layers: bool,

    /// FJ-474: Identify resources with the most dependents (bottleneck analysis)
    #[arg(long)]
    pub critical_resources: bool,

    /// FJ-484: Show edge weights based on dependency strength
    #[arg(long)]
    pub weight: bool,

    /// FJ-494: Extract and display a resource's dependency subgraph
    #[arg(long)]
    pub subgraph: Option<String>,

    /// FJ-504: Show blast radius of changing a resource
    #[arg(long)]
    pub impact_radius: Option<String>,

    /// FJ-514: Output resource dependency matrix (CSV/JSON)
    #[arg(long)]
    pub dependency_matrix: bool,

    /// FJ-524: Highlight resources with most changes/failures (heat map)
    #[arg(long)]
    pub hotspots: bool,

    /// FJ-534: Show resource application order as ASCII timeline
    #[arg(long)]
    pub timeline_graph: bool,

    /// FJ-544: Simulate removing a resource, show impact
    #[arg(long)]
    pub what_if: Option<String>,

    /// FJ-554: Show all resources affected by a change to target
    #[arg(long)]
    pub blast_radius: Option<String>,

    /// FJ-564: Show direct + indirect impact of changing a resource
    #[arg(long)]
    pub change_impact: Option<String>,

    /// FJ-574: Show graph colored/grouped by resource type
    #[arg(long)]
    pub resource_types: bool,

    /// FJ-584: Show resources grouped by topological depth level
    #[arg(long)]
    pub topological_levels: bool,

    /// FJ-594: Show exact execution order with timing estimates
    #[arg(long)]
    pub execution_order: bool,

    /// FJ-604: Highlight resources crossing security boundaries
    #[arg(long)]
    pub security_boundaries: bool,

    /// FJ-614: Show resource age based on last apply timestamp
    #[arg(long)]
    pub resource_age: bool,

    /// FJ-624: Show which resources can execute in parallel
    #[arg(long)]
    pub parallel_groups: bool,

    /// FJ-634: Show longest dependency chain (critical path analysis)
    #[arg(long)]
    pub critical_chain: bool,

    /// FJ-644: Show max dependency depth per resource
    #[arg(long)]
    pub dependency_depth: bool,

    /// FJ-654: Find resources with no dependents or dependencies
    #[arg(long)]
    pub orphan_detection: bool,

    /// FJ-664: Visualize dependencies across machines
    #[arg(long)]
    pub cross_machine_deps: bool,

    /// FJ-674: Group resources by machine in graph output
    #[arg(long)]
    pub machine_groups: bool,

    /// FJ-684: Identify tightly-coupled resource clusters
    #[arg(long)]
    pub resource_clusters: bool,

    /// FJ-694: Show resource fan-out metrics
    #[arg(long)]
    pub fan_out: bool,

    /// FJ-704: Show leaf resources (no dependents)
    #[arg(long)]
    pub leaf_resources: bool,

    /// FJ-714: Show reverse dependency graph
    #[arg(long)]
    pub reverse_deps: bool,

    /// FJ-724: Show depth-first traversal order
    #[arg(long)]
    pub depth_first: bool,

    /// FJ-734: Show breadth-first traversal order
    #[arg(long)]
    pub breadth_first: bool,

    /// FJ-743: Show node/edge/depth stats for each connected component
    #[arg(long)]
    pub subgraph_stats: bool,

    /// FJ-747: Show in-degree and out-degree per resource
    #[arg(long)]
    pub dependency_count: bool,

    /// FJ-751: Show root resources (no dependencies)
    #[arg(long)]
    pub root_resources: bool,

    /// FJ-755: Output graph as edge list (source→target pairs)
    #[arg(long)]
    pub edge_list: bool,

    /// FJ-759: Show disconnected subgraphs (connected components)
    #[arg(long)]
    pub connected_components: bool,

    /// FJ-763: Output graph as adjacency matrix
    #[arg(long)]
    pub adjacency_matrix: bool,

    /// FJ-767: Show longest dependency chain length
    #[arg(long)]
    pub longest_path: bool,

    /// FJ-771: Show in-degree (number of dependents) per resource
    #[arg(long)]
    pub in_degree: bool,

    /// FJ-775: Show out-degree (number of dependencies) per resource
    #[arg(long)]
    pub out_degree: bool,

    /// FJ-779: Show graph density (edges / max-possible-edges)
    #[arg(long)]
    pub density: bool,

    /// FJ-783: Output resources in valid topological execution order
    #[arg(long)]
    pub topological_sort: bool,

    /// FJ-787: Show resources on the longest dependency chain
    #[arg(long)]
    pub critical_path_resources: bool,

    /// FJ-791: Show resources that nothing depends on (leaf/sink nodes)
    #[arg(long)]
    pub sink_resources: bool,

    /// FJ-795: Check if dependency graph is bipartite
    #[arg(long)]
    pub bipartite_check: bool,

    /// FJ-799: Find strongly connected components (Tarjan's algorithm)
    #[arg(long)]
    pub strongly_connected: bool,

    /// FJ-803: Export dependency matrix as CSV
    #[arg(long)]
    pub dependency_matrix_csv: bool,

    /// FJ-807: Assign weights to edges by dependency criticality
    #[arg(long)]
    pub resource_weight: bool,

    /// FJ-811: Show max dependency chain depth per resource
    #[arg(long)]
    pub dependency_depth_per_resource: bool,

    /// FJ-815: Fan-in count per resource (how many depend on it)
    #[arg(long)]
    pub resource_fanin: bool,

    /// FJ-819: Detect disconnected subgraphs in the DAG
    #[arg(long)]
    pub isolated_subgraphs: bool,

    /// FJ-823: Full dependency chain from root to leaf per resource
    #[arg(long)]
    pub resource_dependency_chain: Option<String>,

    /// FJ-827: Resources with highest fan-in AND fan-out (bottlenecks)
    #[arg(long)]
    pub bottleneck_resources: bool,

    /// FJ-831: Longest weighted path through the DAG
    #[arg(long)]
    pub critical_dependency_path: bool,

    /// FJ-835: Histogram of dependency depths
    #[arg(long)]
    pub resource_depth_histogram: bool,

    /// FJ-839: Coupling score between resource pairs
    #[arg(long)]
    pub resource_coupling_score: bool,

    /// FJ-843: Overlay change frequency on dependency graph
    #[arg(long)]
    pub resource_change_frequency: bool,

    /// FJ-847: Impact score based on dependents + depth
    #[arg(long)]
    pub resource_impact_score: bool,

    /// FJ-851: Stability score based on status history
    #[arg(long)]
    pub resource_stability_score: bool,

    /// FJ-855: Fan-out count per resource
    #[arg(long)]
    pub resource_dependency_fanout: bool,

    /// FJ-859: Weighted edges based on resource coupling
    #[arg(long)]
    pub resource_dependency_weight: bool,

    /// FJ-863: Identify bottleneck resources with high fan-in + fan-out
    #[arg(long)]
    pub resource_dependency_bottleneck: bool,

    /// FJ-867: Cluster resources by type and show interconnections
    #[arg(long)]
    pub resource_type_clustering: bool,

    /// FJ-871: Identify near-cycle patterns in dependency graph
    #[arg(long)]
    pub resource_dependency_cycle_risk: bool,

    /// FJ-875: Calculate blast radius of resource changes
    #[arg(long)]
    pub resource_impact_radius: bool,

    /// FJ-879: Overlay health status on dependency graph
    #[arg(long)]
    pub resource_dependency_health_map: bool,

    /// FJ-883: Trace how changes propagate through dependencies
    #[arg(long)]
    pub resource_change_propagation: bool,
    /// FJ-887: Show max dependency chain depth per resource
    #[arg(long)]
    pub resource_dependency_depth_analysis: bool,
    /// FJ-891: Combined fan-in/fan-out analysis per resource
    #[arg(long)]
    pub resource_dependency_fan_analysis: bool,
    /// FJ-895: Isolation score per resource in dependency graph
    #[arg(long)]
    pub resource_dependency_isolation_score: bool,
    /// FJ-899: Stability score based on dependency change frequency
    #[arg(long)]
    pub resource_dependency_stability_score: bool,
    /// FJ-903: Critical path length through dependency graph
    #[arg(long)]
    pub resource_dependency_critical_path_length: bool,
    /// FJ-907: Redundancy score for resources with fallbacks
    #[arg(long)]
    pub resource_dependency_redundancy_score: bool,
    /// FJ-911: Betweenness centrality for critical resources
    #[arg(long)]
    pub resource_dependency_centrality_score: bool,
    /// FJ-915: Find bridge edges whose removal disconnects the graph
    #[arg(long)]
    pub resource_dependency_bridge_detection: bool,
}

