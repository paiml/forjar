#!/usr/bin/env bash
# Files a ticket complete: parent (sub-issue), the parent's milestone, and the
# `triaged` label, in one step. Refuses when no parent is given (rc 2).
# Usage: file-ticket.sh --parent N --title T --body-file F [--repo O/R]
# Writes go through REST (the real error comes back); check what it returns.
set -u
gh=${FORJAR_GH:-gh}
repo=paiml/forjar parent="" title="" body=""
while [ $# -gt 0 ]; do
  case "$1" in
    --parent | --title | --body-file | --repo)
      if [ $# -lt 2 ]; then echo "refused: $1 requires a value" >&2; exit 2; fi
      case "$1" in
        --parent) parent=$2 ;;
        --title) title=$2 ;;
        --body-file) body=$2 ;;
        --repo) repo=$2 ;;
      esac
      shift 2 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done
case "$parent" in
  "" | *[!0-9]*) echo "refused: --parent <issue number> is required (look for a group ticket first; at 50 open, fold or close one)" >&2; exit 2 ;;
esac
if [ -z "$title" ] || [ ! -r "$body" ]; then
  echo "refused: --title and a readable --body-file are required" >&2
  exit 2
fi
ms=$("$gh" api "repos/$repo/issues/$parent" --jq '.milestone.number // empty') || exit 3
if [ -z "$ms" ]; then
  echo "refused: parent #$parent has no milestone to inherit" >&2
  exit 2
fi
out=$("$gh" api -X POST "repos/$repo/issues" -f "title=$title" -F "body=@$body" \
  -F "milestone=$ms" -f 'labels[]=triaged') || exit 3
num=$(jq -r '.number // empty' <<<"$out")
id=$(jq -r '.id // empty' <<<"$out")
if [ -z "$num" ] || [ -z "$id" ]; then
  echo "create returned no issue: $out" >&2
  exit 3
fi
"$gh" api -X POST "repos/$repo/issues/$parent/sub_issues" -F "sub_issue_id=$id" >/dev/null || {
  echo "filed #$num but linking to #$parent failed" >&2
  exit 3
}
echo "filed #$num under #$parent (milestone $ms, triaged)"
