//! FJ-036: Disk budget types — watermarks and reclaim rules.
//!
//! A `disk_budget` resource declares, per filesystem, how much free space the
//! machine is required to keep and what forjar is allowed to delete to get it
//! back. The design follows the kubelet's image/container GC: a **high
//! watermark** triggers a reclaim pass, and the pass runs until a **low
//! watermark** (expressed here as `target_free_pct`) is restored — not until a
//! fixed age rule has been applied once.
//!
//! That distinction is the whole point. The predecessor of this resource was a
//! shell reaper with a fixed 7-day idle TTL and no notion of free space. It ran
//! daily on lambda-labs, on schedule, exiting 0, through two separate
//! 100%-full events — reclaiming 1.6G in a month while `/` slid from 370G free
//! to 1.2G. Every candidate it saw was younger than its TTL, so "keep (recent
//! build)" was the correct answer to the wrong question. A budget that cannot
//! observe the resource it is budgeting cannot defend it.

use serde::{Deserialize, Serialize};

/// Default used-% at or above which a reclaim pass is triggered.
pub const DEFAULT_HIGH_WATERMARK_PCT: u8 = 85;
/// Default free-% a reclaim pass must restore before it stops.
pub const DEFAULT_TARGET_FREE_PCT: u8 = 20;
/// Default free-GiB below which the budget is CRITICAL (hard drift failure).
pub const DEFAULT_CRITICAL_FREE_GB: u64 = 50;
/// Default reaper cadence. Hourly, not daily: a box that can burn 250G/day
/// cannot be defended by a once-a-day pass.
pub const DEFAULT_SCHEDULE: &str = "hourly";

/// What a reclaim rule looks for under its roots.
///
/// Detection is deliberately **behavioural, never name-based**. The shell
/// predecessor matched `target|target-local|target-private` by name and so was
/// blind to `.target`, which is where 189G of abandoned agent-worktree build
/// output was sitting. Name lists also cut the other way: the `cc` crate ships
/// a real *source* directory at `src/target/`, and a by-name sweep of the cargo
/// registry deletes it, leaving `.cargo-ok` in place so cargo never notices
/// until a much later build fails with an inscrutable `could not compile cc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ReclaimKind {
    /// A cargo build directory, identified by the markers cargo itself writes
    /// (`CACHEDIR.TAG` + `.rustc_info.json`). Name-independent by construction.
    #[default]
    CargoTarget,
    /// A Claude Code agent scratchpad (`<root>/<project>/<session>/scratchpad`).
    /// Never reclaimed while its session is live.
    ClaudeScratchpad,
    /// An abandoned git worktree: fully merged/pushed, clean tree, no live
    /// process inside it. Removed whole, then `git worktree prune`d.
    AbandonedWorktree,
    /// Literal paths matched by glob — leaked test fixtures and similar.
    Glob,
}

impl std::fmt::Display for ReclaimKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CargoTarget => write!(f, "cargo_target"),
            Self::ClaudeScratchpad => write!(f, "claude_scratchpad"),
            Self::AbandonedWorktree => write!(f, "abandoned_worktree"),
            Self::Glob => write!(f, "glob"),
        }
    }
}

/// One reclaim rule. Rules are applied in declaration order, most-disposable
/// first, and each stops early once the target watermark is reached.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ReclaimRule {
    /// Rule name, surfaced in the reaper log and the reclaim ledger.
    pub name: String,
    /// Absolute roots to scan.
    #[serde(default)]
    pub roots: Vec<String>,
    /// What to detect under `roots`.
    #[serde(default)]
    pub kind: ReclaimKind,
    /// Minimum minutes since last modification before a candidate is eligible.
    ///
    /// This is a floor that protects in-flight work, NOT the reclaim policy —
    /// the policy is the watermark. Keep it small (an hour, not a week): its
    /// only job is to avoid deleting a build that is running right now.
    #[serde(default = "default_min_idle_minutes")]
    pub min_idle_minutes: u64,
}

const fn default_min_idle_minutes() -> u64 {
    60
}

/// Resolved, validated budget for one filesystem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiskBudget {
    /// Filesystem mount path being budgeted.
    pub path: String,
    /// Used-% at or above which a reclaim pass runs.
    pub high_watermark_pct: u8,
    /// Free-% a reclaim pass must restore before stopping.
    pub target_free_pct: u8,
    /// Free-GiB below which the budget is CRITICAL.
    pub critical_free_gb: u64,
    /// systemd `OnCalendar` cadence for the reaper.
    pub schedule: String,
    /// Ordered reclaim rules.
    pub reclaim: Vec<ReclaimRule>,
}

impl DiskBudget {
    /// Used-% the reclaim pass must get at or below to satisfy the target.
    ///
    /// ```
    /// use forjar::core::types::DiskBudget;
    /// let b = DiskBudget::new("/", 85, 20, 50, "hourly", vec![]).unwrap();
    /// assert_eq!(b.target_used_pct(), 80);
    /// ```
    pub const fn target_used_pct(&self) -> u8 {
        100 - self.target_free_pct
    }

    /// Build a budget, rejecting incoherent watermark pairs.
    ///
    /// # Errors
    ///
    /// Returns `Err` when a percentage is out of range, or when the pair lacks
    /// hysteresis — see [`Self::validate_hysteresis`].
    pub fn new(
        path: &str,
        high_watermark_pct: u8,
        target_free_pct: u8,
        critical_free_gb: u64,
        schedule: &str,
        reclaim: Vec<ReclaimRule>,
    ) -> Result<Self, String> {
        validate_pct("high_watermark_pct", high_watermark_pct)?;
        validate_pct("target_free_pct", target_free_pct)?;
        Self::validate_hysteresis(high_watermark_pct, target_free_pct)?;
        Ok(Self {
            path: path.to_string(),
            high_watermark_pct,
            target_free_pct,
            critical_free_gb,
            schedule: schedule.to_string(),
            reclaim,
        })
    }

    /// The reclaim target must sit strictly BELOW the trigger.
    ///
    /// If a pass stops while still at or above the high watermark, the next run
    /// re-triggers immediately and the reaper thrashes — deleting on every tick
    /// while never clearing the alarm. This is the kubelet's
    /// `imageGCLowThreshold < imageGCHighThreshold` rule, and it is the reason
    /// the two numbers cannot be collapsed into one.
    ///
    /// # Errors
    ///
    /// Returns `Err` when `100 - target_free_pct >= high_watermark_pct`.
    pub fn validate_hysteresis(high_watermark_pct: u8, target_free_pct: u8) -> Result<(), String> {
        if Self::hysteresis_holds(high_watermark_pct, target_free_pct) {
            return Ok(());
        }
        let target_used = 100u8.saturating_sub(target_free_pct);
        Err(format!(
            "disk_budget hysteresis violated: reclaiming to {target_free_pct}% free leaves \
             used at {target_used}%, which is still at or above the {high_watermark_pct}% \
             trigger — every pass would immediately re-trigger. Require \
             100 - target_free_pct < high_watermark_pct."
        ))
    }

    /// The hysteresis rule itself: allocation-free, so a proof can reach it.
    ///
    /// GH-242. `proof_disk_budget_hysteresis_total` had already been retargeted
    /// away from `DiskBudget::new` at this predicate and STILL cost **48 GB of
    /// RSS at 48 minutes** — to prove integer algebra over 65,536 points. The
    /// remaining cost was `format!` on the error path of `validate_hysteresis`:
    /// CBMC models every path, not merely the one the property asserts on, so
    /// `core::fmt` and a `String` allocation entered the model regardless.
    ///
    /// Splitting the decision from the message is the same fix
    /// `classify_remote` got in `backup_sync_types`, and it is now the rule for
    /// any validator a harness drives: return the verdict, render the text in
    /// the caller.
    #[must_use]
    pub fn hysteresis_holds(high_watermark_pct: u8, target_free_pct: u8) -> bool {
        100u8.saturating_sub(target_free_pct) < high_watermark_pct
    }
}

fn validate_pct(field: &str, v: u8) -> Result<(), String> {
    if v == 0 || v >= 100 {
        return Err(format!("disk_budget {field} must be in 1..=99, got {v}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_coherent() {
        // The shipped defaults must themselves satisfy hysteresis.
        assert!(DiskBudget::validate_hysteresis(
            DEFAULT_HIGH_WATERMARK_PCT,
            DEFAULT_TARGET_FREE_PCT
        )
        .is_ok());
    }

    #[test]
    fn rejects_missing_hysteresis() {
        // 10% free => 90% used, which is above the 85% trigger: thrash.
        let err = DiskBudget::validate_hysteresis(85, 10).unwrap_err();
        assert!(err.contains("hysteresis violated"), "{err}");
    }

    #[test]
    fn rejects_exact_equality() {
        // 15% free => 85% used == trigger. Still thrash: must be strict.
        assert!(DiskBudget::validate_hysteresis(85, 15).is_err());
    }

    #[test]
    fn accepts_one_point_of_margin() {
        assert!(DiskBudget::validate_hysteresis(85, 16).is_ok());
    }

    #[test]
    fn rejects_out_of_range_pct() {
        assert!(DiskBudget::new("/", 0, 20, 50, "hourly", vec![]).is_err());
        assert!(DiskBudget::new("/", 100, 20, 50, "hourly", vec![]).is_err());
        assert!(DiskBudget::new("/", 85, 0, 50, "hourly", vec![]).is_err());
        assert!(DiskBudget::new("/", 85, 100, 50, "hourly", vec![]).is_err());
    }

    #[test]
    fn target_used_pct_is_complement() {
        let b = DiskBudget::new("/", 90, 25, 50, "hourly", vec![]).unwrap();
        assert_eq!(b.target_used_pct(), 75);
    }

    #[test]
    fn reclaim_kind_display_is_snake_case() {
        assert_eq!(ReclaimKind::CargoTarget.to_string(), "cargo_target");
        assert_eq!(
            ReclaimKind::ClaudeScratchpad.to_string(),
            "claude_scratchpad"
        );
        assert_eq!(
            ReclaimKind::AbandonedWorktree.to_string(),
            "abandoned_worktree"
        );
        assert_eq!(ReclaimKind::Glob.to_string(), "glob");
    }

    #[test]
    fn reclaim_kind_default_is_cargo_target() {
        assert_eq!(ReclaimKind::default(), ReclaimKind::CargoTarget);
    }

    #[test]
    fn rule_deserializes_with_default_idle_floor() {
        let r: ReclaimRule =
            serde_yaml_ng::from_str("name: agent-targets\nroots: ['/home/x/src']\n").unwrap();
        assert_eq!(r.min_idle_minutes, 60);
        assert_eq!(r.kind, ReclaimKind::CargoTarget);
    }

    #[test]
    fn rule_roundtrips_through_yaml() {
        let r = ReclaimRule {
            name: "scratch".into(),
            roots: vec!["/tmp/claude-1000".into()],
            kind: ReclaimKind::ClaudeScratchpad,
            min_idle_minutes: 120,
        };
        let s = serde_yaml_ng::to_string(&r).unwrap();
        let back: ReclaimRule = serde_yaml_ng::from_str(&s).unwrap();
        assert_eq!(r, back);
    }
}
