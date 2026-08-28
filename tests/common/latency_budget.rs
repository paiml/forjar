//! A latency bound that accounts for coverage instrumentation.
//!
//! Extracted from falsification_competitive_features.rs so the file stays under
//! the repo's 500-line ceiling, and so any other test asserting a wall-clock
//! budget can reach for the same rule rather than inventing its own.

#![allow(dead_code)]

/// A latency bound, widened when the binary is instrumented.
///
/// `cargo llvm-cov` compiles with coverage instrumentation, which costs a
/// multiple of normal runtime — enough that a 100 ms budget measured on an
/// uninstrumented build fails in the coverage lane while the code under test is
/// unchanged. That is what happened: `f_3100_1_event_detection_latency` was red
/// on every coverage run and green everywhere else, so the lane reported a
/// defect that did not exist and the real signal in it got harder to trust.
///
/// The budget is still enforced — a 10x regression fails either way. It is the
/// measurement that is made honest about its own instrument. `LLVM_PROFILE_FILE`
/// is set by `cargo llvm-cov` for exactly this purpose and is the documented way
/// to detect the instrumented build from inside a test.
pub fn latency_budget_ms(uninstrumented: u128) -> u128 {
    if std::env::var_os("LLVM_PROFILE_FILE").is_some() {
        uninstrumented * 10
    } else {
        uninstrumented
    }
}
