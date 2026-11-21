#!/usr/bin/env bash
# The crates.io version index, as gate T's T8b exemption reads it (PMAT-1150).
#
# SOURCED by scripts/dogfood/tagged.sh; kept apart so the predecessor selection
# and declaration check can be exercised directly without running the full gate.
# The caller defines `fail()` before sourcing (see window.sh).

# The `crates_io:` declaration, checked on EVERY row whatever its floor or lock
# (PMAT-1150). Absent is fine; the exact word `unpublished` is fine; any other
# value (empty, null, false, padded, another word) is red, and a jq that cannot
# answer is UNMEASURED, never "absent". Returns 0 when the row declares
# `unpublished`, 1 when it declares nothing.
crates_io_declaration_is_known() {
  local row="$1" tag="$2" rc=0
  jq -se 'if length == 1 and (.[0] | type) == "object" then .[0] | has("crates_io") else error("row is not one object") end' <<<"$row" >/dev/null 2>&1 || rc=$?
  case "$rc" in
    0) ;;
    1) return 1 ;;
    *) fail "${tag}: jq exited ${rc} asking whether the row declares crates_io -- UNMEASURED, and an unanswered question is not an absent field" ;;
  esac
  rc=0
  jq -se 'if length == 1 and (.[0] | type) == "object" then .[0].crates_io == "unpublished" else error("row is not one object") end' <<<"$row" >/dev/null 2>&1 || rc=$?
  case "$rc" in
    0) return 0 ;;
    1) fail "${tag}: the row's crates_io is not a value this gate knows -- the only declaration is exactly \`unpublished\`; a published release carries no such field" ;;
    *) fail "${tag}: jq exited ${rc} reading the row's crates_io -- UNMEASURED" ;;
  esac
}

# The newest NON-YANKED STABLE version strictly below $1, compared as numbers
# (not as text, not with sort -V), from a single crates.io index document on
# stdin. Fails closed on empty stdin, NDJSON, or any malformed version entry.
# Prints nothing when no non-yanked stable version exists below $1.
cratesio_newest_stable_below() {
  jq -rs --arg v "$1" '
    if ($v | test("\\A(0|[1-9][0-9]{0,8})[.](0|[1-9][0-9]{0,8})[.](0|[1-9][0-9]{0,8})\\z")) | not then
      error("invalid target version")
    elif length != 1 then
      error("expected single JSON document")
    else .[0] end
    | if (type == "object") and (.versions | type == "array") and all(.versions[];
        (type == "object")
        and (.yanked | type == "boolean")
        and (.num | type == "string" and test("\\A(0|[1-9][0-9]{0,8})[.](0|[1-9][0-9]{0,8})[.](0|[1-9][0-9]{0,8})(-[0-9A-Za-z.+-]+)?\\z")))
      then .
      else error("malformed crates.io versions")
      end
    | ($v | split(".") | map(tonumber)) as $t
    | [.versions[]
       | select(.yanked == false and (.num | contains("-") | not))
       | .num | split(".") | map(tonumber)]
    | map(select(. < $t)) | sort | last // empty | map(tostring) | join(".")'
}
