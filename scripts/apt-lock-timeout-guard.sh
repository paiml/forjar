#!/usr/bin/env bash
# forjar#492: shared self-hosted runners contend on the apt lock. Every
# `apt-get update|install` in a workflow must carry `-o DPkg::Lock::Timeout=N`
# with N a positive integer, so apt waits instead of failing the leg
# (Timeout=0 means "do not wait", the failure this guards against).
#
# Each apt-get invocation is judged on its own: a line is joined with its
# backslash continuations, stripped of `# comments`, and split on `&&`, `||`,
# `;`, `|` and subshell brackets, so a compliant `update` cannot vouch for a
# bare `install` after it, and a comment naming the option does not count.
# `echo`/`printf` prose and YAML keys other than `run:` (a step `name:`) are
# not calls.
#
# usage: apt-lock-timeout-guard.sh [dir]     scan dir/*.yml and dir/*.yaml
#        apt-lock-timeout-guard.sh --self-test   run the planted-defect cases
# Exit 0 ok, 1 violation, 2 not measured (no directory, no files, no calls).
set -u
here=$(cd "$(dirname "$0")" && pwd) || exit 2
if [ "${1:-}" = "--self-test" ]; then
  exec bash "$here/apt-lock-timeout-guard-test.sh"
fi
dir="${1:-.github/workflows}"
[ -d "$dir" ] || { echo "apt-lock-timeout-guard: $dir missing (not measured)"; exit 2; }
shopt -s nullglob
files=("$dir"/*.yml "$dir"/*.yaml)
shopt -u nullglob
[ "${#files[@]}" -gt 0 ] || { echo "apt-lock-timeout-guard: 0 workflow files (not measured)"; exit 2; }

prog=$(
  cat <<'AWK'
function judge(text, lineno,    t, key, nseg, segs, s, nw, w, i, k, sub_cmd, ok, first) {
  t = text
  gsub(/(^|[ \t])#.*$/, "", t)
  sub(/^[ \t]+/, "", t)
  sub(/^-[ \t]+/, "", t)
  if (match(t, /^[A-Za-z_][A-Za-z0-9_-]*:([ \t]|$)/)) {
    key = substr(t, 1, RLENGTH)
    if (key !~ /^run:/) return
    t = substr(t, RLENGTH + 1)
  }
  gsub(/&&|\|\||[;|(){}&`]/, "\n", t)
  nseg = split(t, segs, "\n")
  for (s = 1; s <= nseg; s++) {
    nw = split(segs[s], w, /[ \t]+/)
    k = 0
    first = ""
    for (i = 1; i <= nw; i++) {
      gsub(/["\047]/, "", w[i])
      if (w[i] == "") continue
      if (first == "" && w[i] !~ /^(then|do|else|elif|if|while|until|!)$/) first = w[i]
      if (k == 0 && (w[i] == "apt-get" || w[i] ~ /\/apt-get$/)) k = i
    }
    if (k == 0 || first == "echo" || first == "printf") continue
    sub_cmd = ""
    for (i = k + 1; i <= nw; i++) {
      if (w[i] == "") continue
      if (w[i] ~ /^-/) {
        if (w[i] ~ /^-[octa]$/) i++
        continue
      }
      sub_cmd = w[i]
      break
    }
    if (sub_cmd != "update" && sub_cmd != "install") continue
    ok = 0
    for (i = k + 1; i <= nw; i++)
      if (w[i] ~ /^(-o)?DPkg::Lock::Timeout=[1-9][0-9]*$/) ok = 1
    print (ok ? "OK" : "BAD") "\t" FILENAME ":" lineno ": " text
  }
}
FNR == 1 { if (buf != "") judge(buf, start); buf = ""; start = 0 }
{
  line = $0
  if (buf == "") start = FNR
  if (line ~ /\\[ \t]*$/) {
    sub(/\\[ \t]*$/, " ", line)
    buf = buf line
    next
  }
  judge(buf line, start)
  buf = ""
}
END { if (buf != "") judge(buf, start) }
AWK
)
calls=$(awk "$prog" "${files[@]}") || { echo "apt-lock-timeout-guard: awk failed (not measured)"; exit 2; }
n=0
[ -z "$calls" ] || n=$(printf '%s\n' "$calls" | grep -c .)
echo "apt-lock-timeout-guard: $n apt-get call(s) over ${#files[@]} workflow file(s)"
[ "$n" -gt 0 ] || { echo "0 calls scanned (not measured)"; exit 2; }
bad=$(printf '%s\n' "$calls" | grep '^BAD' || true)
if [ -n "$bad" ]; then
  echo "VIOLATION (no positive -o DPkg::Lock::Timeout=N on the call):"
  printf '%s\n' "$bad" | cut -f2-
  exit 1
fi
exit 0
