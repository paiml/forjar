//! PMAT-698: the I8 gate refused `ids="$ids $WANT=$id"` as SC1066.
//!
//! bashrs 7.4.1 reported `SC1066: Don't use $ on the left side of assignments`
//! on a `$name=$value` pair INSIDE a double-quoted string, and `forjar apply`
//! refused an infra task at I8. Inside quotes that text is not an assignment.
//! bashrs 7.4.2 judges the same bytes clean (paiml/bashrs#388, fixed by #392;
//! the echo form is paiml/bashrs#437).
//!
//! One fixture per FORM VARIANT: the quoted RHS of an assignment, and an echo
//! argument. Both go RED on bashrs 7.4.1. The last test proves the pass was
//! not bought by dropping SC1066.

use forjar::core::purifier::validate_script;

fn assert_passes(script: &str, form: &str) {
    assert!(
        validate_script(script).is_ok(),
        "the I8 gate refuses {form}: {:?}",
        validate_script(script)
    );
}

#[test]
fn dollar_equals_in_the_quoted_rhs_of_an_assignment_passes() {
    let script = "ids=\"\"\nWANT=x\nfor id in a b; do\n  ids=\"$ids $WANT=$id\"\ndone\nprintf '%s\\n' \"$ids\"\n";
    assert_passes(script, "`ids=\"$ids $WANT=$id\"`");
}

#[test]
fn dollar_equals_in_a_quoted_echo_argument_passes() {
    let script = "p=key\nv=val\necho \"setting $p=$v now\"\n";
    assert_passes(script, "`echo \"setting $p=$v now\"`");
}

#[test]
fn both_forms_in_one_script_pass() {
    let script =
        "ids=\"\"\nWANT=x\np=key\nv=val\nfor id in a b; do\n  ids=\"$ids $WANT=$id\"\ndone\n\
                  echo \"setting $p=$v now\"\nprintf '%s\\n' \"$ids\"\n";
    assert_passes(script, "both `$a=$b`-in-quotes forms");
}

#[test]
fn a_real_dollar_on_the_left_of_an_assignment_is_still_rejected() {
    let script = "$foo=bar\n";
    let err =
        validate_script(script).expect_err("`$foo=bar` was accepted — SC1066 is gone, not fixed");
    assert!(err.contains("SC1066"), "rejected, but not by SC1066: {err}");
}
