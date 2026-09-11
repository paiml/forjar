#!/usr/bin/env bash
# The DECLARED release goals: docs/roadmaps/releases.yaml (PMAT-225, forjar#506).
#
# SOURCED, NOT RUN, after lib/window.sh, by scripts/dogfood/tagged.sh (gate T)
# and scripts/release-goal.sh. The caller defines `fail()` first. Everything
# comes back in a DOGFOOD_* global, never on stdout (see window.sh for why).
#
# The ledger is read AT HEAD by default — a gate measures committed state —
# and from the working tree when DOGFOOD_RELEASES_REF=worktree, which is what
# the status line wants (the PMAT-225 plan grill: a status that ignores the
# row you are editing is not a status).
#
# Timestamps stay STRINGS. PyYAML would otherwise turn `2026-09-08T22:10:01Z`
# into a datetime and json.dumps would refuse it (or `default=str` would
# rewrite it in another format, and the gate's equality against git's date
# would fail for a reason that is not a disagreement).
DOGFOOD_RELEASES_PY='
import json, sys, yaml
class Loader(yaml.SafeLoader):
    pass
Loader.yaml_implicit_resolvers = {
    k: [(t, r) for (t, r) in v if t != "tag:yaml.org,2002:timestamp"]
    for k, v in Loader.yaml_implicit_resolvers.items()
}
print(json.dumps(yaml.load(sys.stdin, Loader=Loader)))
'

# The shape every reader relies on. jq -e exits 1 when the expression is false.
DOGFOOD_RELEASES_SHAPE='
  (type == "object")
  and (.cadence_days | type == "number" and . >= 1 and . == floor)
  and (.floor | type == "string" and test("^v[0-9]+\\.[0-9]+\\.[0-9]+$"))
  and (.harness_floor | type == "string" and test("^v[0-9]+\\.[0-9]+\\.[0-9]+$"))
  and (.dogfood_floor | type == "string" and test("^v[0-9]+\\.[0-9]+\\.[0-9]+$"))
  and (.releases | type == "array")
  and all(.releases[];
        (.tag | type == "string" and test("^v[0-9]+\\.[0-9]+\\.[0-9]+$"))
        and (.cut | type == "string" and test("^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$"))
        and (.prs | type == "array" and all(.[]; type == "number"))
        and (.tickets | type == "array" and all(.[]; type == "string" and test("^PMAT-[0-9]+$"))))
  and (.next | type == "object")
  and (.next.tag | type == "string" and test("^v[0-9]+\\.[0-9]+\\.[0-9]+$"))
  and (.next.due | type == "string" and test("^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$"))
'

# docs/roadmaps/releases.yaml -> DOGFOOD_RELEASES (compact JSON),
# DOGFOOD_RELEASES_TEXT (the bytes), DOGFOOD_RELEASES_NEXT_LINE (the line
# number of `next:`, the basis a status line cites).
dogfood_load_releases() {
  local ref="${DOGFOOD_RELEASES_REF:-HEAD}" rc=0 text json line
  if [ "$ref" = "worktree" ]; then
    text="$(cat docs/roadmaps/releases.yaml)" || rc=$?
  else
    text="$(git show "${ref}:docs/roadmaps/releases.yaml")" || rc=$?
  fi
  if [ "$rc" -ne 0 ]; then
    fail "docs/roadmaps/releases.yaml cannot be read at ${ref} (exit ${rc}): there is no declared release goal to measure against — UNMEASURED"
  fi
  json="$(printf '%s\n' "$text" | python3 -c "$DOGFOOD_RELEASES_PY" 2>&1)" || rc=$?
  if [ "$rc" -ne 0 ]; then
    fail "docs/roadmaps/releases.yaml at ${ref} does not parse (python exit ${rc}: ${json}) — a ledger that cannot be read declares nothing, UNMEASURED"
  fi
  rc=0
  printf '%s' "$json" | jq -e "$DOGFOOD_RELEASES_SHAPE" >/dev/null || rc=$?
  if [ "$rc" -ne 0 ]; then
    fail "docs/roadmaps/releases.yaml at ${ref} is not the declared shape (jq exit ${rc}): cadence_days (integer >= 1), floor / harness_floor / dogfood_floor (vX.Y.Z), releases[] with tag, cut (UTC, ...Z), prs (numbers) and tickets (PMAT-n), and next.tag / next.due"
  fi
  rc=0
  line="$(printf '%s\n' "$text" | grep -n -E '^next:' | cut -d: -f1)" || rc=$?
  if [ "$rc" -gt 1 ]; then
    fail "grep exited ${rc} locating next: in docs/roadmaps/releases.yaml — UNMEASURED"
  fi
  DOGFOOD_RELEASES="$json"
  DOGFOOD_RELEASES_TEXT="$text"
  DOGFOOD_RELEASES_NEXT_LINE="${line:-0}"
}

# One jq expression over the ledger -> DOGFOOD_FIELD (raw; null reads as "").
dogfood_releases_field() {
  local rc=0 v
  v="$(printf '%s' "$DOGFOOD_RELEASES" | jq -r "($1) // \"\"")" || rc=$?
  if [ "$rc" -ne 0 ]; then
    fail "cannot read ${1} out of docs/roadmaps/releases.yaml (jq exit ${rc}) — UNMEASURED"
  fi
  DOGFOOD_FIELD="$v"
}

# The declared row for tag $1 -> DOGFOOD_RELEASE_ROW (compact JSON, or "").
dogfood_release_row() {
  local rc=0 v
  v="$(printf '%s' "$DOGFOOD_RELEASES" | jq -c --arg t "$1" '[.releases[] | select(.tag == $t)] | if length == 1 then .[0] elif length == 0 then "" else error("\(length) rows") end')" || rc=$?
  if [ "$rc" -ne 0 ]; then
    fail "docs/roadmaps/releases.yaml declares ${1} more than once (jq exit ${rc}): two rows for one tag declare nothing"
  fi
  case "$v" in
    '""') DOGFOOD_RELEASE_ROW="" ;;
    *) DOGFOOD_RELEASE_ROW="$v" ;;
  esac
}

# Is version tag $1 >= $2 by semantic order? Exit 0 or 1 — usable in `if`.
dogfood_semver_ge() {
  [ "$(printf '%s\n%s\n' "$1" "$2" | sort -V | tail -1)" = "$1" ]
}

# An ISO-8601 UTC instant -> DOGFOOD_EPOCH (seconds).
dogfood_epoch() {
  local rc=0 v
  # A declared instant is PARSED here; no clock is read.
  v="$(date -u -d "$1" +%s 2>&1)" || rc=$? # bashrs disable-line=DET002
  if [ "$rc" -ne 0 ] || [ -z "$v" ]; then
    fail "cannot read ${1} as an instant (date exit ${rc}: ${v}) — UNMEASURED"
  fi
  DOGFOOD_EPOCH="$v"
}

# Seconds -> DOGFOOD_ISO (UTC, the ledger's spelling).
dogfood_iso() {
  local rc=0 v
  # A given epoch is RENDERED here; no clock is read.
  v="$(date -u -d "@$1" +%Y-%m-%dT%H:%M:%SZ 2>&1)" || rc=$? # bashrs disable-line=DET002
  if [ "$rc" -ne 0 ] || [ -z "$v" ]; then
    fail "cannot render ${1} as an instant (date exit ${rc}: ${v}) — UNMEASURED"
  fi
  DOGFOOD_ISO="$v"
}

# The [package] version of Cargo.toml at ${DOGFOOD_RELEASES_REF:-HEAD} ->
# DOGFOOD_CARGO_VERSION. The first `version = "..."` line: the root manifest's
# [package] table comes first, and the workspace members are path deps.
dogfood_cargo_version() {
  local ref="${DOGFOOD_RELEASES_REF:-HEAD}" rc=0 text v
  if [ "$ref" = "worktree" ]; then
    text="$(cat Cargo.toml)" || rc=$?
  else
    text="$(git show "${ref}:Cargo.toml")" || rc=$?
  fi
  if [ "$rc" -ne 0 ]; then
    fail "Cargo.toml cannot be read at ${ref} (exit ${rc}), so whether a cut is in flight is UNMEASURED"
  fi
  v="$(printf '%s\n' "$text" | sed -n 's/^version = "\([0-9][0-9.]*\)".*/\1/p' | head -1)"
  if [ -z "$v" ]; then
    fail "Cargo.toml at ${ref} has no version = \"x.y.z\" line, so whether a cut is in flight is UNMEASURED"
  fi
  DOGFOOD_CARGO_VERSION="$v"
}

# ---------------------------------------------------------- version requirements
#
# Does version $1 satisfy the requirement whose operator is $2 and whose
# version is $3? (PMAT-241.) Exit 0 admits, 2 is below the requirement, 3 is at
# or past its ceiling, 4 is a version this rule does not evaluate. The ceiling
# is left in DOGFOOD_REQ_UPPER so a caller can name it.
#
# WHY THIS IS NOT `>=`. Cargo reads `forjar = "1.2"` as a CARET — `>=1.2.0,
# <2.0.0` — so `dogfood_semver_ge v2.0.0 v1.2` is true where Cargo refuses.
# That would pass the release that BREAKS the cookbook, which is the one case
# the arm using this exists for. The three operators Cargo spells differently
# have three different ceilings and are kept apart here:
#
#   ^1.2  ^1.2.3  1.2      next increment of the leftmost NON-ZERO component
#   ^0.2  ^0.0.3  ^0       that was SPECIFIED (^0.2 -> <0.3.0, ^0.0.3 -> <0.0.4,
#                          ^0 -> <1.0.0, ^0.0 -> <0.1.0)
#   ~1.2  ~1.2.3           the minor, when one was given (-> <1.3.0); the major
#   ~1                     when it was not (-> <2.0.0)
#   =1.2.3                 exactly that version (-> <1.2.4)
#   =1.2  =1               the last component given (-> <1.3.0, <2.0.0)
#
# A `~` or `=` requirement read as a caret is WIDER than what Cargo admits, so
# treating them alike is a false green by construction, not a rounding error.
#
# 1 to 3 numeric components, no leading zeros: a plain version this rule can do
# arithmetic on. Anything else — a pre-release, a build metadata suffix, a
# fourth component, `1.*` — is REFUSED rather than measured wrong: `sort -V`
# orders `1.2.4-alpha` ABOVE `1.2.4` where Cargo puts it below, and bash reads
# a component of `3-9` as a subtraction and lands on an upper bound of `0.0.-5`.
dogfood_plain_version() {
  local v="$1" part rest n=0
  case "$v" in ''|*[!0-9.]*|.*|*.|*..*) return 1 ;; esac
  rest="$v"
  while [ -n "$rest" ]; do
    part="${rest%%.*}"
    if [ "$part" = "$rest" ]; then rest=""; else rest="${rest#*.}"; fi
    n=$((n + 1))
    [ "$n" -le 3 ] || return 1
    case "$part" in 0) ;; 0*) return 1 ;; esac
  done
  DOGFOOD_VERSION_PARTS="$n"
  return 0
}

dogfood_req_admits() {
  local ver="$1" op="$2" req="$3" n r1 r2 r3 rest lower upper
  dogfood_plain_version "$ver" || return 4
  dogfood_plain_version "$req" || return 4
  n="$DOGFOOD_VERSION_PARTS"
  r1="${req%%.*}"; rest="${req#*.}"
  if [ "$n" -eq 1 ]; then r2=0; r3=0
  elif [ "$n" -eq 2 ]; then r2="$rest"; r3=0
  else r2="${rest%%.*}"; r3="${rest#*.}"
  fi
  lower="${r1}.${r2}.${r3}"
  dogfood_semver_ge "v${ver}" "v${lower}" || return 2
  case "$op" in
    '~')
      if [ "$n" -ge 2 ]; then upper="${r1}.$((r2 + 1)).0"; else upper="$((r1 + 1)).0.0"; fi ;;
    '=')
      if [ "$n" -ge 3 ]; then upper="${r1}.${r2}.$((r3 + 1))"
      elif [ "$n" -eq 2 ]; then upper="${r1}.$((r2 + 1)).0"
      else upper="$((r1 + 1)).0.0"; fi ;;
    *)
      if [ "$r1" != 0 ]; then upper="$((r1 + 1)).0.0"
      elif [ "$n" -ge 2 ] && [ "$r2" != 0 ]; then upper="0.$((r2 + 1)).0"
      elif [ "$n" -ge 3 ] && [ "$r3" != 0 ]; then upper="0.0.$((r3 + 1))"
      elif [ "$n" -eq 1 ]; then upper="1.0.0"
      elif [ "$n" -eq 2 ]; then upper="0.1.0"
      else upper="0.0.1"; fi ;;
  esac
  DOGFOOD_REQ_UPPER="$upper"
  if dogfood_semver_ge "v${ver}" "v${upper}"; then return 3; fi
  return 0
}
