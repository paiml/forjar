//! FJ-1435: Parallel multi-stack apply.
//!
//! `forjar apply --stacks net,compute,storage` runs independent stacks
//! concurrently, respecting cross-stack dependency ordering.

use super::helpers::*;

/// A stack in the parallel execution plan.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StackInfo {
    pub name: String,
    pub path: String,
    pub resources: usize,
    pub dependencies: Vec<String>,
}

/// Parallel execution plan.
#[derive(Debug, serde::Serialize)]
pub struct ParallelPlan {
    pub stacks: Vec<StackInfo>,
    pub waves: Vec<Wave>,
    pub total_stacks: usize,
    pub max_parallelism: usize,
}

/// A wave of stacks that can execute in parallel.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Wave {
    pub index: usize,
    pub stacks: Vec<String>,
    pub parallel: bool,
}

/// Plan parallel multi-stack execution.
pub fn cmd_parallel_stacks(
    files: &[std::path::PathBuf],
    max_parallel: usize,
    json: bool,
) -> Result<(), String> {
    let stacks = load_stacks(files)?;
    let waves = compute_waves(&stacks, max_parallel);
    let max_par = waves.iter().map(|w| w.stacks.len()).max().unwrap_or(0);

    let plan = ParallelPlan {
        total_stacks: stacks.len(),
        max_parallelism: max_par,
        stacks,
        waves,
    };

    if json {
        let out = serde_json::to_string_pretty(&plan).map_err(|e| format!("JSON error: {e}"))?;
        println!("{out}");
    } else {
        print_parallel_plan(&plan);
    }
    Ok(())
}

fn load_stacks(files: &[std::path::PathBuf]) -> Result<Vec<StackInfo>, String> {
    let mut stacks = Vec::new();
    for f in files {
        let config = parse_and_validate(f)?;
        let deps = extract_data_deps(&config);
        stacks.push(StackInfo {
            name: config.name.clone(),
            path: f.display().to_string(),
            resources: config.resources.len(),
            dependencies: deps,
        });
    }
    Ok(stacks)
}

pub(super) fn extract_data_deps(config: &crate::core::types::ForjarConfig) -> Vec<String> {
    let mut deps = Vec::new();
    for (_key, ds) in &config.data {
        if ds.source_type == crate::core::types::DataSourceType::ForjarState {
            if let Some(ref cfg_name) = ds.config {
                deps.push(cfg_name.clone());
            }
        }
    }
    deps.sort();
    deps.dedup();
    deps
}

pub(super) fn compute_waves(stacks: &[StackInfo], max_parallel: usize) -> Vec<Wave> {
    let mut waves = Vec::new();
    let mut placed: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut remaining: Vec<&StackInfo> = stacks.iter().collect();
    let mut wave_idx = 0;

    for _i in 0..100 {
        if remaining.is_empty() {
            break;
        }
        let (ready, still) = partition_ready(&remaining, &placed);
        if ready.is_empty() {
            let forced: Vec<String> = still.iter().map(|s| s.name.clone()).collect();
            waves.push(Wave {
                index: wave_idx,
                stacks: forced,
                parallel: false,
            });
            break;
        }

        // Chunk by max_parallel
        for chunk in ready.chunks(max_parallel) {
            waves.push(Wave {
                index: wave_idx,
                stacks: chunk.to_vec(),
                parallel: chunk.len() > 1,
            });
            for name in chunk {
                placed.insert(name.clone());
            }
            wave_idx += 1;
        }
        remaining = still;
    }
    waves
}

pub(super) fn partition_ready<'a>(
    remaining: &[&'a StackInfo],
    placed: &std::collections::BTreeSet<String>,
) -> (Vec<String>, Vec<&'a StackInfo>) {
    let mut ready = Vec::new();
    let mut still = Vec::new();
    for s in remaining {
        if s.dependencies.iter().all(|d| placed.contains(d)) {
            ready.push(s.name.clone());
        } else {
            still.push(*s);
        }
    }
    (ready, still)
}

pub(super) fn print_parallel_plan(plan: &ParallelPlan) {
    println!("Parallel Multi-Stack Plan");
    println!("=========================");
    println!(
        "Stacks: {} | Max Parallelism: {}",
        plan.total_stacks, plan.max_parallelism
    );
    println!();
    for s in &plan.stacks {
        let deps = if s.dependencies.is_empty() {
            "none".to_string()
        } else {
            s.dependencies.join(", ")
        };
        println!("  {} ({} resources, deps: {})", s.name, s.resources, deps);
    }
    println!();
    println!("Execution Waves:");
    for w in &plan.waves {
        let mode = if w.parallel { "parallel" } else { "serial" };
        println!("  Wave {}: {} [{}]", w.index, w.stacks.join(", "), mode);
    }
}
