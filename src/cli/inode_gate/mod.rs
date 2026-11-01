//! forjar#847: the inode-pressure gate shared by drift, validate and apply.

use crate::core::inode_pressure::{evaluate, host_df, parse_df_i, InodeFinding};
use crate::core::types::ForjarConfig;

/// Judge `df -i` output against the config's policy (defaults when absent).
pub(crate) fn judge(cfg: &ForjarConfig, df: &str) -> Result<Vec<InodeFinding>, String> {
    let policy = cfg.inode_pressure.clone().unwrap_or_default();
    evaluate(&policy, &parse_df_i(df))
}

fn findings(cfg: &ForjarConfig, df: Result<String, String>) -> Result<Vec<InodeFinding>, String> {
    judge(cfg, &df?)
}

/// drift: number of findings (printed), from a `df` result. A `df` that cannot
/// be read is NOT MEASURED and fails closed.
pub(crate) fn drift_count(
    cfg: &ForjarConfig,
    df: Result<String, String>,
    json: bool,
) -> Result<usize, String> {
    let found = findings(cfg, df)?;
    for f in &found {
        if json {
            println!(
                "{}",
                serde_json::json!({"inode_pressure": {
                    "mount": f.mount, "iuse_pct": f.iuse_pct, "threshold_pct": f.threshold_pct}})
            );
        } else {
            println!("  DRIFT inode_pressure: {f}");
        }
    }
    Ok(found.len())
}

/// validate / apply pre-check: `Err` on any finding, or on an unmeasurable
/// host; with `warn_only` (a dry run) a finding is printed, not refused.
pub(crate) fn pre_check(
    cfg: &ForjarConfig,
    df: Result<String, String>,
    stage: &str,
    warn_only: bool,
) -> Result<(), String> {
    let found = findings(cfg, df)?;
    if found.is_empty() {
        return Ok(());
    }
    let list: Vec<String> = found.iter().map(ToString::to_string).collect();
    let msg = format!("{stage}: {}", list.join("; "));
    if warn_only {
        eprintln!("warning: {msg}");
        return Ok(());
    }
    Err(format!(
        "{msg}. A filesystem out of inodes fails writes (ENOSPC) with free blocks left; \
         clear it (a `tmp_reaper` resource does) or raise `inode_pressure` thresholds."
    ))
}

/// The real host: drift.
pub(crate) fn host_drift_count(cfg: &ForjarConfig, json: bool) -> Result<usize, String> {
    drift_count(cfg, host_df(), json)
}

/// The real host: validate and apply pre-checks.
pub(crate) fn host_pre_check(
    cfg: &ForjarConfig,
    stage: &str,
    warn_only: bool,
) -> Result<(), String> {
    pre_check(cfg, host_df(), stage, warn_only)
}

#[cfg(test)]
mod tests;
