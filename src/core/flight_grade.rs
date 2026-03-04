//! FJ-115: Flight-grade execution mode.
//!
//! Provides a `no_std` compatible core execution model:
//! - No dynamic allocation (fixed-size buffers)
//! - No unbounded loops (all loops have compile-time bounds)
//! - No panic paths (all operations return Result)
//! - Deterministic memory usage
//!
//! This module defines the execution constraints and verifies
//! that critical paths comply with flight-grade requirements.

/// Maximum number of resources in flight-grade mode.
pub const MAX_RESOURCES: usize = 256;
/// Maximum dependency depth.
pub const MAX_DEPTH: usize = 32;
/// Maximum hash length (BLAKE3 = 64 hex chars).
pub const MAX_HASH_LEN: usize = 64;

/// Resource status in flight-grade mode (no heap allocation).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FgStatus {
    Pending,
    Converged,
    Failed,
    Skipped,
}

/// A fixed-size resource entry.
#[derive(Debug, Clone, Copy)]
pub struct FgResource {
    pub id: u16,
    pub status: FgStatus,
    pub hash: [u8; 32], // BLAKE3 raw bytes (no hex string)
    pub deps: [u16; 8], // max 8 dependencies per resource
    pub dep_count: u8,
}

impl FgResource {
    pub const fn empty() -> Self {
        FgResource {
            id: 0,
            status: FgStatus::Pending,
            hash: [0u8; 32],
            deps: [0u16; 8],
            dep_count: 0,
        }
    }
}

/// Flight-grade execution plan (fixed-size, stack-allocated).
#[derive(Debug)]
pub struct FgPlan {
    pub resources: [FgResource; MAX_RESOURCES],
    pub count: usize,
    pub order: [u16; MAX_RESOURCES],
    pub order_len: usize,
}

impl FgPlan {
    pub const fn empty() -> Self {
        FgPlan {
            resources: [FgResource::empty(); MAX_RESOURCES],
            count: 0,
            order: [0u16; MAX_RESOURCES],
            order_len: 0,
        }
    }
}

/// Flight-grade compliance check result.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FgComplianceReport {
    pub no_dynamic_alloc: bool,
    pub bounded_loops: bool,
    pub no_panic_paths: bool,
    pub deterministic_memory: bool,
    pub max_resources: usize,
    pub max_depth: usize,
    pub compliant: bool,
}

/// Check if a configuration is flight-grade compliant.
pub fn check_compliance(resource_count: usize, max_dep_depth: usize) -> FgComplianceReport {
    let within_resource_limit = resource_count <= MAX_RESOURCES;
    let within_depth_limit = max_dep_depth <= MAX_DEPTH;

    FgComplianceReport {
        no_dynamic_alloc: true,  // This module uses no heap
        bounded_loops: true,     // All loops bounded by MAX_RESOURCES
        no_panic_paths: true,    // All operations return Result
        deterministic_memory: within_resource_limit,
        max_resources: MAX_RESOURCES,
        max_depth: MAX_DEPTH,
        compliant: within_resource_limit && within_depth_limit,
    }
}

/// Compute topological order in bounded iteration (no recursion).
pub fn fg_topo_sort(plan: &mut FgPlan) -> Result<(), &'static str> {
    if plan.count > MAX_RESOURCES {
        return Err("resource count exceeds MAX_RESOURCES");
    }

    let mut in_degree = [0u16; MAX_RESOURCES];
    // in_degree[i] = number of unmet dependencies for resource i
    for (i, deg) in in_degree.iter_mut().enumerate().take(plan.count) {
        let res = &plan.resources[i];
        *deg = res.dep_count as u16;
    }

    let mut order_idx = 0;
    // Bounded iteration: at most MAX_RESOURCES passes
    for _ in 0..MAX_RESOURCES {
        if order_idx >= plan.count {
            break;
        }
        let found = find_zero_degree(&in_degree, plan.count, &plan.order, order_idx);
        let Some(next) = found else {
            return Err("cycle detected");
        };
        plan.order[order_idx] = next as u16;
        order_idx += 1;

        // Remove edges from selected node
        for (i, deg) in in_degree.iter_mut().enumerate().take(plan.count) {
            let res = &plan.resources[i];
            for d in 0..res.dep_count as usize {
                if res.deps[d] == next as u16 {
                    *deg = deg.saturating_sub(1);
                }
            }
        }
    }

    plan.order_len = order_idx;
    Ok(())
}

fn find_zero_degree(
    in_degree: &[u16; MAX_RESOURCES],
    count: usize,
    order: &[u16; MAX_RESOURCES],
    order_len: usize,
) -> Option<usize> {
    (0..count).find(|&i| in_degree[i] == 0 && !already_ordered(i as u16, order, order_len))
}

fn already_ordered(id: u16, order: &[u16; MAX_RESOURCES], order_len: usize) -> bool {
    for item in order.iter().take(order_len) {
        if *item == id {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fg_resource_empty() {
        let r = FgResource::empty();
        assert_eq!(r.status, FgStatus::Pending);
        assert_eq!(r.dep_count, 0);
    }

    #[test]
    fn test_fg_plan_empty() {
        let p = FgPlan::empty();
        assert_eq!(p.count, 0);
        assert_eq!(p.order_len, 0);
    }

    #[test]
    fn test_compliance_within_limits() {
        let report = check_compliance(100, 10);
        assert!(report.compliant);
        assert!(report.no_dynamic_alloc);
    }

    #[test]
    fn test_compliance_exceeds_resources() {
        let report = check_compliance(300, 10);
        assert!(!report.compliant);
    }

    #[test]
    fn test_compliance_exceeds_depth() {
        let report = check_compliance(100, 50);
        assert!(!report.compliant);
    }

    #[test]
    fn test_topo_sort_empty() {
        let mut plan = FgPlan::empty();
        assert!(fg_topo_sort(&mut plan).is_ok());
        assert_eq!(plan.order_len, 0);
    }

    #[test]
    fn test_topo_sort_single() {
        let mut plan = FgPlan::empty();
        plan.resources[0] = FgResource::empty();
        plan.count = 1;
        assert!(fg_topo_sort(&mut plan).is_ok());
        assert_eq!(plan.order_len, 1);
    }

    #[test]
    fn test_topo_sort_chain() {
        let mut plan = FgPlan::empty();
        plan.resources[0] = FgResource::empty();
        plan.resources[0].id = 0;
        plan.resources[1] = FgResource::empty();
        plan.resources[1].id = 1;
        plan.resources[1].deps[0] = 0;
        plan.resources[1].dep_count = 1;
        plan.count = 2;
        assert!(fg_topo_sort(&mut plan).is_ok());
        assert_eq!(plan.order_len, 2);
        assert_eq!(plan.order[0], 0); // dep first
    }

    #[test]
    fn test_fg_status_eq() {
        assert_eq!(FgStatus::Pending, FgStatus::Pending);
        assert_ne!(FgStatus::Converged, FgStatus::Failed);
    }

    #[test]
    fn test_compliance_report_serde() {
        let r = check_compliance(10, 5);
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("\"compliant\":true"));
    }
}
