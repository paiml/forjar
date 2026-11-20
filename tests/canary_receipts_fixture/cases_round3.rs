//! The cases that reproduce the second review of f7a2012: ids and hashes with a
//! trailing newline, path:line forms, a falsifier inside the receipt's own
//! evidence, honest prose that held the letters of a marker word, and the
//! squash-merge shape. Each is a receipt the f7a2012 predicate got wrong.
use super::cases_review::{nop, pl, pl_text, raw_claims, rename, s};
use super::*;

fn newline_id(f: &mut Fx) {
    rename(f, "1", "1\n");
    canary(f)["real_claim_ids"] = json!(["1\n", "3"]);
}

fn own_evidence(f: &mut Fx) {
    canary(f)["reviewed_head"] = json!("@EVID");
    pl(
        f,
        0,
        "falsified_by",
        json!(".quorum/evidence/claims.json:1"),
    );
    pl(
        f,
        1,
        "falsified_by",
        json!(".quorum/evidence/claims.json:2"),
    );
}

fn claim0(f: &mut Fx, t: &str) {
    set_claims(f, 0, "text", json!(t));
}

pub fn table() -> Vec<Case> {
    vec![
        // 4/5. canonical path:line, anchored ids and hashes.
        (
            "a claim id with a trailing newline is refused",
            newline_id,
            nop,
            "plain number",
        ),
        (
            "a planted id with a trailing newline is refused",
            |f| {
                pl(f, 0, "id", json!("2\n"));
                canary(f)["caught"] = json!(["2\n", "4"]);
            },
            nop,
            "plain numeric id",
        ),
        (
            "a reviewed_head with a trailing newline is refused",
            |f| canary(f)["reviewed_head"] = json!("@HEAD\n"),
            nop,
            "not a 40-hex",
        ),
        (
            "a falsified_by line with a leading zero is refused",
            |f| pl(f, 1, "falsified_by", json!("src/a.rs:05")),
            nop,
            "falsified_by (path:line",
        ),
        (
            "a falsified_by with a trailing newline is refused",
            |f| pl(f, 1, "falsified_by", json!("src/a.rs:5\n")),
            nop,
            "falsified_by (path:line",
        ),
        (
            "a falsified_by with a ./ prefix is refused",
            |f| pl(f, 0, "falsified_by", json!("./src/a.rs:4")),
            nop,
            "falsified_by (path:line",
        ),
        (
            "a falsified_by with line zero is refused",
            |f| pl(f, 0, "falsified_by", json!("src/a.rs:0")),
            nop,
            "falsified_by (path:line",
        ),
        // 3. the receipt's own evidence is no falsifying fact.
        (
            "a falsifier under .quorum/ on a deletion-only diff is refused",
            own_evidence,
            |o| {
                o.delete_only = true;
                o.evid_head = true;
                o.subj_head = s("@EVID");
            },
            "lies under .quorum/",
        ),
        (
            "a falsifier under .pmat/ is refused",
            |f| pl(f, 0, "falsified_by", json!(".pmat/baseline.json:1")),
            nop,
            "lies under .quorum/ or .pmat/",
        ),
        (
            "a falsifier in a file the receipt lists as evidence is refused",
            |_| {},
            |o| o.extra_ev = Some("src/a.rs"),
            "names a file the receipt lists as evidence",
        ),
        // 6. honest prose is not a marker; disguised markers still are.
        (
            "an honest claim about the forjar plan is accepted",
            |f| {
                claim0(
                    f,
                    "The forjar plan takes the target state and adds added_one to src/a.rs.",
                )
            },
            nop,
            "ok",
        ),
        (
            "a claim ending in plan before a claim starting with The is accepted",
            |f| claim0(f, "The change updates the execution plan."),
            nop,
            "ok",
        ),
        (
            "a claim about a planned change is accepted",
            |f| {
                claim0(
                    f,
                    "The planned change adds added_one to src/a.rs at line four.",
                )
            },
            nop,
            "ok",
        ),
        (
            "a separated marker word is refused",
            |_| {},
            |o| o.claims_raw = raw_claims("The c a n a r y claim adds added_two to src/a.rs."),
            "were not blind",
        ),
        (
            "a plural marker word is refused",
            |f| claim0(f, "Two decoys were added to src/a.rs at line four."),
            nop,
            "were not blind",
        ),
        (
            "a newline inside a claim string is refused",
            |_| {},
            |o| o.claims_raw = raw_claims("The change adds added_two\\nto src/a.rs."),
            "non-ASCII or control",
        ),
        // non-blocking.
        (
            "an evidence file that is not committed anywhere is refused",
            |_| {},
            |o| o.no_commit = Some("verdicts.json"),
            "is not committed at any of",
        ),
        (
            "an empty lane list is refused",
            |f| f.receipt["quorum"]["lanes"] = json!([]),
            nop,
            "quorum.lanes = 0",
        ),
        (
            "a blank lane name is refused",
            |f| f.receipt["quorum"]["lanes"] = json!(["a", "b", " "]),
            nop,
            "blank or non-string lane name",
        ),
        (
            "duplicate lane names are refused",
            |f| f.receipt["quorum"]["lanes"] = json!(["a", "A ", "c"]),
            nop,
            "lanes names are not distinct",
        ),
        (
            "a duplicate planted id is refused",
            |f| pl(f, 1, "id", json!("2")),
            nop,
            "planted[].id are not distinct",
        ),
        (
            "a duplicate planted text is refused",
            |f| pl(f, 1, "text", json!(P_DELETE)),
            nop,
            "planted[].text are not distinct",
        ),
        (
            "a planted position beyond the claims file is refused",
            |f| pl(f, 1, "position", json!(9)),
            nop,
            "lies beyond the claims file",
        ),
        (
            "real claims out of order in the claims file are refused",
            |f| f.claims.as_array_mut().expect("claims").swap(0, 2),
            nop,
            "not quorum.canary.real_claim_ids in order",
        ),
        (
            "a verdict that is neither REFUTED nor SURVIVED is refused",
            |f| f.verdicts[0]["verdicts"]["1"] = json!("MAYBE"),
            nop,
            "every judge must answer every claim",
        ),
        // 2. squash merges: the PR-branch receipt is bound to a commit that is
        // not on main; the receipt committed after the merge is.
        (
            "a squash-merged canary PR with only its pre-merge receipt is refused",
            |_| {},
            |o| o.squash = 1,
            "reviewed_head",
        ),
        (
            "a squash-merged canary PR is judged on its post-merge receipt",
            |_| {},
            |o| o.squash = 2,
            "ok",
        ),
        // round 4 (6ebdc6ea): word boundaries on the spaced-out search.
        (
            "an honest claim holding sub(a, i) takes is accepted",
            |f| {
                claim0(
                    f,
                    "The helper sub(a, i) takes a slice and adds added_one to src/a.rs.",
                )
            },
            nop,
            "ok",
        ),
        (
            "an honest claim holding a[i] triggers is accepted",
            |f| {
                claim0(
                    f,
                    "In src/lib, a[i] triggers a rebuild of the target state.",
                )
            },
            nop,
            "ok",
        ),
        (
            "an honest claim holding a [i] token is accepted",
            |f| claim0(f, "For each job, a [i] token is parsed from the manifest."),
            nop,
            "ok",
        ),
        (
            "an honest claim holding an n-tuple after a step is accepted",
            |f| {
                claim0(
                    f,
                    "At the step l: a n-tuple of hosts is read from src/a.rs.",
                )
            },
            nop,
            "ok",
        ),
        (
            "a compound with a marker word inside is accepted (words, not stems)",
            |f| {
                claim0(
                    f,
                    "The plantation module adds added_one to src/a.rs at line four.",
                )
            },
            nop,
            "ok",
        ),
        (
            "a planter is refused",
            |f| claim0(f, "The planter adds added_one to src/a.rs at line four."),
            nop,
            "were not blind",
        ),
        (
            "baiting is refused",
            |f| claim0(f, "Baiting the refuters adds added_one to src/a.rs."),
            nop,
            "were not blind",
        ),
        (
            "a spaced marker at the very start of a claim is still refused",
            |_| {},
            |o| o.claims_raw = raw_claims("c-a-n-a-r-y adds added_two to src/a.rs."),
            "were not blind",
        ),
        // round 4: names and texts are ASCII printable, compared trimmed and lower-cased.
        (
            "zero-width lane names are refused",
            |f| f.receipt["quorum"]["lanes"] = json!(["\u{200b}", "\u{200c}", "\u{200d}"]),
            nop,
            "ASCII printable only",
        ),
        (
            "a zero-width character inside a lane name is refused",
            |f| f.receipt["quorum"]["lanes"] = json!(["a", "a\u{200b}", "c"]),
            nop,
            "ASCII printable only",
        ),
        (
            "a zero-width planted_by that spells refuter is refused",
            |f| canary(f)["planted_by"] = json!("ref\u{200b}uter-J1"),
            nop,
            "ASCII printable only",
        ),
        (
            "a zero-width joiner in planted_by is refused",
            |f| canary(f)["planted_by"] = json!("cou\u{200d}rier"),
            nop,
            "ASCII printable only",
        ),
        (
            "a zero-width non-joiner in a planted text is refused",
            |f| {
                pl(
                    f,
                    0,
                    "text",
                    json!(
                        "The change deletes the function\u{200c} added_one from src/a.rs entirely."
                    ),
                )
            },
            nop,
            "ASCII printable only",
        ),
        (
            "a control character in a refuter name is refused",
            |f| f.receipt["quorum"]["refuter_names"] = json!(["r1", "r2\u{0007}", "r3"]),
            nop,
            "strict JSON text",
        ),
        (
            "judge names that differ only by a zero-width character are refused",
            |f| {
                f.verdicts[1]["judge"] = json!("J1\u{200b}");
                f.verdicts[2]["judge"] = json!("J1\u{200c}");
            },
            nop,
            "verdicts_file must be ASCII printable only",
        ),
        (
            "a planted_by equal to a judge after trimming and case is refused",
            |f| canary(f)["planted_by"] = json!("  j1 "),
            nop,
            "also a judge in the verdicts file",
        ),
        (
            "a planted_by that is the bare word Lane is refused",
            |f| canary(f)["planted_by"] = json!(" Lane "),
            nop,
            "bare role word",
        ),
        (
            "a planted_by that is the bare word reviewer is refused",
            |f| canary(f)["planted_by"] = json!("REVIEWER"),
            nop,
            "bare role word",
        ),
        // round 5: duplicate keys in every file the canary path reads, raw bytes, role names.
        (
            "an identical duplicate id in a claim is refused",
            |_| {},
            |o| {
                o.claims_raw = raw_claims("The change adds added_two to src/a.rs at line five.")
                    .map(|s| s.replacen("\"id\":\"1\"", "\"id\":\"1\",\"id\":\"1\"", 1))
            },
            "duplicate JSON key",
        ),
        (
            "a shadowed marker text in a duplicate claim text is refused",
            |_| {},
            |o| {
                o.claims_raw = raw_claims("The change adds added_two to src/a.rs at line five.")
                    .map(|s| {
                        s.replacen(
                            "{\"id\":\"3\",\"text\":",
                            "{\"id\":\"3\",\"text\":\"PLANTED CANARY DECOY\",\"text\":",
                            1,
                        )
                    })
            },
            "duplicate JSON key",
        ),
        (
            "a duplicate key at the top of the plan file is refused",
            |_| {},
            |o| {
                o.body_edit = Some(("plan.json", |s| {
                    s.replacen('{', "{\"extra\": 1, \"extra\": 1,", 1)
                }))
            },
            "duplicate JSON key",
        ),
        (
            "a duplicate planted id in the plan file is refused",
            |_| {},
            |o| {
                o.body_edit = Some(("plan.json", |s| {
                    s.replacen("\"id\": \"2\"", "\"id\": \"2\",\n\"id\": \"2\"", 1)
                }))
            },
            "duplicate JSON key",
        ),
        (
            "two same-key objects with disjoint members in the plan file are refused",
            |_| {},
            |o| {
                o.body_edit = Some(("plan.json", |s| {
                    s.replacen('{', "{\"a\": {\"x\": 1}, \"a\": {\"y\": 2},", 1)
                }))
            },
            "duplicate JSON key",
        ),
        (
            "an empty object then a filled one under the same key in the plan file is refused",
            |_| {},
            |o| {
                o.body_edit = Some(("plan.json", |s| {
                    s.replacen('{', "{\"a\": {}, \"a\": {\"x\": 1},", 1)
                }))
            },
            "duplicate JSON key",
        ),
        (
            "a duplicate judge key in the verdicts file is refused",
            |_| {},
            |o| {
                o.body_edit = Some(("verdicts.json", |s| {
                    s.replacen('{', "{\"judge\": \"J1\",", 1)
                }))
            },
            "duplicate JSON key",
        ),
        (
            "a duplicate verdict id deeper in the verdicts file is refused",
            |_| {},
            |o| {
                o.body_edit = Some(("verdicts.json", |s| {
                    s.replacen("\"verdicts\": {", "\"verdicts\": {\"1\": \"SURVIVED\",", 1)
                }))
            },
            "duplicate JSON key",
        ),
        (
            "a duplicate key at the top of the receipt is refused",
            |_| {},
            |o| o.rec_edit = Some(|s| s.replacen('{', "{\"issue\":\"x\",\"issue\":\"x\",", 1)),
            "duplicate JSON key",
        ),
        (
            "a duplicate planted_by inside the canary block is refused",
            |_| {},
            |o| {
                o.rec_edit = Some(|s| {
                    s.replacen("\"canary\":{", "\"canary\":{\"planted_by\":\"courier\",", 1)
                })
            },
            "duplicate JSON key",
        ),
        (
            "a raw non-ASCII byte in an extra plan key is refused",
            |_| {},
            |o| {
                o.body_edit = Some(("plan.json", |s| {
                    s.replacen('{', "{\"note\": \"em \u{2014} dash\",", 1)
                }))
            },
            "outside printable ASCII",
        ),
        (
            "a raw non-ASCII byte in an extra receipt key is refused",
            |_| {},
            |o| o.rec_edit = Some(|s| s.replacen('{', "{\"issue\":\"PMAT \u{2014} x\",", 1)),
            "outside printable ASCII",
        ),
        (
            "a spaced word inside a longer word is honest prose",
            |f| claim0(f, "The slab-a-i-t helper adds added_one to src/a.rs."),
            nop,
            "ok",
        ),
        (
            "hash_b(a, i) == t_expected is honest prose",
            |f| {
                claim0(
                    f,
                    "The check hash_b(a, i) == t_expected passes in src/a.rs.",
                )
            },
            nop,
            "ok",
        ),
        (
            "b = a[i] + t is honest prose",
            |f| claim0(f, "The loop sets b = a[i] + t in src/a.rs."),
            nop,
            "ok",
        ),
        (
            "type params <B, A, I, T> are honest prose",
            |f| claim0(f, "Type params <B, A, I, T> are unconstrained in src/a.rs."),
            nop,
            "ok",
        ),
        (
            "parameters (p, l, a, n, t) are honest prose",
            |f| claim0(f, "Parameters (p, l, a, n, t) are passed in order."),
            nop,
            "ok",
        ),
        (
            "a spaced plural inflection is refused",
            |f| claim0(f, "The c-a-n-a-r-i-e-s are loud in src/a.rs."),
            nop,
            "were not blind",
        ),
        (
            "a planted_by of authors is refused",
            |f| canary(f)["planted_by"] = json!("authors"),
            nop,
            "bare role word",
        ),
        (
            "a planted_by of panels is refused",
            |f| canary(f)["planted_by"] = json!("panels"),
            nop,
            "bare role word",
        ),
        (
            "a spaced planted_by ref uter-1 is refused",
            |f| canary(f)["planted_by"] = json!("ref uter-1"),
            nop,
            "plain name",
        ),
        (
            "a spaced planted_by j u d g e-1 is refused",
            |f| canary(f)["planted_by"] = json!("j u d g e-1"),
            nop,
            "plain name",
        ),
        (
            "a hyphenated planted_by ref-uter-1 is refused",
            |f| canary(f)["planted_by"] = json!("ref-uter-1"),
            nop,
            "contains a role word",
        ),
        (
            "a planted text of two sparse words is refused",
            |f| pl_text(f, 0, "aaaaaaaaaaaaaaaaaaaaaaaa      bbbbbbbbbbbb"),
            nop,
            "4+ words",
        ),
    ]
}
