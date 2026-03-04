//! FJ-051: MC/DC (Modified Condition/Decision Coverage) analysis.
//!
//! Generates MC/DC test requirements for boolean decisions in resource handlers.
//! MC/DC requires that each condition independently affects the decision outcome.
//! Used for DO-178C DAL-A structural coverage in safety-critical paths.

/// A boolean condition in a decision.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Condition {
    pub name: String,
    pub index: usize,
}

/// A decision (boolean expression) composed of conditions.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Decision {
    pub name: String,
    pub conditions: Vec<Condition>,
}

/// An MC/DC test pair: two test cases where one condition differs
/// and the decision outcome changes.
#[derive(Debug, Clone, serde::Serialize)]
pub struct McdcPair {
    pub condition: String,
    pub true_case: Vec<bool>,
    pub false_case: Vec<bool>,
}

/// MC/DC coverage report.
#[derive(Debug, serde::Serialize)]
pub struct McdcReport {
    pub decision: String,
    pub num_conditions: usize,
    pub pairs: Vec<McdcPair>,
    pub min_tests_needed: usize,
    pub coverage_achievable: bool,
}

/// Evaluate a conjunction (AND of all conditions).
fn eval_and(values: &[bool]) -> bool {
    values.iter().all(|&v| v)
}

/// Generate MC/DC test pairs for an AND decision.
/// For AND(c1, c2, ..., cn), each condition ci needs a pair where:
/// - ci differs between the two cases
/// - all other conditions are true (to isolate ci's effect)
pub fn generate_mcdc_and(decision: &Decision) -> McdcReport {
    let n = decision.conditions.len();
    let mut pairs = Vec::new();

    for i in 0..n {
        let true_case = vec![true; n];
        let mut false_case = vec![true; n];
        false_case[i] = false;

        // Verify the pair: true_case should evaluate to true,
        // false_case should evaluate to false
        if eval_and(&true_case) != eval_and(&false_case) {
            pairs.push(McdcPair {
                condition: decision.conditions[i].name.clone(),
                true_case,
                false_case,
            });
        }
    }

    McdcReport {
        decision: decision.name.clone(),
        num_conditions: n,
        min_tests_needed: n + 1,
        coverage_achievable: pairs.len() == n,
        pairs,
    }
}

/// Evaluate a disjunction (OR of all conditions).
fn eval_or(values: &[bool]) -> bool {
    values.iter().any(|&v| v)
}

/// Generate MC/DC test pairs for an OR decision.
pub fn generate_mcdc_or(decision: &Decision) -> McdcReport {
    let n = decision.conditions.len();
    let mut pairs = Vec::new();

    for i in 0..n {
        let mut true_case = vec![false; n];
        true_case[i] = true;
        let false_case = vec![false; n];

        if eval_or(&true_case) != eval_or(&false_case) {
            pairs.push(McdcPair {
                condition: decision.conditions[i].name.clone(),
                true_case,
                false_case,
            });
        }
    }

    McdcReport {
        decision: decision.name.clone(),
        num_conditions: n,
        min_tests_needed: n + 1,
        coverage_achievable: pairs.len() == n,
        pairs,
    }
}

/// Build a decision from condition names.
pub fn build_decision(name: &str, conditions: &[&str]) -> Decision {
    Decision {
        name: name.to_string(),
        conditions: conditions
            .iter()
            .enumerate()
            .map(|(i, &c)| Condition {
                name: c.to_string(),
                index: i,
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcdc_and_two_conditions() {
        let d = build_decision("a && b", &["a", "b"]);
        let report = generate_mcdc_and(&d);
        assert_eq!(report.pairs.len(), 2);
        assert_eq!(report.min_tests_needed, 3);
        assert!(report.coverage_achievable);
    }

    #[test]
    fn test_mcdc_and_three_conditions() {
        let d = build_decision("a && b && c", &["a", "b", "c"]);
        let report = generate_mcdc_and(&d);
        assert_eq!(report.pairs.len(), 3);
        assert_eq!(report.min_tests_needed, 4);
    }

    #[test]
    fn test_mcdc_or_two_conditions() {
        let d = build_decision("a || b", &["a", "b"]);
        let report = generate_mcdc_or(&d);
        assert_eq!(report.pairs.len(), 2);
        assert!(report.coverage_achievable);
    }

    #[test]
    fn test_mcdc_single_condition() {
        let d = build_decision("a", &["a"]);
        let report = generate_mcdc_and(&d);
        assert_eq!(report.pairs.len(), 1);
        assert_eq!(report.min_tests_needed, 2);
    }

    #[test]
    fn test_mcdc_report_serde() {
        let d = build_decision("x && y", &["x", "y"]);
        let report = generate_mcdc_and(&d);
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("\"coverage_achievable\":true"));
    }

    #[test]
    fn test_build_decision() {
        let d = build_decision("test", &["c1", "c2"]);
        assert_eq!(d.name, "test");
        assert_eq!(d.conditions.len(), 2);
        assert_eq!(d.conditions[0].index, 0);
    }

    #[test]
    fn test_eval_and() {
        assert!(eval_and(&[true, true]));
        assert!(!eval_and(&[true, false]));
        assert!(!eval_and(&[false, true]));
    }

    #[test]
    fn test_eval_or() {
        assert!(eval_or(&[true, false]));
        assert!(eval_or(&[false, true]));
        assert!(!eval_or(&[false, false]));
    }
}
