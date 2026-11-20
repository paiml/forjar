#!/usr/bin/env bash
# The ONE predicate that says whether a committed quorum receipt is a quorum.
# Sourced by scripts/dogfood/quorum.sh (gate E, before the tag) and by
# scripts/dogfood/release-check.sh Arm 5 (after the tag), so the two cannot
# drift apart: the fourth PMAT-163 merge review found Arm 5 checking a
# top-level `waived` key and the lane and judge floors only, while gate E
# also refused nested waiver/override keys, thin refuter counts, a round that
# killed no claim, and a receipt citing no evidence. A release's last check
# must be at least as strict as its first.
#
# `dogfood_receipt_status BLOB [REV...]` prints "ok" or one sentence naming the
# first floor the receipt is under. Not valid JSON is a reason too, never a pass.
#
# TWO SHAPES CLEAR THE "did the round hunt" FLOOR (PMAT-1170, forjar#990):
#   1. quorum.claims_refuted >= 1: a claim was killed.
#   2. a `quorum.canary` block: a clean round that proves it hunted, because
#      it was handed >= 2 planted FALSE claims, indistinguishable in form from
#      the lanes' claims, and a majority of judges refuted every one. A round
#      that refuted nothing and carries no valid block is refused with the same
#      sentence as before. Nothing is grandfathered: an old 0-refuted receipt
#      has no block, so it is refused and its round must be re-run.
# The block is verified against COMMITTED files read at REV... (default HEAD:
# gate E reads HEAD, Arm 5 reads the merge commit and then HEAD), never trusted
# from its own text. See dogfood_canary_status below for each check.

DOGFOOD_RECEIPT_JQ='
  def num($v): if ($v | type) == "number" then $v else -1 end;
  def len($v): if ($v | type) == "array" then ($v | length) else -1 end;
  def hascanary: (.quorum | type == "object" and has("canary"));
  if ([paths | .[-1]? | tostring] | any(test("waiv|override")))
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
  def hexn($n): isstr and test("^[0-9a-f]{\($n)}$");
  def chk(c; m): [(try ([c] | any) catch true), m];
  def evok($c; $f; $role):
    (($c[$f] | nes)
     and ([.evidence.files[]? | select(.path == $c[$f]
          and ((.roles // []) | type == "array" and (index($role) != null))
          and (.sha256 | hexn(64)))] | length) == 1);
  .quorum.canary as $c
  | [
    chk(($c | type) != "object"; "quorum.canary is not an object"),
    chk((($c.planted_by | nes) | not); "quorum.canary.planted_by is missing: a canary round names who planted it"),
    chk(($c.planted_by | test("refuter|judge"; "i")); "quorum.canary.planted_by names a refuter or a judge: the planter must be neither"),
    chk((($c.reviewed_base | hexn(40)) | not); "quorum.canary.reviewed_base is not a 40-hex commit id"),
    chk((($c.reviewed_head | hexn(40)) | not); "quorum.canary.reviewed_head is not a 40-hex commit id"),
    chk((($c.real_claim_ids | type) != "array"); "quorum.canary.real_claim_ids is not an array"),
    chk(($c.real_claim_ids | length) < 1; "quorum.canary.real_claim_ids is empty: a round with no real claim reviewed nothing"),
    chk(($c.real_claim_ids | all(nes)) | not; "quorum.canary.real_claim_ids holds a non-string or empty id"),
    chk(($c.real_claim_ids | unique | length) != ($c.real_claim_ids | length); "quorum.canary.real_claim_ids are not distinct"),
    chk(($c.planted | type) != "array"; "quorum.canary.planted is not an array"),
    chk(($c.planted | length) < 2; "quorum.canary.planted has \(try ($c.planted | length) catch "?") planted claim(s), below the floor of 2"),
    chk(($c.planted | all(.[];
         (type == "object") and (.id | nes)
         and ((.text | isstr) and ((.text | length) >= 20))
         and ((.falsified_by | isstr) and (.falsified_by | test("^[A-Za-z0-9_./-]+:[0-9]+$")) and ((.falsified_by | test("\\.\\.")) | not))
         and ((.position | type) == "number") and (.position == (.position | floor)) and (.position >= 1))) | not;
        "quorum.canary.planted[] needs id, text (20+ chars), falsified_by (path:line, no ..) and an integer position >= 1"),
    chk(([$c.planted[].id] | unique | length) != ($c.planted | length); "quorum.canary.planted[].id are not distinct"),
    chk(([$c.planted[].text] | unique | length) != ($c.planted | length); "quorum.canary.planted[].text are not distinct"),
    chk(([$c.planted[].position] | unique | length) != ($c.planted | length); "quorum.canary.planted[].position are not distinct"),
    chk(any($c.planted[]; .position > (($c.real_claim_ids | length) + ($c.planted | length)));
        "quorum.canary.planted[].position lies beyond the claims file"),
    chk(any($c.planted[].id; . as $i | any($c.real_claim_ids[]; . == $i));
        "quorum.canary.planted[].id also appears in real_claim_ids: a canary counted as a real claim"),
    chk((($c.caught | type) != "array") or (($c.caught | sort) != ([$c.planted[].id] | sort));
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
  . as $cl
  | ($c.real_claim_ids) as $rid | ($c.planted) as $pl
  | [
    chk(($cl | type) != "array"; "canary claims_file is not a JSON array of claims"),
    chk(($cl | length) != (($rid | length) + ($pl | length));
        "canary claims_file does not hold exactly the real plus planted claims (\(try ($cl | length) catch "?") found)"),
    chk(($cl | all(.[]; (type == "object") and (keys == ["id", "text"]) and (.id | type == "string") and (.text | type == "string"))) | not;
        "canary claims_file claims must all be objects with exactly the keys id and text: a planted claim must have the form of a lane claim"),
    chk(([$cl[].id] | unique | length) != ($cl | length); "canary claims_file ids are not distinct"),
    chk(any($pl[]; . as $p | $cl[$p.position - 1] != {id: $p.id, text: $p.text});
        "canary claims_file does not carry each planted claim verbatim at its recorded position"),
    chk([$cl | to_entries[] | select(((.key + 1) as $pos | any($pl[]; .position == $pos)) | not) | .value.id] != $rid;
        "canary claims_file real claims are not quorum.canary.real_claim_ids in order")] | map(select(.[0])) | (first(.[] | .[1]) // "ok")
'

# Stage 2c: the plan that was hashed must say what the receipt says.
DOGFOOD_CANARY_PLAN_JQ='
  def proj: [.[] | {id, text, falsified_by, position}] | sort_by(.id);
  if (try ((.planted | proj) == ($c.planted | proj)) catch false)
    then "ok"
    else "canary plan_file .planted does not equal quorum.canary.planted: the hashed plan is not the plan the receipt cites"
  end
'

# Stage 2d: the judges. The majority is COMPUTED from the committed verdicts,
# never read from the receipt. $c is quorum.canary; $q is .quorum.
DOGFOOD_CANARY_VERDICTS_JQ='
  def chk(c; m): [(try ([c] | any) catch true), m];
  def refd($v; $id): ([$v[] | .verdicts[$id] | select(. == "REFUTED")] | length);
  def majority($v; $id): (refd($v; $id) * 2) > ($v | length);
  . as $v
  | ($c.real_claim_ids) as $rid | ([$c.planted[].id]) as $pid | ($rid + $pid) as $all
  | (try ([$rid[] | select(majority($v; .))] | length) catch -1) as $realref
  | [
    chk(($v | type) != "array"; "canary verdicts_file is not a JSON array with one entry per judge"),
    chk(($v | length) != $q.judges; "canary verdicts_file judge entries do not equal quorum.judges (\(try ($v | length) catch "?") entries, \($q.judges) judges)"),
    chk(($v | all(.[]; (type == "object") and (keys == ["judge", "verdicts"]) and (.judge | type == "string") and (.verdicts | type == "object"))) | not;
        "canary verdicts_file entries must be objects with exactly the keys judge and verdicts"),
    chk(([$v[].judge] | unique | length) != ($v | length); "canary verdicts_file judges are not distinct"),
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

# Words that would tell a refuter or a judge which claims are planted.
DOGFOOD_CANARY_MARKERS='canary|planted|decoy|honeypot|tripwire'

# dogfood_canary_read OUT PATH REV... : PATH at the first REV that holds it.
dogfood_canary_read() {
  local out="$1" path="$2" rev
  shift 2
  for rev in "$@"; do
    if git show "${rev}:${path}" >"$out" 2>/dev/null; then
      return 0
    fi
  done
  return 1
}

# dogfood_canary_added PATH LINE BASE HEAD : is PATH:LINE an ADDED line of
# `git diff -U0 BASE HEAD`? A fact outside the diff falsifies nothing the
# reviewed change did. Exit 0 yes, 1 no, 2 the diff could not be read.
dogfood_canary_added() {
  local path="$1" line="$2" base="$3" head="$4" d rc=0
  d="$(git diff -U0 --no-color --no-ext-diff "$base" "$head" -- "$path" 2>/dev/null)" || rc=$?
  if [ "$rc" -ne 0 ]; then
    return 2
  fi
  awk -v n="$line" '
    /^@@ / { split($3, a, ","); s = substr(a[1], 2) + 0; c = (a[2] == "" ? 1 : a[2] + 0)
             if (c > 0 && n + 0 >= s && n + 0 < s + c) hit = 1 }
    END { exit(hit ? 0 : 1) }' <<<"$d" || return 1
  return 0
}

# The file-level checks, run inside a scratch dir $1. Prints ok or a sentence.
dogfood_canary_files() {
  local tmp="$1" blob="$2" c q f role path want got out rc i n base head p line
  shift 2
  c="$(printf '%s' "$blob" | jq -c '.quorum.canary')" || return 1
  q="$(printf '%s' "$blob" | jq -c '.quorum')" || return 1
  for role in claims_file plan_file verdicts_file; do
    path="$(printf '%s' "$c" | jq -r --arg f "$role" '.[$f]')"
    want="$(printf '%s' "$blob" | jq -r --arg p "$path" '.evidence.files[] | select(.path == $p) | .sha256')"
    f="${tmp}/${role}"
    if ! dogfood_canary_read "$f" "$path" "$@"; then
      printf 'quorum.canary.%s %s is not committed at %s\n' "$role" "$path" "$*"
      return 0
    fi
    got="$(sha256sum <"$f")"
    got="${got%% *}"
    if [ "$got" != "$want" ]; then
      printf 'quorum.canary.%s %s: evidence.files sha256 %s, committed file %s\n' "$role" "$path" "${want:0:12}" "${got:0:12}"
      return 0
    fi
  done
  rc=0
  grep -qiE "$DOGFOOD_CANARY_MARKERS" "${tmp}/claims_file" || rc=$?
  if [ "$rc" -eq 0 ]; then
    printf 'quorum.canary.claims_file carries a marker word (%s): the refuters and judges were not blind\n' "$DOGFOOD_CANARY_MARKERS"
    return 0
  elif [ "$rc" -ne 1 ]; then
    printf 'quorum.canary.claims_file could not be searched for marker words (grep exit %s): UNMEASURED\n' "$rc"
    return 0
  fi
  out="$(jq -r --argjson c "$c" "$DOGFOOD_CANARY_CLAIMS_JQ" "${tmp}/claims_file" 2>/dev/null)" || out="canary claims_file is not readable JSON"
  if [ "$out" != ok ]; then
    printf '%s\n' "$out"
    return 0
  fi
  out="$(jq -r --argjson c "$c" "$DOGFOOD_CANARY_PLAN_JQ" "${tmp}/plan_file" 2>/dev/null)" || out="canary plan_file is not readable JSON"
  if [ "$out" != ok ]; then
    printf '%s\n' "$out"
    return 0
  fi
  out="$(jq -r --argjson c "$c" --argjson q "$q" "$DOGFOOD_CANARY_VERDICTS_JQ" "${tmp}/verdicts_file" 2>/dev/null)" || out="canary verdicts_file is not readable JSON"
  if [ "$out" != ok ]; then
    printf '%s\n' "$out"
    return 0
  fi
  base="$(printf '%s' "$c" | jq -r '.reviewed_base')"
  head="$(printf '%s' "$c" | jq -r '.reviewed_head')"
  for p in "$base" "$head"; do
    if ! git cat-file -e "${p}^{commit}" 2>/dev/null; then
      printf 'quorum.canary reviewed commit %s is not in this repository: the falsifying lines cannot be read, UNMEASURED\n' "${p:0:12}"
      return 0
    fi
  done
  n="$(printf '%s' "$c" | jq -r '.planted | length')"
  i=0
  while [ "$i" -lt "$n" ]; do
    p="$(printf '%s' "$c" | jq -r --argjson i "$i" '.planted[$i].falsified_by')"
    path="${p%:*}"
    line="${p##*:}"
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

# dogfood_canary_status BLOB [REV...] : ok, or the first thing wrong with
# quorum.canary. Fails closed: anything unreadable is a sentence, never ok.
dogfood_canary_status() {
  local blob="$1" why tmp rc=0
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
  if [ "$#" -eq 0 ]; then
    set -- HEAD
  fi
  tmp="$(mktemp -d)" || {
    printf '%s\n' "quorum.canary: no scratch directory to read the evidence files into, UNMEASURED"
    return 0
  }
  why="$(dogfood_canary_files "$tmp" "$blob" "$@" 2>/dev/null)" || rc=$?
  rm -rf "${tmp:?}"
  if [ "$rc" -ne 0 ] || [ -z "$why" ]; then
    printf '%s\n' "quorum.canary could not be verified against its evidence files (exit ${rc}), UNMEASURED"
    return 0
  fi
  printf '%s\n' "$why"
}

# dogfood_receipt_resolve WHY BLOB [REV...] : stage 1's answer, with the
# `canary` token replaced by the block's verdict. For callers that run
# DOGFOOD_RECEIPT_JQ themselves (gate E).
dogfood_receipt_resolve() {
  local why="$1" blob="$2"
  shift 2
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
