//! The case table: every acceptance and refusal of the canary shape.
use super::*;
pub fn table() -> Vec<(&'static str, Edit, &'static str)> {
    vec![
        ("clean canary round is accepted", |_| {}, "ok"),
        (
            "zero refuted, no canary: the old sentence, unchanged",
            |f| {
                f.receipt["quorum"]
                    .as_object_mut()
                    .unwrap()
                    .remove("canary");
            },
            OLD_ZERO,
        ),
        (
            "a present block is checked even when a claim was refuted",
            |f| {
                f.receipt["quorum"]["claims_refuted"] = json!(1);
                canary(f)["planted"] = json!([]);
            },
            "below the floor of 2",
        ),
        (
            "planted = 1 is refused",
            super::cases_review::one_planted,
            "below the floor of 2",
        ),
        (
            "caught short of planted is refused",
            |f| {
                canary(f)["caught"] = json!(["2"]);
            },
            "caught does not equal",
        ),
        (
            "caught naming an unplanted id is refused",
            |f| {
                canary(f)["caught"] = json!(["2", "1"]);
            },
            "caught does not equal",
        ),
        (
            "a canary a judge majority missed is refused",
            |f| {
                f.verdicts[1]["verdicts"]["4"] = json!("SURVIVED");
                f.verdicts[2]["verdicts"]["4"] = json!("SURVIVED");
            },
            "not refuted by a majority",
        ),
        (
            "canary refuted by one of three is refused",
            |f| {
                f.verdicts[1]["verdicts"]["2"] = json!("SURVIVED");
            },
            "not refuted by a majority",
        ),
        (
            "two judge entries for three judges is refused",
            |f| {
                f.verdicts.as_array_mut().unwrap().pop();
            },
            "judge entries",
        ),
        (
            "a judge skipping a claim is refused",
            |f| {
                f.verdicts[0]["verdicts"]
                    .as_object_mut()
                    .unwrap()
                    .remove("3");
            },
            "every judge must answer every claim",
        ),
        (
            "a canary id also among real claims is refused",
            |f| {
                canary(f)["real_claim_ids"] = json!(["1", "2"]);
            },
            "also appears in real_claim_ids",
        ),
        (
            "an empty real_claim_ids is refused",
            |f| {
                canary(f)["real_claim_ids"] = json!([]);
            },
            "real_claim_ids is empty",
        ),
        (
            "a waived key inside the block is refused",
            |f| {
                canary(f)["waived"] = json!(true);
            },
            "waiver/override key",
        ),
        (
            "an override key inside the block is refused",
            |f| {
                canary(f)["override"] = json!("x");
            },
            "waiver/override key",
        ),
        (
            "a marker word in the claims file is refused",
            |f| {
                set_claims(
                    f,
                    2,
                    "text",
                    json!("This canary claim adds added_two to src/a.rs at line five."),
                );
            },
            "were not blind",
        ),
        (
            "a capitalised marker word is refused",
            |f| {
                set_claims(
                    f,
                    0,
                    "text",
                    json!("Planted: the change adds added_one to src/a.rs at line four."),
                );
            },
            "were not blind",
        ),
        (
            "a planted claim with an extra key is refused",
            |f| {
                f.claims[1]["origin"] = json!("courier");
            },
            "exactly the keys id and text",
        ),
        (
            "a planted claim at the wrong position is refused",
            |f| {
                f.claims.as_array_mut().unwrap().swap(1, 2);
            },
            "at its recorded position",
        ),
        (
            "a planter named refuter is refused",
            |f| {
                canary(f)["planted_by"] = json!("Refuter-2");
            },
            "refuter or a judge",
        ),
        (
            "a planter named judge is refused",
            |f| {
                canary(f)["planted_by"] = json!("the judge");
            },
            "refuter or a judge",
        ),
        (
            "a missing planter is refused",
            |f| {
                canary(f).as_object_mut().unwrap().remove("planted_by");
            },
            "planted_by",
        ),
        (
            "a falsifier on an unchanged line is refused",
            |f| {
                canary(f)["planted"][0]["falsified_by"] = json!("src/a.rs:2");
                f.plan["planted"][0]["falsified_by"] = json!("src/a.rs:2");
            },
            "not an added line",
        ),
        (
            "a falsifier in an untouched file is refused",
            |f| {
                canary(f)["planted"][1]["falsified_by"] = json!("src/z.rs:5");
                f.plan["planted"][1]["falsified_by"] = json!("src/z.rs:5");
            },
            "not exactly a file changed",
        ),
        (
            "a plan that differs from the receipt is refused",
            |f| {
                f.plan["planted"][0]["text"] =
                    json!("A different false claim about the diff, long enough.");
            },
            "is not an array equal to quorum.canary.planted",
        ),
        (
            "a receipt count that counts a canary is refused",
            |f| {
                f.receipt["quorum"]["claims_reviewed"] = json!(4);
            },
            "claims_reviewed",
        ),
        (
            "claims_refuted that disagrees with the judges is refused",
            |f| {
                f.receipt["quorum"]["claims_refuted"] = json!(0);
                f.receipt["quorum"]["claims_confirmed"] = json!(1);
                f.verdicts[0]["verdicts"]["1"] = json!("REFUTED");
                f.verdicts[1]["verdicts"]["1"] = json!("REFUTED");
            },
            "judge verdicts refute 1",
        ),
        (
            "claims_confirmed that counts a canary is refused",
            |f| {
                f.receipt["quorum"]["claims_confirmed"] = json!(4);
            },
            "claims_confirmed",
        ),
        (
            "a wrong type for canary is refused",
            |f| {
                f.receipt["quorum"]["canary"] = json!("yes");
            },
            "not an object",
        ),
        (
            "a null canary is refused",
            |f| {
                f.receipt["quorum"]["canary"] = Value::Null;
            },
            "not an object",
        ),
        (
            "a numeric planted_by is refused",
            |f| {
                canary(f)["planted_by"] = json!(5);
            },
            "planted_by",
        ),
        (
            "a string position is refused",
            |f| {
                canary(f)["planted"][0]["position"] = json!("2");
            },
            "planted[] needs",
        ),
        (
            "duplicate positions are refused",
            |f| {
                canary(f)["planted"][1]["position"] = json!(2);
            },
            "position are not distinct",
        ),
        (
            "a short reviewed_head is refused",
            |f| {
                canary(f)["reviewed_head"] = json!("abc");
            },
            "reviewed_head",
        ),
        (
            "a reviewed commit not in the repo is refused",
            |f| {
                canary(f)["reviewed_head"] = json!("0123456789012345678901234567890123456789");
            },
            "is not the commit this receipt vouches for",
        ),
        (
            "a claims_file without its role is refused",
            |f| {
                f.receipt["quorum"]["canary"]["claims_file"] = json!(".quorum/evidence/plan.json");
            },
            "claims_file must be one",
        ),
        (
            "an evidence file outside .quorum/evidence is refused",
            |f| {
                canary(f)["plan_file"] = json!("../plan.json");
            },
            "outside .quorum/evidence",
        ),
        (
            "a verdicts_file that is not an evidence.files entry is refused",
            |f| {
                canary(f)["verdicts_file"] = json!(".quorum/evidence/absent.json");
            },
            "verdicts_file must be one evidence.files entry",
        ),
        (
            "a claims file that is not an array is refused",
            |f| {
                f.claims = json!({"a": 1});
            },
            "claims_file",
        ),
        (
            "fewer than three lanes keeps its old floor",
            |f| {
                f.receipt["quorum"]["lanes"] = json!(["a", "b"]);
            },
            "quorum.lanes = 2",
        ),
        (
            "fewer than three judges keeps its old floor",
            |f| {
                f.receipt["quorum"]["judges"] = json!(2);
            },
            "quorum.judges = 2",
        ),
    ]
}

/// Every case, the original table and the review-finding table, as one list.
pub fn all() -> Vec<Case> {
    let mut v: Vec<Case> = table()
        .into_iter()
        .map(|(n, e, w)| (n, e, (|_: &mut Opt| {}) as OptEdit, w))
        .collect();
    v.extend(super::cases_review::table());
    v.extend(super::cases_round3::table());
    v
}

/// What case `name` gives with the script `script`.
pub fn answer(script: &Path, name: &str) -> (String, &'static str) {
    let (_, edit, opt, want) = all()
        .into_iter()
        .find(|c| c.0 == name)
        .unwrap_or_else(|| panic!("no case named {name:?}"));
    let mut fx = base_fx();
    edit(&mut fx);
    let mut o = Opt::default();
    opt(&mut o);
    (run(script, &fx, &o), want)
}

/// Every case against `script`; the failures, empty when all hold.
pub fn failures(script: &Path, stop_at_first: bool) -> Vec<String> {
    let mut bad = Vec::new();
    for (name, _, _, _) in all() {
        let (got, want) = answer(script, name);
        let ok = if want == "ok" {
            got == "ok"
        } else {
            got.contains(want)
        };
        if !ok {
            bad.push(format!("{name}: wanted {want:?}, got {got:?}"));
            if stop_at_first {
                return bad;
            }
        }
    }
    bad
}
