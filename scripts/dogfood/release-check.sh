#!/usr/bin/env bash
# Dogfood — the release artifacts, checked against the registries that actually
# serve them.
#
# Exit code is the gate. The only line a caller must read is the final
# `GATE R PASS|FAIL <detail>`. R, not a letter from the A..H sequence: this is
# not one of the eight standing gates, it is the post-tag audit that says
# whether the thing the gates approved actually reached users.
#
# PENDING IS NOT PASS AND IS NOT FAIL
#
# This script runs at two different moments and must not lie at either. Before
# the tag exists there is nothing on GitHub, nothing on crates.io and nothing on
# docs.rs, and reporting that as FAIL trains the operator to ignore the gate;
# reporting it as PASS is worse. Those four arms report PENDING, are listed by
# name in the summary, and are re-checked by the same command after the tag.
#
# The two arms that are NOT pending-able are the ones whose obligation exists
# BEFORE the tag: a quorum receipt per merged PR, and the CRUX reconciliation for
# the version. Those are the operator's own work, and if they are missing the tag
# should not be cut.
#
# Nothing here is skipped for being offline. Once the tag exists, an unreachable
# registry is UNMEASURED and UNMEASURED is a failure — an unreachable crates.io
# and an unpublished crate look identical from a distance, and only one of them
# is fine.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../.."

CRATE="forjar"
REPO="paiml/forjar"

fail() {
  echo "GATE R FAIL $1"
  exit 1
}

# `grep` exits 1 on "no match" and pipefail turns that into a silent death
# mid-assignment. Where "none" is a legitimate answer, rc is captured so that
# "none" (1) and "grep could not run" (>=2) stay distinguishable.
count_lines() {
  local rc=0 n
  n="$(printf '%s\n' "$1" | grep -c .)" || rc=$?
  if [ "$rc" -gt 1 ]; then
    fail "grep exited ${rc} counting lines — UNMEASURED"
  fi
  printf '%s\n' "$n"
}

version="$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -1)"
if [ -z "$version" ]; then
  fail "cannot read version from Cargo.toml — there is no release to check"
fi
TAG="v${version}"
CRUX="docs/audits/crux-${version}.md"

pending=""
note_pending() {
  pending="${pending}${1}, "
}

# ------------------------------------------------ Arm 1: the tag, and where it points
tagged=0
if git rev-parse --verify --quiet "refs/tags/${TAG}" >/dev/null; then
  tagged=1
fi

if [ "$tagged" -eq 0 ]; then
  note_pending "tag ${TAG} not cut"
  note_pending "no GitHub release"
  note_pending "not on crates.io"
  note_pending "not on docs.rs"
else
  # A tag that is not an ancestor of main is a release cut from a branch that
  # was never merged: the published artifact then has no reviewed history.
  main_ref="origin/main"
  if ! git rev-parse --verify --quiet "$main_ref" >/dev/null; then
    main_ref="main"
  fi
  if ! git rev-parse --verify --quiet "$main_ref" >/dev/null; then
    fail "neither origin/main nor main exists here, so '${TAG} is on main' cannot be checked — UNMEASURED"
  fi
  if ! git merge-base --is-ancestor "$TAG" "$main_ref"; then
    fail "${TAG} is not an ancestor of ${main_ref}: this release was cut off main"
  fi

  # ------------------------------------------- Arm 2: the GitHub release object
  rc=0
  rel="$(gh release view "$TAG" --repo "$REPO" --json tagName,isPrerelease,isDraft 2>&1)" || rc=$?
  if [ "$rc" -ne 0 ]; then
    fail "${TAG} is tagged but has no readable GitHub release (gh exit ${rc}: ${rel}) — the tag alone ships nothing to anyone"
  fi
  prerelease="$(printf '%s' "$rel" | jq -r '.isPrerelease')"
  draft="$(printf '%s' "$rel" | jq -r '.isDraft')"
  if [ "$draft" = "true" ]; then
    fail "the GitHub release for ${TAG} is still a DRAFT: it is invisible to everyone but its author"
  fi

  # ------------------------------------------------- Arm 3: crates.io serves it
  rc=0
  search="$(cargo search "$CRATE" --limit 1 2>&1)" || rc=$?
  if [ "$rc" -ne 0 ]; then
    fail "cargo search ${CRATE} exited ${rc} (${search}) — the registry is UNMEASURED, which is not the same as unpublished"
  fi
  published="$(printf '%s' "$search" | awk -v c="$CRATE" '$1 == c { gsub(/"/, "", $3); print $3; exit }')"
  if [ -z "$published" ]; then
    fail "cargo search returned no version line for ${CRATE} — UNMEASURED"
  fi
  if [ "$published" != "$version" ]; then
    fail "crates.io serves ${CRATE} ${published}, this tree is ${version}: the tag exists but the crate was never published"
  fi

  # --------------------------------------------------- Arm 4: docs.rs built it
  rc=0
  status="$(curl -fsS "https://docs.rs/crate/${CRATE}/${version}/status.json" 2>&1)" || rc=$?
  if [ "$rc" -ne 0 ]; then
    fail "docs.rs status.json for ${CRATE} ${version} unreadable (curl exit ${rc}: ${status}) — UNMEASURED"
  fi
  doc_ok="$(printf '%s' "$status" | jq -r '.doc_status')"
  if [ "$doc_ok" != "true" ]; then
    fail "docs.rs reports doc_status=${doc_ok} for ${CRATE} ${version}: the published crate has no rendered documentation"
  fi
fi

# ----------------------------- Arm 5: a quorum receipt per PR merged since the last tag
#
# scripts/quorum-gate.sh already refuses a push whose claims have not survived
# refutation. This is the release-time completeness check on the same
# obligation: every PR that got INTO this release must have left a receipt.
prev_rc=0
prev_tag="$(git tag --list 'v*' --sort=-v:refname --merged HEAD | grep -vxF -- "$TAG" | head -1)" || prev_rc=$?
if [ "$prev_rc" -gt 1 ]; then
  fail "grep exited ${prev_rc} selecting the previous tag — the PR window is UNMEASURED"
fi
if [ -z "$prev_tag" ]; then
  fail "no tag before ${TAG} is reachable from HEAD, so the set of PRs in this release cannot be bounded — UNMEASURED, and an unbounded window would make this arm vacuous"
fi

prs="$(git log --merges --format='%s' "${prev_tag}..HEAD" | sed -n 's/^Merge pull request #\([0-9]*\).*/\1/p' | sort -un)"
n_prs="$(count_lines "$prs")"
missing_q=""
for pr in $prs; do
  if [ ! -f "docs/audits/quorum-${pr}.md" ]; then
    missing_q="${missing_q}  docs/audits/quorum-${pr}.md"$'\n'
  fi
done
if [ -n "$missing_q" ]; then
  printf '%s' "$missing_q"
  fail "$(count_lines "$missing_q") of ${n_prs} PR(s) merged since ${prev_tag} have no quorum receipt in docs/audits/ — those changes are in the release with no record that their claims were refuted"
fi

# ------------------------------------------------- Arm 6: the CRUX reconciliation
if [ ! -f "$CRUX" ]; then
  fail "no ${CRUX}: this version's behaviour changes have no recorded comparison against other systems (run scripts/dogfood/crux-reconcile.sh for the rows it wants)"
fi

if [ -n "$pending" ]; then
  echo "GATE R PASS pre-tag: ${n_prs} PR(s) since ${prev_tag} all have quorum receipts, ${CRUX} present; PENDING until the tag is cut: ${pending%, }"
else
  echo "GATE R PASS ${TAG} is on main; GitHub release published (prerelease=${prerelease}); crates.io serves ${CRATE} ${published}; docs.rs built the docs; ${n_prs} PR(s) since ${prev_tag} all have quorum receipts; ${CRUX} present"
fi

# mutation: delete any one `docs/audits/quorum-<pr>.md` for a PR merged since the
# previous tag — Arm 5 then names that exact path and the gate exits 1, which
# shows the receipts are looked for per PR and not merely counted.
