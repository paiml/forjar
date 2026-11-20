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
    m("marker list emptied", &[("canary|plant|decoy|honeypot|tripwire|bait|sentinel'", "zzzz'")], "a spaced-out marker word is refused"),
    m("marker search is case sensitive", &[("tr 'A-Z' 'a-z' <<<\"$decoded\"", "cat <<<\"$decoded\"")], "a capitalised marker word is refused"),
    m("marker search keeps separators", &[("| tr -cd 'a-z')", "| tr -cd 'a-z-')")], "a spaced-out marker word is refused"),
    m("non-ASCII is allowed", &[("LC_ALL=C grep -q '[^ -~]' <<<\"$decoded\"", "LC_ALL=C grep -q 'ZZZZZ' <<<\"$decoded\"")], "a homoglyph lookalike of a marker word is refused"),
    m("marker search reads raw bytes, not decoded strings", &[("jq -r '[.. | strings] + [.. | objects | keys[]] | .[]' \"$1\" 2>/dev/null", "cat \"$1\"")], "a JSON-escaped marker word is refused"),
    m("a real claim may carry extra keys", &[("(keys == [\"id\", \"text\"]) and (.id | sid)", "(keys | length > 0) and (.id | sid)")], "a real claim with an extra key is refused"),
    m("planted position is not enforced", &[("$cl[$p.position - 1] !=", "($cl | map(select(.id == $p.id)) | .[0]) !="), (".value.id] != $rid;", ".value.id] != $rid and false;")], "a planted claim at the wrong position is refused"),
    m("plan is not compared", &[("== ($c.planted | proj)", "!= ($c.planted | proj)")], "a plan that differs from the receipt is refused"),
    m("planter may be a refuter or judge by name", &[("test(\"refuter|judge\"; \"i\")", "test(\"refuterx|judgex\"; \"i\")")], "a planter named refuter is refused"),
    m("judge count is not checked", &[("chk(($v | length) != $q.judges;", "chk(false;")], "two judge entries for three judges is refused"),
    m("waiver key is case sensitive", &[("test(\"waiv|override\"; \"i\")", "test(\"waiv|override\")")], "an upper-case top-level waiver key is refused"),
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
    m("ids may have a leading zero", &[("def sid: isstr and test(\"^[1-9][0-9]{0,5}$\");", "def sid: isstr and test(\"^[0-9 ]+$\");"), ("def sid: type == \"string\" and test(\"^[1-9][0-9]{0,5}$\");", "def sid: type == \"string\" and test(\"^[0-9 ]+$\");")], "a planted id with a leading zero is refused"),
    m("fewer than three lanes pass", &[("len(.quorum.lanes) < 3", "len(.quorum.lanes) < 0")], "fewer than three lanes keeps its old floor"),
    Mutant { name: "an empty predicate answer is passed on", pairs: &[("if [ -z \"$why\" ]; then\n    printf '%s\\n' \"the receipt predicate gave no answer", "if false; then\n    printf '%s\\n' \"the receipt predicate gave no answer")], kill: "an empty predicate answer is refused as unmeasured", flip: "" },
];
