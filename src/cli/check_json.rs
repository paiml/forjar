//! `forjar check --json` report (split out of check.rs for the 500-line ceiling).

/// Format check results as JSON.
pub(super) fn format_check_json(
    config_name: &str,
    results: &[super::check::CheckResult],
    total_pass: usize,
    total_fail: usize,
    total_skip: usize,
) -> Result<(), String> {
    let json_results: Vec<_> = results
        .iter()
        .map(|r| {
            let mut obj = serde_json::json!({
                "resource": r.resource_id,
                "machine": r.machine,
                "status": r.status,
            });
            if let Some(code) = r.exit_code {
                obj["exit_code"] = serde_json::json!(code);
            }
            if let Some(in_state) = r.in_state {
                obj["in_state"] = serde_json::json!(in_state);
            }
            if !r.detail.is_empty() {
                let key = match r.status.as_str() {
                    "error" => "error",
                    "unmeasured" => "detail",
                    _ => "stderr",
                };
                obj[key] = serde_json::json!(r.detail);
            }
            obj
        })
        .collect();
    let unmeasured = results.iter().filter(|r| r.status == "unmeasured").count();
    let report = serde_json::json!({
        "name": config_name,
        "all_passed": total_fail == 0 && unmeasured == 0,
        "unmeasured": unmeasured,
        "total": total_pass + total_fail + total_skip + unmeasured,
        "pass": total_pass,
        "fail": total_fail,
        "skip": total_skip,
        "results": json_results,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&report).map_err(|e| format!("JSON error: {e}"))?
    );
    Ok(())
}
