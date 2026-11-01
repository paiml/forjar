//! forjar#847: the inode-pressure gate shared by drift, validate and apply.

use crate::core::inode_pressure::{evaluate, host_df, parse_df_i, unmeasured_mounts, InodeFinding};
use crate::core::types::{ForjarConfig, Machine};

#[cfg(test)]
/// Judge `df -i` output against the config's policy (defaults when absent).
pub(crate) fn judge(cfg: &ForjarConfig, df: &str) -> Result<Vec<InodeFinding>, String> {
    let policy = cfg.inode_pressure.clone().unwrap_or_default();
    evaluate(&policy, &parse_df_i(df))
}

/// One machine and what `df -i` said there.
pub(crate) type Target = (String, Result<String, String>);

/// What the gate saw across all targets.
#[derive(Debug, Default)]
pub(crate) struct Report {
    pub findings: Vec<(String, InodeFinding)>,
    pub warnings: Vec<String>,
}

/// Judge every target. A target whose `df` could not be read is NOT MEASURED
/// and fails the whole assessment, naming the machine; a `-` row is a warning.
pub(crate) fn assess(cfg: &ForjarConfig, targets: &[Target]) -> Result<Report, String> {
    let policy = cfg.inode_pressure.clone().unwrap_or_default();
    let mut report = Report::default();
    for (machine, df) in targets {
        let text = df
            .as_ref()
            .map_err(|e| format!("not measured on {machine}: {e}"))?;
        let readings = parse_df_i(text);
        let found = evaluate(&policy, &readings).map_err(|e| format!("{machine}: {e}"))?;
        report
            .findings
            .extend(found.into_iter().map(|f| (machine.clone(), f)));
        report
            .warnings
            .extend(unmeasured_mounts(&policy, &readings).into_iter().map(|m| {
                format!("not measured: {machine}:{m} reports no inode accounting (IUse% is `-`)")
            }));
    }
    Ok(report)
}

/// `df -i -P -T` on a machine, through ITS transport (local, ssh, container,
/// pepita) — the controller's own inodes say nothing about a remote target.
fn measure(machine: &Machine) -> Result<String, String> {
    let out = crate::transport::query(machine, "df -i -P -T")
        .map_err(|e| format!("not measured: cannot run df: {e}"))?;
    // GNU df exits 1 when it could not stat one mount but still prints the rest.
    if out.stdout.lines().count() < 2 {
        return Err(format!(
            "not measured: df -i printed no rows (exit {}: {})",
            out.exit_code,
            out.stderr.trim()
        ));
    }
    Ok(out.stdout)
}

/// One target per machine in scope. `local_only` (validate, which is offline by
/// nature) keeps only machines the controller itself answers for; a config
/// with no machines at all is measured on the controller.
pub(crate) fn collect(cfg: &ForjarConfig, scope: Option<&str>, local_only: bool) -> Vec<Target> {
    if cfg.machines.is_empty() {
        return vec![("localhost".to_string(), host_df())];
    }
    let mut names: Vec<&String> = cfg
        .machines
        .keys()
        .filter(|m| scope.is_none_or(|s| s == m.as_str()))
        .collect();
    names.sort();
    names
        .into_iter()
        .map(|n| (n, &cfg.machines[n]))
        .filter(|(_, m)| !local_only || crate::transport::controller_answers_for(m))
        .map(|(n, m)| (n.clone(), measure(m)))
        .collect()
}

fn warn(report: &Report) {
    for w in &report.warnings {
        eprintln!("warning: {w}");
    }
}

/// drift: number of findings (printed). A machine whose `df` cannot be read
/// is NOT MEASURED and fails closed.
pub(crate) fn drift_count(
    cfg: &ForjarConfig,
    targets: &[Target],
    json: bool,
) -> Result<usize, String> {
    let report = assess(cfg, targets)?;
    warn(&report);
    for (machine, f) in &report.findings {
        if json {
            println!(
                "{}",
                serde_json::json!({"inode_pressure": {"machine": machine,
                    "mount": f.mount, "iuse_pct": f.iuse_pct, "threshold_pct": f.threshold_pct}})
            );
        } else {
            println!("  DRIFT inode_pressure: {machine}: {f}");
        }
    }
    Ok(report.findings.len())
}

/// validate / apply pre-check: `Err` on any finding, or on a machine that
/// cannot be measured. With `warn_only` (a dry run) both are printed, not
/// refused: a preview must survive a host it cannot reach.
pub(crate) fn pre_check(
    cfg: &ForjarConfig,
    targets: &[Target],
    stage: &str,
    warn_only: bool,
) -> Result<(), String> {
    let report = match assess(cfg, targets) {
        Ok(r) => r,
        Err(e) if warn_only => {
            eprintln!("warning: {stage}: {e}");
            return Ok(());
        }
        Err(e) => return Err(format!("{stage}: {e}")),
    };
    warn(&report);
    if report.findings.is_empty() {
        return Ok(());
    }
    let list: Vec<String> = report
        .findings
        .iter()
        .map(|(m, f)| format!("{m}: {f}"))
        .collect();
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

/// drift against the real machines in `scope`.
pub(crate) fn machines_drift_count(
    cfg: &ForjarConfig,
    scope: Option<&str>,
    json: bool,
) -> Result<usize, String> {
    drift_count(cfg, &collect(cfg, scope, false), json)
}

/// apply pre-check against the real machines in `scope`.
pub(crate) fn machines_pre_check(
    cfg: &ForjarConfig,
    scope: Option<&str>,
    warn_only: bool,
) -> Result<(), String> {
    pre_check(cfg, &collect(cfg, scope, false), "apply", warn_only)
}

/// validate pre-check: only machines the controller answers for (validate is
/// offline; remote machines are measured by drift and apply).
pub(crate) fn local_pre_check(cfg: &ForjarConfig) -> Result<(), String> {
    pre_check(cfg, &collect(cfg, None, true), "validate", false)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod tests_machines;
