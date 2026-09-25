//! forjar#633: the I8 gate refused a `break` that IS inside a loop.
//!
//! bashrs 6.68.0 reported `SC2105 'break' is only valid in loops` on a one-line
//! `for` loop nested in an `if`, and `forjar apply` refused the task at I8
//! (paiml/infra#1088, gx10). bashrs 7.x judges the same bytes clean. The first
//! test goes RED on 6.68.0; the second proves the pass was not bought by
//! dropping SC2105.

use forjar::core::purifier::validate_script;

#[test]
fn break_inside_a_for_inside_an_if_passes() {
    let script = "if [ ! -e \"$HOME/.local/state/release-train/active\" ]; then\n  \
                  systemctl --user restart apr-review-serve.service\n  \
                  for _ in $(seq 1 60); do /usr/local/bin/probe health && break; sleep 5; done\n\
                  fi\n";
    assert!(
        validate_script(script).is_ok(),
        "the I8 gate refuses a `break` that is inside a loop: {:?}",
        validate_script(script)
    );
}

#[test]
fn break_outside_any_loop_is_still_rejected() {
    let script = "if [ -e /tmp/x ]; then\n  break\nfi\n";
    let err = validate_script(script)
        .expect_err("a `break` outside any loop was accepted — SC2105 is gone, not fixed");
    assert!(err.contains("SC2105"), "rejected, but not by SC2105: {err}");
}
