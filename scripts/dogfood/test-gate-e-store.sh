#!/usr/bin/env bash
# Cargo-less falsifier for gate E's head-bound store check (forjar#876).
#
# Builds a temp repo with a tag, one committed receipt that passes the
# committed check, and a squash-merge commit, then runs the REAL quorum.sh
# with a stub `gh`. Gate E must refuse in both cases below. Without the store
# AND, case 2 would pass, so case 2 is the falsifier.
#
# Not covered here: a PASS, which needs a real store receipt and the store
# gate committed at MERGE^1. That half is for CI and the clean-room run.
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "${work:?}"' EXIT

fx="$work/fx"
mkdir -p "$fx/scripts"
cp -R "$repo/scripts/dogfood" "$fx/scripts/"
cd "$fx"
git init -q
git config user.email tester@example.com
git config user.name tester
git commit -q --allow-empty -m base
git tag v0.0.1
mkdir -p .quorum
cat > .quorum/PMAT-999-harness-and-quorum.json <<'JSON'
{"quorum": {"lanes": ["a", "b", "c"], "judges": 3, "refuters_per_claim": 3,
  "claims_confirmed": 12, "claims_refuted": 2},
  "evidence": {"files": ["docs/audits/claims.md"]}}
JSON
git add -A
git commit -q -m "PR head carries its committed receipt"
head_sha="$(git rev-parse HEAD)"
git commit -q --allow-empty -m "squash merge"
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

# Case 2: committed receipt passes, store receipt absent. Gate E must refuse
# on the store token, which proves the store check is ANDed in.
rc=0
out2="$(QUORUM_STORE="$work/store" GH="$work/gh" bash scripts/dogfood/quorum.sh 2>&1)" || rc=$?
if [ "$rc" -eq 0 ]; then
  echo "FALSIFIER FAIL case 2: gate E passed with no store receipt"
  exit 1
fi
case "$out2" in
  *"passes, but its head-bound store receipt is missing"*) ;;
  *) echo "FALSIFIER FAIL case 2: refusal did not come from the store check: $out2"; exit 1 ;;
esac

echo "FALSIFIER PASS gate E refuses unset QUORUM_STORE and a missing store receipt"
