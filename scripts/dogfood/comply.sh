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
#   cb-400            (Arm 4/4b) one bashrs ERROR (scripts/ledger-replay.sh:42, SEC011 — a
#                     trap that rm -rf's an unguarded variable) fails the whole
#                     check, and that file is outside PMAT-163's scope. Arm 3
#                     requires 0 bashrs errors in the scripts this ticket ships;
#                     Arm 4 ratchets the repo-wide count so one cannot become
#                     two.
#   cb-2100           (Arm 6) the required check `gate` runs no CB rule (PMAT-202); what it
#                     does gate — a job running scripts/dogfood/*.sh — is measured here. Was:
#                     .github/workflows edit this ticket may not make.
#   cb-200            (Arm 5) owned by the dated ratchet scripts/cb200-ratchet.sh, which runs here.
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
gates=(scripts/dogfood/*.sh scripts/dogfood/lib/*.sh)
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

# ---------------------------------------------------------------- Arm 4b
# Every OTHER tracked shell script is scored at 0 errors. The legacy list above
# is a ceiling for files that predate this gate; a file that is neither a gate
# nor on that list — whenever it was added — is measured on its own and never
# inherited into the ceiling. Without this arm a script added anywhere outside
# scripts/dogfood/ would go unlinted (found by the PMAT-163 merge review).
mapfile -d '' tracked_shell < <(git ls-files -z -- '*.sh')
others_dirty=""
n_others=0
for f in "${tracked_shell[@]}"; do
  case " ${gates[*]} ${LEGACY_SHELL[*]} " in
    *" ${f} "*) continue ;;
  esac
  n_others=$((n_others + 1))
  bashrs_count "$f"
  if [ "$BASHRS_ERR_COUNT" -ne 0 ]; then
    others_dirty="${others_dirty} ${f}(${BASHRS_ERR_COUNT})"
  fi
done
if [ -n "$others_dirty" ]; then
  fail "bashrs errors in tracked shell scripts outside the recorded legacy set — a new script is scored, not inherited:${others_dirty}"
fi

# ------------------------------------------------ Arm 5: the CB-200 ratchet
# CB-200 (the TDG grade gate) is disabled in .pmat.yaml because the dated
# ratchet `scripts/cb200-ratchet.sh` owns that number with a recorded baseline
# that may only shrink. A disable with its replacement running somewhere else
# is a change of instrument only if the replacement runs HERE, in the same
# gate — so it does.
ratchet_rc=0
ratchet_out="$(bash scripts/cb200-ratchet.sh 2>&1)" || ratchet_rc=$?
if [ "$ratchet_rc" -ne 0 ]; then
  printf '%s\n' "$ratchet_out" | tail -5
  fail "the CB-200 ratchet (scripts/cb200-ratchet.sh) exited ${ratchet_rc}: the debt grew past its recorded baseline"
fi

# ----------------------------------------- Arm 6: the required check gates
# CB-2100 (gate-effect) asks whether the required status check enforces any
# CB rule and is disabled in .pmat.yaml because no required check runs
# `pmat comply` (PMAT-202). What the required check DOES enforce is measured
# here instead: every required context that is a job in ci.yml must, through
# its `needs`, reach a job whose steps run at least one scripts/dogfood/*.sh —
# the mechanical gates this file ships. A required check that gated nothing
# of them would be the finding CB-2100 makes, and this arm makes it too.
gate_effect_rc=0
gate_effect="$(python3 - "$contexts" <<'PY_EOF'
import sys
try:
    import yaml
except ImportError:
    print("UNMEASURED: python3 has no yaml module here, so ci.yml cannot be read")
    sys.exit(3)
wanted = [c for c in sys.argv[1].replace(",", " ").split() if c]
try:
    w = yaml.safe_load(open(".github/workflows/ci.yml"))
except Exception as e:
    print(f"UNMEASURED: cannot read .github/workflows/ci.yml: {e}")
    sys.exit(3)
jobs = (w or {}).get("jobs") or {}
def runs_gate(job):
    for s in (jobs.get(job) or {}).get("steps") or []:
        if "scripts/dogfood/" in str(s.get("run", "")):
            return True
    return False
def reach(job, seen):
    if job in seen:
        return False
    seen.add(job)
    if runs_gate(job):
        return True
    needs = (jobs.get(job) or {}).get("needs") or []
    if isinstance(needs, str):
        needs = [needs]
    return any(reach(n, seen) for n in needs)
bad = []
seen_any = False
for ctx in wanted:
    if ctx not in jobs:
        continue
    seen_any = True
    if not reach(ctx, set()):
        bad.append(ctx)
if not seen_any:
    print(f"UNMEASURED: none of the required contexts [{' '.join(wanted)}] is a job in ci.yml")
    sys.exit(3)
if bad:
    print("required check(s) reach no job that runs a dogfood gate: " + " ".join(bad))
    sys.exit(1)
print("ok")
PY_EOF
)" || gate_effect_rc=$?
if [ "$gate_effect_rc" -ne 0 ]; then
  fail "gate effect (Arm 6, the CB-2100 replacement): ${gate_effect}"
fi

echo "GATE B PASS comply clean; ruleset ${ruleset_id} requires [${contexts}]; ${#gates[@]} gate script(s) and ${n_others} other tracked script(s) at 0 bashrs errors; ratchet CB-200 held; required check(s) [${contexts}] reach a dogfood gate; legacy bashrs errors ${errors} <= ${BASHRS_ERROR_CEILING}"

# mutation: set BASHRS_ERROR_CEILING=0 — Arm 4 then reports the known
# scripts/ledger-replay.sh SEC011 finding as a regression and the gate exits 1.
