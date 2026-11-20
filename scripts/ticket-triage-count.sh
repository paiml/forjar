#!/usr/bin/env bash
# LAB count (nightly, never a PR-path check): open tickets and how many lack a
# parent, a milestone or the `triaged` label. Prints key=value lines; rc 3 and
# not_measured when the read fails or does not parse (never zeros).
# TRIAGE_COUNT_JSON=<file> reads a saved reply instead of calling GraphQL.
set -u
gh=${FORJAR_GH:-gh}
cap=50
q='query{repository(owner:"paiml",name:"forjar"){issues(states:OPEN,first:100){totalCount nodes{number milestone{title} parent{number} labels(first:20){nodes{name}}}}}}'
if [ -n "${TRIAGE_COUNT_JSON:-}" ]; then
  json=$(cat "$TRIAGE_COUNT_JSON") || json=""
else
  json=$("$gh" api graphql -f "query=$q") || json=""
fi
res=$(jq -r --argjson cap "$cap" '.data.repository.issues as $i | select($i != null) |
  "open=\($i.totalCount)",
  "no_parent=\([$i.nodes[] | select(.parent == null)] | length)",
  "no_milestone=\([$i.nodes[] | select(.milestone == null)] | length)",
  "no_triaged=\([$i.nodes[] | select(([.labels.nodes[].name] | index("triaged")) == null)] | length)",
  "over_cap=\(if $i.totalCount > $cap then "yes" else "no" end)"' <<<"$json" 2>/dev/null)
if [ -z "$res" ]; then
  echo "not_measured"
  exit 3
fi
echo "$res"
