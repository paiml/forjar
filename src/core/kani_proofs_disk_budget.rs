//! FJ-036: Kani bounded-model proofs for the `disk_budget` resource.
//!
//! Gated behind `#[cfg(kani)]`; normal builds ignore them.
//! Run with: `cargo kani --harness proof_disk_budget_hysteresis_total`.
//!
//! These prove the watermark algebra exhaustively over the whole u8 x u8 input
//! space, which is the part of this resource a unit test can only sample. The
//! deletion behaviour is proved by execution instead, in
//! `tests/falsification_disk_budget.rs` — a bounded model cannot say anything
//! useful about `rm -rf` on a real filesystem.

// ── Disk Budget Proofs (FJ-036 / disk-budget-v1) ──────────────────────

/// Contract `watermark_hysteresis`: no accepted budget can thrash.
///
/// For EVERY (high, target_free) pair in the u8 x u8 space, `DiskBudget::new`
/// either rejects it or returns a budget whose reclaim target sits strictly
/// below its trigger. A pass that satisfies its target therefore always clears
/// the alarm, so the reaper cannot re-trigger on the next tick forever.
#[cfg(kani)]
#[kani::proof]
fn proof_disk_budget_hysteresis_total() {
    use super::types::DiskBudget;

    let high: u8 = kani::any();
    let target_free: u8 = kani::any();

    if let Ok(b) = DiskBudget::new("/", high, target_free, 50, "hourly", vec![]) {
        // Accepted => strict hysteresis holds.
        assert!(b.target_used_pct() < b.high_watermark_pct);
        // ...and the accepted ranges are the documented ones.
        assert!(b.high_watermark_pct >= 1 && b.high_watermark_pct <= 99);
        assert!(b.target_free_pct >= 1 && b.target_free_pct <= 99);
    }
}

/// Contract `watermark_hysteresis`: `target_used_pct` never underflows.
///
/// `100 - target_free_pct` is computed on u8. Any accepted budget must keep
/// `target_free_pct` in 1..=99, so the subtraction is total. An accepted
/// budget with target_free_pct > 100 would panic in release-with-overflow
/// checks and silently wrap otherwise.
#[cfg(kani)]
#[kani::proof]
fn proof_disk_budget_target_used_no_underflow() {
    use super::types::DiskBudget;

    let high: u8 = kani::any();
    let target_free: u8 = kani::any();

    if let Ok(b) = DiskBudget::new("/", high, target_free, 50, "hourly", vec![]) {
        assert!(b.target_free_pct <= 100);
        let used = b.target_used_pct();
        assert!(used <= 99);
    }
}

/// Contract `watermark_hysteresis`: the shipped defaults are themselves valid.
///
/// A default pair that violated hysteresis would ship a thrashing reaper to
/// every machine that omits the tuning knobs — the common case.
#[cfg(kani)]
#[kani::proof]
fn proof_disk_budget_defaults_are_accepted() {
    use super::types::{
        DiskBudget, DEFAULT_CRITICAL_FREE_GB, DEFAULT_HIGH_WATERMARK_PCT, DEFAULT_SCHEDULE,
        DEFAULT_TARGET_FREE_PCT,
    };

    let b = DiskBudget::new(
        "/",
        DEFAULT_HIGH_WATERMARK_PCT,
        DEFAULT_TARGET_FREE_PCT,
        DEFAULT_CRITICAL_FREE_GB,
        DEFAULT_SCHEDULE,
        vec![],
    );
    assert!(b.is_ok());
    assert!(b.unwrap().target_used_pct() < DEFAULT_HIGH_WATERMARK_PCT);
}
