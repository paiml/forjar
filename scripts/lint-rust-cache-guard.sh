#!/usr/bin/env bash
# No self-hosted job may use `Swatinem/rust-cache` (or a wrapper of it) on the
# fleet's SHARED $CARGO_HOME.
#
# WHY THE RULE CHANGED (paiml/infra#775, 2026-09-19)
#
# This lint used to say: every `Swatinem/rust-cache@v2` use must set
# `cache-bin: "false"`. That was the mitigation from paiml/infra#208, and it was
# wrong — not incomplete, wrong, in a way that made this file bless the hazard
# it was written to stop.
#
# `cache-bin: false` skips `cleanBin` and nothing else. rust-cache's save step
# also runs `cleanRegistry`, unconditionally, which deletes
# ${CARGO_HOME}/registry/src — the extracted crate sources every OTHER job on
# the box is compiling from at that moment. All 16 intel-clean-room-* runners
# share HOME=/home/noah, so that directory is shared by every concurrent job.
#
# Measured on intel 2026-09-19: eight rust-cache save steps in 100 minutes, the
# birth time of every crate directory under the shared registry pinned to the
# second one of them finished, and an infra `cargo kani` build killed mid-crate
# with `could not parse/generate dep info at .../serde_core-<hash>.d: No such
# file or directory`. The step that killed it carried `cache-bin: "false"`. That
# error message is what aprender#2822 attributes to the ci-reaper, which had
# last run 27 minutes earlier.
#
# WHAT EXONERATES A USE, AND WHAT DOES NOT
#
# Only a job-private CARGO_HOME. It is the only thing that moves the SOURCES out
# of the shared directory. `--target-dir`, `cargo install --root` and a fresh
# $HOME do NOT: cargo reads crate sources from $CARGO_HOME whatever the target
# dir is, so a job that writes nothing shared is still compiling out of a
# directory another job's save step can delete underneath it. Measured example:
# paiml-implement's t12 install leg runs in a fresh $HOME and deliberately hands
# its legs the real shared CARGO_HOME, so it is fully exposed.
#
# On a persistent runner ~/.cargo persists between jobs by itself, so the
# RESTORE is a no-op and the SAVE is the entire effect. The remedy is to delete
# the step.
#
# WHY THIS REPO IN PARTICULAR
#
# rmedia hit the bin half FOUR TIMES. #283 fixed one of two call sites in one
# file and #289 then moved the other onto the fleet, deleting cargo mid-run.
# This lint was written then, and it encoded the mitigation rather than the
# hazard — so for a month it certified five steps that were deleting the shared
# registry, and one of them killed an infra build on 2026-09-19.
#
# WHY THE DENOMINATOR IS PRINTED AND ZERO USES IS NOT AUTOMATICALLY OK
#
# The previous version's pass line was "(N rust-cache use(s) across M workflows,
# all guarded)". With the action deleted everywhere, N is 0 and the old check
# passes having verified nothing — the vacuity this fleet is bitten by roughly
# once a month. So this version reports what it SCANNED (workflows, and
# self-hosted jobs among them) and refuses to pass on a zero denominator of
# either. `--selftest` proves it can still reject.
set -uo pipefail

# The actions that cache $CARGO_HOME. `setup-rust-toolchain` wraps rust-cache
# and its `cache:` input defaults to true, so an unconfigured use prunes and
# deletes exactly as a bare one does — and `cache-bin: false` does not turn it
# off either.
CACHE_ACTIONS='Swatinem/rust-cache|actions-rust-lang/setup-rust-toolchain'

usage() { printf 'usage: lint-rust-cache-guard.sh [--selftest]\n'; }

# HERE-STRINGS, NOT `printf ... | grep -q`, AND THIS IS LOAD-BEARING.
#
# Under `set -o pipefail`, `grep -q` exits at the first match and SIGPIPEs the
# writer, so the PIPELINE's status can be 141 even though the test matched —
# which makes a membership test answer the opposite of the truth, silently.
# rmedia bans the shape repo-wide (falsify-pipefail-contains.sh) and its CI
# caught this very file on the first run of the sibling copy of this lint. A lint that reports a wrong answer
# about a fleet-destroying step is worse than no lint, so the shape is gone
# rather than worked around.

# PER JOB, NOT PER FILE (review round 1 on forjar#589). The first version asked
# "does this FILE have a self-hosted job?" and "is a private CARGO_HOME declared
# ANYWHERE in the file?" — so one job's private CARGO_HOME exonerated a second
# self-hosted job that had none, and a hosted job's cache was flagged because a
# self-hosted job shared its file. Both are wrong verdicts about a real job.
#
# classify_jobs reads one comment-stripped workflow and prints, per job:
#   JOB <name> <selfhosted 0|1>
#   USE <line> <exonerated 0|1>      one per cache-action step in that job
# A job is self-hosted unless its runs-on names ONLY a hosted image
# (ubuntu-/macos-/windows-): an expression or a label list this cannot classify
# counts as self-hosted, because the cost of a false negative is the fleet.
# Exonerated means a non-shared CARGO_HOME at workflow `env:`, at the job's own
# `env:`, or in the cache step's own `env:` — the three places the value can
# actually reach the action. A CARGO_HOME on a DIFFERENT step does not reach it
# and does not count.
classify_jobs() { # stdin: stripped workflow text
    awk -v ACTIONS="$CACHE_ACTIONS" '
    function ind(s) { match(s, /^ */); return RLENGTH }
    function key(s) { t = s; sub(/^ *(- )?/, "", t); return t }
    function shared(v) {
        gsub(/^[ \t"\047]+|[ \t"\047]+$/, "", v)
        return (v == "" || v ~ /^(~|\$\{?HOME\}?|\/home\/[^\/]+|\/Users\/[^\/]+)\/\.cargo/)
    }
    function hosted(v) { return (v ~ /(ubuntu|macos|windows)-/ && v !~ /self-hosted|clean-room/) }
    function end_step() { if (s_uses) printf "USE %d %d\n", s_uses, (wf_priv || j_priv || s_priv); s_uses = 0; s_priv = 0; in_senv = 0 }
    function end_job() { end_step(); if (job != "") printf "JOB %s %d\n", job, (ro_seen ? !ro_hosted : 1); job = ""; j_priv = 0; ro_seen = 0; ro_hosted = 1; in_jenv = 0; in_steps = 0; in_ro = 0 }
    {
        line = $0
        if (line ~ /^[ \t]*$/) next
        i = ind(line); k = key(line)
        if (i == 0) {
            end_job(); in_jobs = (k ~ /^jobs:/); in_wfenv = (k ~ /^env:/); next
        }
        if (in_wfenv) { if (k ~ /^CARGO_HOME:/) { v = k; sub(/^CARGO_HOME:/, "", v); if (!shared(v)) wf_priv = 1 }; next }
        if (!in_jobs) next
        if (i == 2 && k ~ /^[A-Za-z0-9_-]+:[ \t]*$/) { end_job(); job = k; sub(/:.*/, "", job); jind = i; next }
        if (job == "") next
        if (i == jind + 2) {
            end_step(); in_jenv = 0; in_steps = 0; in_ro = 0
            if (k ~ /^env:/) { in_jenv = 1; next }
            if (k ~ /^steps:/) { in_steps = 1; next }
            if (k ~ /^runs-on:/) {
                v = k; sub(/^runs-on:[ \t]*/, "", v); ro_seen = 1
                if (v == "") { in_ro = 1; ro_hosted = 1; ro_items = "" } else ro_hosted = hosted(v)
            }
            next
        }
        if (in_ro) { ro_items = ro_items " " k; ro_hosted = hosted(ro_items); next }
        if (in_jenv) { if (k ~ /^CARGO_HOME:/) { v = k; sub(/^CARGO_HOME:/, "", v); if (!shared(v)) j_priv = 1 }; next }
        if (in_steps) {
            if (line ~ /^ *- /) { end_step(); sind = i }
            if (k ~ ("^uses:[ \t]*(" ACTIONS ")")) s_uses = NR
            if (k ~ /^env:/ && i > sind) { in_senv = 1; next }
            if (in_senv && k ~ /^CARGO_HOME:/) { v = k; sub(/^CARGO_HOME:/, "", v); if (!shared(v)) s_priv = 1 }
        }
    }
    END { end_job() }'
}

scan() { # $1 = directory holding .github/workflows
    local root="$1" wf out kind a b
    local scanned=0 selfhosted=0 uses=0 failed=0
    for wf in "$root"/.github/workflows/*.yml "$root"/.github/workflows/*.yaml; do
        [ -f "$wf" ] || continue
        scanned=$((scanned + 1))
        out=$(sed -e 's/[[:space:]]#.*$//' -e 's/^#.*$//' "$wf" | classify_jobs)
        # USE lines precede their job's JOB line; hold them until the verdict on the job.
        local pending=""
        while read -r kind a b; do
            case "$kind" in
                USE) pending="$pending $a:$b" ;;
                JOB)
                    if [ "$b" = 1 ]; then
                        selfhosted=$((selfhosted + 1))
                        for u in $pending; do
                            uses=$((uses + 1))
                            [ "${u#*:}" = 1 ] && continue
                            failed=1
                            printf 'ERROR: %s:%s (job `%s`) uses a $CARGO_HOME cache action on a self-hosted job.\n' "$wf" "${u%%:*}" "$a"
                            printf '       Its save step deletes ${CARGO_HOME}/registry/src — the crate sources\n'
                            printf '       every other job on the box is compiling from (paiml/infra#775).\n'
                            printf '       cache-bin: "false" does NOT prevent this; it skips cleanBin only.\n'
                            printf '       Remedy: delete the step, or give the job a private CARGO_HOME.\n'
                        done
                    fi
                    pending="" ;;
            esac
        done <<<"$out"
    done

    # Two floors, because two different breakages produce a silent pass: no
    # workflows at all (wrong cwd, bad glob) and no self-hosted JOBS among them
    # (a runs-on matcher that stopped matching). Either one is a NO-GO.
    if [ "$scanned" -eq 0 ]; then
        printf 'ERROR: scanned 0 workflows — refusing to pass vacuously\n' >&2
        return 1
    fi
    if [ "$selfhosted" -eq 0 ]; then
        printf 'ERROR: 0 self-hosted jobs in %d workflows — this repo runs on the fleet,\n' "$scanned" >&2
        printf '       so a zero here means the runs-on matcher is broken, not that the fleet\n' >&2
        printf '       is unused. Refusing to pass vacuously.\n' >&2
        return 1
    fi
    if [ "$failed" -ne 0 ]; then
        printf 'lint-rust-cache-guard: FAIL (%d use(s) in %d self-hosted job(s) across %d workflow(s))\n' \
            "$uses" "$selfhosted" "$scanned" >&2
        return 1
    fi
    printf 'lint-rust-cache-guard: OK (%d $CARGO_HOME cache use(s) in %d self-hosted job(s) across %d workflow(s))\n' \
        "$uses" "$selfhosted" "$scanned"
    return 0
}

# --selftest: prove the instrument can still REJECT, and that the exoneration
# works. A lint whose corpus is a repo that has already been fixed is a lint
# nobody has seen fail — which is indistinguishable from one that cannot.
selftest() {
    local tmp rc=0
    tmp="$(mktemp -d)" || { printf 'selftest: no temp dir\n' >&2; return 2; }
    trap 'if [ -n "${tmp:-}" ] && [ -d "${tmp:-}" ]; then rm -rf "${tmp:?}"; fi' RETURN
    mkdir -p "$tmp/bad/.github/workflows" "$tmp/good/.github/workflows" \
             "$tmp/hosted/.github/workflows" "$tmp/private/.github/workflows"

    cat > "$tmp/bad/.github/workflows/ci.yml" <<'YML'
on: [push]
jobs:
  test:
    runs-on: [self-hosted, clean-room]
    steps:
      - uses: Swatinem/rust-cache@v2
        with:
          cache-bin: "false"
      - run: cargo test
YML
    cat > "$tmp/good/.github/workflows/ci.yml" <<'YML'
on: [push]
jobs:
  test:
    runs-on: [self-hosted, clean-room]
    steps:
      - run: cargo test
YML
    cat > "$tmp/hosted/.github/workflows/ci.yml" <<'YML'
on: [push]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: Swatinem/rust-cache@v2
      - run: cargo test
YML
    cat > "$tmp/private/.github/workflows/ci.yml" <<'YML'
on: [push]
jobs:
  test:
    runs-on: [self-hosted, clean-room]
    env:
      CARGO_HOME: ${{ github.workspace }}/.cargo-home
    steps:
      - uses: Swatinem/rust-cache@v2
      - run: cargo test
YML

    # 1. the shape that shipped for a month must be REJECTED, cache-bin and all
    if scan "$tmp/bad" >/dev/null 2>&1; then
        printf '  FAIL a self-hosted rust-cache with cache-bin: "false" was accepted\n'; rc=1
    else
        printf '  ok   a self-hosted rust-cache is rejected even with cache-bin: "false"\n'
    fi
    # 2. and the fixed shape must PASS — a lint that only ever fails is no lint
    if scan "$tmp/good" >/dev/null 2>&1; then
        printf '  ok   a self-hosted workflow with no cache action passes\n'
    else
        printf '  FAIL a clean self-hosted workflow was rejected\n'; rc=1
    fi
    # 3. a hosted runner has an ephemeral $HOME: out of scope, and the
    #    self-hosted floor must then refuse to report a verdict at all
    if scan "$tmp/hosted" >/dev/null 2>&1; then
        printf '  FAIL a repo with no self-hosted job reported a pass instead of NO-GO\n'; rc=1
    else
        printf '  ok   no self-hosted job -> NO-GO, never a vacuous pass\n'
    fi
    # 4. the one exoneration
    if scan "$tmp/private" >/dev/null 2>&1; then
        printf '  ok   a job-private CARGO_HOME is exonerated\n'
    else
        printf '  FAIL a job-private CARGO_HOME was rejected\n'; rc=1
    fi

    # 5-12: one fixture per SHAPE, after review round 1 found the file-level
    # version wrong in three of them (5, 6, 7). Each is a whole workflow.
    case_scan() { # $1 name, $2 want pass|fail, stdin = workflow
        mkdir -p "$tmp/c/$1/.github/workflows"; cat > "$tmp/c/$1/.github/workflows/ci.yml"
        local got=fail; scan "$tmp/c/$1" >/dev/null 2>&1 && got=pass
        if [ "$got" = "$2" ]; then printf '  ok   %s -> %s\n' "$1" "$2"; else printf '  FAIL %s -> %s, wanted %s\n' "$1" "$got" "$2"; rc=1; fi
    }
    case_scan 'two-self-hosted-jobs-only-one-private' fail <<'YML'
on: [push]
jobs:
  a:
    runs-on: [self-hosted, clean-room]
    env:
      CARGO_HOME: ${{ github.workspace }}/../cargo-home-a
    steps:
      - uses: Swatinem/rust-cache@v2
  b:
    runs-on: [self-hosted, clean-room]
    steps:
      - uses: Swatinem/rust-cache@v2
YML
    case_scan 'hosted-cache-beside-a-clean-self-hosted-job' pass <<'YML'
on: [push]
jobs:
  hosted:
    runs-on: ubuntu-latest
    steps:
      - uses: Swatinem/rust-cache@v2
  fleet:
    runs-on: [self-hosted, clean-room]
    steps:
      - run: cargo test
YML
    case_scan 'cargo-home-only-in-a-comment' fail <<'YML'
on: [push]
jobs:
  t:
    runs-on: [self-hosted, clean-room]
    # CARGO_HOME: /srv/private  <- a comment, not a declaration
    steps:
      - uses: Swatinem/rust-cache@v2
YML
    case_scan 'workflow-level-private-env' pass <<'YML'
on: [push]
env:
  CARGO_HOME: ${{ github.workspace }}/../cargo-home
jobs:
  t:
    runs-on: [self-hosted, clean-room]
    steps:
      - uses: Swatinem/rust-cache@v2
YML
    case_scan 'private-cargo-home-on-a-different-step' fail <<'YML'
on: [push]
jobs:
  t:
    runs-on: [self-hosted, clean-room]
    steps:
      - run: cargo build
        env:
          CARGO_HOME: /srv/other
      - uses: Swatinem/rust-cache@v2
YML
    case_scan 'private-cargo-home-on-the-cache-step' pass <<'YML'
on: [push]
jobs:
  t:
    runs-on: [self-hosted, clean-room]
    steps:
      - uses: Swatinem/rust-cache@v2
        env:
          CARGO_HOME: /srv/job-private
YML
    case_scan 'runs-on-as-a-block-list' fail <<'YML'
on: [push]
jobs:
  t:
    runs-on:
      - self-hosted
      - clean-room
    steps:
      - uses: actions-rust-lang/setup-rust-toolchain@v1
YML
    case_scan 'shared-cargo-home-spelled-out' fail <<'YML'
on: [push]
jobs:
  t:
    runs-on: [self-hosted, clean-room]
    env:
      CARGO_HOME: /home/noah/.cargo
    steps:
      - uses: Swatinem/rust-cache@v2
YML

    if [ "$rc" -ne 0 ]; then
        printf 'lint-rust-cache-guard --selftest: NO-GO — the instrument is broken\n' >&2
        return 1
    fi
    printf 'lint-rust-cache-guard --selftest: OK (12 cases)\n'
    return 0
}

case "${1:-}" in
    --selftest) selftest; exit $? ;;
    -h|--help) usage; exit 0 ;;
    "") scan "."; exit $? ;;
    *) printf 'unknown argument: %s\n' "$1" >&2; usage >&2; exit 2 ;;
esac
