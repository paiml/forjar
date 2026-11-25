//! The rest of the round-3 review cases (see cases_round3.rs).
use super::cases_review::{nop, pl, pl_text, raw_claims};
use super::*;

use super::cases_round3::claim0;

pub fn table() -> Vec<Case> {
    vec![
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
