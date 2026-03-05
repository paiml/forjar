//! Advanced graph analysis — bipartite, SCC, CSV export.

use super::graph_export::{build_adjacency_matrix, build_undirected_graph, compute_in_degrees};
use super::helpers::*;
#[allow(unused_imports)]
use crate::core::{codegen, executor, migrate, parser, planner, resolver, secrets, state, types};
use std::path::Path;

/// FJ-795: Check if dependency graph is bipartite.
pub(crate) fn cmd_graph_bipartite_check(file: &Path, json: bool) -> Result<(), String> {
    let cfg = parse_and_validate(file)?;
    let is_bip = check_bipartite(&cfg);
    if json {
        println!("{{\"is_bipartite\":{is_bip}}}");
    } else if is_bip {
        println!("The dependency graph is bipartite.");
    } else {
        println!("The dependency graph is NOT bipartite (contains odd-length cycle).");
    }
    Ok(())
}

/// Check bipartite using 2-coloring BFS on undirected graph.
pub(super) fn check_bipartite(cfg: &types::ForjarConfig) -> bool {
    let adj = build_undirected_graph(cfg);
    let mut color: std::collections::HashMap<&str, bool> = std::collections::HashMap::new();
    for &start in adj.keys() {
        if color.contains_key(start) {
            continue;
        }
        color.insert(start, false);
        if !bfs_2color(start, &adj, &mut color) {
            return false;
        }
    }
    true
}

/// BFS 2-coloring from a start node. Returns false if odd cycle found.
pub(super) fn bfs_2color<'a>(
    start: &'a str,
    adj: &std::collections::HashMap<&str, Vec<&'a str>>,
    color: &mut std::collections::HashMap<&'a str, bool>,
) -> bool {
    let mut queue = std::collections::VecDeque::new();
    queue.push_back(start);
    while let Some(n) = queue.pop_front() {
        let c = color[n];
        if let Some(neighbors) = adj.get(n) {
            for &next in neighbors {
                if let Some(&nc) = color.get(next) {
                    if nc == c {
                        return false;
                    }
                } else {
                    color.insert(next, !c);
                    queue.push_back(next);
                }
            }
        }
    }
    true
}

/// FJ-799: Find strongly connected components using Tarjan's algorithm.
pub(crate) fn cmd_graph_strongly_connected(file: &Path, json: bool) -> Result<(), String> {
    let cfg = parse_and_validate(file)?;
    let sccs = tarjan_scc(&cfg);
    let nontrivial: Vec<&Vec<String>> = sccs.iter().filter(|c| c.len() > 1).collect();
    if json {
        let items: Vec<String> = sccs.iter().map(|c| format!("{c:?}")).collect();
        println!(
            "{{\"strongly_connected_components\":[{}],\"total\":{},\"nontrivial\":{}}}",
            items.join(","),
            sccs.len(),
            nontrivial.len()
        );
    } else if sccs.is_empty() {
        println!("No resources (empty graph).");
    } else {
        println!(
            "Strongly connected components ({}, {} nontrivial):",
            sccs.len(),
            nontrivial.len()
        );
        for (i, comp) in sccs.iter().enumerate() {
            let marker = if comp.len() > 1 { " [CYCLE]" } else { "" };
            println!(
                "  SCC {} ({} nodes{}): {}",
                i + 1,
                comp.len(),
                marker,
                comp.join(", ")
            );
        }
    }
    Ok(())
}

/// Mutable state for Tarjan's SCC algorithm.
pub(super) struct TarjanState<'a> {
    pub(super) counter: usize,
    pub(super) indices: Vec<usize>,
    pub(super) lowlinks: Vec<usize>,
    pub(super) on_stack: Vec<bool>,
    pub(super) stack: Vec<usize>,
    pub(super) result: Vec<Vec<String>>,
    pub(super) names: &'a [&'a str],
}

/// Tarjan's SCC algorithm — recursive with state struct.
pub(super) fn tarjan_scc(cfg: &types::ForjarConfig) -> Vec<Vec<String>> {
    let names: Vec<&str> = cfg.resources.keys().map(|k| k.as_str()).collect();
    let idx_map: std::collections::HashMap<&str, usize> =
        names.iter().enumerate().map(|(i, &n)| (n, i)).collect();
    let adj = build_directed_adj(cfg, &idx_map);
    let n = names.len();
    let mut st = TarjanState {
        counter: 0,
        indices: vec![usize::MAX; n],
        lowlinks: vec![0; n],
        on_stack: vec![false; n],
        stack: Vec::new(),
        result: Vec::new(),
        names: &names,
    };
    for i in 0..n {
        if st.indices[i] == usize::MAX {
            tarjan_visit(i, &adj, &mut st);
        }
    }
    st.result.iter_mut().for_each(|c| c.sort());
    st.result.sort_by(|a, b| a[0].cmp(&b[0]));
    st.result
}

/// Build directed adjacency list (node index → vec of neighbor indices).
pub(super) fn build_directed_adj(
    cfg: &types::ForjarConfig,
    idx: &std::collections::HashMap<&str, usize>,
) -> Vec<Vec<usize>> {
    let n = idx.len();
    let mut adj = vec![Vec::new(); n];
    for (name, resource) in &cfg.resources {
        if let Some(&from) = idx.get(name.as_str()) {
            for dep in &resource.depends_on {
                if let Some(&to) = idx.get(dep.as_str()) {
                    adj[from].push(to);
                }
            }
        }
    }
    adj
}

/// Recursive Tarjan visit for a single node.
pub(super) fn tarjan_visit(v: usize, adj: &[Vec<usize>], st: &mut TarjanState<'_>) {
    st.indices[v] = st.counter;
    st.lowlinks[v] = st.counter;
    st.counter += 1;
    st.stack.push(v);
    st.on_stack[v] = true;
    for &w in &adj[v] {
        if st.indices[w] == usize::MAX {
            tarjan_visit(w, adj, st);
            st.lowlinks[v] = st.lowlinks[v].min(st.lowlinks[w]);
        } else if st.on_stack[w] {
            st.lowlinks[v] = st.lowlinks[v].min(st.indices[w]);
        }
    }
    if st.lowlinks[v] == st.indices[v] {
        let mut comp = Vec::new();
        while let Some(w) = st.stack.pop() {
            st.on_stack[w] = false;
            comp.push(st.names[w].to_string());
            if w == v {
                break;
            }
        }
        st.result.push(comp);
    }
}

/// FJ-803: Export dependency graph as CSV adjacency matrix.
pub(crate) fn cmd_graph_dependency_matrix_csv(file: &Path, json: bool) -> Result<(), String> {
    let cfg = parse_and_validate(file)?;
    let (names, matrix) = build_adjacency_matrix(&cfg);
    if json {
        let rows: Vec<String> = matrix
            .iter()
            .map(|row| {
                format!(
                    "[{}]",
                    row.iter()
                        .map(|&v| if v { "1" } else { "0" })
                        .collect::<Vec<_>>()
                        .join(",")
                )
            })
            .collect();
        let labels: Vec<String> = names.iter().map(|n| format!("\"{n}\"")).collect();
        println!(
            "{{\"labels\":[{}],\"matrix\":[{}]}}",
            labels.join(","),
            rows.join(",")
        );
    } else if names.is_empty() {
        println!("No resources (empty graph).");
    } else {
        // CSV header
        print!(",");
        println!("{}", names.join(","));
        // CSV rows
        for (i, name) in names.iter().enumerate() {
            print!("{name}");
            for cell in &matrix[i] {
                print!(",{}", if *cell { 1 } else { 0 });
            }
            println!();
        }
    }
    Ok(())
}

/// FJ-807: Assign weights to edges by dependency criticality.
pub(crate) fn cmd_graph_resource_weight(file: &Path, json: bool) -> Result<(), String> {
    let config = parse_and_validate(file)?;
    let in_deg_vec = compute_in_degrees(&config);
    let in_deg: std::collections::HashMap<&str, usize> =
        in_deg_vec.iter().map(|(n, d)| (n.as_str(), *d)).collect();
    let mut weights: Vec<(&str, &str, usize)> = Vec::new();
    for (name, resource) in &config.resources {
        for dep in &resource.depends_on {
            let dep_fan = in_deg.get(dep.as_str()).copied().unwrap_or(0);
            let w = dep_fan + 1;
            weights.push((name.as_str(), dep.as_str(), w));
        }
    }
    weights.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(b.0)));
    if json {
        let items: Vec<String> = weights
            .iter()
            .map(|(from, to, w)| format!("{{\"from\":\"{from}\",\"to\":\"{to}\",\"weight\":{w}}}"))
            .collect();
        println!("{{\"weighted_edges\":[{}]}}", items.join(","));
    } else if weights.is_empty() {
        println!("No dependency edges.");
    } else {
        println!("Weighted dependency edges ({}):", weights.len());
        for (from, to, w) in &weights {
            println!("  {from} -> {to} (weight: {w})");
        }
    }
    Ok(())
}

pub(super) use super::graph_advanced_b::*;
