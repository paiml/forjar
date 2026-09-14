#!/usr/bin/env bash
# Dogfood gate H — every behaviour this release claims has been reconciled
# against the field.
#
# Exit code is the gate. The only line a caller must read is the final
# `GATE H PASS|FAIL <detail>`.
#
# WHAT THIS GATE IS FOR
#
# `docs/specifications/provable-iac.md` requires "quorum-validated design (>=3
# world-class systems)" per phase, and `docs/specifications/quorum-spec.md`
# restates it: validate against >=3 world-class systems, NAMED per feature. The
# 1.24.0 CRUX audit is what that looks like when it is done — 118 findings
# against Ansible/Salt/Chef/pyinfra/cdist, Terraform/Pulumi/Crossplane,
# Nix/Guix/bootc, Sigstore/in-toto/SOPS/Vault — and its central finding was that
# "forjar's declared capability ladder has outrun its executed behaviour". A
# CHANGELOG bullet is exactly such a declaration.
#
# So: every behaviour bullet under [Unreleased] must appear as a row in
# `docs/audits/crux-<version>.md`, and that row must NAME at least three of the
# systems the behaviour was checked against. An unreconciled bullet is a claim
# about how forjar behaves that no competitor comparison has been held against,
# which is precisely the class of claim the audit found to be wrong.
#
# THE MISSING FILE IS THE POINT. Before a release exists there is no
# `crux-<version>.md`, and this gate is RED until someone writes one. That is
# why gate H is in `make dogfood-release` and not in `make dogfood`: it blocks a
# publish, not a commit.
#
# WHAT THIS GATE DOES NOT PROVE. It cannot tell a real comparison from three
# system names typed into a cell. It asserts the reconciliation was WRITTEN and
# is COMPLETE over the bullets — the cheapest failure (a behaviour shipped with
# no comparison at all) is the one it catches.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../.."

CHANGELOG="CHANGELOG.md"

# The roster the CRUX audits actually survey, from
# docs/specifications/forjar-architecture-crux-audit.md's competitor list. No
# name here is a substring of another, so a row that says "Nix" once is counted
# once.
SYSTEMS=(
  "Ansible" "Salt" "Chef" "Puppet" "CFEngine" "pyinfra" "cdist"
  "Terraform" "OpenTofu" "Pulumi" "CDKTF" "Crossplane"
  "Nix" "Guix" "bootc" "OSTree" "Ignition" "Kairos"
  "Sigstore" "in-toto" "SLSA" "SOPS" "Vault"
  "Bazel" "Kubernetes" "systemd" "Podman" "Docker"
)

# provable-iac.md's floor, verbatim.
MIN_SYSTEMS=3

fail() {
  echo "GATE H FAIL $1"
  exit 1
}

# `grep` exits 1 on "no match", and `set -o pipefail` turns that into a silent
# script death in the middle of an assignment — a gate that prints nothing and
# exits non-zero is indistinguishable from one that never ran. This gate asks
# grep questions whose answer is legitimately "none" (a bullet with no row), so
# every such call captures rc and keeps "none" (1) apart from "grep could not
# run" (>=2, which is UNMEASURED and fatal).

# The first markdown table row of $CRUX containing $1, or nothing.
crux_row() {
  local rc=0 hits
  hits="$(grep -i -F -- "$1" "$CRUX")" || rc=$?
  if [ "$rc" -gt 1 ]; then
    fail "grep exited ${rc} reading ${CRUX} — the reconciliation is UNMEASURED"
  fi
  # PMAT-240: one awk over a here-string. `printf | sed | head -1` is three
  # processes and two pipes, the last of which leaves after one line — and a
  # rule that only inspected the stage after the FIRST pipe walked past it,
  # which is how this one survived the census that named twelve others.
  awk '/^\|/ { print; exit }' <<<"$hits"
}

# The number of non-empty lines in $1.
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
  fail "cannot read version from Cargo.toml, so the crux document has no name to look for"
fi
CRUX="docs/audits/crux-${version}.md"

# PENDING when Cargo.toml's version is the SAME as the most recently cut
# release's: nobody has bumped it, so no release is currently being prepared
# and demanding a reconciliation for a version nobody is cutting would be
# ceremony over a version that already shipped. `scripts/dogfood/release-check.sh`
# Arm 6 applies the identical rule and calls this script once a version differs.
# PMAT-240: capture, then take the first line with a parameter expansion.
# `git tag … | head -1` makes git take SIGPIPE when head leaves, and under
# pipefail that is a 141 the caller reads as "the tag list is UNMEASURED".
_tags="$(git tag --list 'v*' --sort=-v:refname --merged HEAD)"
latest_tag="${_tags%%$'\n'*}"
latest_tag_version="${latest_tag#v}"
if [ -n "$latest_tag" ] && [ "$version" = "$latest_tag_version" ]; then
  echo "GATE H PENDING Cargo.toml is still at ${latest_tag}'s version (${version}); no release is being cut, so there is nothing to reconcile yet"
  exit 0
fi

if [ ! -f "$CHANGELOG" ]; then
  fail "no ${CHANGELOG}: there is nothing to reconcile and no way to notice that"
fi

# ------------------------------------- which section: [Unreleased], or [<version>] at the cut
#
# The release-cut PR renames `## [Unreleased]` to `## [<version>]` before the
# tag exists, so between that PR and the tag the paragraphs live under the
# version heading. Read [Unreleased] when it opens at least one bold paragraph;
# otherwise read [<version>]. After the tag the PENDING rule above applies.
section="Unreleased"
if ! awk '/^## \[Unreleased\]/{inu=1;blank=1;next} /^## \[/{inu=0} inu&&blank&&/^\*\*/{found=1} inu{blank=($0 ~ /^[[:space:]]*$/)} END{exit !found}' "$CHANGELOG"; then
  section="$version"
fi

# ------------------------------------- the behaviour bullets under [Unreleased] or [<version>]
#
# A behaviour bullet is a bold span that OPENS a paragraph: `**` at column 0 on
# a line preceded by a blank one. The restriction matters — the same file uses
# bold mid-paragraph for emphasis ("**parse -> validate -> filter ...**"), and
# counting those would make this gate demand a crux row for a sentence fragment.
bullets="$(awk -v ver="$section" '
  $0 == "## [" ver "]" || $0 ~ ("^## \\[" ver "\\]") { inu = 1; blank = 1; next }
  /^## \[/            { inu = 0 }
  inu {
    if (!collecting && blank && $0 ~ /^\*\*/) { collecting = 1; buf = "" }
    if (collecting) {
      buf = buf " " $0
      if (gsub(/\*\*/, "**", buf) >= 2) {
        s = buf
        i = index(s, "**"); s = substr(s, i + 2)
        j = index(s, "**"); t = substr(s, 1, j - 1)
        gsub(/`/, "", t)
        gsub(/[[:space:]]+/, " ", t)
        sub(/^ /, "", t)
        print t
        collecting = 0
      }
    }
    blank = ($0 ~ /^[[:space:]]*$/)
  }
' "$CHANGELOG")"

n_bullets="$(count_lines "$bullets")"
if [ "$n_bullets" -eq 0 ]; then
  fail "no behaviour bullet under [${section}] in ${CHANGELOG}: either this release changes no behaviour — in which case there is nothing to publish — or the bullet parser stopped matching and every check below is vacuous"
fi

if [ ! -f "$CRUX" ]; then
  echo "The ${n_bullets} behaviour bullet(s) awaiting a row, and the key each row must contain:"
  while IFS= read -r b; do
    [ -n "$b" ] || continue
    echo "  | $(printf '%s' "$b" | cut -d' ' -f1-6) | <>=${MIN_SYSTEMS} systems> | <what the comparison found> |"
  done <<<"$bullets"
  fail "no ${CRUX}: ${n_bullets} behaviour(s) are being released against no recorded comparison with any other system (provable-iac.md: quorum-validated design, >=3 world-class systems)"
fi

# ------------------------------------------------- one row per bullet, >=3 systems
missing=""
thin=""
matched=0
while IFS= read -r b; do
  [ -n "$b" ] || continue
  key="$(printf '%s' "$b" | cut -d' ' -f1-6)"
  row="$(crux_row "$key")"
  if [ -z "$row" ]; then
    missing="${missing}  ${key}"$'\n'
    continue
  fi
  matched=$((matched + 1))
  n=0
  named=""
  for s in "${SYSTEMS[@]}"; do
    # PMAT-240: a here-string, not a pipe. This one runs once per surveyed
    # system per row — 28 x 12 on the 1.29.0 audit — so the odds of one of
    # them losing the race are not small.
    if grep -qiF -- "$s" <<<"$row"; then
      n=$((n + 1))
      named="${named}${s} "
    fi
  done
  if [ "$n" -lt "$MIN_SYSTEMS" ]; then
    thin="${thin}  ${key} names ${n} system(s) [${named}]"$'\n'
  fi
done <<<"$bullets"

if [ -n "$missing" ]; then
  printf '%s' "$missing"
  fail "$(count_lines "$missing") behaviour bullet(s) under [${section}] have no row in ${CRUX} — the rows above are the keys each must contain"
fi
if [ -n "$thin" ]; then
  printf '%s' "$thin"
  fail "$(count_lines "$thin") crux row(s) name fewer than ${MIN_SYSTEMS} world-class systems — provable-iac.md's floor is ${MIN_SYSTEMS}, named"
fi

echo "GATE H PASS ${matched} of ${n_bullets} behaviour bullet(s) under [${section}] reconciled in ${CRUX}, each naming >= ${MIN_SYSTEMS} of the ${#SYSTEMS[@]} surveyed systems"

# mutation: set MIN_SYSTEMS=4 — every crux row written to provable-iac.md's
# floor of 3 is then reported as thin and the gate exits 1, which shows the
# system count is read out of the rows and not assumed.
