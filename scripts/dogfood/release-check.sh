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
# PENDING is decided by ORIGIN, never by the local tag list: see Arm 1. A
# version the remote already carries has happened, and nothing about it is
# reported as not-yet-happened.
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

# The GitHub client. Named in one place because it is a REQUIREMENT of this
# gate, not a convenience: the set of PRs in a release is a fact only GitHub
# holds (see Arm 5), so a gh that cannot answer is an unmeasured window and an
# unmeasured window is a FAIL. Overridable only so that the falsification tests
# in tests/falsification_dogfood_release_check_pr_window.rs can hand the script
# a stub instead of the network.
GH="${GH:-gh}"

# The page the PR enumeration asks for. If GitHub fills it exactly, the window
# may have been truncated and the arm no longer knows the set it is checking.
PR_PAGE_LIMIT=200

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

# PMAT-240: one awk rather than `sed Cargo.toml | head -1`, which leaves sed
# writing into a pipe head has already closed.
#
# NOT byte-for-byte the same reader, and a review lane measured the
# difference: the old sed anchored the closing quote at end of line, so
# `version = "1.29.0" # a comment` — valid TOML — yielded NOTHING and this
# script failed with "cannot read version". The awk stops at the closing
# quote and reads 1.29.0. That is a behaviour CHANGE and it is the right
# direction, but it is a change rather than a preservation.
version="$(awk '/^version = "/ { v = $0; sub(/^version = "/, "", v); sub(/".*/, "", v); print v; exit }' Cargo.toml)"
if [ -z "$version" ]; then
  fail "cannot read version from Cargo.toml — there is no release to check"
fi
TAG="v${version}"
CRUX="docs/audits/crux-${version}.md"

# TWO KINDS OF PENDING, AND THEY ARE NOT THE SAME SENTENCE (PMAT-234).
#
# There was one flat `pending` string, and the verdict read
# `PASS pre-tag: … PENDING until the tag is cut: …` whenever ANY note was in
# it. Arm 6 adds a note after EVERY SUCCESSFUL RELEASE — Cargo.toml is back at
# the tag's version, so no cut is in flight and no crux document is owed — so
# the line said `pre-tag` and `until the tag is cut` about a release that was
# tagged, published, on crates.io and rendered on docs.rs. A verdict that says
# the opposite of the truth at the one moment a reader most wants to believe it
# is worse than no verdict.
#
# `note_pretag` is for the arms whose obligation does not exist YET because the
# tag does not: the tag, the GitHub release, crates.io, docs.rs. `note_pending`
# is for everything else — a note about the tree, true whether or not a tag
# exists — and it never licenses the words `pre-tag`.
pretag=""
note_pretag() {
  pretag="${pretag}${1}, "
}
crux_state=""
pending=""
note_pending() {
  pending="${pending}${1}, "
}

# ------------------------------------- Arm 1: the tag, as the REMOTE serves it
#
# LOCAL ABSENCE IS NOT ABSENCE. `git tag` lists what this checkout happens to
# have fetched, and deciding PENDING from that list made the one state that must
# never be excused — a version tagged and pushed with no release behind it —
# read as "not cut yet", which is the reading that stops anybody looking. The
# remote is the authority for whether this version exists: PENDING is available
# only while origin has no such tag. An unreachable origin is UNMEASURED and
# UNMEASURED is a failure, because from here an unreachable remote and an uncut
# tag are the same silence.
local_tagged=0
if git rev-parse --verify --quiet "refs/tags/${TAG}" >/dev/null; then
  local_tagged=1
fi

rc=0
remote_ls="$(git ls-remote --tags origin "refs/tags/${TAG}" 2>&1)" || rc=$?
if [ "$rc" -ne 0 ]; then
  fail "git ls-remote --tags origin ${TAG} exited ${rc} (${remote_ls}) — whether this version was ever released is UNMEASURED, and unmeasured is not the same as unreleased"
fi
remote_tagged=0
if [ -n "$remote_ls" ]; then
  remote_tagged=1
fi

if [ "$remote_tagged" -eq 0 ]; then
  if [ "$local_tagged" -eq 1 ]; then
    note_pretag "tag ${TAG} exists only in this checkout and was never pushed"
  else
    note_pretag "tag ${TAG} not cut"
  fi
  note_pretag "no GitHub release"
  note_pretag "not on crates.io"
  note_pretag "not on docs.rs"
else
  if [ "$local_tagged" -eq 0 ]; then
    fail "origin serves ${TAG} ($(printf '%s\n' "$remote_ls" | awk 'NR == 1 {print $1}')) and this checkout does not have it, so nothing about the released commit can be examined here — UNMEASURED (run: git fetch --tags origin), and a version the remote already carries is never reported as not-yet-cut"
  fi

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
  rel="$("$GH" release view "$TAG" --repo "$REPO" --json tagName,isPrerelease,isDraft 2>&1)" || rc=$?
  if [ "$rc" -ne 0 ]; then
    fail "${TAG} is tagged but has no readable GitHub release (${GH} exit ${rc}: ${rel}) — the tag alone ships nothing to anyone"
  fi
  prerelease="$(printf '%s' "$rel" | jq -r '.isPrerelease')"
  draft="$(printf '%s' "$rel" | jq -r '.isDraft')"
  if [ "$draft" = "true" ]; then
    fail "the GitHub release for ${TAG} is still a DRAFT: it is invisible to everyone but its author"
  fi

  # --------------------------- Arm 2b: /releases/latest resolves to this release
  #
  # MEASURED 2026-09-12, before it was corrected by hand:
  # repos/paiml/forjar/releases/latest resolved to v1.25.2 while v1.26.0,
  # v1.27.0 and v1.28.0 were all `prerelease=false draft=false` and days newer.
  # Anyone following that URL got a four-version-old binary -- every
  # `curl -L .../releases/latest/download/...`, every badge, every script.
  #
  # WHY. A release is BORN a prerelease here (release.yml, PMAT-166) and GitHub
  # never makes a prerelease latest. Clearing the flag afterwards does NOT
  # recompute the pointer: `make_latest` is decided when the flag is written, so
  # `gh release edit <tag> --prerelease=false` leaves latest exactly where it
  # was. The promotion has to say `--latest`, and the step that was supposed to
  # perform it was written down nowhere and checked by nothing.
  #
  # A PRERELEASE IS A DELIBERATE STATE and this arm does not force it. forjar
  # publishes a rolling `nightly` prerelease, and a release still waiting on
  # `make dogfood-published VERSION=` is legitimately one. That case is
  # REPORTED, with the command that ends it. What is refused is a FULL release
  # that is not the latest one -- the shape every stale pointer above had.
  rc=0
  latest_rel="$("$GH" api "repos/${REPO}/releases/latest" --jq '.tag_name' 2>&1)" || rc=$?
  unread=""
  if [ "$rc" -ne 0 ]; then
    unread="${GH} api repos/${REPO}/releases/latest exited ${rc} (${latest_rel})"
  elif [ -z "$latest_rel" ]; then
    unread="repos/${REPO}/releases/latest answered with exit 0 and no tag_name"
  fi
  # A pointer that did not answer is judged against what the release IS.
  #
  # /releases/latest serves the newest release that is neither a prerelease nor
  # a draft, and 404s when there is none -- MEASURED 2026-09-12 on
  # electron/electron, whose newest release v45.0.0-alpha.6 is a prerelease and
  # whose /releases/latest answers v44.3.0, two releases older, and on a
  # repository with no releases at all, where gh exits 1 with HTTP 404. So for a
  # repository whose releases are ALL prereleases the 404 is the ORDINARY state,
  # and refusing it would contradict the paragraph above -- the arm would force
  # to latest exactly the deliberate prerelease it promises not to. Reported.
  #
  # For a FULL release the same silence is UNMEASURED and refused: a full
  # release the pointer cannot be read for is the state the four stale releases
  # above were in, and assuming it points here is the silence they lived in.
  if [ -n "$unread" ] && [ "$prerelease" = "true" ]; then
    note_pending "the release for ${TAG} is a PRERELEASE and where repos/${REPO}/releases/latest points could not be read (${unread}) — GitHub never makes a prerelease latest, so nothing following that URL gets ${TAG} either way (promote it with: gh release edit ${TAG} --repo ${REPO} --prerelease=false --latest)"
  elif [ -n "$unread" ]; then
    fail "${unread} — where that URL points is UNMEASURED, and unmeasured is not the same as pointing here"
  elif [ "$prerelease" = "true" ]; then
    note_pending "the release for ${TAG} is a PRERELEASE, so repos/${REPO}/releases/latest resolves to ${latest_rel} and nothing following that URL will get ${TAG} (promote it with: gh release edit ${TAG} --repo ${REPO} --prerelease=false --latest)"
  elif [ "$latest_rel" != "$TAG" ]; then
    fail "the GitHub release for ${TAG} is a full release and repos/${REPO}/releases/latest still resolves to ${latest_rel}: every installer, badge and script that follows that URL gets ${latest_rel}, not ${TAG}. GitHub fixes 'latest' when the prerelease flag is WRITTEN and never recomputes it, so clearing the flag alone leaves the pointer where it was — run: gh release edit ${TAG} --repo ${REPO} --latest"
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
# PMAT-240: one awk over a captured list. The old shape was `git tag … |
# grep -vxF … | head -1`: head leaves after one line, grep takes SIGPIPE, git
# takes SIGPIPE, and under pipefail the whole gate reported the PR window
# UNMEASURED at random — which is exactly the symptom PMAT-239 chased in gate T.
_all_tags="$(git tag --list 'v*' --sort=-v:refname --merged HEAD)"
prev_tag="$(awk -v skip="$TAG" '$0 != skip { print; exit }' <<<"$_all_tags")" || prev_rc=$?
if [ "$prev_rc" -gt 1 ]; then
  fail "grep exited ${prev_rc} selecting the previous tag — the PR window is UNMEASURED"
fi
if [ -z "$prev_tag" ]; then
  fail "no tag before ${TAG} is reachable from HEAD, so the set of PRs in this release cannot be bounded — UNMEASURED, and an unbounded window would make this arm vacuous"
fi

# THE PR SET COMES FROM GITHUB, NOT FROM `git log --merges`.
#
# This repository squash-merges — #472 was a squash, and so is most of the
# recent history — and a squash merge leaves ONE ORDINARY COMMIT with no second
# parent. `git log --merges` therefore enumerated nothing on a squash-merged
# release, the receipt loop below it ran zero times, and this arm printed
# "0 PR(s) ... all have quorum receipts" over a release full of unrefuted work.
# A loop over an empty set is the vacuity the whole dogfood contract exists to
# refuse, and it is invisible in CI output.
#
# So the window is asked of GitHub, which knows what was merged however it was
# merged. gh is a RELEASE-TIME REQUIREMENT: a gh that cannot answer leaves the
# window UNMEASURED, and UNMEASURED is a FAIL naming the tool — never a PASS
# over "no PRs found".
prev_tag_date="$(TZ=UTC git log -1 --format=%cd --date=format-local:%Y-%m-%dT%H:%M:%SZ "$prev_tag")"
if [ -z "$prev_tag_date" ]; then
  fail "cannot read the commit date of ${prev_tag}, so the PR window has no lower bound — UNMEASURED"
fi

rc=0
pr_json="$("$GH" pr list --repo "$REPO" --state merged --base main --search "merged:>=${prev_tag_date}" --limit "$PR_PAGE_LIMIT" --json number,mergedAt,mergeCommit,headRefName 2>&1)" || rc=$?
if [ "$rc" -ne 0 ]; then
  fail "${GH} pr list exited ${rc} (${pr_json}) — the PRs in this release cannot be enumerated without GitHub, and an unenumerable window is UNMEASURED, which is not the same as a release with no PRs"
fi
jrc=0
pr_rows="$(printf '%s' "$pr_json" | jq -r '.[] | "\(.number) \(.mergeCommit.oid // "-") \(.headRefName // "-")"')" || jrc=$?
if [ "$jrc" -ne 0 ]; then
  fail "cannot parse what ${GH} pr list returned as JSON (jq exit ${jrc}): ${pr_json} — UNMEASURED"
fi
n_returned="$(count_lines "$pr_rows")"
if [ "$n_returned" -ge "$PR_PAGE_LIMIT" ]; then
  fail "${GH} pr list returned ${n_returned} PR(s), which fills the --limit of ${PR_PAGE_LIMIT}: the window may be truncated and this arm would then check a subset while reporting the whole — UNMEASURED"
fi

# GitHub's window is a time range, so it also contains PRs merged after this
# tree's HEAD. Membership of THIS release is decided by ancestry, on the merge
# commit GitHub names, in this checkout.
prs=""
pr_meta=""
n_after=0
while read -r pr oid headref; do
  if [ -z "$pr" ]; then
    continue
  fi
  if [ "$oid" = "-" ]; then
    fail "GitHub reports PR #${pr} merged since ${prev_tag} with no merge commit, so whether it is in this release cannot be decided — UNMEASURED"
  fi
  if [ -z "$headref" ] || [ "$headref" = "-" ]; then
    fail "GitHub reports PR #${pr} merged since ${prev_tag} with no headRefName, so its receipt slug (.quorum/<branch>.json) cannot be derived — UNMEASURED"
  fi
  arc=0
  git merge-base --is-ancestor "$oid" HEAD >/dev/null 2>&1 || arc=$?
  case "$arc" in
    0) prs="${prs}${pr}"$'\n'; pr_meta="${pr_meta}${pr} ${oid} ${headref}"$'\n' ;;
    1) n_after=$((n_after + 1)) ;;
    *) fail "git merge-base --is-ancestor ${oid} HEAD exited ${arc} for PR #${pr}: the commit GitHub names is not in this checkout (run: git fetch origin), so membership of this release is UNMEASURED" ;;
  esac
done <<EOF
$pr_rows
EOF

n_prs="$(count_lines "$prs")"

# An empty PR set is only honest if nothing landed. Commits since the previous
# tag with no PR containing any of them means either the enumeration is broken —
# which is exactly how this arm used to pass — or work reached the release
# without review. Both block a tag.
crc=0
commits_since="$(git log --format=%H "${prev_tag}..HEAD" | grep -c .)" || crc=$?
if [ "$crc" -gt 1 ]; then
  fail "grep exited ${crc} counting the commits since ${prev_tag} — UNMEASURED"
fi
if [ "$n_prs" -eq 0 ] && [ "$commits_since" -gt 0 ]; then
  fail "${commits_since} commit(s) landed since ${prev_tag} and GitHub reports no merged PR containing any of them (${n_returned} PR(s) in the time window, ${n_after} of them merged after this HEAD): either this enumeration is broken or the release carries work that never went through a PR — and an empty set is how this arm used to report a squash-merged release as clean"
fi

# THE RECEIPT IS THE COMMITTED ONE, NOT A PLACEHOLDER NAME.
#
# `scripts/quorum-gate.sh` already enforces, on every push, that a branch's
# claims live in `.quorum/${branch//\//-}.json` before that branch can be
# pushed. That is the receipt this repository actually keeps, and it survives
# a squash merge because it was committed ON THE BRANCH — so it is either in
# the tree at the PR's merge commit, or (if the PR's author rebased onto a
# later main and the squash commit dropped the file from its own diff, which
# git allows) still present at HEAD. `docs/audits/quorum-<pr>.md` was a
# placeholder name that was never written by anything; checking for it is how
# this arm always reported every PR as missing its receipt, or — worse, before
# PMAT-178 — never checked at all.
#
# A receipt is not enough that it exists. This arm is the release's last
# chance to refuse a waived or thin one, and it applies the SAME predicate
# gate E (scripts/dogfood/quorum.sh) applies before the tag — lib/receipt.sh:
# any waiver or override key anywhere, fewer than 3 lanes, judges or refuters
# per claim, a round that refuted nothing, or no evidence file is not a quorum,
# whatever its number of lines.
# shellcheck source=scripts/dogfood/lib/receipt.sh
. "$(dirname "${BASH_SOURCE[0]}")/lib/receipt.sh"

receipt_status() {
  # $1 = raw JSON blob, passed as an argv. Prints one of ok|waived|thin, the
  # tokens the report below and its falsification tests read; the reason
  # itself comes from the shared predicate in lib/receipt.sh, the same one
  # gate E applies before the tag.
  local why
  why="$(dogfood_receipt_status "$1")"
  case "$why" in
    ok) printf 'ok\n' ;;
    *waiver*) printf 'waived\n' ;;
    *) printf 'thin\n' ;;
  esac
}

report=""
any_bad=0
while read -r pr oid headref; do
  [ -n "$pr" ] || continue
  slug="${headref//\//-}"
  path=".quorum/${slug}.json"
  blob=""
  if git cat-file -e "${oid}:${path}" 2>/dev/null; then
    blob="$(git cat-file -p "${oid}:${path}")"
  elif git cat-file -e "HEAD:${path}" 2>/dev/null; then
    blob="$(git cat-file -p "HEAD:${path}")"
  fi
  if [ -z "$blob" ]; then
    status="missing"
  else
    status="$(receipt_status "$blob")"
  fi
  report="${report}#${pr} ${slug} receipt=${status}"$'\n'
  if [ "$status" != "ok" ]; then
    any_bad=1
  fi
done <<EOF
$pr_meta
EOF

if [ -n "$report" ]; then
  printf '%s' "$report"
fi
if [ "$any_bad" -eq 1 ]; then
  fail "one or more PR(s) merged since ${prev_tag} have a missing, waived or thin (< 3 lanes, judges or refuters per claim, no refuted claim, or no evidence) quorum receipt at .quorum/<branch>.json — see the receipt=... rows above; those changes are in the release with no record that their claims survived refutation, and a waiver is never a pass"
fi

# ------------------------------------------------- Arm 6: the CRUX reconciliation
#
# PENDING when Cargo.toml's version is the SAME as the most recently cut
# release's: nobody has bumped it, so no release is currently being prepared
# and demanding a crux doc for a version nobody is cutting would be ceremony
# over a version that already shipped. The moment the version differs from
# that tag's, the cut for THIS version has happened (or is happening) and the
# obligation is live: the file must exist AND the reconciliation must actually
# PASS, not merely be present.
# PMAT-240: capture, then take the first line with a parameter expansion.
# `git tag … | head -1` makes git take SIGPIPE when head leaves, and under
# pipefail that is a 141 the caller reads as "the tag list is UNMEASURED".
_latest_tags="$(git tag --list 'v*' --sort=-v:refname --merged HEAD)"
latest_tag="${_latest_tags%%$'\n'*}"
latest_tag_version="${latest_tag#v}"
if [ -n "$latest_tag" ] && [ "$version" = "$latest_tag_version" ]; then
  # THE NOTE SAYS WHAT IS TRUE, NOT WHAT IS CONVENIENT (PMAT-234, found by two
  # review lanes). It used to open with "no ${CRUX}", asserting the document
  # was absent — and on this repository right now it is present and 13KB long.
  # A verdict line being fixed for saying false things must not keep one of its
  # own. What is true here is that no crux document is OWED, because no cut is
  # in flight; whether one happens to exist is a separate fact, and it is
  # measured rather than assumed.
  crux_state="absent"
  if [ -f "$CRUX" ]; then
    crux_state="present"
  fi
  note_pending "no ${CRUX} is owed (it is ${crux_state}): Cargo.toml is still at ${latest_tag}'s version (${version}), so no release is being cut and its reconciliation is not re-run here"
else
  if [ ! -f "$CRUX" ]; then
    fail "no ${CRUX}: this version's behaviour changes have no recorded comparison against other systems (run scripts/dogfood/crux-reconcile.sh for the rows it wants)"
  fi
  crc2=0
  crux_out="$(bash scripts/dogfood/crux-reconcile.sh 2>&1)" || crc2=$?
  if [ "$crc2" -ne 0 ]; then
    printf '%s\n' "$crux_out"
    fail "scripts/dogfood/crux-reconcile.sh exited ${crc2}: ${CRUX} exists but does not reconcile every [Unreleased] behaviour bullet"
  fi
fi

# The words `pre-tag` and `until the tag is cut` are spoken ONLY when the tag
# is genuinely not there. Any other note is reported as what it is, on either
# side of that line, so a reader is never told a published release is pending.
if [ -n "$pending" ]; then
  also="; also pending: ${pending%, }"
  post="; also pending: ${pending%, }"
else
  also=""
  post="; ${CRUX} present"
fi
if [ -n "$pretag" ]; then
  echo "GATE R PASS pre-tag: ${n_prs} PR(s) since ${prev_tag} (GitHub reports ${n_returned} merged in that window, ${n_after} of them after this HEAD) all carry receipt=ok; PENDING until the tag is cut: ${pretag%, }${also}"
else
  echo "GATE R PASS ${TAG} is on main and on origin; GitHub release published (prerelease=${prerelease}); crates.io serves ${CRATE} ${published}; docs.rs built the docs; ${n_prs} PR(s) since ${prev_tag} (of ${n_returned} GitHub reports merged in that window) all carry receipt=ok${post}"
fi

# mutation: change Arm 2b's `elif [ "$latest_rel" != "$TAG" ]` to
# `elif false`, and a full release that /releases/latest does not point at
# passes -- exactly the four-release staleness this arm was written for;
# `a_full_release_that_is_not_latest_is_named_and_red` goes RED.
# mutation: change `if [ "$status" != "ok" ]` below the report loop to
# `if [ "$status" = "missing" ]` — a waived or thin receipt then sets nothing
# and this arm passes a release over a PR whose quorum was waived or never
# reached the floor; the falsification tests a_waived_receipt_is_red,
# a_nested_override_is_red and a_thin_receipt_is_red go RED.