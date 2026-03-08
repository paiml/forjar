//! CLI handler for `forjar score` — recipe quality grading.

use crate::core::scoring;
use std::path::Path;

/// Execute the `forjar score` command.
pub(crate) fn cmd_score(
    file: &Path,
    status: &str,
    idempotency: &str,
    budget_ms: u64,
    json: bool,
) -> Result<(), String> {
    let input = scoring::ScoringInput {
        status: status.to_string(),
        idempotency: idempotency.to_string(),
        budget_ms,
        runtime: None,
        raw_yaml: None, // compute_from_file reads the file
    };

    let result = scoring::compute_from_file(file, &input)?;

    if json {
        let dims: Vec<String> = result
            .dimensions
            .iter()
            .map(|d| {
                format!(
                    "{{\"code\":\"{}\",\"name\":\"{}\",\"score\":{},\"weight\":{}}}",
                    d.code, d.name, d.score, d.weight
                )
            })
            .collect();
        println!(
            "{{\"composite\":{},\"grade\":\"{}\",\"static_grade\":\"{}\",\"runtime_grade\":{},\"hard_fail\":{},\"dimensions\":[{}]}}",
            result.composite,
            result.grade,
            result.static_grade,
            result.runtime_grade.map_or("null".to_string(), |g| format!("\"{g}\"")),
            result.hard_fail,
            dims.join(","),
        );
    } else {
        print!("{}", scoring::format_score_report(&result));
    }

    // Exit 0 for A-C static grade, exit 1 for D-F
    if result.static_grade == 'D' || result.static_grade == 'F' {
        Err(format!("grade {} — below threshold", result.grade))
    } else {
        Ok(())
    }
}
