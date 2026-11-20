//! Mutants of scripts/dogfood/lib/receipt.sh. Each is a list of (needle,
//! replacement) pairs, applied together so a guard that is written twice is
//! broken everywhere. The test runs every mutant itself and requires its named
//! kill case to FLIP: a case that wants "ok" must stop being ok, a refusal case
//! must come back "ok" (or `flip`). A mutant that merely makes everything refuse
//! does not count.
pub struct Mutant {
    pub name: &'static str,
    pub pairs: &'static [(&'static str, &'static str)],
    pub kill: &'static str,
    pub flip: &'static str,
}

const fn m(
    name: &'static str,
    pairs: &'static [(&'static str, &'static str)],
    kill: &'static str,
) -> Mutant {
    Mutant {
        name,
        pairs,
        kill,
        flip: "ok",
    }
}

#[rustfmt::skip]
pub const MUTANTS: &[Mutant] = &[
    m("stage 1 ignores the canary block", &[("(hascanary | not)\n    then \"quorum.claims_refuted", "false\n    then \"quorum.claims_refuted")], "zero refuted, no canary: the old sentence, unchanged"),
    m("planted floor of 2 drops to 1", &[("chk(($c.planted | length) < 2;", "chk(($c.planted | length) < 1;")], "planted = 1 is refused"),
    m("caught is not compared", &[("chk(($c.caught | sort) != ([$c.planted[].id] | sort);", "chk(false;")], "caught short of planted is refused"),
    m("majority becomes one judge", &[("(refd($v; $id) * 2) > ($v | length)", "refd($v; $id) >= 1")], "canary refuted by one of three is refused"),
    m("marker list emptied", &[(r#"(?<![a-z])(canar(y|ies)|plant(s|ed|ing|er|ers)?|decoy(s|ed|ing)?|honeypots?|tripwires?|bait(s|ed|ing)?|sentinels?)(?![a-z])"#, "zzzz")], "a capitalised marker word is refused"),
    m("marker search is case sensitive", &[("elif any($s[] | ascii_downcase;", "elif any($s[];")], "a capitalised marker word is refused"),
    m("spaced-out marker words are not searched", &[("\n        or (. as $t | any(stems[]; . as $w | $t | test(spaced($w)))))", ")")], "a spaced-out marker word is refused"),
    m("non-ASCII is allowed", &[(r#"any($s[]; test("[^ -~]"))"#, r#"any($s[]; test("ZZZZZ"))"#)], "a homoglyph lookalike of a marker word is refused"),
    m("marker search reads raw text, not decoded strings", &[("jq -r \"$DOGFOOD_CANARY_MARKER_JQ\"", "jq -rR \"$DOGFOOD_CANARY_MARKER_JQ\"")], "a JSON-escaped marker word is refused"),
    m("a real claim may carry extra keys", &[("(keys == [\"id\", \"text\"]) and (.id | sid)", "(keys | length > 0) and (.id | sid)")], "a real claim with an extra key is refused"),
    m("planted position is not enforced", &[("$cl[$p.position - 1] !=", "($cl | map(select(.id == $p.id)) | .[0]) !="), (".value.id] != $rid;", ".value.id] != $rid and false;")], "a planted claim at the wrong position is refused"),
    m("plan is not compared", &[("== ($c.planted | proj)", "!= ($c.planted | proj)")], "a plan that differs from the receipt is refused"),
    m("planter may be a refuter or judge by name", &[("test(\"refuter|judge\"; \"i\")", "test(\"refuterx|judgex\"; \"i\")")], "a planter named refuter is refused"),
    m("judge count is not checked", &[("chk(($v | length) != $q.judges;", "chk(false;")], "two judge entries for three judges is refused"),
    m("waiver key is case sensitive", &[("if ([paths | .[-1]? | tostring] | any(test(\"waiv|override\"; \"i\")))", "if ([paths | .[-1]? | tostring] | any(test(\"waiv|override\")))")], "an upper-case top-level waiver key is refused"),
    m("resolver drops the canary verdict", &[("if [ \"$why\" = canary ]; then\n    dogfood_canary_status \"$blob\" \"$@\"\n    return 0", "if [ \"$why\" = canary ]; then\n    printf '%s\\n' ok\n    return 0")], "planted = 1 is refused"),
    m("claims_refuted is not recomputed", &[("chk($q.claims_refuted != $realref;", "chk(false;")], "claims_refuted that disagrees with the judges is refused"),
    m("claims_confirmed is not recomputed", &[("($q | has(\"claims_confirmed\")) and", "false and")], "claims_confirmed that counts a canary is refused"),
    m("claims_reviewed is not recomputed", &[("($q | has(\"claims_reviewed\")) and", "false and")], "a receipt count that counts a canary is refused"),
    m("a round may have no real claim", &[("chk(($c.real_claim_ids | length) < 1;", "chk(($c.real_claim_ids | length) < 0;")], "a round with no real claim is refused"),
    m("falsifiers may be shared", &[("chk(([$c.planted[].falsified_by] | unique | length) != ($c.planted | length);", "chk(false;")], "two canaries sharing one falsifying line are refused"),
    m("falsifier path need not be a changed file", &[("grep -qxF -- \"$path\" <<<\"$names\" || rc=$?", ": || rc=$?")], "a directory falsifier is refused"),
    m("falsifier line need not be added", &[("dogfood_canary_added \"$path\" \"$line\" \"$base\" \"$head\" || rc=$?", ": || rc=$?")], "a falsifier on an unchanged line is refused"),
    m("a zero-count hunk counts as added", &[("if (c > 0 && n + 0 >= s && n + 0 < s + c) hit = 1", "if (n + 0 >= s) hit = 1")], "a diff with no added line cannot carry a canary"),
    m("reviewed_head is not bound", &[("if [ \"$p\" != \"$head\" ]; then\n    printf 'quorum.canary.reviewed_head", "if false; then\n    printf 'quorum.canary.reviewed_head")], "a real but other reviewed_head is refused"),
    m("reviewed_base is not bound", &[("if [ \"$p\" != \"$base\" ]; then", "if false; then")], "a real but other reviewed_base is refused"),
    m("evidence is taken from the first rev that has the path", &[("if [ \"$got\" = \"$want\" ]; then", "if [ -s \"$out\" ]; then")], "a stale copy at the first rev does not hide the matching copy"),
    m("evidence hash is not compared", &[("if [ \"$got\" = \"$want\" ]; then", "if true; then")], "a hash that matches no committed copy is refused"),
    m("an empty evidence rev is not refused", &[("dogfood_canary_commit \"$rev\" >/dev/null || {", "true || {")], "an empty evidence rev is refused"),
    m("planter may be a lane", &[("chk(($c.planted_by | norm) as $pb | any($others[]; . == $pb);", "chk(false;")], "a planter who is also a lane is refused"),
    m("planter may be a judge", &[("chk(($c.planted_by | norm) as $pb | any($v[].judge | norm; . == $pb);", "chk(false;")], "a planter who is also a judge is refused"),
    m("judge names may be blank", &[("(.judge | type == \"string\") and ((.judge | gsub(\"\\\\s\"; \"\") | length) > 0)", "(.judge | type == \"string\")")], "an empty judge name is refused"),
    m("judge names may repeat", &[("chk(([$v[].judge | norm] | unique | length) != ($v | length);", "chk(false;")], "two judges with one name are refused"),
    m("planted text length floor is dropped", &[(">= 20)", ">= 0)")], "a short planted text is refused"),
    m("blank planted text is accepted", &[(">= 20)", ">= 0)"), ("def nes: isstr and (gsub(\"\\\\s\"; \"\") | length > 0);", "def nes: isstr;"), ("(.text | gsub(\"\\\\s\"; \"\") | length) > 0)", "(.text | gsub(\"\\\\s\"; \"\") | length) > -1)")], "a whitespace-only planted text is refused"),
    m("ids may have a leading zero", &[(r#"def sid: isstr and test("\\A[1-9][0-9]{0,5}\\z");"#, r#"def sid: isstr and test("^[0-9 ]+$");"#), (r#"def sid: type == "string" and test("\\A[1-9][0-9]{0,5}\\z");"#, r#"def sid: type == "string" and test("^[0-9 ]+$");"#)], "a planted id with a leading zero is refused"),
    m("hashes accept a trailing newline", &[(r#"def hexn($n): isstr and test("\\A[0-9a-f]{\($n)}\\z");"#, r#"def hexn($n): isstr and test("^[0-9a-f]{\($n)}$");"#), (r#"chk(([$lanes, .quorum.refuters, .quorum.refuter_names, .quorum.judge_names, $c][] | [.. | strings] | any(test("[^ -~]")));"#, "chk(false;")], "a reviewed_head with a trailing newline is refused"),
    m("falsified_by accepts a leading zero line", &[(r#":[1-9][0-9]{0,8}\\z"#, r#":[0-9]+\\z"#)], "a falsified_by line with a leading zero is refused"),
    m("falsified_by accepts a trailing newline", &[(r#"{0,8}\\z"#, "{0,8}$"), (r#"chk(([$lanes, .quorum.refuters, .quorum.refuter_names, .quorum.judge_names, $c][] | [.. | strings] | any(test("[^ -~]")));"#, "chk(false;")], "a falsified_by with a trailing newline is refused"),
    m("a falsifier may be a listed evidence file", &[(r#"chk(([.evidence.files[]?.path] as $ev |"#, r#"chk(false and ([.evidence.files[]?.path] as $ev |"#)], "a falsifier in a file the receipt lists as evidence is refused"),
    m("a blank or short lane list passes", &[(r#"chk(($lanes | type) != "array" or ($lanes | length) < 3 or ($lanes | all(nes) | not);"#, "chk(false;")], "a blank lane name is refused"),
    m("duplicate lane names pass", &[(r#"chk(($lanes | map(norm) | unique | length) != ($lanes | length);"#, "chk(false;")], "duplicate lane names are refused"),
    m("claim order is not compared", &[(r#"chk([$cl | to_entries[] | select(((.key + 1) as $pos | any($pl[]; .position == $pos)) | not) | .value.id] != $rid;"#, "chk(false;")], "real claims out of order in the claims file are refused"),
    m("a verdict may be any string", &[(r#"or ([.verdicts[]] | all(. == "REFUTED" or . == "SURVIVED") | not)"#, "or false")], "a verdict that is neither REFUTED nor SURVIVED is refused"),
    m("the plan's planted type is not checked", &[(r#"(((.planted | type) == "array") and ((.planted | proj) == ($c.planted | proj)))"#, r#"(((.planted | proj) == ($c.planted | proj)))"#)], "a plan whose planted is an object is refused"),
    m("a block with no verdict token is ok", &[("elif hascanary\n    then \"canary\"", "elif false\n    then \"canary\"")], "planted = 1 is refused"),
    m("a squash-merged canary keeps the merge-commit copy", &[(r#"if [ -n "$later" ] && printf"#, r#"if false && printf"#)], "a squash-merged canary PR is judged on its post-merge receipt"),
    m("the spaced-out search has no word boundary", &[(r#"def spaced($w): "(?<![a-z])" + ($w | split("") | join("[^a-z]+")) + "(?![a-z])";"#, r#"def spaced($w): $w | split("") | join("[^a-z]+");"#)], "an honest claim holding sub(a, i) takes is accepted"),
    m("a planter is not a marker", &[("plant(s|ed|ing|er|ers)?", "plant(s|ed|ing)?")], "a planter is refused"),
    m("baiting is not a marker", &[("bait(s|ed|ing)?", "bait(s|ed)?")], "baiting is refused"),
    m("zero-width names pass", &[(r#"chk(([$lanes, .quorum.refuters, .quorum.refuter_names, .quorum.judge_names, $c][] | [.. | strings] | any(test("[^ -~]")));"#, "chk(false;")], "zero-width lane names are refused"),
    m("the verdicts file may hold non-ASCII judge names", &[(r#"chk([$v | .. | strings] | any(test("[^ -~]"));"#, "chk(false;")], "judge names that differ only by a zero-width character are refused"),
    m("planted_by may be a bare role word", &[(r#"any(["lane", "lanes", "reviewer", "reviewers", "author", "panel", "refuter", "refuters", "judge", "judges"][]; . == $pb)"#, "false")], "a planted_by that is the bare word Lane is refused"),
    Mutant { name: "an empty predicate answer is passed on", pairs: &[("if [ -z \"$why\" ]; then\n    printf '%s\\n' \"the receipt predicate gave no answer", "if false; then\n    printf '%s\\n' \"the receipt predicate gave no answer")], kill: "an empty predicate answer is refused as unmeasured", flip: "" },
];
