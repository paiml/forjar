//! PMAT-698 / forjar#715: bashrs 7.4.1 -> 7.4.2 clears the SEC005 "disk-watch"
//! false positive (paiml/infra#1858). The text holds no secret: `sk-` is the tail
//! of `di`+`sk-watch`. The pin lives in `Cargo.toml` (`bashrs = "7.4.2"`), so this
//! test goes red when the dependency is reverted to 7.4.1.
//!
//! Usage: cargo test --test falsification_715_bashrs_742_sec005

use forjar::core::purifier::validate_script;

#[test]
fn ci_disk_watch_timer_enable_script_passes_i8() {
    let script = "systemctl daemon-reload\n\
        systemctl enable ci-disk-watch.timer\n\
        systemctl restart ci-disk-watch.timer\n\
        systemctl is-enabled ci-disk-watch.timer >/dev/null 2>&1 && \\\n\
        systemctl is-active ci-disk-watch.timer >/dev/null 2>&1 && \\\n\
        [ \"$(systemctl show ci-disk-watch.timer -p NeedDaemonReload --value 2>/dev/null)\" = \"no\" ]\n";
    validate_script(script).unwrap_or_else(|e| panic!("false SEC005: {e}"));
}

#[test]
fn a_real_looking_openai_key_is_still_refused() {
    let script = "KEY=\"sk-proj-abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGH\"\n";
    let err = validate_script(script).expect_err("a hardcoded key must be refused");
    assert!(err.contains("SEC005"), "{err}");
}
