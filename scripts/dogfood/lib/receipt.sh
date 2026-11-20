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
  if ([paths | .[-1]? | tostring] | any(test("waiv|override"; "i")))
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

# Stage 2a: the SHAPE of quorum.canary and its pointers into evidence.files.
# Every condition is "bad when true"; one that raises (a wrong type) is bad too.
DOGFOOD_CANARY_SHAPE_JQ='
  def isstr: type == "string";
  def nes: isstr and (gsub("\\s"; "") | length > 0);
  def sid: isstr and test("^[1-9][0-9]{0,5}$");
  def norm: tostring | ascii_downcase | gsub("\\s"; "");
  def hexn($n): isstr and test("^[0-9a-f]{\($n)}$");
  def chk(c; m): [(try ([c] | any) catch true), m];
  def evok($c; $f; $role):
    (($c[$f] | nes)
     and ([.evidence.files[]? | select(.path == $c[$f]
          and ((.roles // []) | type == "array" and (index($role) != null))
          and (.sha256 | hexn(64)))] | length) == 1);
  ([.quorum | (.lanes, .refuters, .refuter_names, .judge_names)? | arrays | .[] | norm]) as $others
  | .quorum.canary as $c
  | [
    chk(($c | type) != "object"; "quorum.canary is not an object"),
    chk((($c.planted_by | nes) | not); "quorum.canary.planted_by is missing: a canary round names who planted it"),
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
         and ((.text | nes) and ((.text | gsub("^\\s+|\\s+$"; "") | length) >= 20))
         and ((.falsified_by | isstr) and (.falsified_by | test("^[A-Za-z0-9_./-]+:[0-9]+$")) and ((.falsified_by | test("\\.\\.")) | not))
         and ((.position | type) == "number") and (.position == (.position | floor)) and (.position >= 1))) | not;
        "quorum.canary.planted[] needs a plain numeric id, text (20+ non-blank chars), falsified_by (path:line, no ..) and an integer position >= 1"),
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
            ((try test("^\\.quorum/evidence/[A-Za-z0-9_./-]+$") catch false) | not) or (try test("\\.\\.") catch true));
        "quorum.canary names an evidence file outside .quorum/evidence/"),
    chk((evok($c; "claims_file"; "claims-blind")) | not;
        "quorum.canary.claims_file must be one evidence.files entry with role claims-blind and a sha256"),
    chk((evok($c; "plan_file"; "canary-plan")) | not;
        "quorum.canary.plan_file must be one evidence.files entry with role canary-plan and a sha256"),
    chk((evok($c; "verdicts_file"; "judge-verdicts")) | not;
        "quorum.canary.verdicts_file must be one evidence.files entry with role judge-verdicts and a sha256"),
    chk(([$c.claims_file, $c.plan_file, $c.verdicts_file] | unique | length) != 3;
        "quorum.canary claims_file, plan_file and verdicts_file are not three distinct files")] | map(select(.[0])) | (first(.[] | .[1]) // "ok")
'

# Stage 2b: the claims file the refuters and judges were handed. Input: that
# file. $c is quorum.canary. Same form for planted and real claims.
DOGFOOD_CANARY_CLAIMS_JQ='
  def chk(c; m): [(try ([c] | any) catch true), m];
  def sid: type == "string" and test("^[1-9][0-9]{0,5}$");
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

# Words that would tell a refuter or a judge which claims are planted. They
# are searched in the DECODED strings and keys of the claims file, lower-cased
# and reduced to letters, so case, spacing, punctuation and \u escapes do not
# hide one. A claims file with any non-ASCII character is refused outright.
DOGFOOD_CANARY_MARKERS='canary|plant|decoy|honeypot|tripwire|bait|sentinel'

# dogfood_canary_commit REV : the 40-hex commit REV names, or fail.
dogfood_canary_commit() {
  [ -n "$1" ] || return 1
  git rev-parse --verify -q "${1}^{commit}" 2>/dev/null
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
  local path="$1" line="$2" base="$3" head="$4" d rc=0
  d="$(git --literal-pathspecs diff -U0 --no-color --no-ext-diff "$base" "$head" -- "$path" 2>/dev/null)" || rc=$?
  if [ "$rc" -ne 0 ]; then
    return 2
  fi
  awk -v n="$line" '
    /^@@ / { split($3, a, ","); s = substr(a[1], 2) + 0; c = (a[2] == "" ? 1 : a[2] + 0)
             if (c > 0 && n + 0 >= s && n + 0 < s + c) hit = 1 }
    END { exit(hit ? 0 : 1) }' <<<"$d" || return 1
  return 0
}

# dogfood_canary_marker FILE : 0 clean, 1 a marker word or a non-ASCII char
# (sentence in DOGFOOD_CANARY_WHY), 2 unreadable.
dogfood_canary_marker() {
  local decoded folded rc=0
  decoded="$(jq -r '[.. | strings] + [.. | objects | keys[]] | .[]' "$1" 2>/dev/null)" || return 2
  LC_ALL=C grep -q '[^ -~]' <<<"$decoded" || rc=$?
  if [ "$rc" -eq 0 ]; then
    DOGFOOD_CANARY_WHY="quorum.canary.claims_file carries a non-ASCII or control character: a homoglyph could hide a marker word"
    return 1
  elif [ "$rc" -ne 1 ]; then
    return 2
  fi
  folded="$(tr 'A-Z' 'a-z' <<<"$decoded" | tr -cd 'a-z')" || return 2
  rc=0
  grep -qE "$DOGFOOD_CANARY_MARKERS" <<<"$folded" || rc=$?
  if [ "$rc" -eq 0 ]; then
    DOGFOOD_CANARY_WHY="quorum.canary.claims_file carries a marker word (${DOGFOOD_CANARY_MARKERS}): the refuters and judges were not blind"
    return 1
  elif [ "$rc" -ne 1 ]; then
    return 2
  fi
  return 0
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
  local tmp="$1" blob="$2" head="$3" base="$4" c q f role path want out rc i n p line names
  shift 4
  c="$(printf '%s' "$blob" | jq -c '.quorum.canary')" || return 1
  q="$(printf '%s' "$blob" | jq -c '.quorum')" || return 1
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
  names="$(git diff --name-only --no-renames "$base" "$head" 2>/dev/null)" || {
    printf 'quorum.canary the diff %s..%s could not be listed: UNMEASURED\n' "${base:0:12}" "${head:0:12}"
    return 0
  }
  n="$(printf '%s' "$c" | jq -r '.planted | length')" || return 1
  i=0
  while [ "$i" -lt "$n" ]; do
    p="$(printf '%s' "$c" | jq -r --argjson i "$i" '.planted[$i].falsified_by')" || return 1
    path="${p%:*}"
    line="${p##*:}"
    rc=0
    grep -qxF -- "$path" <<<"$names" || rc=$?
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
  why="$(printf '%s' "$blob" | jq -r "$DOGFOOD_CANARY_SHAPE_JQ" 2>/dev/null)" || rc=$?
  if [ "$rc" -ne 0 ] || [ -z "$why" ]; then
    printf '%s\n' "quorum.canary is not readable by this predicate (jq exit ${rc})"
    return 0
  fi
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
  tmp="$(mktemp -d)" || {
    printf '%s\n' "quorum.canary: no scratch directory to read the evidence files into, UNMEASURED"
    return 0
  }
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
