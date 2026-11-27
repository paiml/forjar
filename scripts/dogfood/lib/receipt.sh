#!/usr/bin/env bash
# The ONE predicate that says whether a committed quorum receipt is a quorum.
# Sourced by scripts/dogfood/quorum.sh (gate E, before the tag) and by
# scripts/dogfood/release-check.sh Arm 5 (after the tag), so the two cannot
# drift apart: the fourth PMAT-163 merge review found Arm 5 checking a
# top-level `waived` key and the lane and judge floors only, while gate E
# also refused nested waiver/override keys, thin refuter counts, a round that
# killed no claim, and a receipt citing no evidence. A release's last check
# must be at least as strict as its first.
# `dogfood_receipt_status BLOB [HEAD BASE [EVREV...]]` prints "ok" or one
# sentence naming the first floor the receipt is under. Not valid JSON is a
# reason too, never a pass. HEAD and BASE are the commits the caller says the
# receipt vouches for (a merge commit and its first parent); only a canary
# block reads them, and a canary block without them is refused.
#
# TWO SHAPES CLEAR THE "did the round hunt" FLOOR (PMAT-1170, forjar#990):
#   1. quorum.claims_refuted >= 1: a claim was killed.
#   2. a `quorum.canary` block: a clean round that proves it hunted, because
#      it was handed >= 2 planted FALSE claims, indistinguishable in form from
#      the lanes' claims, and a majority of judges refuted every one. A round
#      that refuted nothing and carries no valid block is refused with the same
#      sentence as before. Nothing is grandfathered: an old 0-refuted receipt
#      has no block, so it is refused and its round must be re-run.
# The block is verified against COMMITTED files read at EVREV... (default: HEAD
# arg, then the checked-out HEAD): the copy whose sha256 matches wins. Its
# reviewed_head and reviewed_base must BE the HEAD and BASE the caller passed.
# A diff with no added line (a deletion-only change) cannot carry a canary.

DOGFOOD_RECEIPT_JQ='
  def num($v): if ($v | type) == "number" then $v else -1 end;
  def len($v): if ($v | type) == "array" then ($v | length) else -1 end;
  def hascanary: (.quorum | type == "object" and has("canary"));
  hascanary as $hc
  | if ([paths | .[-1]? | tostring] | any(test("waiv|override"; (if $hc then "i" else null end))))
    then "carries a waiver/override key: a waived quorum is an unrefuted claim, not a passed one"
  elif len(.quorum.lanes) < 3
    then "quorum.lanes = \(len(.quorum.lanes)), below the floor of 3 that scripts/quorum-gate.sh enforces at push time"
  elif num(.quorum.judges) < 3
    then "quorum.judges = \(num(.quorum.judges)), below the floor of 3: a majority needs three"
  elif num(.quorum.refuters_per_claim) < 3
    then "quorum.refuters_per_claim = \(num(.quorum.refuters_per_claim)), below the floor of 3: a claim one refuter failed to kill only met one refuter"
  elif num(.quorum.claims_refuted) < 1 and (hascanary | not)
    then "quorum.claims_refuted = \(num(.quorum.claims_refuted)): a round that killed no claim did not hunt, which is the vacuous receipt this floor exists to refuse"
  elif len(.evidence.files) < 1
    then "evidence.files is empty: a verdict that cites no file is an opinion"
  elif hascanary
    then "canary"
  else "ok"
  end
'

# A canary block and ANY waiver/override key, at any depth, in any case, with
# any value (an empty reason, false, null, a nested key), never coexist. Run it
# with `jq -e`: exit 0 = the two are together (refuse), 1 = not. The push gate
# runs it BEFORE its waiver early exit, which only reads waived.reason.
DOGFOOD_CANARY_WAIVER_JQ='
  (.quorum | type == "object" and has("canary"))
  and ([paths | .[-1]? | tostring] | any(test("waiv|override"; "i")))
'

# Stage 2a: the SHAPE of quorum.canary and its pointers into evidence.files.
# Every condition is "bad when true"; one that raises (a wrong type) is bad too.
DOGFOOD_CANARY_SHAPE_JQ='
  def isstr: type == "string";
  def nes: isstr and (gsub("\\s"; "") | length > 0);
  def sid: isstr and test("\\A[1-9][0-9]{0,5}\\z");
  def norm: tostring | ascii_downcase | gsub("\\s"; "");
  def hexn($n): isstr and test("\\A[0-9a-f]{\($n)}\\z");
  def chk(c; m): [(try ([c] | any) catch true), m];
  def evok($c; $f; $role):
    (($c[$f] | nes)
     and ([.evidence.files[]? | select(.path == $c[$f]
          and ((.roles // []) | type == "array" and (index($role) != null))
          and (.sha256 | hexn(64)))] | length) == 1);
  ([.quorum | (.lanes, .refuters, .refuter_names, .judge_names)? | arrays | .[] | norm]) as $others
  | (.quorum.lanes) as $lanes
  | .quorum.canary as $c
  | [
    chk(($c | type) != "object"; "quorum.canary is not an object"),
    chk(($lanes | type) != "array" or ($lanes | length) < 3 or ($lanes | all(nes) | not);
        "quorum.lanes is empty, short, or holds a blank or non-string lane name"),
    chk(($lanes | map(norm) | unique | length) != ($lanes | length); "quorum.lanes names are not distinct: one lane counted twice"),
    chk((($c.planted_by | nes) | not); "quorum.canary.planted_by is missing: a canary round names who planted it"),
    chk(($c.planted_by | norm) as $pb | any(["lane", "lanes", "reviewer", "reviewers", "author", "authors", "panel", "panels", "refuter", "refuters", "judge", "judges"][]; . == $pb);
        "quorum.canary.planted_by is a bare role word, not a name: the planter must be named"),
    chk(($c.planted_by | test("refuter|judge"; "i")); "quorum.canary.planted_by names a refuter or a judge: the planter must be neither"),
    chk(($c.planted_by | norm) as $pb | any($others[]; . == $pb); "quorum.canary.planted_by is also a lane, refuter or judge name: the planter must be none of them"),
    chk((($c.reviewed_base | hexn(40)) | not); "quorum.canary.reviewed_base is not a 40-hex commit id"),
    chk((($c.reviewed_head | hexn(40)) | not); "quorum.canary.reviewed_head is not a 40-hex commit id"),
    chk((($c.real_claim_ids | type) != "array"); "quorum.canary.real_claim_ids is not an array"),
    chk(($c.real_claim_ids | length) < 1; "quorum.canary.real_claim_ids is empty: a round with no real claim reviewed nothing"),
    chk(($c.real_claim_ids | all(sid)) | not; "quorum.canary.real_claim_ids holds an id that is not a plain number without a leading zero"),
    chk(($c.real_claim_ids | unique | length) != ($c.real_claim_ids | length); "quorum.canary.real_claim_ids are not distinct"),
    chk(($c.planted | type) != "array"; "quorum.canary.planted is not an array"),
    chk(($c.planted | length) < 2; "quorum.canary.planted has \(try ($c.planted | length) catch "?") planted claim(s), below the floor of 2"),
    chk(($c.planted | all(.[];
         (type == "object") and (.id | sid)
         and ((.text | nes) and ((.text | gsub("\\s"; "") | length) >= 20) and (([.text | scan("[^\\s]+")] | length) >= 4))
         and ((.falsified_by | isstr) and (.falsified_by | test("\\A(?!\\./)[A-Za-z0-9_./-]+:[1-9][0-9]{0,8}\\z")) and ((.falsified_by | test("\\.\\.")) | not))
         and ((.position | type) == "number") and (.position == (.position | floor)) and (.position >= 1))) | not;
        "quorum.canary.planted[] needs a plain numeric id, text (20+ non-blank chars in 4+ words), falsified_by (path:line, no ..) and an integer position >= 1"),
    chk(any($c.planted[].falsified_by; test("\\A(\\.quorum|\\.pmat)/"));
        "quorum.canary.planted[].falsified_by lies under .quorum/ or .pmat/: a receipt cannot falsify a claim with its own evidence"),
    chk(([.evidence.files[]?.path] as $ev | any($c.planted[].falsified_by; sub(":[0-9]+\\z"; "") as $fp | any($ev[]; . == $fp)));
        "quorum.canary.planted[].falsified_by names a file the receipt lists as evidence"),
    chk(([$c.planted[].id] | unique | length) != ($c.planted | length); "quorum.canary.planted[].id are not distinct"),
    chk(([$c.planted[].text | norm] | unique | length) != ($c.planted | length); "quorum.canary.planted[].text are not distinct"),
    chk(([$c.planted[].falsified_by] | unique | length) != ($c.planted | length); "quorum.canary.planted[].falsified_by are not distinct: two canaries may not share one falsifying line"),
    chk(([$c.planted[].position] | unique | length) != ($c.planted | length); "quorum.canary.planted[].position are not distinct"),
    chk(any($c.planted[]; .position > (($c.real_claim_ids | length) + ($c.planted | length)));
        "quorum.canary.planted[].position lies beyond the claims file"),
    chk(any($c.planted[].id; . as $i | any($c.real_claim_ids[]; . == $i));
        "quorum.canary.planted[].id also appears in real_claim_ids: a canary counted as a real claim"),
    chk(($c.caught | sort) != ([$c.planted[].id] | sort);
        "quorum.canary.caught does not equal the planted ids: a missed canary means the round did not hunt"),
    chk(any([$c.claims_file, $c.plan_file, $c.verdicts_file][];
            ((try test("\\A\\.quorum/evidence/[A-Za-z0-9_./-]+\\z") catch false) | not) or (try test("\\.\\.") catch true));
        "quorum.canary names an evidence file outside .quorum/evidence/"),
    chk((evok($c; "claims_file"; "claims-blind")) | not;
        "quorum.canary.claims_file must be one evidence.files entry with role claims-blind and a sha256"),
    chk((evok($c; "plan_file"; "canary-plan")) | not;
        "quorum.canary.plan_file must be one evidence.files entry with role canary-plan and a sha256"),
    chk((evok($c; "verdicts_file"; "judge-verdicts")) | not;
        "quorum.canary.verdicts_file must be one evidence.files entry with role judge-verdicts and a sha256"),
    chk(([$c.claims_file, $c.plan_file, $c.verdicts_file] | unique | length) != 3;
        "quorum.canary claims_file, plan_file and verdicts_file are not three distinct files"),
    chk(([$lanes, .quorum.refuters, .quorum.refuter_names, .quorum.judge_names, $c][] | [.. | strings] | any(test("[^ -~]")));
        "quorum names and canary texts must be ASCII printable only: a zero-width, non-ASCII or control character can make two names look different"),
    chk(($c.planted_by | ascii_downcase | gsub("^\\s+|\\s+$"; "") | test("\\A[a-z][a-z0-9-]*\\z")) | not;
        "quorum.canary.planted_by must be a plain name ([a-z][a-z0-9-]*, no spaces): a spaced-out role word is not a name"),
    chk(($c.planted_by | ascii_downcase | gsub("[^a-z]"; "")) as $pl | any(["lane", "review", "author", "panel", "judge", "refut"][]; . as $w | $pl | contains($w));
        "quorum.canary.planted_by contains a role word (lane, review, author, panel, judge, refut): the planter must be named, not titled")] | map(select(.[0])) | (first(.[] | .[1]) // "ok")
'

# Stage 2b: the claims file the refuters and judges were handed. Input: that
# file. $c is quorum.canary. Same form for planted and real claims.
DOGFOOD_CANARY_CLAIMS_JQ='
  def chk(c; m): [(try ([c] | any) catch true), m];
  def sid: type == "string" and test("\\A[1-9][0-9]{0,5}\\z");
  . as $cl
  | ($c.real_claim_ids) as $rid | ($c.planted) as $pl
  | [
    chk(($cl | type) != "array"; "canary claims_file is not a JSON array of claims"),
    chk(($cl | length) != (($rid | length) + ($pl | length));
        "canary claims_file does not hold exactly the real plus planted claims (\(try ($cl | length) catch "?") found)"),
    chk(($cl | all(.[]; (type == "object") and (keys == ["id", "text"]) and (.id | sid) and (.text | type == "string") and ((.text | gsub("\\s"; "") | length) > 0))) | not;
        "canary claims_file claims must all be objects with exactly the keys id and text, a plain numeric id and a non-blank text: a planted claim must have the form of a lane claim"),
    chk(([$cl[].id] | unique | length) != ($cl | length); "canary claims_file ids are not distinct"),
    chk(any($pl[]; . as $p | $cl[$p.position - 1] != {id: $p.id, text: $p.text});
        "canary claims_file does not carry each planted claim verbatim at its recorded position"),
    chk([$cl | to_entries[] | select(((.key + 1) as $pos | any($pl[]; .position == $pos)) | not) | .value.id] != $rid;
        "canary claims_file real claims are not quorum.canary.real_claim_ids in order")] | map(select(.[0])) | (first(.[] | .[1]) // "ok")
'

# Stage 2c: the plan that was hashed must say what the receipt says.
DOGFOOD_CANARY_PLAN_JQ='
  def proj: [.[] | {id, text, falsified_by, position}] | sort_by(.id);
  if (try (((.planted | type) == "array") and ((.planted | proj) == ($c.planted | proj))) catch false)
    then "ok"
    else "canary plan_file .planted is not an array equal to quorum.canary.planted: the hashed plan is not the plan the receipt cites"
  end
'

# Stage 2d: the judges. The majority is COMPUTED from the committed verdicts,
# never read from the receipt. $c is quorum.canary; $q is .quorum.
DOGFOOD_CANARY_VERDICTS_JQ='
  def chk(c; m): [(try ([c] | any) catch true), m];
  def norm: tostring | ascii_downcase | gsub("\\s"; "");
  def refd($v; $id): ([$v[] | .verdicts[$id] | select(. == "REFUTED")] | length);
  def majority($v; $id): (refd($v; $id) * 2) > ($v | length);
  . as $v
  | ($c.real_claim_ids) as $rid | ([$c.planted[].id]) as $pid | ($rid + $pid) as $all
  | (try ([$rid[] | select(majority($v; .))] | length) catch -1) as $realref
  | [
    chk(($v | type) != "array"; "canary verdicts_file is not a JSON array with one entry per judge"),
    chk(($v | length) != $q.judges; "canary verdicts_file judge entries do not equal quorum.judges (\(try ($v | length) catch "?") entries, \($q.judges) judges)"),
    chk(($v | all(.[]; (type == "object") and (keys == ["judge", "verdicts"]) and (.judge | type == "string") and ((.judge | gsub("\\s"; "") | length) > 0) and (.verdicts | type == "object"))) | not;
        "canary verdicts_file entries must be objects with exactly the keys judge (a non-blank name) and verdicts"),
    chk([$v | .. | strings] | any(test("[^ -~]"));
        "canary verdicts_file must be ASCII printable only: a zero-width or non-ASCII character can make two judges look different"),
    chk(([$v[].judge | norm] | unique | length) != ($v | length); "canary verdicts_file judges are not distinct"),
    chk(($c.planted_by | norm) as $pb | any($v[].judge | norm; . == $pb); "quorum.canary.planted_by is also a judge in the verdicts file: the planter must not judge"),
    chk($v | any(.[]; ((.verdicts | keys) != ($all | sort)) or ([.verdicts[]] | all(. == "REFUTED" or . == "SURVIVED") | not));
        "canary verdicts_file: every judge must answer every claim id (real and planted) with REFUTED or SURVIVED"),
    chk(any($pid[]; majority($v; .) | not);
        "a planted claim was not refuted by a majority of judges: a missed canary means the round did not hunt"),
    chk($q.claims_refuted != $realref;
        "quorum.claims_refuted = \($q.claims_refuted) but the judge verdicts refute \($realref) real claim(s); planted claims are never counted"),
    chk(($q | has("claims_confirmed")) and ($q.claims_confirmed != (($rid | length) - $realref));
        "quorum.claims_confirmed does not equal the real claims the judge verdicts confirm; planted claims are never counted"),
    chk(($q | has("claims_reviewed")) and ($q.claims_reviewed != ($rid | length));
        "quorum.claims_reviewed does not equal the real claim count; planted claims are never counted")] | map(select(.[0])) | (first(.[] | .[1]) // "ok")
'

# Words that would tell a refuter or a judge which claims are planted. They are
# searched in each DECODED string and object key of the claims file on its own,
# lower-cased, so case and \u escapes do not hide one. A marker counts as a
# whole word (so "plan", "planned" and "plan takes" are honest prose), with the
# inflections listed in the regex below (stems are NOT matched: "plantation" and
# a compound such as "canaryclaim" pass, as honest prose must), or as its letters
# each separated by one to three of space . _ * | / - ("p-l-a-n-t", "c a n a r y",
# "c-a-n-a-r-i-e-s"), where the first letter must start a word and the last must
# end one. Brackets, commas and operators are not separators, so code prose such
# as "sub(a, i) takes", "b = a[i] + t" or "(p, l, a, n, t)" is not read as a marker.
# A claims file with any non-ASCII or control character in a string is refused
# outright, so a homoglyph cannot hide a word. Not caught: a word split with a
# separator in only some gaps ("ca nary"), and a compound ("canaryclaim").
DOGFOOD_CANARY_MARKERS='canary|plant|decoy|honeypot|tripwire|bait|sentinel'
DOGFOOD_CANARY_MARKER_JQ='
  def stems: ["canary", "canaries", "plant", "plants", "planted", "planting", "planter", "planters", "decoy", "decoys", "decoyed", "decoying", "honeypot", "honeypots", "tripwire", "tripwires", "bait", "baits", "baited", "baiting", "sentinel", "sentinels"];
  def spaced($w): "(?<![a-z])" + ($w | split("") | join("[ ._*|/-]{1,3}")) + "(?![a-z])";
  ([.. | strings] + [.. | objects | keys[]]) as $s
  | if any($s[]; test("[^ -~]")) then "nonascii"
    elif any($s[] | ascii_downcase;
        test("(?<![a-z])(canar(y|ies)|plant(s|ed|ing|er|ers)?|decoy(s|ed|ing)?|honeypots?|tripwires?|bait(s|ed|ing)?|sentinels?)(?![a-z])")
        or (. as $t | any(stems[]; . as $w | $t | test(spaced($w)))))
      then "marker"
    else "ok"
    end
'

# dogfood_canary_commit REV : the 40-hex commit REV names, or fail.
dogfood_canary_commit() {
  local out rc=0
  [ -n "$1" ] || return 1
  out="$(git rev-parse --verify -q "${1}^{commit}" 2>/dev/null)" || rc=$?
  # A commit id is 40 hex digits (or 64): empty or other output from a git
  # that exited 0 is a git that did not answer, never a commit.
  [ "$rc" -eq 0 ] || return 1
  case "$out" in
    *[!0-9a-f]* | "") return 1 ;;
  esac
  [ "${#out}" -eq 40 ] || [ "${#out}" -eq 64 ] || return 1
  printf '%s\n' "$out"
}

# dogfood_canary_fetch OUT PATH WANT REV... : copy PATH from the first REV
# whose bytes hash to WANT. 0 found, 1 PATH is at no REV, 2 present but no
# copy matches (DOGFOOD_CANARY_GOT is the last hash seen).
dogfood_canary_fetch() {
  local out="$1" path="$2" want="$3" rev got seen=0
  shift 3
  DOGFOOD_CANARY_GOT=""
  for rev in "$@"; do
    git show "${rev}:${path}" >"$out" 2>/dev/null || continue
    seen=1
    got="$(sha256sum <"$out")" || return 2
    got="${got%% *}"
    DOGFOOD_CANARY_GOT="$got"
    if [ "$got" = "$want" ]; then
      return 0
    fi
  done
  [ "$seen" -eq 1 ] && return 2
  return 1
}

# dogfood_canary_added PATH LINE BASE HEAD : is PATH:LINE an ADDED line of
# `git diff -U0 BASE HEAD`? A fact outside the diff falsifies nothing the
# reviewed change did. Exit 0 yes, 1 no, 2 the diff could not be read.
dogfood_canary_added() {
  local path="$1" line="$2" base="$3" head="$4" d out rc=0
  d="$(git --literal-pathspecs diff -U0 --no-color --no-ext-diff "$base" "$head" -- "$path" 2>/dev/null)" || rc=$?
  if [ "$rc" -ne 0 ]; then
    return 2
  fi
  # The answer is a word awk prints, never awk's exit status or its silence: an
  # awk that is missing or shimmed to exit 0 prints no word, and no word is
  # not a hit.
  out="$(awk -v n="$line" '
    /^@@ / { split($3, a, ","); s = substr(a[1], 2) + 0; c = (a[2] == "" ? 1 : a[2] + 0)
             if (c > 0 && n + 0 >= s && n + 0 < s + c) hit = 1 }
    END { print (hit ? "hit" : "miss") }' <<<"$d")" || return 2
  case "$out" in
    hit) return 0 ;;
    miss) return 1 ;;
    *) return 2 ;;
  esac
}

# dogfood_canary_marker FILE : 0 clean, 1 a marker word or a non-ASCII char
# (sentence in DOGFOOD_CANARY_WHY), 2 unreadable.
dogfood_canary_marker() {
  local out
  out="$(jq -r "$DOGFOOD_CANARY_MARKER_JQ" "$1" 2>/dev/null)" || return 2
  case "$out" in
    ok) return 0 ;;
    nonascii)
      DOGFOOD_CANARY_WHY="quorum.canary.claims_file carries a non-ASCII or control character: a homoglyph could hide a marker word"
      return 1
      ;;
    marker)
      DOGFOOD_CANARY_WHY="quorum.canary.claims_file carries a marker word (${DOGFOOD_CANARY_MARKERS}): the refuters and judges were not blind"
      return 1
      ;;
    *) return 2 ;;
  esac
}

# Duplicate JSON keys. jq keeps the last of two equal keys, so a file could show
# a human or another parser one claim and this predicate another. Read as a jq
# --stream (tested on jq 1.8.2 only, the build host's; --stream and [0:-1] are in jq 1.5 and later, but no other version was run): a leaf path may
# occur once, no leaf path may be the prefix of another path, and each container
# closes once (so two same-key objects with disjoint members are caught too).
DOGFOOD_CANARY_DUP_JQ='
  [inputs] as $e
  | [$e[] | select(length == 2) | .[0]] as $L
  | [$e[] | select(length == 1) | .[0][0:-1]] as $C
  | if ($L | length) != ($L | unique | length) then "dup"
    elif ($C | length) != ($C | unique | length) then "dup"
    elif any($L[]; . as $l | any($e[]; .[0] as $p | ($p | length) > ($l | length) and $p[0:($l | length)] == $l)) then "dup"
    else "ok" end
'

# dogfood_pgrep FLAGS PATTERN FILE POS NEG : prints hit, miss or unmeasured.
# No decision in this library rests on the silence of a command: a grep that is
# missing, shimmed to exit 0 or 1, or broken is caught by two positive controls
# run in the same call, a sample that MUST match (POS) and one that MUST NOT
# (NEG), both given as printf %b text so they may hold a NUL or a high byte.
# Only then is FILE judged, and only exit 0 (hit) or 1 (miss) of grep counts.
dogfood_pgrep() {
  local flags="$1" pat="$2" file="$3" pos="$4" neg="$5" pf nf rc prc nrc
  pf="$(mktemp)" || pf=""
  nf="$(mktemp)" || nf=""
  if [ -z "$pf" ] || [ -z "$nf" ] || [ ! -f "$pf" ] || [ ! -f "$nf" ]; then
    [ -n "$pf" ] && rm -f "${pf:?}"
    [ -n "$nf" ] && rm -f "${nf:?}"
    printf 'unmeasured\n'
    return 0
  fi
  printf '%b' "$pos" >"$pf" 2>/dev/null
  printf '%b' "$neg" >"$nf" 2>/dev/null
  prc=0
  LC_ALL=C grep -q"${flags}P" -e "$pat" "$pf" >/dev/null 2>&1 || prc=$?
  nrc=0
  LC_ALL=C grep -q"${flags}P" -e "$pat" "$nf" >/dev/null 2>&1 || nrc=$?
  rm -f "${pf:?}"
  rm -f "${nf:?}"
  # Both controls first: the pattern must hit its positive sample and miss its
  # negative one, or this grep is not judging anything.
  if [ "$prc" -ne 0 ] || [ "$nrc" -ne 1 ]; then
    printf 'unmeasured\n'
    return 0
  fi
  rc=0
  LC_ALL=C grep -q"${flags}P" -e "$pat" "$file" >/dev/null 2>&1 || rc=$?
  case "$rc" in
    0) printf 'hit\n' ;;
    1) printf 'miss\n' ;;
    *) printf 'unmeasured\n' ;;
  esac
}

# dogfood_nul_judge CMD...: exit 0 the bytes CMD prints hold no NUL, 1 they do,
# 2 could not be judged. A NUL cannot survive a bash variable, so a receipt read
# through $(...) is judged WITHOUT it and invalid JSON reaches `ok`; every reader
# below asks this on the bytes git (or the store) has, before they become a
# variable. It needs only tr and wc, which main already needs, never grep -P or
# mktemp, so a receipt without a canary block is read with the tools it always
# was: CMD is run twice, once counted and once through tr -d NUL, and a control
# in the same call (a sample with a NUL, which must shrink by exactly one byte)
# decides whether tr and wc are judging anything. CMD failing is "not judged".
dogfood_nul_judge() {
  local c1 c2 n m
  c1="$(printf 'a\000b' | wc -c 2>/dev/null)" || return 2
  c2="$(printf 'a\000b' | tr -d '\000' 2>/dev/null | wc -c 2>/dev/null)" || return 2
  case "$c1" in "" | *[!0-9\ ]*) return 2 ;; esac
  case "$c2" in "" | *[!0-9\ ]*) return 2 ;; esac
  [ "$((c1 + 0))" -eq 3 ] && [ "$((c2 + 0))" -eq 2 ] || return 2
  n="$(set -o pipefail; "$@" 2>/dev/null | wc -c 2>/dev/null)" || return 2
  m="$(set -o pipefail; "$@" 2>/dev/null | tr -d '\000' 2>/dev/null | wc -c 2>/dev/null)" || return 2
  case "$n" in "" | *[!0-9\ ]*) return 2 ;; esac
  case "$m" in "" | *[!0-9\ ]*) return 2 ;; esac
  [ "$((m + 0))" -le "$((n + 0))" ] || return 2
  [ "$((m + 0))" -eq "$((n + 0))" ]
  # 0 equal (no NUL), 1 the stripped copy is shorter (a NUL)
}

# dogfood_blob_nul FILE : dogfood_nul_judge on a file's bytes.
dogfood_blob_nul() {
  dogfood_nul_judge cat -- "$1"
}

# dogfood_file_blob FILE : prints FILE when it holds no NUL; exit 0 ok, 3 a NUL
# byte, 2 unreadable or not judged. dogfood_git_blob SPEC does the same for the
# git object SPEC (exit 1 when git has no such object).
dogfood_file_blob() {
  local rc=0
  dogfood_blob_nul "$1" || rc=$?
  case "$rc" in
    0) cat -- "$1" || return 2 ;;
    1) return 3 ;;
    *) return 2 ;;
  esac
}
dogfood_git_blob() {
  local rc=0
  git cat-file -e "$1" 2>/dev/null || return 1
  dogfood_nul_judge git cat-file -p "$1" || rc=$?
  case "$rc" in
    0) git cat-file -p "$1" || return 2 ;;
    1) return 3 ;;
    *) return 2 ;;
  esac
}

# Lexical RFC 8259 for the canary files: whitespace, strings without \u escapes
# or raw control characters, integers of at most 15 digits with no sign other
# than -, no leading zero, fraction or exponent, and true false null and the
# structural characters. jq accepts NaN, Infinity, +3, 03, 1e309 and a BOM; this
# does not, and with no \u escape a string's raw bytes are its decoded bytes.
DOGFOOD_CANARY_LEX='\A[ \t\n\r]*+(?:(?:"(?:[^"\\\x00-\x1f]|\\["\\/bfnrt])*+"|-?(?:0|[1-9][0-9]{0,14})(?![0-9.eE+\-])|true|false|null|[\[\]{}:,])[ \t\n\r]*+)*+\z'

# dogfood_canary_raw FILE LABEL nul|dup|lex|bytes : prints ok, or one sentence.
# ok is printed only after a positive result, never because a command said
# nothing; a caller treats anything but ok as a refusal.
#   nul   a NUL byte anywhere.
#   dup   exactly one top-level value, no duplicate JSON key at any depth.
#   lex   the strict lexical check above (also refuses a BOM and trailing text).
#   bytes any byte outside printable ASCII plus tab, LF and CR in the RAW file.
dogfood_canary_raw() {
  local file="$1" label="$2" mode="$3" out
  case "$mode" in
    nul)
      out="$(dogfood_pgrep a '\x00' "$file" 'a\0000b' 'ab')"
      case "$out" in
        miss) printf 'ok\n' ;;
        hit) printf '%s\n' "canary ${label} holds a NUL byte: no JSON reader agrees on what a NUL hides" ;;
        *) printf '%s\n' "canary ${label} could not be scanned for NUL bytes: UNMEASURED" ;;
      esac
      ;;
    dup)
      out="$(jq -s -r 'length | tostring' "$file" 2>/dev/null)" || out="unreadable"
      case "$out" in
        1) ;;
        unreadable | "") printf '%s\n' "canary ${label} is not readable JSON" && return 0 ;;
        *) printf '%s\n' "canary ${label} holds ${out} top-level JSON values, not one" && return 0 ;;
      esac
      out="$(jq -n -r --stream "$DOGFOOD_CANARY_DUP_JQ" "$file" 2>/dev/null)" || out="unreadable"
      case "$out" in
        ok) printf 'ok\n' ;;
        dup) printf '%s\n' "canary ${label} holds a duplicate JSON key: jq keeps the last one, so the file reads differently to a human or another parser" ;;
        *) printf '%s\n' "canary ${label} is not readable JSON" ;;
      esac
      ;;
    lex)
      out="$(dogfood_pgrep az "$DOGFOOD_CANARY_LEX" "$file" '{"a":[1,"x",-0,true]}\n' '{"a":+3}')"
      case "$out" in
        hit) printf 'ok\n' ;;
        miss) printf '%s\n' "canary ${label} is not strict JSON text (a \\u escape, a number that is not a plain integer of at most 15 digits such as NaN, Infinity, +3, 03, 1.5 or 1e309, a BOM, a raw control character or trailing text)" ;;
        *) printf '%s\n' "canary ${label} could not be checked for strict JSON text: UNMEASURED" ;;
      esac
      ;;
    bytes)
      out="$(dogfood_pgrep a '[^\x09\x0a\x0d\x20-\x7e]' "$file" '\0177' 'ab\n')"
      case "$out" in
        miss) printf 'ok\n' ;;
        hit) printf '%s\n' "canary ${label} holds a byte outside printable ASCII in its raw file: nothing may hide where jq does not look" ;;
        *) printf '%s\n' "canary ${label} could not be scanned for non-ASCII bytes: UNMEASURED" ;;
      esac
      ;;
    *) printf '%s\n' "canary ${label}: unknown raw check ${mode}, UNMEASURED" ;;
  esac
}

# dogfood_canary_rawall FILE LABEL MODE... : the first non-ok answer of the
# modes in order, else ok.
dogfood_canary_rawall() {
  local file="$1" label="$2" mode out
  shift 2
  for mode in "$@"; do
    out="$(dogfood_canary_raw "$file" "$label" "$mode")"
    if [ "$out" != ok ]; then
      printf '%s\n' "${out:-canary ${label} gave no answer to the ${mode} check, UNMEASURED}"
      return 0
    fi
  done
  printf 'ok\n'
}

# dogfood_canary_jqfile PROG FILE LABEL [jq args...] : run a stage-2 program
# on FILE; prints ok or a sentence. A jq failure is a sentence, never ok.
dogfood_canary_jqfile() {
  local prog="$1" file="$2" label="$3" out
  shift 3
  out="$(jq -r "$@" "$prog" "$file" 2>/dev/null)" || out="canary ${label} is not readable JSON"
  [ -n "$out" ] || out="canary ${label} gave no answer, UNMEASURED"
  printf '%s\n' "$out"
}

# The file-level checks, run inside a scratch dir $1 for the already-resolved
# commits HEAD $3 and BASE $4, reading evidence at the revs after them.
dogfood_canary_files() {
  local tmp="$1" blob="$2" head="$3" base="$4" c q f role path want out rc i n p line names nm
  shift 4
  c="$(printf '%s' "$blob" | jq -c '.quorum.canary')" || return 1
  q="$(printf '%s' "$blob" | jq -c '.quorum')" || return 1
  [ -n "$c" ] && [ -n "$q" ] || return 1
  p="$(printf '%s' "$c" | jq -r '.reviewed_head')" || return 1
  if [ "$p" != "$head" ]; then
    printf 'quorum.canary.reviewed_head %s is not the commit this receipt vouches for (%s)\n' "${p:0:12}" "${head:0:12}"
    return 0
  fi
  p="$(printf '%s' "$c" | jq -r '.reviewed_base')" || return 1
  if [ "$p" != "$base" ]; then
    printf 'quorum.canary.reviewed_base %s is not the base of the commit this receipt vouches for (%s)\n' "${p:0:12}" "${base:0:12}"
    return 0
  fi
  for role in claims_file plan_file verdicts_file; do
    path="$(printf '%s' "$c" | jq -r --arg f "$role" '.[$f]')" || return 1
    want="$(printf '%s' "$blob" | jq -r --arg p "$path" '.evidence.files[] | select(.path == $p) | .sha256')" || return 1
    [ -n "$want" ] || return 1
    f="${tmp}/${role}"
    rc=0
    dogfood_canary_fetch "$f" "$path" "$want" "$@" || rc=$?
    if [ "$rc" -eq 1 ]; then
      printf 'quorum.canary.%s %s is not committed at any of %s\n' "$role" "$path" "$*"
      return 0
    elif [ "$rc" -ne 0 ]; then
      printf 'quorum.canary.%s %s: evidence.files sha256 %s, no committed copy matches (last seen %s)\n' "$role" "$path" "${want:0:12}" "${DOGFOOD_CANARY_GOT:0:12}"
      return 0
    fi
    out="$(dogfood_canary_rawall "$f" "$role" nul dup lex)"
    if [ "$out" != ok ]; then
      printf '%s\n' "$out"
      return 0
    fi
  done
  rc=0
  dogfood_canary_marker "${tmp}/claims_file" || rc=$?
  if [ "$rc" -eq 1 ]; then
    printf '%s\n' "$DOGFOOD_CANARY_WHY"
    return 0
  elif [ "$rc" -ne 0 ]; then
    printf 'quorum.canary.claims_file could not be searched for marker words (exit %s): UNMEASURED\n' "$rc"
    return 0
  fi
  out="$(dogfood_canary_jqfile "$DOGFOOD_CANARY_CLAIMS_JQ" "${tmp}/claims_file" claims_file --argjson c "$c")"
  if [ "$out" != ok ]; then
    printf '%s\n' "$out"
    return 0
  fi
  out="$(dogfood_canary_jqfile "$DOGFOOD_CANARY_PLAN_JQ" "${tmp}/plan_file" plan_file --argjson c "$c")"
  if [ "$out" != ok ]; then
    printf '%s\n' "$out"
    return 0
  fi
  out="$(dogfood_canary_jqfile "$DOGFOOD_CANARY_VERDICTS_JQ" "${tmp}/verdicts_file" verdicts_file --argjson c "$c" --argjson q "$q")"
  if [ "$out" != ok ]; then
    printf '%s\n' "$out"
    return 0
  fi
  for role in claims_file plan_file verdicts_file; do
    out="$(dogfood_canary_rawall "${tmp}/${role}" "$role" bytes)"
    if [ "$out" != ok ]; then
      printf '%s\n' "$out"
      return 0
    fi
  done
  names="$(git diff --name-only --no-renames "$base" "$head" 2>/dev/null)" || {
    printf 'quorum.canary the diff %s..%s could not be listed: UNMEASURED\n' "${base:0:12}" "${head:0:12}"
    return 0
  }
  n="$(printf '%s' "$c" | jq -r '.planted | length')" || return 1
  case "$n" in
    "" | *[!0-9]*) return 1 ;;
  esac
  [ "$n" -ge 2 ] || return 1
  i=0
  while [ "$i" -lt "$n" ]; do
    p="$(printf '%s' "$c" | jq -r --argjson i "$i" '.planted[$i].falsified_by')" || return 1
    path="${p%:*}"
    line="${p##*:}"
    rc=1
    while IFS= read -r nm; do
      if [ "$nm" = "$path" ]; then
        rc=0
        break
      fi
    done <<<"$names"
    if [ "$rc" -ne 0 ]; then
      printf 'quorum.canary.planted[%s].falsified_by %s names a path that is not exactly a file changed by the reviewed diff\n' "$i" "$p"
      return 0
    fi
    rc=0
    dogfood_canary_added "$path" "$line" "$base" "$head" || rc=$?
    if [ "$rc" -ne 0 ]; then
      printf 'quorum.canary.planted[%s].falsified_by %s is not an added line of the reviewed diff (%s): the falsifying fact must lie in the diff itself\n' "$i" "$p" "$rc"
      return 0
    fi
    i=$((i + 1))
  done
  printf 'ok\n'
}

# dogfood_canary_status BLOB [HEAD BASE [EVREV...]] : ok, or the first thing
# wrong with quorum.canary. Fails closed: anything unreadable, an absent or
# unresolvable HEAD, BASE or EVREV is a sentence, never ok.
dogfood_canary_status() {
  local blob="$1" why tmp rc=0 head base rev
  local -a revs=()
  shift
  tmp="$(mktemp)" || tmp=""
  if [ -z "$tmp" ] || [ ! -f "$tmp" ]; then
    printf '%s\n' "quorum.canary: no scratch file to read the receipt into, UNMEASURED"
    return 0
  fi
  printf '%s' "$blob" >"$tmp"
  why="$(dogfood_canary_rawall "$tmp" receipt nul dup lex)"
  if [ "$why" != ok ]; then
    rm -f "${tmp:?}"
    printf '%s\n' "$why"
    return 0
  fi
  why="$(printf '%s' "$blob" | jq -r "$DOGFOOD_CANARY_SHAPE_JQ" 2>/dev/null)" || rc=$?
  if [ "$rc" -ne 0 ] || [ -z "$why" ]; then
    rm -f "${tmp:?}"
    printf '%s\n' "quorum.canary is not readable by this predicate (jq exit ${rc})"
    return 0
  fi
  if [ "$why" != ok ]; then
    rm -f "${tmp:?}"
    printf '%s\n' "$why"
    return 0
  fi
  why="$(dogfood_canary_rawall "$tmp" receipt bytes)"
  rm -f "${tmp:?}"
  if [ "$why" != ok ]; then
    printf '%s\n' "$why"
    return 0
  fi
  if [ "$#" -lt 2 ]; then
    printf '%s\n' "quorum.canary needs the commit this receipt vouches for and its base, and the caller named none: UNMEASURED"
    return 0
  fi
  head="$(dogfood_canary_commit "$1")" || {
    printf '%s\n' "quorum.canary: the commit this receipt vouches for ('${1}') is empty or does not resolve: UNMEASURED"
    return 0
  }
  base="$(dogfood_canary_commit "$2")" || {
    printf '%s\n' "quorum.canary: the base of the commit this receipt vouches for ('${2}') is empty or does not resolve: UNMEASURED"
    return 0
  }
  shift 2
  if [ "$#" -eq 0 ]; then
    set -- "$head" HEAD
  fi
  for rev in "$@"; do
    dogfood_canary_commit "$rev" >/dev/null || {
      printf '%s\n' "quorum.canary: the evidence rev '${rev}' is empty or does not resolve: UNMEASURED"
      return 0
    }
    revs+=("$(dogfood_canary_commit "$rev")")
  done
  tmp="$(mktemp -d)" || tmp=""
  if [ -z "$tmp" ] || [ ! -d "$tmp" ]; then
    printf '%s\n' "quorum.canary: no scratch directory to read the evidence files into, UNMEASURED"
    return 0
  fi
  rc=0
  why="$(dogfood_canary_files "$tmp" "$blob" "$head" "$base" "${revs[@]}" 2>/dev/null)" || rc=$?
  rm -rf "${tmp:?}"
  if [ "$rc" -ne 0 ] || [ -z "$why" ]; then
    printf '%s\n' "quorum.canary could not be verified against its evidence files (exit ${rc}), UNMEASURED"
    return 0
  fi
  printf '%s\n' "$why"
}

# dogfood_receipt_resolve WHY BLOB [HEAD BASE [EVREV...]] : stage 1's answer,
# with the `canary` token replaced by the block's verdict. For callers that
# run DOGFOOD_RECEIPT_JQ themselves (gate E). An empty WHY is a refusal.
dogfood_receipt_resolve() {
  local why="$1" blob="$2"
  shift 2
  if [ -z "$why" ]; then
    printf '%s\n' "the receipt predicate gave no answer: UNMEASURED"
    return 0
  fi
  if [ "$why" = canary ]; then
    dogfood_canary_status "$blob" "$@"
    return 0
  fi
  printf '%s\n' "$why"
}

dogfood_receipt_status() {
  local rc=0 why blob="$1"
  shift
  why="$(printf '%s' "$blob" | jq -r "$DOGFOOD_RECEIPT_JQ" 2>/dev/null)" || rc=$?
  if [ "$rc" -ne 0 ] || [ -z "$why" ]; then
    printf '%s\n' "is not a JSON object this predicate can read (jq exit ${rc}): an unreadable receipt is no receipt"
    return 0
  fi
  dogfood_receipt_resolve "$why" "$blob" "$@"
}

# dogfood_receipt_pick OID PATH : the receipt blob Arm 5 judges for the PR whose
# merge commit is OID. For a receipt without a canary block it is exactly what
# Arm 5 chose before PMAT-1170: the copy in OID's tree, else the copy at HEAD.
# A canary receipt is bound to the commit it vouches for and a squash or merge
# commit cannot be named in the tree it labels, so it is committed AFTER the
# merge; when the copy at HEAD carries a canary block, that copy is the one
# judged, which is also the one gate E reads (it always reads HEAD). The choice
# looks at the HEAD copy, the receipt it will actually judge, never at the old
# copy: an old 0-refuted receipt at OID is replaced by a canary receipt at HEAD
# on a re-run, and an old copy that merely mentions "canary" does not let an
# unbound non-canary stub at HEAD stand in for it.
# dogfood_pick_read SPEC : the receipt blob at SPEC; when it holds a NUL byte or
# cannot be read, a sentence that is not JSON, so every reader of the pick
# refuses it instead of judging bytes a shell variable dropped.
dogfood_pick_read() {
  local rc=0 out
  out="$(dogfood_git_blob "$1")" || rc=$?
  case "$rc" in
    0) printf '%s' "$out" ;;
    3) printf '%s' "receipt holds a NUL byte" ;;
    *) printf '%s' "receipt unreadable (exit ${rc}), UNMEASURED" ;;
  esac
}

dogfood_receipt_pick() {
  local oid="$1" path="$2" blob="" later="" have_blob=0 jrc=0 old
  if git cat-file -e "${oid}:${path}" 2>/dev/null; then
    have_blob=1
    blob="$(dogfood_pick_read "${oid}:${path}")"
  fi
  if git cat-file -e "HEAD:${path}" 2>/dev/null; then
    later="$(dogfood_pick_read "HEAD:${path}")"
  fi
  # A HEAD copy with a NUL byte or that cannot be read is no canary receipt:
  # with a copy in OID's tree, behave exactly as main did and judge that one.
  case "$later" in
    "receipt holds a NUL byte" | "receipt unreadable "*)
      if [ "$have_blob" -eq 1 ]; then
        printf '%s' "$blob"
      else
        printf '%s' "$later"
      fi
      return 0
      ;;
  esac
  if [ -n "$later" ]; then
    printf '%s' "$later" | jq -e '(.quorum | type == "object") and (.quorum | has("canary"))' >/dev/null 2>&1 || jrc=$?
    case "$jrc" in
      0)
        # The canary shape may only lift the zero-refuted refusal: a copy in
        # OID's tree that main refuses for any other reason (a waiver, a thin
        # lane count) or accepts is judged as it always was.
        if [ "$have_blob" -eq 1 ]; then
          old="$(printf '%s' "$blob" | jq -r "$DOGFOOD_RECEIPT_JQ" 2>/dev/null)" || old=""
          case "$old" in
            canary | "quorum.claims_refuted = "*) ;;
            *)
              printf '%s' "$blob"
              return 0
              ;;
          esac
        fi
        printf '%s' "$later"
        return 0
        ;;
      1 | 2 | 4 | 5)
        # Not a canary receipt (4: no value, a whitespace-only copy), but only if jq demonstrably answers 0 on one.
        if ! printf '%s' '{"quorum":{"canary":{}}}' | jq -e '(.quorum | type == "object") and (.quorum | has("canary"))' >/dev/null 2>&1; then
          printf '%s' "receipt unreadable: jq cannot tell a canary receipt, UNMEASURED"
          return 0
        fi
        ;;
      *)
        # jq itself failed (absent, killed): which copy is canary is not known.
        printf '%s' "receipt unreadable: jq exit ${jrc}, UNMEASURED"
        return 0
        ;;
    esac
  fi
  if [ "$have_blob" -eq 1 ]; then
    printf '%s' "$blob"
  else
    printf '%s' "$later"
  fi
}
