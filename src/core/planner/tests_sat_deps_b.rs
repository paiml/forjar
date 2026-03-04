//! Additional SAT solver tests for boosting sat_deps.rs coverage.

use super::sat_deps::*;

#[test]
fn test_unsatisfiable_direct_conflict() {
    // A and NOT A
    let problem = SatProblem {
        num_vars: 1,
        clauses: vec![vec![1], vec![-1]],
        var_names: {
            let mut m = std::collections::BTreeMap::new();
            m.insert(1, "A".to_string());
            m
        },
    };
    let result = solve(&problem);
    assert!(matches!(result, SatResult::Unsatisfiable { .. }));
    if let SatResult::Unsatisfiable { conflict_clause } = result {
        assert!(!conflict_clause.is_empty());
    }
}

#[test]
fn test_unsatisfiable_chain_conflict() {
    // A => B, B => C, C => !A, but A must be true
    let problem = SatProblem {
        num_vars: 3,
        clauses: vec![
            vec![-1, 2],  // A => B
            vec![-2, 3],  // B => C
            vec![-3, -1], // C => !A
            vec![1],      // A must be true
            vec![2],      // B must be true
            vec![3],      // C must be true
        ],
        var_names: {
            let mut m = std::collections::BTreeMap::new();
            m.insert(1, "A".to_string());
            m.insert(2, "B".to_string());
            m.insert(3, "C".to_string());
            m
        },
    };
    let result = solve(&problem);
    assert!(matches!(result, SatResult::Unsatisfiable { .. }));
}

#[test]
fn test_build_sat_problem_missing_dep() {
    // Dependency references a resource not in the list
    let resources = vec!["A".into(), "B".into()];
    let deps = vec![("A".into(), "MISSING".into())];
    let problem = build_sat_problem(&resources, &deps);
    // Should still work — missing dep is simply ignored
    assert_eq!(problem.num_vars, 2);
    // Only unit clauses (no implication since MISSING not in vars)
    assert_eq!(problem.clauses.len(), 2);
}

#[test]
fn test_build_sat_problem_empty() {
    let resources: Vec<String> = vec![];
    let deps: Vec<(String, String)> = vec![];
    let problem = build_sat_problem(&resources, &deps);
    assert_eq!(problem.num_vars, 0);
    assert!(problem.clauses.is_empty());
}

#[test]
fn test_satisfiable_complex_5_vars() {
    let resources: Vec<String> = (1..=5).map(|i| format!("R{i}")).collect();
    let deps = vec![
        ("R2".into(), "R1".into()),
        ("R3".into(), "R1".into()),
        ("R4".into(), "R2".into()),
        ("R4".into(), "R3".into()),
        ("R5".into(), "R4".into()),
    ];
    let problem = build_sat_problem(&resources, &deps);
    let result = solve(&problem);
    if let SatResult::Satisfiable { assignment } = result {
        // All must be true since they all have unit clauses
        for r in &resources {
            assert!(assignment[r]);
        }
    } else {
        panic!("expected satisfiable");
    }
}

#[test]
fn test_sat_result_unsatisfiable_serde() {
    let result = SatResult::Unsatisfiable {
        conflict_clause: vec!["A".into(), "!B".into()],
    };
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("Unsatisfiable"));
    assert!(json.contains("conflict_clause"));
}

#[test]
fn test_solve_with_redundant_clauses() {
    // Same clause repeated
    let problem = SatProblem {
        num_vars: 2,
        clauses: vec![vec![1], vec![2], vec![1, 2], vec![1, 2], vec![1]],
        var_names: {
            let mut m = std::collections::BTreeMap::new();
            m.insert(1, "A".to_string());
            m.insert(2, "B".to_string());
            m
        },
    };
    let result = solve(&problem);
    assert!(matches!(result, SatResult::Satisfiable { .. }));
}

#[test]
fn test_solve_negative_unit_clause() {
    // Force variable to be false via negative unit clause
    let problem = SatProblem {
        num_vars: 2,
        clauses: vec![vec![-1], vec![2], vec![-1, 2]],
        var_names: {
            let mut m = std::collections::BTreeMap::new();
            m.insert(1, "A".to_string());
            m.insert(2, "B".to_string());
            m
        },
    };
    let result = solve(&problem);
    if let SatResult::Satisfiable { assignment } = result {
        assert!(!assignment["A"]);
        assert!(assignment["B"]);
    } else {
        panic!("expected satisfiable");
    }
}

#[test]
fn test_solve_all_negative() {
    let problem = SatProblem {
        num_vars: 2,
        clauses: vec![vec![-1], vec![-2]],
        var_names: {
            let mut m = std::collections::BTreeMap::new();
            m.insert(1, "A".to_string());
            m.insert(2, "B".to_string());
            m
        },
    };
    let result = solve(&problem);
    if let SatResult::Satisfiable { assignment } = result {
        assert!(!assignment["A"]);
        assert!(!assignment["B"]);
    } else {
        panic!("expected satisfiable");
    }
}

#[test]
fn test_build_sat_problem_var_names() {
    let resources = vec!["nginx".into(), "curl".into(), "vim".into()];
    let deps = vec![];
    let problem = build_sat_problem(&resources, &deps);
    assert_eq!(problem.var_names.len(), 3);
    assert!(problem.var_names.values().any(|v| v == "nginx"));
    assert!(problem.var_names.values().any(|v| v == "curl"));
}
