//! forjar#847: inode-pressure check — a filesystem can be out of inodes while it
//! still has free blocks.

use serde::{Deserialize, Serialize};

/// Default red line: inode use at or above this percentage is a finding.
pub const DEFAULT_THRESHOLD_PCT: u8 = 90;

/// The inode probe run on every target (forjar#995). GNU `df -i -P -T` prints
/// a type column; BSD/macOS `df` reads `-T` as a type FILTER that needs an
/// argument (exit 64), and its `-P` drops the inode columns, so BSD gets plain
/// `df -i` (blocks, then iused / ifree / %iused, no type).
pub const DF_INODES_CMD: &str = "case \"$(uname -s)\" in Darwin|FreeBSD|OpenBSD|NetBSD|DragonFly) df -i ;; *) df -i -P -T ;; esac";

/// Declared inode-pressure policy (`inode_pressure:` in the config).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InodePressure {
    /// Red line for every watched mount without its own override.
    #[serde(default = "default_threshold")]
    pub threshold_pct: u8,
    /// Watched mounts. Empty means `/` and every tmpfs.
    #[serde(default)]
    pub mounts: Vec<WatchedMount>,
}

fn default_threshold() -> u8 {
    DEFAULT_THRESHOLD_PCT
}

impl Default for InodePressure {
    fn default() -> Self {
        Self {
            threshold_pct: DEFAULT_THRESHOLD_PCT,
            mounts: Vec::new(),
        }
    }
}

/// One explicitly watched mount, with an optional per-mount threshold.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatchedMount {
    /// Mount point as `df` prints it.
    pub path: String,
    /// Overrides the policy threshold for this mount.
    #[serde(default)]
    pub threshold_pct: Option<u8>,
}

/// One row of [`DF_INODES_CMD`] output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InodeReading {
    /// Mount point.
    pub mount: String,
    /// Filesystem type (empty on BSD, whose `df -i` prints none).
    pub fstype: String,
    /// `IUse%`, or `None` where the filesystem has no inode accounting (`-`).
    pub iuse_pct: Option<u8>,
}

/// A watched mount at or above its threshold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InodeFinding {
    /// Mount point.
    pub mount: String,
    /// Measured `IUse%`.
    pub iuse_pct: u8,
    /// The red line it crossed.
    pub threshold_pct: u8,
}

impl std::fmt::Display for InodeFinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "inode use on {} is {}% (red at {}% or more)",
            self.mount, self.iuse_pct, self.threshold_pct
        )
    }
}

/// Parse the output of [`DF_INODES_CMD`]: GNU `df -i -P -T` (fs, type, inodes,
/// used, free, IUse%, mount) or BSD `df -i` (fs, blocks, used, avail,
/// capacity, iused, ifree, %iused, mount), told apart by the BSD `%iused`
/// header.
pub fn parse_df_i(out: &str) -> Vec<InodeReading> {
    let mut lines = out.lines();
    let bsd = lines.next().is_some_and(|h| h.contains("%iused"));
    lines
        .filter_map(|line| {
            let f: Vec<&str> = line.split_whitespace().collect();
            if bsd {
                bsd_row(&f)
            } else {
                gnu_row(&f)
            }
        })
        .collect()
}

fn pct(s: &str) -> Option<u8> {
    s.strip_suffix('%').and_then(|n| n.parse().ok())
}

fn gnu_row(f: &[&str]) -> Option<InodeReading> {
    (f.len() >= 7).then(|| InodeReading {
        mount: f[6..].join(" "),
        fstype: f[1].to_string(),
        iuse_pct: pct(f[5]),
    })
}

/// A BSD row. The filesystem name can hold spaces (`map auto_home`), so the
/// columns are found from the first `Capacity` cell: three numbers then `N%`.
fn bsd_row(f: &[&str]) -> Option<InodeReading> {
    let num = |s: &str| s.parse::<u64>().is_ok();
    let cap = (4..f.len())
        .find(|&k| f[k].ends_with('%') && num(f[k - 1]) && num(f[k - 2]) && num(f[k - 3]))?;
    (f.len() > cap + 4).then(|| InodeReading {
        mount: f[cap + 4..].join(" "),
        fstype: String::new(),
        iuse_pct: pct(f[cap + 3]),
    })
}

fn crossing(r: &InodeReading, pct: u8, threshold: u8) -> Option<InodeFinding> {
    (pct >= threshold).then(|| InodeFinding {
        mount: r.mount.clone(),
        iuse_pct: pct,
        threshold_pct: threshold,
    })
}

/// Judge readings against the policy.
///
/// # Errors
///
/// A watched mount with no reading is NOT MEASURED, never a pass.
pub fn evaluate(
    cfg: &InodePressure,
    readings: &[InodeReading],
) -> Result<Vec<InodeFinding>, String> {
    if cfg.mounts.is_empty() {
        if !readings.iter().any(|r| r.mount == "/") {
            return Err("not measured: `df -i` listed no `/` row".to_string());
        }
        return Ok(readings
            .iter()
            .filter(|r| r.mount == "/" || r.fstype == "tmpfs")
            .filter_map(|r| crossing(r, r.iuse_pct?, cfg.threshold_pct))
            .collect());
    }
    let mut out = Vec::new();
    for w in &cfg.mounts {
        let r = readings
            .iter()
            .find(|r| r.mount == w.path)
            .ok_or_else(|| format!("not measured: mount {} is not in `df -i` output", w.path))?;
        // A `-` (btrfs and friends) is reported by `unmeasured_mounts`, not red.
        let Some(pct) = r.iuse_pct else { continue };
        out.extend(crossing(
            r,
            pct,
            w.threshold_pct.unwrap_or(cfg.threshold_pct),
        ));
    }
    Ok(out)
}

/// Watched mounts whose `IUse%` is `-` (btrfs and friends: no inode table to
/// count). They are NOT MEASURED: a warning, never red, never a pass.
pub fn unmeasured_mounts(cfg: &InodePressure, readings: &[InodeReading]) -> Vec<String> {
    let watched = |r: &InodeReading| {
        if cfg.mounts.is_empty() {
            r.mount == "/" || r.fstype == "tmpfs"
        } else {
            cfg.mounts.iter().any(|w| w.path == r.mount)
        }
    };
    readings
        .iter()
        .filter(|r| r.iuse_pct.is_none() && watched(r))
        .map(|r| r.mount.clone())
        .collect()
}

/// Config-time checks: thresholds in 1..=100, mounts absolute.
pub fn validate_config(cfg: &InodePressure) -> Vec<String> {
    let mut errs = Vec::new();
    let ok = |t: u8| (1..=100).contains(&t);
    if !ok(cfg.threshold_pct) {
        errs.push(format!(
            "inode_pressure.threshold_pct {} must be 1..=100",
            cfg.threshold_pct
        ));
    }
    for m in &cfg.mounts {
        if !m.path.starts_with('/') {
            errs.push(format!("inode_pressure mount '{}' is not absolute", m.path));
        }
        if m.threshold_pct.is_some_and(|t| !ok(t)) {
            errs.push(format!(
                "inode_pressure mount '{}' threshold_pct must be 1..=100",
                m.path
            ));
        }
    }
    errs
}

/// Run [`DF_INODES_CMD`] on this host.
///
/// # Errors
///
/// A `df` that cannot run or exits non-zero is NOT MEASURED.
pub fn host_df() -> Result<String, String> {
    let out = std::process::Command::new("sh")
        .args(["-c", DF_INODES_CMD])
        .output()
        .map_err(|e| format!("not measured: cannot run df: {e}"))?;
    // GNU df exits 1 when it could not stat one mount but still prints the rest.
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    if text.lines().count() < 2 {
        return Err(format!(
            "not measured: df -i printed no rows ({})",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(text)
}

#[cfg(test)]
mod tests;
#[cfg(test)]
pub(crate) mod tests_bsd;
