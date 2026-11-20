//! Round 6 (the review of 5f9fe079): no decision may rest on the silence of a
//! command, a NUL byte is refused, and JSON that only some readers accept
//! (NaN, Infinity, +3, 03, 1e309, a BOM, trailing text, a second value) is not
//! a canary file. Tool cases run the predicate with a PATH where one tool is
//! absent or a script that prints nothing and exits 0 or 1.
use super::cases_review::{nop, pl, pl_text};
use super::*;
use serde_json::json;

fn nul_claims(b: String) -> String {
    b.replacen('[', "[\u{0}", 1)
}
fn nul_plan(b: String) -> String {
    b.replacen('{', "{\u{0}", 1)
}
fn nul_verdicts(b: String) -> String {
    b.replacen('[', "[\u{0}", 1)
}
fn zz(lit: &str) -> impl Fn(String) -> String + '_ {
    move |t| t.replacen('{', &format!("{{\"zz\":{lit},"), 1)
}
fn rec_nan(t: String) -> String {
    zz("NaN")(t)
}
fn rec_inf(t: String) -> String {
    zz("Infinity")(t)
}
fn rec_plus(t: String) -> String {
    zz("+3")(t)
}
fn rec_zero(t: String) -> String {
    zz("03")(t)
}
fn rec_big(t: String) -> String {
    zz("1e309")(t)
}
fn rec_frac(t: String) -> String {
    zz("1.5")(t)
}
fn rec_wide(t: String) -> String {
    zz("1234567890123456")(t)
}
fn claims_tab(b: String) -> String {
    b.replacen("added_one", "added\tone", 1)
}
fn plan_is_claims(t: String) -> String {
    t.replace("evidence/plan.json", "evidence/claims.json")
}
fn rec_bom(t: String) -> String {
    format!("\u{feff}{t}")
}
fn claims_plus(b: String) -> String {
    b.replacen('[', "[+3,", 1)
}
fn plan_nan(b: String) -> String {
    b.replacen('{', "{\"zz\":NaN,", 1)
}
fn verdicts_zero(b: String) -> String {
    b.replacen('[', "[03,", 1)
}
fn claims_bom(b: String) -> String {
    format!("\u{feff}{b}")
}
fn claims_tail(b: String) -> String {
    format!("{b} x")
}
fn claims_two(b: String) -> String {
    format!("{b}\n[]")
}
fn plan_two(b: String) -> String {
    format!("{b}\n{{}}")
}

macro_rules! sub {
    ($name:expr, $edit:expr) => {
        ($name, ($edit) as Edit, nop as OptEdit, "!ok")
    };
}

macro_rules! shim {
    ($name:expr, $tool:expr, $mode:expr) => {
        (
            $name,
            (|_: &mut Fx| {}) as Edit,
            (|o: &mut Opt| o.shim = Some(($tool, $mode))) as OptEdit,
            "!ok",
        )
    };
}

pub fn table() -> Vec<Case> {
    vec![
        (
            "a NUL byte in the claims file is refused",
            |_| {},
            |o| o.body_edit = Some(("claims.json", nul_claims)),
            "holds a NUL byte",
        ),
        (
            "a NUL byte in the plan file is refused",
            |_| {},
            |o| o.body_edit = Some(("plan.json", nul_plan)),
            "holds a NUL byte",
        ),
        (
            "a NUL byte in the verdicts file is refused",
            |_| {},
            |o| o.body_edit = Some(("verdicts.json", nul_verdicts)),
            "holds a NUL byte",
        ),
        (
            "NaN in the receipt is refused",
            |_| {},
            |o| o.rec_edit = Some(rec_nan),
            "strict JSON text",
        ),
        (
            "Infinity in the receipt is refused",
            |_| {},
            |o| o.rec_edit = Some(rec_inf),
            "strict JSON text",
        ),
        (
            "+3 in the receipt is refused",
            |_| {},
            |o| o.rec_edit = Some(rec_plus),
            "strict JSON text",
        ),
        (
            "03 in the receipt is refused",
            |_| {},
            |o| o.rec_edit = Some(rec_zero),
            "strict JSON text",
        ),
        (
            "1e309 in the receipt is refused",
            |_| {},
            |o| o.rec_edit = Some(rec_big),
            "strict JSON text",
        ),
        (
            "1.5 in the receipt is refused",
            |_| {},
            |o| o.rec_edit = Some(rec_frac),
            "strict JSON text",
        ),
        (
            "+3 in the claims file is refused",
            |_| {},
            |o| o.body_edit = Some(("claims.json", claims_plus)),
            "strict JSON text",
        ),
        (
            "NaN in the plan file is refused",
            |_| {},
            |o| o.body_edit = Some(("plan.json", plan_nan)),
            "strict JSON text",
        ),
        (
            "03 in the verdicts file is refused",
            |_| {},
            |o| o.body_edit = Some(("verdicts.json", verdicts_zero)),
            "strict JSON text",
        ),
        (
            "a BOM before the receipt is refused",
            |_| {},
            |o| o.rec_edit = Some(rec_bom),
            "strict JSON text",
        ),
        (
            "a BOM before the claims file is refused",
            |_| {},
            |o| o.body_edit = Some(("claims.json", claims_bom)),
            "strict JSON text",
        ),
        (
            "trailing text after the claims file is refused",
            |_| {},
            |o| o.body_edit = Some(("claims.json", claims_tail)),
            "not readable JSON",
        ),
        (
            "a second top-level value after the claims file is refused",
            |_| {},
            |o| o.body_edit = Some(("claims.json", claims_two)),
            "top-level JSON values",
        ),
        (
            "a second top-level value after the plan file is refused",
            |_| {},
            |o| o.body_edit = Some(("plan.json", plan_two)),
            "top-level JSON values",
        ),
        (
            "a planted text of spaces and single letters is refused",
            |f| pl_text(f, 0, "a b c d e f g h i j k l"),
            nop,
            "planted[] needs",
        ),
        (
            "a 16-digit integer in the receipt is refused",
            |_| {},
            |o| o.rec_edit = Some(rec_wide),
            "strict JSON text",
        ),
        (
            "a raw tab inside a claim text is refused",
            |_| {},
            |o| o.body_edit = Some(("claims.json", claims_tab)),
            "not readable JSON",
        ),
        sub!("a whitespace-only planted_by is refused", |f: &mut Fx| {
            f.receipt["quorum"]["canary"]["planted_by"] = json!("   ")
        }),
        sub!(
            "a reviewed_base that is not 40 hex is refused",
            |f: &mut Fx| { f.receipt["quorum"]["canary"]["reviewed_base"] = json!("abc123") }
        ),
        sub!(
            "real_claim_ids that is not an array is refused",
            |f: &mut Fx| { f.receipt["quorum"]["canary"]["real_claim_ids"] = json!("1") }
        ),
        sub!("duplicate real_claim_ids are refused", |f: &mut Fx| {
            f.receipt["quorum"]["canary"]["real_claim_ids"] = json!(["1", "1"])
        }),
        sub!("a planted that is not an array is refused", |f: &mut Fx| {
            f.receipt["quorum"]["canary"]["planted"] = json!("x")
        }),
        sub!("a planted position below 1 is refused", |f: &mut Fx| pl(
            f,
            0,
            "position",
            json!(0)
        )),
        (
            "a plan_file that is the claims file is refused",
            |_| {},
            |o| o.rec_edit = Some(plan_is_claims),
            "!ok",
        ),
        sub!(
            "a claims_file that is not a claims array is refused",
            |f: &mut Fx| { f.claims = json!({"claims": []}) }
        ),
        sub!(
            "a claims_file with a duplicate id is refused",
            |f: &mut Fx| {
                let c = f.claims[0].clone();
                f.claims.as_array_mut().expect("claims").push(c);
            }
        ),
        sub!("a claims_file with one claim is refused", |f: &mut Fx| {
            f.claims.as_array_mut().expect("claims").truncate(1)
        }),
        sub!(
            "a verdicts_file that is not an array is refused",
            |f: &mut Fx| { f.verdicts = json!({"judges": []}) }
        ),
        shim!(
            "a jq that gives no planted count is refused",
            "jq",
            "when:*\".planted | length\"*"
        ),
        shim!(
            "a jq that gives no canary block is refused",
            "jq",
            "when:\"-c .quorum.canary\""
        ),
        shim!(
            "a jq that gives no quorum block is refused",
            "jq",
            "when:\"-c .quorum\""
        ),
        shim!("an absent jq is refused", "jq", "absent"),
        shim!(
            "a jq that prints nothing and exits 0 is refused",
            "jq",
            "silent0"
        ),
        shim!(
            "a jq that prints nothing and exits 1 is refused",
            "jq",
            "silent1"
        ),
        shim!("an absent git is refused", "git", "absent"),
        shim!(
            "a git that prints nothing and exits 0 is refused",
            "git",
            "silent0"
        ),
        shim!(
            "a git that prints nothing and exits 1 is refused",
            "git",
            "silent1"
        ),
        shim!("an absent grep is refused", "grep", "absent"),
        shim!(
            "a grep that prints nothing and exits 0 is refused",
            "grep",
            "silent0"
        ),
        shim!(
            "a grep that prints nothing and exits 1 is refused",
            "grep",
            "silent1"
        ),
        shim!("an absent mktemp is refused", "mktemp", "absent"),
        shim!(
            "a mktemp that prints nothing and exits 0 is refused",
            "mktemp",
            "silent0"
        ),
        shim!("an absent sha256sum is refused", "sha256sum", "absent"),
        shim!(
            "a sha256sum that prints nothing and exits 0 is refused",
            "sha256sum",
            "silent0"
        ),
        shim!("an absent awk is refused", "awk", "absent"),
        shim!(
            "an awk that prints nothing and exits 0 is refused",
            "awk",
            "silent0"
        ),
        shim!(
            "an awk that prints nothing and exits 1 is refused",
            "awk",
            "silent1"
        ),
        (
            "an absent od, tr, head and rm change nothing: no decision rests on them",
            |_| {},
            |o| o.shim = Some(("od", "absent")),
            "ok",
        ),
        (
            "an absent head changes nothing",
            |_| {},
            |o| o.shim = Some(("head", "absent")),
            "ok",
        ),
        (
            "an absent tr changes nothing",
            |_| {},
            |o| o.shim = Some(("tr", "absent")),
            "ok",
        ),
        (
            "an absent rm only leaves its scratch files",
            |_| {},
            |o| o.shim = Some(("rm", "absent")),
            "ok",
        ),
    ]
}
