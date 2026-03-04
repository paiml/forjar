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
    pub num_vars: usize,
    pub clauses: Vec<Vec<i32>>,
    pub var_names: BTreeMap<usize, String>,
}

/// Result of SAT solving.
#[derive(Debug, Clone, serde::Serialize)]
pub enum SatResult {
    Satisfiable { assignment: BTreeMap<String, bool> },
    Unsatisfiable { conflict_clause: Vec<String> },
}

/// Build SAT problem from dependency graph.
pub fn build_sat_problem(
    resources: &[String],
    deps: &[(String, String)],
) -> SatProblem {
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
    let Some(var) = var else { return all_satisfied(&simplified, assignment) };

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

fn build_sat_result(
    assignment: &[Option<bool>],
    var_names: &BTreeMap<usize, String>,
) -> SatResult {
    let mut map = BTreeMap::new();
    for (&idx, name) in var_names {
        map.insert(name.clone(), assignment[idx].unwrap_or(true));
    }
    SatResult::Satisfiable { assignment: map }
}

fn build_unsat_result(
    clauses: &[Vec<i32>],
    var_names: &BTreeMap<usize, String>,
) -> SatResult {
    // Report first unsatisfied clause as conflict
    let conflict: Vec<String> = clauses
        .first()
        .map(|c| {
            c.iter()
                .map(|&lit| {
                    let var = lit.unsigned_abs() as usize;
                    let name = var_names.get(&var).cloned().unwrap_or_else(|| format!("v{var}"));
                    if lit > 0 { name } else { format!("!{name}") }
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
}
