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
