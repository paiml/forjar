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

/// A mutant whose guard sits behind another: it only has to change the answer.
const fn mc(
    name: &'static str,
    pairs: &'static [(&'static str, &'static str)],
    kill: &'static str,
) -> Mutant {
    Mutant {
        name,
        pairs,
        kill,
        flip: "",
    }
}

#[rustfmt::skip]
pub const MUTANTS: &[Mutant] = &[
    m("stage 1 ignores the canary block", &[("(hascanary | not)\n    then \"quorum.claims_refuted", "false\n    then \"quorum.claims_refuted")], "zero refuted, no canary: the old sentence, unchanged"),
    m("planted floor of 2 drops to 1", &[("chk(($c.planted | length) < 2;", "chk(($c.planted | length) < 1;"), (r#"[ "$n" -ge 2 ] || return 1"#, r#"[ "$n" -ge 1 ] || return 1"#)], "planted = 1 is refused"),
    m("caught is not compared", &[("chk(($c.caught | sort) != ([$c.planted[].id] | sort);", "chk(false;")], "caught short of planted is refused"),
    m("majority becomes one judge", &[("(refd($v; $id) * 2) > ($v | length)", "refd($v; $id) >= 1")], "canary refuted by one of three is refused"),
    m("marker list emptied", &[(r#"(?<![a-z])(canar(y|ies)|plant(s|ed|ing|er|ers)?|decoy(s|ed|ing)?|honeypots?|tripwires?|bait(s|ed|ing)?|sentinels?)(?![a-z])"#, "zzzz")], "a capitalised marker word is refused"),
    m("marker search is case sensitive", &[("elif any($s[] | ascii_downcase;", "elif any($s[];")], "a capitalised marker word is refused"),
    m("spaced-out marker words are not searched", &[("\n        or (. as $t | any(stems[]; . as $w | $t | test(spaced($w)))))", ")")], "a spaced-out marker word is refused"),
    m("non-ASCII is allowed", &[(r#"any($s[]; test("[^ -~]"))"#, r#"any($s[]; test("ZZZZZ"))"#), (r#"out="$(dogfood_canary_rawall "${tmp}/${role}" "$role" bytes)""#, r#"out=ok"#)], "a homoglyph lookalike of a marker word is refused"),
    m("marker search reads raw text, not decoded strings", &[("jq -r \"$DOGFOOD_CANARY_MARKER_JQ\"", "jq -rR \"$DOGFOOD_CANARY_MARKER_JQ\""), (r#""$f" "$role" nul dup lex)"#, r#""$f" "$role" nul dup)"#)], "a JSON-escaped marker word is refused"),
    m("a real claim may carry extra keys", &[("(keys == [\"id\", \"text\"]) and (.id | sid)", "(keys | length > 0) and (.id | sid)")], "a real claim with an extra key is refused"),
    m("planted position is not enforced", &[("$cl[$p.position - 1] !=", "($cl | map(select(.id == $p.id)) | .[0]) !="), (".value.id] != $rid;", ".value.id] != $rid and false;")], "a planted claim at the wrong position is refused"),
    m("plan is not compared", &[("== ($c.planted | proj)", "!= ($c.planted | proj)")], "a plan that differs from the receipt is refused"),
    m("planter may be a refuter or judge by name", &[("test(\"refuter|judge\"; \"i\")", "test(\"refuterx|judgex\"; \"i\")"), (r#"any(["lane", "review", "author", "panel", "judge", "refut"][]; . as $w | $pl | contains($w))"#, "false")], "a planter named refuter is refused"),
    m("judge count is not checked", &[("chk(($v | length) != $q.judges;", "chk(false;")], "two judge entries for three judges is refused"),
    m("waiver key is case sensitive", &[("if ([paths | .[-1]? | tostring] | any(test(\"waiv|override\"; \"i\")))", "if ([paths | .[-1]? | tostring] | any(test(\"waiv|override\")))")], "an upper-case top-level waiver key is refused"),
    m("resolver drops the canary verdict", &[("if [ \"$why\" = canary ]; then\n    dogfood_canary_status \"$blob\" \"$@\"\n    return 0", "if [ \"$why\" = canary ]; then\n    printf '%s\\n' ok\n    return 0")], "planted = 1 is refused"),
    m("claims_refuted is not recomputed", &[("chk($q.claims_refuted != $realref;", "chk(false;")], "claims_refuted that disagrees with the judges is refused"),
    m("claims_confirmed is not recomputed", &[("($q | has(\"claims_confirmed\")) and", "false and")], "claims_confirmed that counts a canary is refused"),
    m("claims_reviewed is not recomputed", &[("($q | has(\"claims_reviewed\")) and", "false and")], "a receipt count that counts a canary is refused"),
    m("a round may have no real claim", &[("chk(($c.real_claim_ids | length) < 1;", "chk(($c.real_claim_ids | length) < 0;")], "a round with no real claim is refused"),
    m("falsifiers may be shared", &[("chk(([$c.planted[].falsified_by] | unique | length) != ($c.planted | length);", "chk(false;")], "two canaries sharing one falsifying line are refused"),
    m("falsifier path need not be a changed file", &[(r#"if [ "$nm" = "$path" ]; then"#, "if true; then")], "a directory falsifier is refused"),
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
    m("planted text length floor is dropped", &[(">= 20)", ">= 0)"), (r#" and (([.text | scan("[^\\s]+")] | length) >= 4)"#, "")], "a short planted text is refused"),
    m("blank planted text is accepted", &[(">= 20)", ">= 0)"), (r#" and (([.text | scan("[^\\s]+")] | length) >= 4)"#, ""), ("def nes: isstr and (gsub(\"\\\\s\"; \"\") | length > 0);", "def nes: isstr;"), ("(.text | gsub(\"\\\\s\"; \"\") | length) > 0)", "(.text | gsub(\"\\\\s\"; \"\") | length) > -1)")], "a whitespace-only planted text is refused"),
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
    m("a squash-merged canary keeps the merge-commit copy", &[(r#"      0)
        printf '%s' "$later""#, r#"      9)
        printf '%s' "$later""#)], "a squash-merged canary PR is judged on its post-merge receipt"),
    m("the spaced-out search has no word boundary", &[(r#"def spaced($w): "(?<![a-z])" + ($w | split("") | join("[ ._*|/-]{1,3}")) + "(?![a-z])";"#, r#"def spaced($w): $w | split("") | join("[ ._*|/-]{1,3}");"#)], "a spaced word inside a longer word is honest prose"),
    m("the spaced-out separator is any non-letter", &[(r#"join("[ ._*|/-]{1,3}")) + "(?![a-z])""#, r#"join("[^a-z]+")) + "(?![a-z])""#)], "b = a[i] + t is honest prose"),
    m("spaced inflections are not searched", &[(r#""canary", "canaries", "plant", "plants","#, r#""canary", "plant","#)], "a spaced plural inflection is refused"),
    m("a duplicate leaf path is allowed", &[(r#"if ($L | length) != ($L | unique | length) then "dup""#, r#"if false then "dup""#)], "an identical duplicate id in a claim is refused"),
    m("a leaf that is also a container is allowed", &[(r#"elif any($L[]; . as $l"#, r#"elif false and any($L[]; . as $l"#)], "an empty object then a filled one under the same key in the plan file is refused"),
    m("two same-key containers are allowed", &[(r#"elif ($C | length) != ($C | unique | length) then "dup""#, r#"elif false then "dup""#)], "two same-key objects with disjoint members in the plan file are refused"),
    m("the duplicate check is skipped for evidence files", &[(r#"out="$(dogfood_canary_rawall "$f" "$role" nul dup lex)""#, r#"out=ok"#)], "a shadowed marker text in a duplicate claim text is refused"),
    m("the duplicate check is skipped for the receipt", &[(r#"why="$(dogfood_canary_rawall "$tmp" receipt nul dup lex)""#, r#"why=ok"#)], "a duplicate key at the top of the receipt is refused"),
    m("raw bytes are not checked in evidence files", &[(r#"out="$(dogfood_canary_rawall "${tmp}/${role}" "$role" bytes)""#, r#"out=ok"#)], "a raw non-ASCII byte in an extra plan key is refused"),
    m("raw bytes are not checked in the receipt", &[(r#"why="$(dogfood_canary_rawall "$tmp" receipt bytes)""#, r#"why=ok"#)], "a raw non-ASCII byte in an extra receipt key is refused"),
    m("the raw byte set admits non-ASCII", &[(r#"'[^\x09\x0a\x0d\x20-\x7e]'"#, r#"'[^\x09\x0a\x0d\x20-\x7e\x80-\xff]'"#)], "a raw non-ASCII byte in an extra plan key is refused"),
    m("planted_by may be spaced out", &[(r#"test("\\A[a-z][a-z0-9-]*\\z")) | not;"#, r#"test("")) | not;"#), (r#"any(["lane", "review", "author", "panel", "judge", "refut"][]; . as $w | $pl | contains($w))"#, "false")], "a spaced planted_by ref uter-1 is refused"),
    m("planted_by may contain a role word", &[(r#"any(["lane", "review", "author", "panel", "judge", "refut"][]; . as $w | $pl | contains($w))"#, "false")], "a hyphenated planted_by ref-uter-1 is refused"),
    m("a planted text needs no words", &[(r#" and (([.text | scan("[^\\s]+")] | length) >= 4)"#, "")], "a planted text of two sparse words is refused"),
    m("a planter is not a marker", &[("plant(s|ed|ing|er|ers)?", "plant(s|ed|ing)?")], "a planter is refused"),
    m("baiting is not a marker", &[("bait(s|ed|ing)?", "bait(s|ed)?")], "baiting is refused"),
    m("zero-width names pass", &[(r#"chk(([$lanes, .quorum.refuters, .quorum.refuter_names, .quorum.judge_names, $c][] | [.. | strings] | any(test("[^ -~]")));"#, "chk(false;"), (r#"why="$(dogfood_canary_rawall "$tmp" receipt bytes)""#, r#"why=ok"#)], "zero-width lane names are refused"),
    m("the verdicts file may hold non-ASCII judge names", &[(r#"chk([$v | .. | strings] | any(test("[^ -~]"));"#, "chk(false;"), (r#"out="$(dogfood_canary_rawall "${tmp}/${role}" "$role" bytes)""#, r#"out=ok"#)], "judge names that differ only by a zero-width character are refused"),
    m("planted_by may be a bare role word", &[(r#"any(["lane", "lanes", "reviewer", "reviewers", "author", "authors", "panel", "panels", "refuter", "refuters", "judge", "judges"][]; . == $pb)"#, "false"), (r#"any(["lane", "review", "author", "panel", "judge", "refut"][]; . as $w | $pl | contains($w))"#, "false")], "a planted_by that is the bare word Lane is refused"),
    Mutant { name: "an empty predicate answer is passed on", pairs: &[("if [ -z \"$why\" ]; then\n    printf '%s\\n' \"the receipt predicate gave no answer", "if false; then\n    printf '%s\\n' \"the receipt predicate gave no answer")], kill: "an empty predicate answer is refused as unmeasured", flip: "" },
    m("the awk answer is its exit status, not a word", &[("    hit) return 0 ;;\n    miss) return 1 ;;\n    *) return 2 ;;", "    miss) return 1 ;;\n    *) return 0 ;;")], "an awk that prints nothing and exits 0 is refused"),
    mc("a grep that finds nothing needs no positive control", &[(r#"if [ "$prc" -ne 0 ] || [ "$nrc" -ne 1 ]; then"#, r#"if [ "$nrc" -ne 1 ]; then"#)], "a grep that prints nothing and exits 1 is refused"),
    mc("a NUL byte is not looked for in a file", &[(r#"out="$(dogfood_pgrep a '\x00' "$file" 'a\0000b' 'ab')"
      case "$out" in
        miss) printf 'ok\n' ;;"#, r#"out="miss"
      case "$out" in
        miss) printf 'ok\n' ;;"#)], "a NUL byte in the plan file is refused"),
    mc("a git that answers nothing is a commit", &[(r#"  case "$out" in
    *[!0-9a-f]* | "") return 1 ;;
  esac
  [ "${#out}" -eq 40 ] || [ "${#out}" -eq 64 ] || return 1
"#, "")], "a git that prints nothing and exits 0 is refused"),
    mc("a second top-level value is not counted", &[("        1) ;;\n        unreadable", "        [0-9]*) ;;\n        unreadable")], "a second top-level value after the plan file is refused"),
];
