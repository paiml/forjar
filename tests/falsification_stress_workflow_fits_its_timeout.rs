//! forjar#673: the weekly Stress Tests lane must be able to go green.
//!
//! One `cargo test --lib -- --test-threads=1` pass is 6-9 minutes; ten passes
//! never fit a 30-minute leg, and fail-fast cancelled the sibling legs.

fn stress() -> String {
    std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/.github/workflows/stress.yml"
    ))
    .expect("read stress.yml")
}

#[test]
fn passes_per_leg_fit_the_timeout() {
    let w = stress();
    let passes: u32 = w
        .lines()
        .find_map(|l| {
            l.trim()
                .strip_prefix("for run in $(seq 1 ")?
                .strip_suffix("); do")?
                .parse()
                .ok()
        })
        .expect("seq loop");
    let timeout: u32 = w
        .lines()
        .rev()
        .find_map(|l| l.trim().strip_prefix("timeout-minutes: ")?.parse().ok())
        .expect("timeout-minutes");
    // 9 min per pass worst case, plus a 10 min build.
    assert!(
        passes * 9 + 10 <= timeout,
        "{passes} passes x 9 min + build > {timeout} min"
    );
}

#[test]
fn a_red_leg_does_not_cancel_its_siblings() {
    assert!(
        stress().contains("fail-fast: false"),
        "stress.yml lacks fail-fast: false"
    );
}
