//! forjar#673: the weekly Stress Tests lane must be able to go green.
//!
//! One `cargo test --lib -- --test-threads=1` pass is 6-9 minutes; ten passes
//! never fit the old 30-minute leg, and fail-fast cancelled the sibling legs.

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
    // Pin to the stress step's own timeout: the line after the seq loop's step.
    let step = w
        .find("name: Stress test iteration")
        .expect("stress test step");
    // Bound the search to the stress step's own block (up to the next `- name:`).
    let rest = &w[step + 1..];
    let block = &rest[..rest.find("\n      - name:").unwrap_or(rest.len())];
    let timeout: u32 = block
        .lines()
        .find_map(|l| l.trim().strip_prefix("timeout-minutes: ")?.parse().ok())
        .expect("timeout-minutes on the stress step");
    // 9 min per pass worst case, plus a 10 min build.
    assert!(
        passes * 9 + 10 <= timeout,
        "{passes} passes x 9 min + build > {timeout} min"
    );
}

#[test]
fn a_red_leg_does_not_cancel_its_siblings() {
    // Parse `strategy:` -> `fail-fast:` on live lines; a comment must not satisfy it.
    let w = stress();
    let live: Vec<&str> = w
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .collect();
    let strat = live
        .iter()
        .position(|l| l.trim() == "strategy:")
        .expect("strategy block");
    let ff = live[strat + 1..]
        .iter()
        .take_while(|l| l.starts_with("      ") || l.trim().is_empty())
        .find_map(|l| l.trim().strip_prefix("fail-fast:"))
        .expect("fail-fast key inside strategy");
    assert_eq!(ff.trim(), "false", "stress.yml lacks fail-fast: false");
}
