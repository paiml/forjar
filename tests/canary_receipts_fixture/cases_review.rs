//! The cases that reproduce the findings of the pre-PR review of 14007abe.
//! Each was a forged or unbound receipt the first predicate accepted.
use super::*;

pub fn s(x: &str) -> Option<String> {
    Some(x.to_string())
}

/// Set a planted claim's field in the receipt and in the hashed plan.
pub fn pl(f: &mut Fx, i: usize, k: &str, v: Value) {
    f.receipt["quorum"]["canary"]["planted"][i][k] = v.clone();
    f.plan["planted"][i][k] = v;
}

/// Set a planted claim's text everywhere it is copied: receipt, plan, claims.
pub fn pl_text(f: &mut Fx, i: usize, t: &str) {
    pl(f, i, "text", json!(t));
    let pos = f.plan["planted"][i]["position"].as_u64().expect("position") as usize;
    f.claims[pos - 1]["text"] = json!(t);
}

/// Rename a claim id in the claims file and in every judge's verdicts.
pub fn rename(f: &mut Fx, old: &str, new: &str) {
    for c in f.claims.as_array_mut().expect("claims") {
        if c["id"] == old {
            c["id"] = json!(new);
        }
    }
    for v in f.verdicts.as_array_mut().expect("verdicts") {
        let m = v["verdicts"].as_object_mut().expect("verdicts map");
        if let Some(x) = m.remove(old) {
            m.insert(new.to_string(), x);
        }
    }
}

fn rename_planted(f: &mut Fx, i: usize, new: &str) {
    let old = f.plan["planted"][i]["id"].as_str().expect("id").to_string();
    rename(f, &old, new);
    pl(f, i, "id", json!(new));
    canary(f)["caught"] = json!(["2", "4"].map(|x| if x == old { new } else { x }));
}

fn rename_real(f: &mut Fx, old: &str, new: &str) {
    rename(f, old, new);
    canary(f)["real_claim_ids"] = json!(["1", "3"].map(|x| if x == old { new } else { x }));
}

/// A round with no real claim at all: the claims file holds only the canaries.
fn only_planted(f: &mut Fx) {
    canary(f)["real_claim_ids"] = json!([]);
    f.claims = json!([{"id": "2", "text": P_DELETE}, {"id": "4", "text": P_FIRST}]);
    pl(f, 0, "position", json!(1));
    pl(f, 1, "position", json!(2));
    for v in f.verdicts.as_array_mut().expect("verdicts") {
        let m = v["verdicts"].as_object_mut().expect("map");
        m.remove("1");
        m.remove("3");
    }
    f.receipt["quorum"]["claims_reviewed"] = json!(0);
    f.receipt["quorum"]["claims_confirmed"] = json!(0);
}

pub fn raw_claims(third: &str) -> Option<String> {
    Some(format!(
        "[{{\"id\":\"1\",\"text\":\"{A_ADDED_1}\"}},{{\"id\":\"2\",\"text\":\"{P_DELETE}\"}},{{\"id\":\"3\",\"text\":\"{third}\"}},{{\"id\":\"4\",\"text\":\"{P_FIRST}\"}}]"
    ))
}

pub fn nop(_: &mut Opt) {}

fn zero_hashes(f: &mut Fx) {
    let z = "0".repeat(64);
    f.receipt["evidence"]["files"] = json!([
        {"path": ".quorum/evidence/claims.json", "roles": ["claims-blind"], "sha256": z},
        {"path": ".quorum/evidence/plan.json", "roles": ["canary-plan"], "sha256": z},
        {"path": ".quorum/evidence/verdicts.json", "roles": ["judge-verdicts"], "sha256": z}]);
}

pub fn table() -> Vec<Case> {
    vec![
        // 1. any two commits passed as reviewed_base / reviewed_head.
        (
            "reviewed_head and reviewed_base swapped are refused",
            |f| {
                canary(f)["reviewed_head"] = json!("@BASE");
                canary(f)["reviewed_base"] = json!("@HEAD");
            },
            nop,
            "reviewed_head",
        ),
        (
            "a real but other reviewed_head is refused",
            |f| {
                canary(f)["reviewed_head"] = json!("@OTHER");
            },
            nop,
            "reviewed_head",
        ),
        (
            "a real but other reviewed_base is refused",
            |f| {
                canary(f)["reviewed_base"] = json!("@OTHER");
            },
            nop,
            "reviewed_base",
        ),
        (
            "a caller naming another subject commit is refused",
            |_| {},
            |o| o.subj_head = s("@OTHER"),
            "reviewed_head",
        ),
        // 3. empty or unresolvable revs read the index or fall back.
        (
            "an empty subject head is refused",
            |_| {},
            |o| o.subj_head = s(""),
            "empty or does not resolve",
        ),
        (
            "an empty subject base is refused",
            |_| {},
            |o| o.subj_base = s(""),
            "empty or does not resolve",
        ),
        (
            "an unresolvable subject base is refused",
            |_| {},
            |o| o.subj_base = s("nosuchrev"),
            "empty or does not resolve",
        ),
        (
            "an empty evidence rev is refused",
            |_| {},
            |o| o.revs = vec![String::new()],
            "evidence rev",
        ),
        (
            "an unresolvable evidence rev is refused",
            |_| {},
            |o| o.revs = vec!["nosuchrev".into(), "HEAD".into()],
            "does not resolve",
        ),
        // 4. store.sh named no commit at all.
        (
            "a canary block with no subject commit is refused",
            |_| {},
            |o| o.no_subject = true,
            "caller named none",
        ),
        // 2. the falsifier must be an exact changed path, and unique.
        (
            "a directory falsifier is refused",
            |f| {
                pl(f, 0, "falsified_by", json!("src:4"));
                pl(f, 1, "falsified_by", json!("src:5"));
            },
            nop,
            "not exactly a file changed",
        ),
        (
            "a dot falsifier is refused",
            |f| {
                pl(f, 0, "falsified_by", json!(".:4"));
                pl(f, 1, "falsified_by", json!(".:5"));
            },
            nop,
            "not exactly a file changed",
        ),
        (
            "a dot-slash falsifier is refused",
            |f| {
                pl(f, 0, "falsified_by", json!("./src/a.rs:4"));
            },
            nop,
            "needs a plain numeric id",
        ),
        (
            "two canaries sharing one falsifying line are refused",
            |f| {
                pl(f, 0, "falsified_by", json!("src/a.rs:4"));
                pl(f, 1, "falsified_by", json!("src/a.rs:4"));
            },
            nop,
            "falsified_by are not distinct",
        ),
        (
            "a diff with no added line cannot carry a canary",
            |_| {},
            |o| o.delete_only = true,
            "not an added line",
        ),
        // 5. the waiver key, any case.
        (
            "an upper-case top-level waiver key is refused",
            |f| {
                f.receipt["WAIVED"] = json!({"reason": "x"});
            },
            nop,
            "waiver/override key",
        ),
        (
            "a capitalised override key inside the block is refused",
            |f| {
                canary(f)["Override"] = json!("x");
            },
            nop,
            "waiver/override key",
        ),
        // 6. evidence lookup takes the copy whose hash matches.
        (
            "a stale copy at the first rev does not hide the matching copy",
            |_| {},
            |o| {
                o.stale_first = true;
                o.revs = vec!["@OTHER".into(), "HEAD".into()];
            },
            "ok",
        ),
        (
            "a hash that matches no committed copy is refused",
            zero_hashes,
            |o| o.skip_hash = true,
            "evidence.files sha256",
        ),
        // 7. the marker search sees decoded strings.
        (
            "a JSON-escaped marker word is refused",
            |_| {},
            |o| o.claims_raw = raw_claims("The \\u0063anary claim adds added_two to src/a.rs."),
            "strict JSON text",
        ),
        (
            "a homoglyph lookalike of a marker word is refused",
            |_| {},
            |o| o.claims_raw = raw_claims("The \u{441}anary claim adds added_two to src/a.rs."),
            "non-ASCII",
        ),
        (
            "a spaced-out marker word is refused",
            |_| {},
            |o| o.claims_raw = raw_claims("The p-l-a-n-t-e-d claim adds added_two to src/a.rs."),
            "were not blind",
        ),
        (
            "a marker word as an object key is refused",
            |f| {
                f.claims[0]["decoy"] = json!("x");
            },
            nop,
            "were not blind",
        ),
        (
            "a non-ASCII claim id is refused",
            |f| {
                f.claims[0]["id"] = json!("\u{663}");
            },
            nop,
            "non-ASCII",
        ),
        // 8. blank texts and names; the planter is nobody else.
        (
            "a whitespace-only planted text is refused",
            |f| {
                pl_text(f, 0, "                         ");
            },
            nop,
            "planted[] needs",
        ),
        (
            "a whitespace-only real claim text is refused",
            |f| {
                f.claims[0]["text"] = json!("     ");
            },
            nop,
            "non-blank",
        ),
        (
            "an empty judge name is refused",
            |f| {
                f.verdicts[0]["judge"] = json!("");
            },
            nop,
            "non-blank name",
        ),
        (
            "a whitespace judge name is refused",
            |f| {
                f.verdicts[0]["judge"] = json!("   ");
            },
            nop,
            "non-blank name",
        ),
        (
            "a planter who is also a judge is refused",
            |f| {
                canary(f)["planted_by"] = json!("J1");
            },
            nop,
            "also a judge",
        ),
        (
            "a planter who is a judge in another case and spacing is refused",
            |f| {
                canary(f)["planted_by"] = json!(" j2 ");
            },
            nop,
            "also a judge",
        ),
        (
            "a planter who is also a lane is refused",
            |f| {
                canary(f)["planted_by"] = json!(" A ");
            },
            nop,
            "also a lane",
        ),
        (
            "a planter who is a named refuter is refused",
            |f| {
                f.receipt["quorum"]["refuter_names"] = json!(["courier"]);
            },
            nop,
            "also a lane, refuter or judge",
        ),
        // 9. one strict id format; planted must be an array.
        (
            "a planted id with a leading space is refused",
            |f| {
                rename_planted(f, 0, " 2");
            },
            nop,
            "planted[] needs",
        ),
        (
            "a planted id with a leading zero is refused",
            |f| {
                rename_planted(f, 0, "02");
            },
            nop,
            "planted[] needs",
        ),
        (
            "a real id with a leading zero is refused",
            |f| {
                rename_real(f, "1", "01");
            },
            nop,
            "not a plain number",
        ),
        (
            "a short planted text is refused",
            |f| {
                pl_text(f, 0, "too short text");
            },
            nop,
            "planted[] needs",
        ),
        (
            "two judges with one name are refused",
            |f| {
                f.verdicts[1]["judge"] = json!(" j1 ");
            },
            nop,
            "judges are not distinct",
        ),
        (
            "a round with no real claim is refused",
            only_planted,
            nop,
            "real_claim_ids is empty",
        ),
        (
            "a plan whose planted is an object is refused",
            |f| {
                let p = f.plan["planted"].clone();
                f.plan = json!({"planted": {"k1": p[0], "k2": p[1]}});
            },
            nop,
            "not an array",
        ),
        // non-blocking.
        (
            "a real claim with an extra key is refused",
            |f| {
                f.claims[0]["origin"] = json!("lane");
            },
            nop,
            "exactly the keys id and text",
        ),
        (
            "an empty predicate answer is refused as unmeasured",
            |_| {},
            |o| o.resolve_empty = true,
            "UNMEASURED",
        ),
    ]
}

/// A round with exactly one canary: the second is removed everywhere.
pub fn one_planted(f: &mut Fx) {
    f.receipt["quorum"]["canary"]["planted"]
        .as_array_mut()
        .expect("planted")
        .pop();
    f.plan["planted"].as_array_mut().expect("planted").pop();
    canary(f)["caught"] = json!(["2"]);
    f.claims.as_array_mut().expect("claims").pop();
    for v in f.verdicts.as_array_mut().expect("verdicts") {
        v["verdicts"].as_object_mut().expect("map").remove("4");
    }
}
