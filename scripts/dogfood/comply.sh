#!/usr/bin/env bash
# Dogfood gate B — pmat comply, plus the two things comply cannot see here.
#
# Exit code is the gate. The only line a caller must read is the final
# `GATE B PASS|FAIL <detail>`.
#
# .pmat.yaml disables five checks, each because something else measures the
# same subject better. This script RUNS those replacements, so the disables are
# a change of instrument and not a reduction in what is asserted:
#
#   cb-1700, cb-1701  pmat reads `branches/main/protection`, which 404s because
#                     paiml/forjar is protected by a RULESET. Arm 2 reads the
#                     ruleset.
#   cb-400            one bashrs ERROR (scripts/ledger-replay.sh:42, SEC011 — a
#                     trap that rm -rf's an unguarded variable) fails the whole
#                     check, and that file is outside PMAT-163's scope. Arm 3
#                     requires 0 bashrs errors in the scripts this ticket ships;
#                     Arm 4 ratchets the repo-wide count so one cannot become
#                     two.
#   cb-2100           the required check `gate` runs no CB rule; the fix is a
#                     .github/workflows edit this ticket may not make.
#   cb-200            owned by the dated ratchet `make cb200-ratchet`.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../.."

REPO="paiml/forjar"

# The number of bashrs ERROR-severity findings the repository is known to carry.
# MAY ONLY SHRINK. Recorded 1: scripts/ledger-replay.sh SEC011. Raising it is
# not a permitted edit; fixing the finding and lowering it is.
BASHRS_ERROR_CEILING=1

# The pre-existing shell files, named so that a file added later is scored
# rather than inherited into the ceiling.
LEGACY_SHELL=(
  "install.sh"
  "examples/files/app-entrypoint.sh"
  "scripts/cb200-ratchet.sh"
  "scripts/install-hooks.sh"
  "scripts/ledger-replay.sh"
  "scripts/quorum-gate.sh"
  "scripts/release-object-audit.sh"
  "scripts/update_bench_table.sh"
)

fail() {
  echo "GATE B FAIL $1"
  exit 1
}

# ERROR-severity bashrs findings in one file, left in BASHRS_ERR_COUNT.
#
# The exit code cannot be the predicate: `bashrs lint` exits 1 on warnings and 2
# both for "this file has errors" and for "this file does not exist", so a typo
# in a path would read as a finding and a missing file as a clean one. The
# `Summary:` line proves the linter actually ran; anything else is UNMEASURED
# and fails, because a file nobody linted is not a file that passed.
BASHRS_ERR_COUNT=0
bashrs_count() {
  local out rc=0 n=0 line
  out="$(bashrs lint "$1" 2>&1)" || rc=$?
  case "$out" in
    *"Summary:"*) : ;;
    *) fail "bashrs did not lint $1 (exit ${rc}) — unmeasured is not clean" ;;
  esac
  while IFS= read -r line; do
    case "$line" in
      *"[error]"*) n=$((n + 1)) ;;
      *) : ;;
    esac
  done <<<"$out"
  BASHRS_ERR_COUNT="$n"
}

# --------------------------------------------------------------- Arm 1: comply
rc=0
out="$(pmat comply check --failures-only 2>&1)" || rc=$?
if [ "$rc" -ne 0 ]; then
  echo "$out"
  fail "pmat comply check exited ${rc} against the committed .pmat.yaml"
fi

# ------------------------------------------------ Arm 2: the protection ruleset
# Replaces cb-1700 and cb-1701. An unreadable ruleset is UNMEASURED, and
# unmeasured is a failure here — never a skip.
rc=0
rulesets="$(gh api "repos/${REPO}/rulesets" 2>&1)" || rc=$?
if [ "$rc" -ne 0 ]; then
  fail "branch protection UNMEASURED: gh api repos/${REPO}/rulesets exited ${rc} (${rulesets}); authenticate with a repo-scoped token"
fi
ruleset_id="$(printf '%s' "$rulesets" | jq -r '[.[] | select(.target=="branch" and .enforcement=="active")][0].id // empty')"
if [ -z "$ruleset_id" ]; then
  fail "no ACTIVE branch ruleset on ${REPO}: main is not protected"
fi
rc=0
ruleset="$(gh api "repos/${REPO}/rulesets/${ruleset_id}" 2>&1)" || rc=$?
if [ "$rc" -ne 0 ]; then
  fail "branch protection UNMEASURED: ruleset ${ruleset_id} unreadable (${ruleset})"
fi
contexts="$(printf '%s' "$ruleset" | jq -r '[.rules[]? | select(.type=="required_status_checks") | .parameters.required_status_checks[]?.context] | join(",")')"
if [ -z "$contexts" ]; then
  fail "ruleset ${ruleset_id} requires no status check: a merge into main is gated by nothing"
fi
# cb-1701's actual question: is cargo-deny inside a required check? Read the
# workflows (never edit them) for the job that implements a required context.
if ! grep -rqE 'cargo[- ]deny' .github/workflows; then
  fail "no cargo-deny in .github/workflows, so the required check(s) [${contexts}] cannot be running it"
fi

# ------------------------------------------ Arm 3: the scripts THIS gate ships
shopt -s nullglob
gates=(scripts/dogfood/*.sh)
shopt -u nullglob
if [ "${#gates[@]}" -lt 1 ]; then
  fail "scripts/dogfood/ holds no gate scripts — every loop in this file is vacuous"
fi
dirty=""
for g in "${gates[@]}"; do
  bashrs_count "$g"
  if [ "$BASHRS_ERR_COUNT" -ne 0 ]; then
    dirty="${dirty} ${g}(${BASHRS_ERR_COUNT})"
  fi
done
if [ -n "$dirty" ]; then
  fail "bashrs reports errors in the gate scripts themselves:${dirty}"
fi

# ----------------------------------------------- Arm 4: the legacy error ratchet
errors=0
for f in "${LEGACY_SHELL[@]}"; do
  if [ ! -f "$f" ]; then
    continue
  fi
  bashrs_count "$f"
  errors=$((errors + BASHRS_ERR_COUNT))
done
if [ "$errors" -gt "$BASHRS_ERROR_CEILING" ]; then
  fail "bashrs errors in pre-existing shell REGRESSED: ${errors} > recorded ceiling ${BASHRS_ERROR_CEILING}"
fi

echo "GATE B PASS comply clean; ruleset ${ruleset_id} requires [${contexts}]; ${#gates[@]} gate script(s) at 0 bashrs errors; legacy bashrs errors ${errors} <= ${BASHRS_ERROR_CEILING}"

# mutation: set BASHRS_ERROR_CEILING=0 — Arm 4 then reports the known
# scripts/ledger-replay.sh SEC011 finding as a regression and the gate exits 1.
