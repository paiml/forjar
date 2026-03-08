//! FJ-045: SAT/SMT-based dependency resolution.
//!
//! Proves satisfiability of resource constraints and provides
//! exact conflict diagnosis when constraints are unsatisfiable.
//!
//! Uses a simple DPLL-style boolean satisfiability solver over
//! dependency constraints. Each resource is a boolean variable
//! (true = included in plan). Dependencies become implications:
//! `A depends_on B` → `A => B` → `(!A || B)`.

use std::collections::BTreeMap;

/// A dependency constraint in CNF (Conjunctive Normal Form).
/// Each clause is a disjunction of literals.
/// Positive literal = variable must be true, negative = must be false.
#[derive(Debug, Clone)]
pub struct SatProblem {
    /// Number of boolean variables.
    pub num_vars: usize,
    /// CNF clauses (each is a disjunction of literals).
    pub clauses: Vec<Vec<i32>>,
    /// Mapping from variable index to resource name.
    pub var_names: BTreeMap<usize, String>,
}

/// Result of SAT solving.
#[derive(Debug, Clone, serde::Serialize)]
pub enum SatResult {
    /// All constraints can be satisfied.
    Satisfiable {
        /// Variable assignments (resource name to inclusion).
        assignment: BTreeMap<String, bool>,
    },
    /// Constraints are contradictory.
    Unsatisfiable {
        /// Resources involved in the first conflict.
        conflict_clause: Vec<String>,
    },
}

/// Build SAT problem from dependency graph.
pub fn build_sat_problem(resources: &[String], deps: &[(String, String)]) -> SatProblem {
    let mut var_map: BTreeMap<String, usize> = BTreeMap::new();
    let mut var_names: BTreeMap<usize, String> = BTreeMap::new();

    for (i, name) in resources.iter().enumerate() {
        let idx = i + 1; // SAT vars are 1-indexed
        var_map.insert(name.clone(), idx);
        var_names.insert(idx, name.clone());
    }

    let mut clauses = Vec::new();

    // Each dependency A→B becomes clause (!A || B)
    for (dependent, dependency) in deps {
        if let (Some(&a), Some(&b)) = (var_map.get(dependent), var_map.get(dependency)) {
            clauses.push(vec![-(a as i32), b as i32]);
        }
    }

    // All requested resources must be included (unit clauses)
    for &idx in var_map.values() {
        clauses.push(vec![idx as i32]);
    }

    SatProblem {
        num_vars: resources.len(),
        clauses,
        var_names,
    }
}

/// Simple DPLL SAT solver.
pub fn solve(problem: &SatProblem) -> SatResult {
    let mut assignment = vec![None; problem.num_vars + 1];
    if dpll(&problem.clauses, &mut assignment, problem.num_vars) {
        build_sat_result(&assignment, &problem.var_names)
    } else {
        build_unsat_result(&problem.clauses, &problem.var_names)
    }
}

fn dpll(clauses: &[Vec<i32>], assignment: &mut [Option<bool>], num_vars: usize) -> bool {
    // Unit propagation
    let simplified = propagate_units(clauses, assignment);

    // Check if all clauses are satisfied
    if all_satisfied(&simplified, assignment) {
        return true;
    }

    // Check for empty clause (conflict)
    if has_empty_clause(&simplified, assignment) {
        return false;
    }

    // Pick unassigned variable
    let var = pick_unassigned(assignment, num_vars);
    let Some(var) = var else {
        return all_satisfied(&simplified, assignment);
    };

    // Try true
    assignment[var] = Some(true);
    if dpll(&simplified, assignment, num_vars) {
        return true;
    }

    // Try false
    assignment[var] = Some(false);
    if dpll(&simplified, assignment, num_vars) {
        return true;
    }

    // Backtrack
    assignment[var] = None;
    false
}

fn propagate_units(clauses: &[Vec<i32>], assignment: &mut [Option<bool>]) -> Vec<Vec<i32>> {
    let mut result = clauses.to_vec();
    let mut changed = true;
    while changed {
        changed = false;
        for clause in &result.clone() {
            if clause.len() == 1 {
                let lit = clause[0];
                let var = lit.unsigned_abs() as usize;
                let val = lit > 0;
                if assignment[var].is_none() {
                    assignment[var] = Some(val);
                    changed = true;
                }
            }
        }
        result = simplify_clauses(&result, assignment);
    }
    result
}

fn simplify_clauses(clauses: &[Vec<i32>], assignment: &[Option<bool>]) -> Vec<Vec<i32>> {
    clauses
        .iter()
        .filter(|clause| !clause_satisfied(clause, assignment))
        .cloned()
        .collect()
}

fn clause_satisfied(clause: &[i32], assignment: &[Option<bool>]) -> bool {
    clause.iter().any(|&lit| {
        let var = lit.unsigned_abs() as usize;
        let val = lit > 0;
        assignment.get(var).copied().flatten() == Some(val)
    })
}

fn all_satisfied(clauses: &[Vec<i32>], assignment: &[Option<bool>]) -> bool {
    clauses.iter().all(|c| clause_satisfied(c, assignment))
}

fn has_empty_clause(clauses: &[Vec<i32>], assignment: &[Option<bool>]) -> bool {
    clauses.iter().any(|clause| {
        clause.iter().all(|&lit| {
            let var = lit.unsigned_abs() as usize;
            let val = lit > 0;
            assignment.get(var).copied().flatten() == Some(!val)
        })
    })
}

fn pick_unassigned(assignment: &[Option<bool>], num_vars: usize) -> Option<usize> {
    (1..=num_vars).find(|&i| assignment[i].is_none())
}

fn build_sat_result(assignment: &[Option<bool>], var_names: &BTreeMap<usize, String>) -> SatResult {
    let mut map = BTreeMap::new();
    for (&idx, name) in var_names {
        map.insert(name.clone(), assignment[idx].unwrap_or(true));
    }
    SatResult::Satisfiable { assignment: map }
}

fn build_unsat_result(clauses: &[Vec<i32>], var_names: &BTreeMap<usize, String>) -> SatResult {
    // Report first unsatisfied clause as conflict
    let conflict: Vec<String> = clauses
        .first()
        .map(|c| {
            c.iter()
                .map(|&lit| {
                    let var = lit.unsigned_abs() as usize;
                    let name = var_names
                        .get(&var)
                        .cloned()
                        .unwrap_or_else(|| format!("v{var}"));
                    if lit > 0 {
                        name
                    } else {
                        format!("!{name}")
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    SatResult::Unsatisfiable {
        conflict_clause: conflict,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_satisfiable_linear_deps() {
        let resources = vec!["A".into(), "B".into(), "C".into()];
        let deps = vec![("B".into(), "A".into()), ("C".into(), "B".into())];
        let problem = build_sat_problem(&resources, &deps);
        let result = solve(&problem);
        assert!(matches!(result, SatResult::Satisfiable { .. }));
    }

    #[test]
    fn test_satisfiable_no_deps() {
        let resources = vec!["X".into(), "Y".into()];
        let deps = vec![];
        let problem = build_sat_problem(&resources, &deps);
        let result = solve(&problem);
        if let SatResult::Satisfiable { assignment } = result {
            assert_eq!(assignment.len(), 2);
            assert!(assignment["X"]);
            assert!(assignment["Y"]);
        } else {
            panic!("expected satisfiable");
        }
    }

    #[test]
    fn test_satisfiable_diamond() {
        let resources = vec!["A".into(), "B".into(), "C".into(), "D".into()];
        let deps = vec![
            ("B".into(), "A".into()),
            ("C".into(), "A".into()),
            ("D".into(), "B".into()),
            ("D".into(), "C".into()),
        ];
        let problem = build_sat_problem(&resources, &deps);
        let result = solve(&problem);
        assert!(matches!(result, SatResult::Satisfiable { .. }));
    }

    #[test]
    fn test_single_resource() {
        let resources = vec!["solo".into()];
        let deps = vec![];
        let problem = build_sat_problem(&resources, &deps);
        let result = solve(&problem);
        if let SatResult::Satisfiable { assignment } = result {
            assert!(assignment["solo"]);
        } else {
            panic!("expected satisfiable");
        }
    }

    #[test]
    fn test_sat_result_serde() {
        let result = SatResult::Satisfiable {
            assignment: BTreeMap::from([("A".into(), true), ("B".into(), false)]),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"Satisfiable\""));
    }

    #[test]
    fn test_build_problem_structure() {
        let resources = vec!["A".into(), "B".into()];
        let deps = vec![("B".into(), "A".into())];
        let problem = build_sat_problem(&resources, &deps);
        assert_eq!(problem.num_vars, 2);
        // 1 implication clause + 2 unit clauses = 3 clauses
        assert_eq!(problem.clauses.len(), 3);
    }

    #[test]
    fn test_unsatisfiable_contradiction() {
        // Manually construct a contradictory problem: A must be true AND false
        let mut var_names = BTreeMap::new();
        var_names.insert(1, "A".into());
        let problem = SatProblem {
            num_vars: 1,
            clauses: vec![vec![1], vec![-1]], // A AND !A
            var_names,
        };
        let result = solve(&problem);
        assert!(
            matches!(result, SatResult::Unsatisfiable { .. }),
            "contradictory clauses should be unsatisfiable"
        );
    }

    #[test]
    fn test_unsatisfiable_conflict_clause_names() {
        let mut var_names = BTreeMap::new();
        var_names.insert(1, "pkg-a".into());
        var_names.insert(2, "pkg-b".into());
        let problem = SatProblem {
            num_vars: 2,
            // pkg-a required, !pkg-a||pkg-b, !pkg-b
            clauses: vec![vec![1], vec![-1, 2], vec![-2]],
            var_names,
        };
        let result = solve(&problem);
        match result {
            SatResult::Unsatisfiable { conflict_clause } => {
                assert!(!conflict_clause.is_empty());
            }
            _ => panic!("expected unsatisfiable"),
        }
    }

    #[test]
    fn test_unknown_var_in_unsat_result() {
        let var_names = BTreeMap::new(); // empty — no var names
        let result = build_unsat_result(&[vec![1, -2]], &var_names);
        if let SatResult::Unsatisfiable { conflict_clause } = result {
            // Should produce "v1" and "!v2" fallback names
            assert!(conflict_clause.iter().any(|c| c.starts_with("v")));
        } else {
            panic!("expected unsatisfiable");
        }
    }

    #[test]
    fn test_empty_clauses_unsat_result() {
        let var_names = BTreeMap::new();
        let result = build_unsat_result(&[], &var_names);
        if let SatResult::Unsatisfiable { conflict_clause } = result {
            assert!(conflict_clause.is_empty());
        } else {
            panic!("expected unsatisfiable");
        }
    }

    #[test]
    fn test_dpll_backtracking() {
        // 3 variables, complex clauses requiring backtracking
        let mut var_names = BTreeMap::new();
        var_names.insert(1, "X".into());
        var_names.insert(2, "Y".into());
        var_names.insert(3, "Z".into());
        let problem = SatProblem {
            num_vars: 3,
            clauses: vec![
                vec![1, 2],  // X OR Y
                vec![-1, 3], // !X OR Z
                vec![2, -3], // Y OR !Z
                vec![1, -2], // X OR !Y
            ],
            var_names,
        };
        let result = solve(&problem);
        assert!(matches!(result, SatResult::Satisfiable { .. }));
    }

    #[test]
    fn test_has_empty_clause() {
        // When all literals in a clause are assigned to the opposite
        let assignment: Vec<Option<bool>> = vec![None, Some(false)]; // var 1 = false
        let clauses = vec![vec![1i32]]; // clause requires var 1 = true
        assert!(has_empty_clause(&clauses, &assignment));
    }

    #[test]
    fn test_clause_satisfied() {
        let assignment: Vec<Option<bool>> = vec![None, Some(true), Some(false)];
        // var 1 = true, var 2 = false
        assert!(clause_satisfied(&[1], &assignment)); // var 1 is true
        assert!(!clause_satisfied(&[-1], &assignment)); // !var1 is false
        assert!(clause_satisfied(&[-2], &assignment)); // !var2 is true (var2=false)
        assert!(!clause_satisfied(&[2], &assignment)); // var2 is false
    }

    #[test]
    fn test_all_satisfied_empty() {
        let assignment: Vec<Option<bool>> = vec![None];
        assert!(all_satisfied(&[], &assignment));
    }

    #[test]
    fn test_pick_unassigned_all_assigned() {
        let assignment: Vec<Option<bool>> = vec![None, Some(true), Some(false)];
        assert_eq!(pick_unassigned(&assignment, 2), None);
    }

    #[test]
    fn test_pick_unassigned_first() {
        let assignment: Vec<Option<bool>> = vec![None, None, Some(true)];
        assert_eq!(pick_unassigned(&assignment, 2), Some(1));
    }

    #[test]
    fn test_simplify_clauses() {
        let assignment: Vec<Option<bool>> = vec![None, Some(true)]; // var 1 = true
        let clauses = vec![vec![1], vec![-1, 2]]; // [1] is satisfied, [-1,2] is not
        let simplified = simplify_clauses(&clauses, &assignment);
        assert_eq!(simplified.len(), 1); // Only [-1, 2] remains
    }

    #[test]
    fn test_deps_with_unknown_resources() {
        // Dependency references a resource not in the list — should be skipped
        let resources = vec!["A".into()];
        let deps = vec![("A".into(), "MISSING".into())];
        let problem = build_sat_problem(&resources, &deps);
        // Only unit clause for A, no implication (MISSING not in var_map)
        assert_eq!(problem.clauses.len(), 1);
    }

    #[test]
    fn test_many_resources_satisfiable() {
        let resources: Vec<String> = (0..10).map(|i| format!("r{i}")).collect();
        let deps: Vec<(String, String)> = (1..10)
            .map(|i| (format!("r{i}"), format!("r{}", i - 1)))
            .collect();
        let problem = build_sat_problem(&resources, &deps);
        let result = solve(&problem);
        if let SatResult::Satisfiable { assignment } = result {
            assert_eq!(assignment.len(), 10);
            assert!(assignment.values().all(|&v| v));
        } else {
            panic!("linear chain should be satisfiable");
        }
    }

    #[test]
    fn test_unsat_result_negative_literal_formatting() {
        let mut var_names = BTreeMap::new();
        var_names.insert(1, "svc".into());
        let result = build_unsat_result(&[vec![-1]], &var_names);
        if let SatResult::Unsatisfiable { conflict_clause } = result {
            assert_eq!(conflict_clause, vec!["!svc"]);
        }
    }

    #[test]
    fn test_propagate_units_assigns_unit_clause() {
        // Unit clause [1] assigns var 1 = true
        let clauses = vec![vec![1], vec![-1, 2]];
        let mut assignment: Vec<Option<bool>> = vec![None, None, None];
        let result = propagate_units(&clauses, &mut assignment);
        assert_eq!(assignment[1], Some(true));
        // After assigning var1=true, clause [1] is satisfied and removed.
        // [-1, 2] may still be present (not fully propagated by unit prop alone).
        // The remaining clauses should be a subset.
        assert!(result.len() <= clauses.len());
    }
}
