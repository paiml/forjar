//! FJ-095/113/115: Reproducible builds, Ferrocene certification, flight-grade execution.
//!
//! Popperian rejection criteria for:
//! - FJ-095: Repro build environment checks, Cargo profile checks, source hashing,
//!   binary hashing, report generation, CI snippet
//! - FJ-113: Safety standards, ASIL/DAL levels, toolchain detection, source
//!   compliance, certification evidence, Ferrocene CI config
//! - FJ-115: Flight-grade constants, FgStatus/FgResource/FgPlan, compliance
//!   check, topological sort with bounded iteration
//!
//! Usage: cargo test --test falsification_repro_ferrocene_flight

use forjar::core::ferrocene::{
    check_source_compliance, detect_toolchain, ferrocene_ci_config, generate_evidence, AsilLevel,
    CertificationEvidence, DalLevel, SafetyStandard, ViolationSeverity,
};
use forjar::core::flight_grade::{
    check_compliance, fg_topo_sort, FgPlan, FgResource, FgStatus, MAX_DEPTH, MAX_HASH_LEN,
    MAX_RESOURCES,
};
use forjar::core::repro_build::{
    check_cargo_profile, check_environment, generate_report, hash_binary, hash_source_dir,
    repro_ci_snippet,
};

// ============================================================================
// FJ-095: Reproducible Build — Environment Checks
#[test]
fn flight_topo_sort_three_chain() {
    let mut plan = FgPlan::empty();
    // A → B → C
    plan.resources[0] = FgResource::empty();
    plan.resources[0].id = 0;

    plan.resources[1] = FgResource::empty();
    plan.resources[1].id = 1;
    plan.resources[1].deps[0] = 0;
    plan.resources[1].dep_count = 1;

    plan.resources[2] = FgResource::empty();
    plan.resources[2].id = 2;
    plan.resources[2].deps[0] = 1;
    plan.resources[2].dep_count = 1;

    plan.count = 3;

    assert!(fg_topo_sort(&mut plan).is_ok());
    assert_eq!(plan.order_len, 3);
    assert_eq!(plan.order[0], 0);
    assert_eq!(plan.order[1], 1);
    assert_eq!(plan.order[2], 2);
}

#[test]
fn flight_topo_sort_independent_resources() {
    let mut plan = FgPlan::empty();
    for i in 0..5 {
        plan.resources[i] = FgResource::empty();
        plan.resources[i].id = i as u16;
    }
    plan.count = 5;

    assert!(fg_topo_sort(&mut plan).is_ok());
    assert_eq!(plan.order_len, 5);
}

#[test]
fn flight_topo_sort_cycle_detected() {
    let mut plan = FgPlan::empty();
    // A depends on B, B depends on A → cycle
    plan.resources[0] = FgResource::empty();
    plan.resources[0].id = 0;
    plan.resources[0].deps[0] = 1;
    plan.resources[0].dep_count = 1;

    plan.resources[1] = FgResource::empty();
    plan.resources[1].id = 1;
    plan.resources[1].deps[0] = 0;
    plan.resources[1].dep_count = 1;

    plan.count = 2;

    let result = fg_topo_sort(&mut plan);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("cycle"));
}
