//! forjar#1858 / paiml-infra#1858: intel `ci-disk-watch-timer-enable` was refused by
//! the I8 gate with "SEC005: Hardcoded secret detected: OpenAI API key pattern"
//! (twice). The text holds no secret: `sk-` is the tail of `di`+`sk-watch`, and the
//! completion_check line ends in `= "no"`. bashrs 7.4.2 (bashrs#350/#351) only reads
//! a token prefix at a token start. This pins the pin: the real resource text must
//! pass, and a real-looking key must still be refused.

use super::purifier::validate_script;

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
