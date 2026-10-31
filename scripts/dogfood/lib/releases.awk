# The ledger loader for scripts/dogfood/lib/releases.sh (PMAT-607, forjar#607).
# It replaced a python3 + PyYAML one-liner: the release path runs no Python.
#
# docs/roadmaps/releases.yaml -> one line of JSON on stdout, or a reason on
# stderr and exit 2. The ledger's subset of YAML, and nothing else: top-level
# `key: scalar`, `key: [flow, list]`, `key: []`; `key:` opening either a list of
# maps (`  - k: v` then `    k: v`) or a map (`  k: v`); full-line and trailing
# ` #` comments. Any other line shape FAILS, by line number — a loader that
# guessed at a shape it does not know would hand the gate a ledger nobody wrote.
# Scalars resolve as PyYAML's SafeLoader did minus timestamps (which the old
# loader removed): ints, floats, null, booleans; everything else a string.
function bad(why) {
  printf "line %d: %s: %s\n", NR, why, $0 > "/dev/stderr"
  failed = 1
  exit 2
}
function str(s) {
  gsub(/\\/, "\\\\", s); gsub(/"/, "\\\"", s); gsub(/\t/, "\\t", s)
  return "\"" s "\""
}
function scalar(v) {
  if (v ~ /^'[^']*'$/ || v ~ /^"[^"\\]*"$/) return str(substr(v, 2, length(v) - 2))
  sub(/[ \t]+#.*$/, "", v)
  sub(/[ \t]+$/, "", v)
  if (v ~ /^['"]/) bad("a quoted scalar this loader does not read")
  if (v ~ /^[&*!|>%@`{}\[\]]/) bad("a YAML form outside the ledger's subset")
  if (v == "" || v ~ /^(~|null|Null|NULL)$/) return "null"
  if (v ~ /^(true|True|TRUE|yes|Yes|YES|on|On|ON)$/) return "true"
  if (v ~ /^(false|False|FALSE|no|No|NO|off|Off|OFF)$/) return "false"
  if (v ~ /^[-+]?(0|[1-9][0-9]*)$/) { sub(/^\+/, "", v); return v }
  if (v ~ /^[-+]?(0|[1-9][0-9]*)\.[0-9]+$/) { sub(/^\+/, "", v); return v }
  # PyYAML resolves these to numbers too; this loader does not model them,
  # and a value it would read differently must not be read at all.
  if (v ~ /^[-+]?0[0-7_]+$/ || v ~ /^[-+]?0b[01_]+$/ || v ~ /^[-+]?0x[0-9a-fA-F_]+$/ \
      || v ~ /^[-+]?[0-9][0-9_]*$/ \
      || v ~ /^[-+]?[0-9][0-9_]*(:[0-5]?[0-9])+(\.[0-9_]*)?$/ \
      || v ~ /^[-+]?[0-9][0-9_]*\.[0-9_]*([eE][-+][0-9]+)?$/ \
      || v ~ /^[-+]?\.[0-9_]+([eE][-+][0-9]+)?$/ \
      || v ~ /^[-+]?\.(inf|Inf|INF)$/ || v ~ /^\.(nan|NaN|NAN)$/) bad("a numeric form this loader does not read")
  return str(v)
}
function value(v,   inner, n, parts, i, out, p) {
  sub(/^[ \t]+/, "", v)
  if (v ~ /^\[/) {
    sub(/[ \t]+#.*$/, "", v)
    sub(/[ \t]+$/, "", v)
    if (v !~ /^\[[^\]\[{}'"]*\]$/) bad("a flow list this loader does not read")
    inner = substr(v, 2, length(v) - 2)
    if (inner ~ /^[ \t]*$/) return "[]"
    n = split(inner, parts, ",")
    out = ""
    for (i = 1; i <= n; i++) {
      p = parts[i]
      sub(/^[ \t]+/, "", p); sub(/[ \t]+$/, "", p)
      if (p == "") bad("an empty flow-list item")
      out = out (i > 1 ? "," : "") scalar(p)
    }
    return "[" out "]"
  }
  return scalar(v)
}
# Close whatever the previous top-level key opened.
function close_top() {
  if (pend != "") { out = out sep str(pend) ":null"; sep = ","; pend = "" }
  if (ctx == "arr") { out = out (item ? "}" : "") "]" }
  if (ctx == "map") { out = out "}" }
  ctx = ""; item = 0; isep = ""
}
# A pending `key:` with no value becomes a container on its first child line.
function open_child(kind) {
  if (pend != "") {
    out = out sep str(pend) ":" (kind == "arr" ? "[" : "{")
    sep = ","; pend = ""; ctx = kind; item = 0; isep = ""
  } else if (ctx != kind) {
    bad("an indented line with no parent key that opens a " (kind == "arr" ? "list" : "map"))
  }
}
BEGIN { out = "{"; sep = ""; pend = ""; ctx = ""; failed = 0 }
/^[ \t]*$/ { next }
/^[ \t]*#/ { next }
/\t/ { bad("a tab") }
/^[A-Za-z_][A-Za-z0-9_-]*:([ ].*)?$/ {
  close_top()
  k = $0; sub(/:.*$/, "", k)
  v = $0; sub(/^[^:]*:/, "", v)
  vv = v; sub(/[ \t]+#.*$/, "", vv); sub(/^[ \t]+/, "", vv); sub(/[ \t]+$/, "", vv)
  if (vv == "") { pend = k; next }
  out = out sep str(k) ":" value(v); sep = ","
  next
}
/^  - [A-Za-z_][A-Za-z0-9_-]*:([ ].*)?$/ {
  open_child("arr")
  k = $0; sub(/^  - /, "", k); sub(/:.*$/, "", k)
  v = $0; sub(/^  - [^:]*:/, "", v)
  out = out (item ? "}," : "") "{" str(k) ":" value(v)
  item = 1; isep = ","
  next
}
/^    [A-Za-z_][A-Za-z0-9_-]*:([ ].*)?$/ {
  if (ctx != "arr" || !item) bad("an item key outside a list item")
  k = $0; sub(/^    /, "", k); sub(/:.*$/, "", k)
  v = $0; sub(/^    [^:]*:/, "", v)
  out = out isep str(k) ":" value(v)
  next
}
/^  [A-Za-z_][A-Za-z0-9_-]*:([ ].*)?$/ {
  open_child("map")
  k = $0; sub(/^  /, "", k); sub(/:.*$/, "", k)
  v = $0; sub(/^  [^:]*:/, "", v)
  out = out isep str(k) ":" value(v); isep = ","
  next
}
{ bad("a line shape outside the ledger's subset of YAML") }
END {
  if (failed) exit 2
  close_top()
  print out "}"
}
