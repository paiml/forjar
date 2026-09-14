#!/usr/bin/env bash
# Dogfood gate G — the contract corpus.
#
# Exit code is the gate. The only line a caller must read is the final
# `GATE G PASS|FAIL <detail>`.
#
# What this gate is for. `pv audit` printed "Falsification tests: 14 / No audit
# findings" over a contract two of whose falsifiers named functions that did not
# exist (GH-298): it counts DECLARATIONS and never resolves one. So a corpus can
# be 100% green and govern nothing. The four arms below are ordered from
# cheapest to most falsifiable:
#
#   1  every contract PARSES and validates          (pv validate, per file)
#   2  the corpus LINTS at zero errors              (pv lint contracts)
#   3  every contract has the DEPTH its kind claims (kernel ⇒ equations+kani;
#      any kind ⇒ at least one falsifier)
#   4  every falsifier citation RESOLVES to the item it names
#   5  every shipped verb and resource kind is NAMED by some contract
#
# Two arms are ratchets rather than absolutes, and each says why in place. A
# ratchet is not a weakened assertion: the ceiling MAY ONLY SHRINK, so the
# property it guards ("this does not get worse") is falsifiable on every run,
# which "all 35 contracts are perfect" was not on the day this gate was written.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/../.."

# contracts/binding.yaml is NOT a contract. It is the binding registry that the
# repo-root build.rs reads (`std::fs::read_to_string(binding_path)` ->
# `BindingRegistryYaml`) to resolve equation -> Rust item; it carries no
# `metadata.kind`, no equations and no falsifiers, and `pv validate` would
# rightly refuse it. tests/common/contract_citations.rs excludes it by the same
# name for the same reason.
NOT_A_CONTRACT="binding.yaml"

# The corpus is not allowed to shrink below this without someone saying so.
# 35 yaml files today, one of which is the registry above.
MIN_CONTRACTS=30

# Contracts that declare falsifiers but NONE of them names a `.rs` file that
# exists — the falsifier is prose ("proptest with random DAGs") or a bare
# function name with no path. Those are real gaps and each is a ticket of its
# own; PMAT-163 is not that ticket and may not edit these files. Recorded so
# that a NEW contract cannot join them silently.
#
# MAY ONLY SHRINK. Was 15 before PMAT-163 anchored apply-receipt-v1,
# apply-summary-distinguishability-v1 and plan-declares-its-quantifier-v1.
# flag-has-effect-v1.yaml left this list on 2026-09-08 (PMAT-165): three of its
# entries cite a cargo test target in `command:` — falsification_yes_is_not_an_
# integrity_override and falsification_plan_file_runs_every_gate — which the
# rule above now resolves to tests/<target>.rs. Nothing about the contract
# changed; the gate stopped looking in one field only.
UNANCHORED=(
  "codegen-dispatch-v1.yaml"
  "config-load-consistency-v1.yaml"
  "copia-provisioning-v1.yaml"
  "dag-ordering-v1.yaml"
  "execution-safety-v1.yaml"
  "input-rejection-v1.yaml"
  "machine-output-parses-v1.yaml"
  "provable-iac-v1.yaml"
  "recipe-determinism-v1.yaml"
  "state-dir-visibility-v1.yaml"
  "verified-effect-v1.yaml"
)

# Surface tokens no contract mentions. Arm 5's match is a NECESSARY condition,
# not a sufficient one: a contract that never writes the word `gpu` certainly
# does not govern gpu resources, while one that does may only be using the word
# in prose. The negative direction is the one worth gating.
#
# MAY ONLY SHRINK.
UNGOVERNED=(
  "verb:workspace"
  "kind:gpu"
)

fail() {
  echo "GATE G FAIL $1"
  exit 1
}

BIN="$(bash "$(dirname "${BASH_SOURCE[0]}")/lib/binary.sh")"

# ------------------------------------------------- Arm 1: every contract validates
shopt -s nullglob
all=(contracts/*.yaml)
shopt -u nullglob
contracts=()
for c in "${all[@]}"; do
  if [ "$(basename "$c")" = "$NOT_A_CONTRACT" ]; then
    continue
  fi
  contracts+=("$c")
done
if [ "${#contracts[@]}" -lt "$MIN_CONTRACTS" ]; then
  fail "only ${#contracts[@]} contract(s) under contracts/, floor is ${MIN_CONTRACTS} — every arm below would be measuring a corpus that had been emptied"
fi
for c in "${contracts[@]}"; do
  rc=0
  out="$(pv validate "$c" 2>&1)" || rc=$?
  if [ "$rc" -ne 0 ]; then
    echo "$out"
    fail "pv validate ${c} exited ${rc}"
  fi
  case "$out" in
    *"Contract is valid."*) : ;;
    *) fail "pv validate ${c} exited 0 without saying the contract is valid — unmeasured is not valid" ;;
  esac
done

# --------------------------------------------------------- Arm 2: the corpus lints
#
# `pv lint <one-file.yaml>` reports `Result: PASS` over ZERO contracts: the
# positional argument is a CONTRACT_DIR, so a file path matches nothing and the
# gate is vacuous while exiting 0. The directory form is the only real signal.
# The tree carries ~123 style warnings; 0 ERRORS is the bar.
rc=0
lint="$(pv lint contracts 2>&1)" || rc=$?
if [ "$rc" -ne 0 ]; then
  echo "$lint"
  fail "pv lint contracts exited ${rc}"
fi
# `grep` exits 1 on "no match", and `set -o pipefail` turns that into a silent
# script death mid-assignment. Capturing rc keeps the two apart: 1 means the
# Summary line is absent (handled below, and fatal), >=2 means grep itself
# failed and the arm is UNMEASURED.
rc=0
summary="$(printf '%s' "$lint" | grep -E '^Summary: [0-9]+ errors')" || rc=$?
if [ "$rc" -gt 1 ]; then
  fail "grep exited ${rc} scanning pv lint output — the Summary line is UNMEASURED, and unmeasured is not clean"
fi
if [ -z "$summary" ]; then
  echo "$lint"
  fail "pv lint printed no 'Summary: N errors' line — the linter cannot be shown to have run, and unmeasured is not clean"
fi
lint_errors="$(printf '%s' "$summary" | sed -E 's/^Summary: ([0-9]+) errors.*/\1/')"
if [ "$lint_errors" -ne 0 ]; then
  echo "$lint"
  fail "pv lint contracts reports ${lint_errors} error(s)"
fi

# ------------------------------------------- Arm 3 + 4: depth, and the two ratchets
#
# Delegated to python3 because the questions are about YAML structure. The
# script prints one machine-readable line per finding and exits non-zero on the
# first hard violation; the ratchets are compared here in shell so the ceiling
# lives beside the other ceilings in this file.
depth_out="$(
  python3 - "${contracts[@]}" <<'PY'
import os, re, sys
import yaml

hard, anchored_gaps = [], []
kernels = 0
for path in sys.argv[1:]:
    name = os.path.basename(path)
    with open(path, encoding="utf-8") as fh:
        doc = yaml.safe_load(fh)
    meta = doc.get("metadata") or {}
    kind = meta.get("kind")
    eqs = doc.get("equations") or {}
    kani = doc.get("kani_harnesses") or []
    tests = doc.get("falsification_tests") or []

    # A `kernel` claims a bounded, machine-checkable property. PROVABILITY-001
    # is the rule; a kernel with no equation states nothing and a kernel with
    # no harness proves nothing.
    if kind == "kernel":
        kernels += 1
        if not eqs:
            hard.append(f"{name}: kind=kernel with no equations")
        if not kani:
            hard.append(f"{name}: kind=kernel with no kani_harnesses")

    # Every kind, kernel or pattern, must name at least one way to be wrong.
    if not tests:
        hard.append(f"{name}: no falsification_tests — the contract cannot be refuted")

    # An ANCHORED falsifier names a `.rs` file that exists. Prose falsifiers
    # ("proptest with random DAGs") describe an experiment nobody runs.
    anchored = 0
    for t in tests:
        # WHERE A CITATION MAY LIVE. `pv` treats `test` and `command` as the
        # same field for some contract kinds — measured 2026-09-08 on
        # undo-refuses-multi-stack-state-dir-v1.yaml, where adding `test:`
        # beside `command:` made pv refuse the file as a duplicate field, and
        # renaming `command:` let the same `test:` through. A contract that
        # cites its falsifier in `command:` is wired, and reading only `test`
        # called sixteen wired falsifiers prose.
        #
        # HOW A CITATION RESOLVES. Either a path ending in `.rs` that exists,
        # or cargo's own `--test <target>`, which by cargo's layout IS
        # `tests/<target>.rs` — a name that resolves to a file on disk is an
        # address a reviewer can follow, which is the whole point of the rule.
        cite = " ".join(
            str((t or {}).get(k) or "") for k in ("test", "test_secondary", "command")
        )
        # EVERY named path must exist, not merely one of them: a citation that
        # names a file that is not there is an address a reviewer cannot follow,
        # and counting it because a sibling resolved is how an unwired falsifier
        # hides behind a wired one (found by the PMAT-165 quorum).
        paths = [m.group(1) for m in re.finditer(r"([\w./-]+\.rs)", cite)]
        targets = [
            os.path.join("tests", m.group(1) + ".rs")
            for m in re.finditer(r"--test\s+([A-Za-z0-9_]+)", cite)
        ]
        named = paths + targets
        if named and all(os.path.isfile(x) for x in named):
            anchored += 1
    if tests and anchored == 0:
        anchored_gaps.append(name)

print(f"KERNELS {kernels}")
for g in sorted(anchored_gaps):
    print(f"UNANCHORED {g}")
for h in hard:
    print(f"HARD {h}")
PY
)"

# PMAT-240: a here-string, not a pipe — `grep -q` exits on its first match and
# printf then takes SIGPIPE, which under pipefail kills the gate with no verdict.
if grep -q '^HARD ' <<<"$depth_out"; then
  printf '%s\n' "$depth_out" | grep '^HARD '
  fail "a contract does not have the depth its kind claims (see HARD lines above)"
fi
kernels="$(printf '%s\n' "$depth_out" | sed -n 's/^KERNELS //p')"
if [ "${kernels:-0}" -lt 1 ]; then
  fail "no contract declares kind=kernel, so the kernel depth rule above ran over nothing"
fi

# The ratchet: the observed set must be a SUBSET of the recorded one. A name
# that leaves the list is progress and is required to leave the list; a name
# that joins it is a regression.
observed="$(printf '%s\n' "$depth_out" | sed -n 's/^UNANCHORED //p' | sort)"
recorded="$(printf '%s\n' "${UNANCHORED[@]}" | sort)"
new_gaps="$(comm -23 <(printf '%s\n' "$observed") <(printf '%s\n' "$recorded"))"
if [ -n "$new_gaps" ]; then
  fail "contract(s) whose every falsifier is prose, and which are not in the recorded ceiling: $(printf '%s' "$new_gaps" | tr '\n' ' ')"
fi
healed="$(comm -13 <(printf '%s\n' "$observed") <(printf '%s\n' "$recorded"))"
if [ -n "$healed" ]; then
  fail "UNANCHORED has shrunk (good) but the list in this file still names: $(printf '%s' "$healed" | tr '\n' ' ') — remove them, a ceiling that outlives the gap stops being a ceiling"
fi

# Arm 4 is the repo's own resolver, run rather than reimplemented. A second
# resolver here would disagree with it on the corpus's four citation shapes
# (`path.rs`, `path.rs::fn`, `path.rs::mod::fn`, `path.rs mod::fn`), and the
# looser of two resolvers is the one that gets believed.
rc=0
cites="$(cargo test --locked --test falsification_contract_citations_resolve 2>&1)" || rc=$?
if [ "$rc" -ne 0 ]; then
  printf '%s\n' "$cites" | tail -40
  fail "falsification_contract_citations_resolve is RED: a falsifier names an item that is not in the file it names"
fi
case "$cites" in
  *"test result: ok."*) : ;;
  *) fail "falsification_contract_citations_resolve produced no 'test result: ok.' line — a suite that did not run is not a suite that passed" ;;
esac

# --------------------------------- Arm 5: contract-surface reconciliation
#
# Derived from the BUILT BINARY, never from a list in this file. `verb list` is
# the unified surface; `schema` carries the resource `type` enum.
verbs="$("$BIN" verb list)"
if [ "$(printf '%s\n' "$verbs" | grep -c .)" -lt 1 ]; then
  fail "'${BIN} verb list' is empty — the reconciliation below would be vacuous"
fi
kinds="$("$BIN" schema | python3 -c 'import json,sys; print("\n".join(json.load(sys.stdin)["properties"]["resources"]["additionalProperties"]["properties"]["type"]["enum"]))')"
if [ "$(printf '%s\n' "$kinds" | grep -c .)" -lt 1 ]; then
  fail "the resource type enum in '${BIN} schema' is empty — the reconciliation below would be vacuous"
fi

ungoverned=""
for v in $verbs; do
  if ! grep -rqwF -- "$v" "${contracts[@]}"; then
    ungoverned="${ungoverned}verb:${v}"$'\n'
  fi
done
for k in $kinds; do
  if ! grep -rqwF -- "$k" "${contracts[@]}"; then
    ungoverned="${ungoverned}kind:${k}"$'\n'
  fi
done
# An EMPTY observed set is the good case here, and `grep` reports it with exit
# 1, which pipefail would turn into a silent death. rc is captured so that
# "nothing ungoverned" and "grep could not run" stay distinguishable.
rc=0
observed_ung="$(printf '%s' "$ungoverned" | grep . | sort)" || rc=$?
if [ "$rc" -gt 1 ]; then
  fail "grep exited ${rc} collecting the ungoverned surface — the ratchet is UNMEASURED"
fi
recorded_ung="$(printf '%s\n' "${UNGOVERNED[@]}" | sort)"
new_ung="$(comm -23 <(printf '%s\n' "$observed_ung") <(printf '%s\n' "$recorded_ung"))"
if [ -n "$new_ung" ]; then
  fail "shipped surface no contract names: $(printf '%s' "$new_ung" | tr '\n' ' ') — a capability governed by nothing"
fi
healed_ung="$(comm -13 <(printf '%s\n' "$observed_ung") <(printf '%s\n' "$recorded_ung"))"
if [ -n "$healed_ung" ]; then
  fail "UNGOVERNED still names $(printf '%s' "$healed_ung" | tr '\n' ' '), which a contract now covers — remove them from this file"
fi

n_verbs="$(printf '%s\n' "$verbs" | grep -c .)"
n_kinds="$(printf '%s\n' "$kinds" | grep -c .)"
echo "GATE G PASS ${#contracts[@]} contract(s) validate; pv lint 0 errors; ${kernels} kernel(s) carry equations and kani harnesses; every contract names a falsifier; citations resolve; ${n_verbs} verb(s) and ${n_kinds} resource kind(s) reconciled against the corpus (${#UNANCHORED[@]} unanchored, ${#UNGOVERNED[@]} ungoverned, both at their recorded ceiling)"

# mutation: delete the `falsification_tests:` block appended to
# contracts/apply-receipt-v1.yaml — Arm 3 then emits `HARD apply-receipt-v1.yaml:
# no falsification_tests` and the gate exits 1.
