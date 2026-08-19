use crate::core::types::*;
use provable_contracts_macros::contract;
use std::collections::{HashMap, HashSet, VecDeque};

type Dag = (HashMap<String, usize>, HashMap<String, Vec<String>>);

/// Build a topological execution order from resource dependencies.
/// Uses Kahn's algorithm with alphabetical tie-breaking for determinism.
#[contract("dag-ordering-v1", equation = "topological_sort")]
pub fn build_execution_order(config: &ForjarConfig) -> Result<Vec<String>, String> {
    // Contract: dag-ordering-v1.yaml precondition (pv codegen)
    contract_pre_topological_sort!(config);
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

    // FJ-2200: Postcondition — valid topological order:
    // every resource appears before its dependents
    debug_assert!(
        {
            let pos: HashMap<&str, usize> = order
                .iter()
                .enumerate()
                .map(|(i, s)| (s.as_str(), i))
                .collect();
            config.resources.iter().all(|(id, res)| {
                res.depends_on.iter().all(|dep| {
                    pos.get(dep.as_str()).is_none_or(|&dep_pos| {
                        pos.get(id.as_str()).is_none_or(|&id_pos| dep_pos < id_pos)
                    })
                })
            })
        },
        "build_execution_order: topological ordering violated"
    );

    contract_post_configuration!(&order);
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
    let mut in_degree: HashMap<String, usize> = HashMap::with_capacity(resource_ids.len());
    let mut adjacency: HashMap<String, Vec<String>> = HashMap::with_capacity(resource_ids.len());

    for id in resource_ids {
        in_degree.insert(id.clone(), 0);
        adjacency.insert(id.clone(), Vec::new());
    }

    for (id, resource) in &config.resources {
        for dep in &resource.depends_on {
            if !config.resources.contains_key(dep) {
                return Err(format!("resource '{id}' depends on unknown '{dep}'"));
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
    // Contract: dag-ordering-v1.yaml precondition (pv codegen)
    contract_pre_kahn_sort!(in_degree);
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

    let mut order = Vec::with_capacity(in_degree.len());
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

    contract_post_configuration!(&order);
    order
}

/// FJ-2724 (PMAT-199): make's prerequisite closure over N goals.
///
/// Returns the goals plus every resource reachable from them through
/// `depends_on`, and nothing else. `make foo` builds foo and what foo needs;
/// this is the set that makes that possible.
///
/// # Why this is the safe filter
///
/// `--subset` and `--exclude` can cut a resource out from under a dependent,
/// so a targeted apply can execute against prerequisites that were never
/// converged. A `depends_on` closure is downward-closed by construction: if a
/// resource is in the set, everything it needs is too. That is exactly why
/// `make` is safe where an arbitrary pattern filter is not.
///
/// `-r` has the opposite problem — it is exact-match with no closure, so
/// `apply -r link` silently skips the compile step `link` depends on and
/// builds against whatever happened to be on disk. That is `make -o`, not
/// `make`.
///
/// An unknown goal is an error. Silently applying nothing is the failure mode
/// this release exists to remove.
///
/// Cycles are not reported here: `visited` guarantees termination, and
/// `build_execution_order` reports the cycle with its member list when the
/// pruned config is ordered.
pub fn goal_closure(config: &ForjarConfig, goals: &[String]) -> Result<HashSet<String>, String> {
    let mut visited: HashSet<String> = HashSet::new();
    let mut stack: Vec<String> = Vec::new();

    for goal in goals {
        if !config.resources.contains_key(goal) {
            let mut known: Vec<&str> = config.resources.keys().map(String::as_str).collect();
            known.sort_unstable();
            return Err(format!(
                "no rule to make target '{goal}'. Known targets: {}",
                known.join(", ")
            ));
        }
        stack.push(goal.clone());
    }

    while let Some(id) = stack.pop() {
        if !visited.insert(id.clone()) {
            continue;
        }
        let Some(resource) = config.resources.get(&id) else {
            continue;
        };
        for dep in &resource.depends_on {
            if !visited.contains(dep) {
                // An unknown dependency is reported by build_execution_order
                // with full context; skipping here keeps the closure total.
                stack.push(dep.clone());
            }
        }
    }

    Ok(visited)
}
