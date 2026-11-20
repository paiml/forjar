#!/usr/bin/env bash
# Proves file-ticket.sh and ticket-triage-count.sh behave, using a stub `gh`
# (FORJAR_GH) that logs every call. One planted case per rule.
set -u
here=$(cd "$(dirname "$0")" && pwd) || exit 2
T=$(mktemp -d) || exit 2
trap 'rm -rf -- "${T:?}"' EXIT
cat >"$T/gh" <<'STUB'
#!/usr/bin/env bash
echo "$*" >>"$STUB_LOG"
case "$*" in
  *"issues/879"*"--jq"*) echo 10 ;;
  *"-X POST"*"/sub_issues"*) echo '{"id":1}' ;;
  *"-X POST"*"/issues"*) echo '{"number":2000,"id":777}' ;;
esac
STUB
chmod +x "$T/gh"
export FORJAR_GH="$T/gh" STUB_LOG="$T/log"
printf 'body\n' >"$T/body.md"
fail=0
ck() { if ! "$@"; then echo "FAIL: $*"; fail=$((fail + 1)); fi; }

# 1. no parent: refuses (rc 2) and makes no gh call.
: >"$T/log"
"$here/file-ticket.sh" --title t --body-file "$T/body.md" >/dev/null 2>&1
rc=$?
ck test "$rc" -eq 2
ck test ! -s "$T/log"

# 2. with a parent: creates with the parent's milestone and `triaged`, then links it.
: >"$T/log"
"$here/file-ticket.sh" --parent 879 --title t --body-file "$T/body.md" >/dev/null 2>&1
ck test $? -eq 0
ck grep -q 'labels\[\]=triaged' "$T/log"
ck grep -q 'milestone=10' "$T/log"
ck grep -q 'sub_issues' "$T/log"
ck grep -q 'sub_issue_id=777' "$T/log"

# 3. count: tallies the three gaps and the cap from a GraphQL reply.
cat >"$T/q.json" <<'J'
{"data":{"repository":{"issues":{"totalCount":3,"nodes":[
{"number":1,"milestone":{"title":"m"},"parent":{"number":9},"labels":{"nodes":[{"name":"triaged"}]}},
{"number":2,"milestone":null,"parent":null,"labels":{"nodes":[]}},
{"number":3,"milestone":{"title":"m"},"parent":null,"labels":{"nodes":[{"name":"triaged"}]}}]}}}}
J
out=$(TRIAGE_COUNT_JSON="$T/q.json" "$here/ticket-triage-count.sh")
ck test "$?" -eq 0
ck grep -qx 'open=3' <<<"$out"
ck grep -qx 'no_parent=2' <<<"$out"
ck grep -qx 'no_milestone=1' <<<"$out"
ck grep -qx 'no_triaged=1' <<<"$out"
ck grep -qx 'over_cap=no' <<<"$out"

# 4. unreadable input is not_measured (rc 3), never zeros.
printf 'garbage' >"$T/bad.json"
TRIAGE_COUNT_JSON="$T/bad.json" "$here/ticket-triage-count.sh" >/dev/null 2>&1
ck test $? -eq 3

if [ "$fail" -eq 0 ]; then echo "ok: file-ticket tests passed"; else echo "$fail failed"; exit 1; fi
