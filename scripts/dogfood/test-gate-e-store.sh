#!/usr/bin/env bash
# Cargo-less falsifier for gate E's head-bound store check (forjar#876).
#
# Builds a temp repo: a tag on base, a PR branch whose head commit carries a
# committed receipt and a Co-Authored-By trailer, and a squash-merge commit on
# main whose parent is base. Then runs the REAL quorum.sh with a stub `gh`.
# Gate E must refuse in both cases below. Case 2 is the falsifier: with the
# store check removed, the committed receipt alone would pass.
#
# Not covered here: a PASS, which needs a real store receipt and the store
# gate committed at MERGE^1. That half is for CI and the clean-room run.
#
# mutation: delete the dogfood_store_check call in quorum.sh; case 2 then exits non-zero without the store=missing refusal, so this falsifier fails.
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "${work:?}"' EXIT

fx="$work/fx"
mkdir -p "$fx/scripts"
cp -R "$repo/scripts/dogfood" "$fx/scripts/"
cp "$repo/scripts/quorum-store-gate.sh" "$repo/scripts/quorum-models.json" "$fx/scripts/"
cd "$fx"
git init -q -b main
git config core.hooksPath /dev/null
git config user.email tester@example.com
git config user.name tester
git add -A
git commit -q -m "base: the gate and its judge are committed here (MERGE^1)"
git tag v0.0.1

# The PR branch: its head commit carries the committed receipt and the author
# trailer that the store gate maps through the allowlist.
git checkout -q -b PMAT-999/harness-and-quorum
mkdir -p .quorum docs/audits
cat > .quorum/PMAT-999-harness-and-quorum.json <<'JSON'
{"quorum": {"lanes": ["a", "b", "c"], "judges": 3, "refuters_per_claim": 3,
  "claims_confirmed": 12, "claims_refuted": 2},
  "evidence": {"files": ["docs/audits/claims.md"]}}
JSON
echo "claims" > docs/audits/claims.md
git add -A
git commit -q -m "PR head carries its committed receipt" \
  -m "Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
head_sha="$(git rev-parse HEAD)"

# The squash merge lands on main, parented on base, with the PR's tree.
git checkout -q main
git checkout -q "$head_sha" -- .quorum docs
git add -A
git commit -q -m "squash merge (#77)"
merge_sha="$(git rev-parse HEAD)"

cat > "$work/gh" <<STUB
#!/usr/bin/env bash
cat <<'JSON'
[{"number":77,"mergedAt":"2026-09-05T00:00:00Z","mergeCommit":{"oid":"$merge_sha"},"headRefName":"PMAT-999/harness-and-quorum","headRefOid":"$head_sha","title":"t","body":"b"}]
JSON
STUB
chmod +x "$work/gh"

mkdir -p "$work/store"

# Case 1: QUORUM_STORE unset. Gate E must refuse as UNMEASURED.
rc=0
out1="$(env -u QUORUM_STORE GH="$work/gh" bash scripts/dogfood/quorum.sh 2>&1)" || rc=$?
if [ "$rc" -eq 0 ]; then
  echo "FALSIFIER FAIL case 1: gate E passed with QUORUM_STORE unset"
  exit 1
fi
case "$out1" in
  *"GATE E FAIL"*UNMEASURED*) ;;
  *) echo "FALSIFIER FAIL case 1: no GATE E FAIL UNMEASURED line: $out1"; exit 1 ;;
esac

# Case 2: committed receipt passes, the store is EMPTY (no receipt for this
# head). Gate E must refuse with store=missing, and the refusal must come from
# the store receipt, not from a judge file that is absent at MERGE^1.
rc=0
out2="$(QUORUM_STORE="$work/store" GH="$work/gh" bash scripts/dogfood/quorum.sh 2>&1)" || rc=$?
if [ "$rc" -eq 0 ]; then
  echo "FALSIFIER FAIL case 2: gate E passed with no store receipt"
  exit 1
fi
case "$out2" in
  *"passes, but its head-bound store receipt is missing (store=missing)"*) ;;
  *) echo "FALSIFIER FAIL case 2: refusal is not store=missing: $out2"; exit 1 ;;
esac
case "$out2" in
  *"is not committed at"*) echo "FALSIFIER FAIL case 2: the judge was absent, so the store check never read the store: $out2"; exit 1 ;;
  *) ;;
esac

echo "FALSIFIER PASS gate E refuses unset QUORUM_STORE and an empty store (store=missing)"
