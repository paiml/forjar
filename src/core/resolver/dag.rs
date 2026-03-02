use crate::core::types::*;
use provable_contracts_macros::contract;
use std::collections::{HashMap, HashSet, VecDeque};

type Dag = (HashMap<String, usize>, HashMap<String, Vec<String>>);

/// Build a topological execution order from resource dependencies.
/// Uses Kahn's algorithm with alphabetical tie-breaking for determinism.
#[contract("dag-ordering-v1", equation = "topological_sort")]
pub fn build_execution_order(config: &ForjarConfig) -> Result<Vec<String>, String> {
    let resource_ids: Vec<String> = config.resources.keys().cloned().collect();
    let (mut in_degree, mut adjacency) = build_dag(config, &resource_ids)?;
    let order = kahn_sort(&resource_ids, &mut in_degree, &mut adjacency);

    if order.len() != resource_ids.len() {
        let remaining: HashSet<_> = resource_ids.iter().collect();
        let ordered: HashSet<_> = order.iter().collect();
        let cycle_members: Vec<_> = remaining.difference(&ordered).collect();
        return Err(format!(
            "dependency cycle detected involving: {}",
            cycle_members
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    Ok(order)
}

/// FJ-216: Compute parallel waves from the DAG.
///
/// Groups resources into waves where all resources in a wave have no
/// inter-dependencies and can execute concurrently. Wave order respects
/// the DAG: all dependencies of a resource are in earlier waves.
///
/// Returns `Vec<Vec<String>>` where each inner Vec is a concurrent wave.
pub fn compute_parallel_waves(config: &ForjarConfig) -> Result<Vec<Vec<String>>, String> {
    let resource_ids: Vec<String> = config.resources.keys().cloned().collect();
    let (mut in_degree, adjacency) = build_dag(config, &resource_ids)?;

    let mut waves = Vec::new();

    loop {
        // Collect all nodes with in-degree 0 (no remaining deps)
        let mut wave: Vec<String> = in_degree
            .iter()
            .filter(|(_, &d)| d == 0)
            .map(|(id, _)| id.clone())
            .collect();

        if wave.is_empty() {
            break;
        }

        wave.sort(); // Deterministic order within wave

        // Remove this wave from the graph
        for id in &wave {
            in_degree.remove(id);
            if let Some(neighbors) = adjacency.get(id) {
                for neighbor in neighbors {
                    if let Some(deg) = in_degree.get_mut(neighbor) {
                        *deg -= 1;
                    }
                }
            }
        }

        waves.push(wave);
    }

    if !in_degree.is_empty() {
        let cycle_members: Vec<_> = in_degree.keys().cloned().collect();
        return Err(format!(
            "dependency cycle detected involving: {}",
            cycle_members.join(", ")
        ));
    }

    Ok(waves)
}

/// Build adjacency list and in-degree map from resource dependencies.
fn build_dag(config: &ForjarConfig, resource_ids: &[String]) -> Result<Dag, String> {
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();

    for id in resource_ids {
        in_degree.insert(id.clone(), 0);
        adjacency.insert(id.clone(), Vec::new());
    }

    for (id, resource) in &config.resources {
        for dep in &resource.depends_on {
            if !config.resources.contains_key(dep) {
                return Err(format!("resource '{}' depends on unknown '{}'", id, dep));
            }
            if let Some(adj) = adjacency.get_mut(dep) {
                adj.push(id.clone());
            }
            if let Some(deg) = in_degree.get_mut(id) {
                *deg += 1;
            }
        }
    }

    Ok((in_degree, adjacency))
}

/// Run Kahn's algorithm with alphabetical tie-breaking.
fn kahn_sort(
    _resource_ids: &[String],
    in_degree: &mut HashMap<String, usize>,
    adjacency: &mut HashMap<String, Vec<String>>,
) -> Vec<String> {
    let mut queue: VecDeque<String> = VecDeque::new();
    let mut zero_degree: Vec<String> = in_degree
        .iter()
        .filter(|(_, &d)| d == 0)
        .map(|(id, _)| id.clone())
        .collect();
    zero_degree.sort();
    for id in zero_degree {
        queue.push_back(id);
    }

    let mut order = Vec::new();
    while let Some(current) = queue.pop_front() {
        let mut next_ready: Vec<String> = Vec::new();
        if let Some(neighbors) = adjacency.get(&current) {
            for neighbor in neighbors {
                if let Some(deg) = in_degree.get_mut(neighbor) {
                    *deg -= 1;
                    if *deg == 0 {
                        next_ready.push(neighbor.clone());
                    }
                }
            }
        }
        next_ready.sort();
        for id in next_ready {
            queue.push_back(id);
        }
        order.push(current);
    }

    order
}
