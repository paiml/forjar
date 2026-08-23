#!/usr/bin/env bash
# Re-run the defect ledger's own repros and fail when one still reproduces.
#
# WHY THIS EXISTS. docs/cli-defects.json holds 85 defects under `confirmed`,
# every one investigated, reproduced and written down against forjar 1.12.3.
# A 1.16.0 audit re-ran them: FOURTEEN still reproduced. They survived 1.13,
# 1.14, 1.15 and 1.16 — not because they were hard, but because nothing ever
# re-ran the ledger. A defect register written once and never replayed is not a
# register; it is a list of things you have decided to tolerate, whether or not
# you decided that. (GH #299, #300.)
#
# WHY IT IS OPT-IN, AND WHY THAT IS NOT A LOOPHOLE. The `repro` field is human
# prose — "write 3-file config (params.sandbox=$D)" — not a script. A harness
# that tried to execute `repro` would run nothing and report success, which is
# precisely the vacuous-pass shape this whole corpus is about. So replay runs
# only entries carrying an explicit `replay` script, AND it reports the
# denominator on every run, AND it refuses to succeed when nothing ran. The
# number of unreplayable entries is visible in every log line rather than
# implied by silence.
#
# EXIT CODES
#   0  every replayable entry is fixed (and at least one ran)
#   1  an entry still reproduces, or nothing was replayable, or the instrument
#      itself could not run
set -uo pipefail

LEDGER="${LEDGER:-docs/cli-defects.json}"
FORJAR="${FORJAR:-$(command -v forjar || true)}"
ONLY="${ONLY:-}"    # optional: replay a single id

[ -f "$LEDGER" ] || { echo "FAIL: no ledger at $LEDGER — unmeasured is not passing"; exit 1; }
[ -n "$FORJAR" ] && [ -x "$FORJAR" ] || {
    echo "FAIL: no forjar binary (set \$FORJAR). An absent subject is a NO-GO, not a skip."
    exit 1
}

echo "ledger-replay against $("$FORJAR" --version)"
echo "ledger: $LEDGER"
echo

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# Hand each replay script a clean sandbox and the binary under test. Scripts
# MUST confine themselves to $SANDBOX: the ledger's own rules are file/command
# resources under /tmp, machine addr 127.0.0.1, never a real host or state dir.
run_one() {
    local id="$1" script="$2"
    local sandbox="$WORK/$id"
    mkdir -p "$sandbox"
    (
        cd "$sandbox" || exit 90
        export FORJAR SANDBOX="$sandbox"
        # shellcheck disable=SC2086
        timeout 120 sh -c "$script"
    ) >"$WORK/$id.out" 2>&1
    return $?
}

total=0; replayable=0; fixed=0; still=0; errored=0
declare -a STILL_LIST=() ERR_LIST=()

while IFS=$'\t' read -r id sev has script; do
    total=$((total+1))
    [ -n "$ONLY" ] && [ "$ONLY" != "$id" ] && continue
    [ "$has" = "yes" ] || continue
    replayable=$((replayable+1))

    run_one "$id" "$script"; rc=$?
    case "$rc" in
        # A replay script asserts THE DEFECT IS GONE. It exits 0 when the
        # behaviour is correct now, and non-zero when the old defect still
        # reproduces — so the script reads as a test, not as a re-enactment.
        0)   fixed=$((fixed+1));  printf '  fixed  %-58s %s\n' "$id" "$sev" ;;
        124) errored=$((errored+1)); ERR_LIST+=("$id (timed out after 120s)")
             printf '  ERROR  %-58s timeout\n' "$id" ;;
        90)  errored=$((errored+1)); ERR_LIST+=("$id (sandbox unusable)")
             printf '  ERROR  %-58s sandbox\n' "$id" ;;
        *)   still=$((still+1)); STILL_LIST+=("$id")
             printf '  STILL  %-58s %s (rc=%s)\n' "$id" "$sev" "$rc" ;;
    esac
done < <(python3 - "$LEDGER" <<'PY'
import json, sys
led = json.load(open(sys.argv[1]))
for e in led.get("confirmed", []):
    r = e.get("replay")
    has = "yes" if isinstance(r, str) and r.strip() else "no"
    # tabs and newlines would break the read loop; the script travels base64-free
    # by being joined onto one line, so replay scripts must be `;`-separated or
    # use explicit newline escapes.
    script = " ".join((r or "").splitlines()) if has == "yes" else ""
    print(f'{e.get("id","?")}\t{e.get("severity","?")}\t{has}\t{script}')
PY
)

echo
echo "replayed $replayable of $total confirmed entries: $fixed fixed, $still still reproducing, $errored errored"
echo "$((total - replayable)) entr(ies) carry no \`replay\` script and were NOT tested"

rc=0

# NOTHING RAN IS A FAILURE. A sweep that replayed zero entries and reported
# success is the defect this harness exists to catch, one level up.
if [ "$replayable" -eq 0 ]; then
    echo
    echo "FAIL: no entry carried a \`replay\` script, so this run measured NOTHING."
    echo "  Add \`\"replay\": \"<sh that exits 0 iff the defect is gone>\"\` to ledger entries."
    rc=1
fi

if [ "$still" -gt 0 ]; then
    echo
    echo "FAIL: $still ledger defect(s) still reproduce:"
    for i in "${STILL_LIST[@]}"; do
        echo "  - $i"
        sed 's/^/      /' "$WORK/$i.out" | head -12
    done
    rc=1
fi

if [ "$errored" -gt 0 ]; then
    echo
    echo "FAIL: $errored replay script(s) could not run. Unmeasured is not passing:"
    for i in "${ERR_LIST[@]}"; do echo "  - $i"; done
    rc=1
fi

[ "$rc" -eq 0 ] && echo && echo "every replayable ledger defect is fixed"
exit "$rc"
