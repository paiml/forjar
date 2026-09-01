#!/usr/bin/env bash
# Quorum gate: a branch may not be pushed until its claims have survived refutation.
#
# WHY THIS EXISTS
#
# The methodology was already written down in two places and enforced in
# neither: `docs/specifications/provable-iac.md` requires "quorum-validated
# design (>=3 world-class systems)" per phase, and the fleet's own notes record
# that on the 1.21.0 sweep EVERY high-value finding came from an agent told to
# refute another agent's claim -- not from the implementations. Two regressions
# were caught that a fully green suite had passed.
#
# A methodology that lives only in a spec is a suggestion. `main` here is NOT
# branch-protected (`gh api .../branches/main/protection` -> 404) and CODEOWNERS
# routes everything to one person, so nothing downstream of the push will catch
# an unrefuted claim either. The push is the last chokepoint before a PR exists,
# so the gate goes here.
#
# WHAT THIS GATE DOES AND DOES NOT PROVE
#
# It does NOT re-run the quorum -- that costs millions of tokens and minutes of
# wall clock, and re-running it here would only re-derive what the receipt
# already records. It verifies the RECEIPT: that a quorum ran against THIS EXACT
# DIFF, that it met its thresholds, and that the falsification it claims is real
# enough to check.
#
# The split is deliberate and worth stating plainly, because a gate that
# overclaims is worse than no gate:
#
#   VERIFIED here  - the receipt exists and parses
#                  - it is bound to this diff (hash), so it cannot be recycled
#                  - lane/refuter/judge counts meet the declared floors
#                  - the falsification test EXISTS in the tree
#                  - that test PASSES right now, with the fix in place
#   ATTESTED only  - that the same test went RED when the fix was reverted
#
# The red half is attested because verifying it means reverting production code
# and rebuilding, which a pre-push hook must not do to a developer's tree. The
# green half is checked, and a receipt naming a test that does not exist or does
# not pass is rejected -- which kills the cheapest way to fake a falsification.
#
# UNMEASURED IS A FAILURE, NOT A PASS. `scripts/cb200-ratchet.sh` learned this
# the hard way: CB-200 reported Skip ("no .pmat/context.db") and that green was
# the index's absence rather than the tree's quality. Every early exit below is
# therefore a non-zero exit with a reason, never a silent 0.
set -euo pipefail

RECEIPT_DIR=".quorum"

# THE BASE IS NOT OVERRIDABLE BY THE ENVIRONMENT.
#
# It was, via $QUORUM_BASE, and that was a one-word bypass of the entire gate:
# `QUORUM_BASE=HEAD git push` makes `git diff HEAD...` mathematically empty, the
# empty-diff branch below then reports "nothing to refute" and exits 0. Verified
# against the first draft of this script. An escape hatch on the one input that
# defines "what is being reviewed" is not a convenience, it is the hole.
#
# A branch based on something other than main is handled by merge-base below,
# which is what the override was actually for.
BASE_REF="origin/main"

# Floors. Deliberately low: this gate enforces that a quorum HAPPENED, not that
# it was large. A 3-lane/3-refuter quorum that actually killed a claim beats a
# 9-lane one that rubber-stamped. Raise them by editing here, in the open.
MIN_LANES=3
MIN_REFUTERS=3
MIN_JUDGES=3

die() { echo "✗ QUORUM GATE: $*" >&2; exit 1; }

command -v python3 >/dev/null 2>&1 || die "python3 is required to read the receipt"

# THE BRANCH IS THE ONE BEING PUSHED TO, NOT THE ONE CHECKED OUT.
#
# Reading the local name was a rename bypass: `git checkout -b main && git push
# origin main:real-feature` hits the main|master exemption below and skips the
# gate entirely, while pushing a feature branch to the remote. git's pre-push
# protocol hands the real target on stdin as
#   <local ref> <local sha> <remote ref> <remote sha>
# so the hook passes it here with --remote-ref. Standalone runs (no hook) fall
# back to the local name, which is fine because a human running this by hand is
# not the adversary it defends against.
remote_ref=""
while [ $# -gt 0 ]; do
    case "$1" in
        --remote-ref) remote_ref="${2:-}"; shift 2 ;;
        *) shift ;;
    esac
done

if [ -n "$remote_ref" ]; then
    branch="${remote_ref#refs/heads/}"
else
    branch="$(git rev-parse --abbrev-ref HEAD)"
    [ "$branch" != "HEAD" ] || die "detached HEAD -- cannot identify the branch under review"
fi

# main is exempt ONLY as a push target: there is no PR to gate, and a release
# commit legitimately has no new claims of its own to refute.
case "$branch" in
    main|master) echo "✓ quorum gate: '$branch' is not a PR branch, skipping"; exit 0 ;;
esac

# THE DIFF THIS RECEIPT MUST MATCH.
#
# Bound to the merge-base, not to HEAD~1: a receipt has to cover everything the
# PR proposes, not the last commit someone happened to make. `git diff <base>...`
# (three dots) is the same set GitHub shows in the PR.
if ! git rev-parse --verify "$BASE_REF" >/dev/null 2>&1; then
    # A missing base is UNMEASURED, not clean. Say which state we are in.
    die "base ref '"$BASE_REF"' does not exist -- cannot compute the diff under review.
     Fetch it (git fetch origin main) or set QUORUM_BASE to the right base."
fi

# `git hash-object --stdin`, not `sha256sum`: the latter is GNU coreutils and does
# not exist on macOS, where `set -euo pipefail` would turn its absence into exit
# 127 and block every push for every macOS contributor. git is by definition
# present in a git hook.
merge_base="$(git merge-base HEAD "$BASE_REF")" \
    || die "no merge-base between HEAD and $BASE_REF"

# A base equal to HEAD means the diff is empty by construction -- the shape the
# removed $QUORUM_BASE override exploited. Refuse rather than report clean.
if [ "$merge_base" = "$(git rev-parse HEAD)" ]; then
    die "HEAD is at the merge-base with $BASE_REF -- there is nothing on this branch.
     If you expected changes, you are on the wrong branch or have not committed."
fi

diff_text="$(git diff "$merge_base" HEAD -- . ':(exclude).quorum')"
diff_hash="$(printf '%s' "$diff_text" | git hash-object --stdin)"
[ -n "$diff_hash" ] || die "could not compute a diff hash"

# An empty diff with commits ahead of the base is not "nothing to review" -- it is
# a doc-only or whitespace-only change, or a pathspec that ate everything. Say so.
if [ -z "$diff_text" ]; then
    die "the diff against $BASE_REF is empty, but HEAD is ahead of the merge-base.
     Nothing can be refuted because nothing is visible to the gate. Investigate
     before pushing rather than treating this as clean."
fi

# ONE IMPLEMENTATION OF THE HASH, EXPOSED.
#
# Writing a receipt means putting this hash in it, and the obvious way to get it
# -- `git diff ... | git hash-object --stdin` at a shell -- silently disagrees
# with the line above: command substitution strips the trailing newline, a bare
# pipeline does not, so the two hashes differ and the receipt looks stale for a
# reason nobody can see. Hit while writing the FIRST receipt this gate ever
# checked. Callers ask the gate rather than reimplementing it.
if [ "${PRINT_HASH:-0}" = "1" ]; then
    printf '%s\n' "$diff_hash"
    exit 0
fi

receipt="$RECEIPT_DIR/${branch//\//-}.json"
[ -f "$receipt" ] || die "no quorum receipt at $receipt

  This branch proposes changes that have not survived refutation.
  Run the quorum, then write the receipt. See docs/quorum.md.
  Emergency bypass (recorded in the reflog): git push --no-verify"

# Everything below is one python pass so a malformed receipt fails once, loudly,
# instead of eight times through eight greps.
# The files this branch actually touches, so the falsification test can be
# required to be one of them (see the free-rider defence below).
touched="$(git diff --name-only "$merge_base" HEAD)"

python3 - "$receipt" "$diff_hash" "$MIN_LANES" "$MIN_REFUTERS" "$MIN_JUDGES" "$touched" <<'PY' || exit 1
import json, os, subprocess, sys

path, want_hash, min_lanes, min_refuters, min_judges, touched_raw = sys.argv[1:7]
min_lanes, min_refuters, min_judges = int(min_lanes), int(min_refuters), int(min_judges)
touched = set(filter(None, touched_raw.splitlines()))

def die(msg):
    print(f"✗ QUORUM GATE: {msg}", file=sys.stderr)
    sys.exit(1)

try:
    r = json.load(open(path))
except Exception as e:
    die(f"{path} is not valid JSON: {e}")

for field in ("issue", "diff_sha256", "quorum", "falsification", "crux", "agy_teamwork", "pmat"):
    if field not in r:
        die(f"receipt is missing required field '{field}' -- all four lanes are mandatory\n"
            "  (crux = competitive survey, quorum = adversarial, agy_teamwork, pmat = mechanical)")

# THE BINDING. Without this the whole gate is theater: one receipt would clear
# every future branch, and an amended commit would keep a verdict about code
# that no longer exists.
got = r["diff_sha256"]
if got != want_hash:
    die(f"""receipt is STALE -- it describes a different diff.
     receipt: {got[:16]}...
     tree:    {want_hash[:16]}...
  The code changed after the quorum ran, so its verdict no longer covers it.
  Re-run the quorum against the current diff and rewrite the receipt.""")

q = r["quorum"]
lanes = q.get("lanes", [])
if len(lanes) < min_lanes:
    die(f"quorum had {len(lanes)} evidence lanes, floor is {min_lanes}")
if len(set(lanes)) != len(lanes):
    die("evidence lanes are not distinct -- duplicate lanes are one lane")

refuters = int(q.get("refuters_per_claim", 0))
if refuters < min_refuters:
    die(f"{refuters} refuters per claim, floor is {min_refuters}")

judges = int(q.get("judges", 0))
if judges < min_judges:
    die(f"{judges} judges, floor is {min_judges}")

confirmed = int(q.get("claims_confirmed", 0))
refuted = int(q.get("claims_refuted", 0))
if confirmed + refuted == 0:
    die("the quorum adjudicated 0 claims -- that is not a quorum, it is a formality")

# A QUORUM THAT NEVER KILLS ANYTHING IS NOT REFUTING.
#
# This is the anti-rubber-stamp check and the one most likely to be argued with.
# The memory this gate encodes says every high-value finding came from refutation;
# a panel that confirmed 100% of what it was handed was not adversarial, it was
# an echo. Set `refutation_waived` with a reason if a run genuinely found nothing
# wrong -- it is then visible in the receipt and in review, which is the point.
# THE CLAIMS MUST BE PRESENT AS TEXT, NOT ONLY AS TALLIES.
#
# `claims_confirmed: 43` is a black box: no human can review it, and it is exactly
# as easy to type as `4300`. An outside review made the Goodhart case against a
# bare kill-count -- an agent needing refuted>0 can manufacture a throwaway claim
# to shoot down -- and the answer is not to drop the count but to make what was
# killed legible. A fabricated "the sky is green" is invisible as a number and
# obvious as a sentence.
refuted_texts = q.get("refuted_claims", [])
if refuted and not refuted_texts:
    die("""the receipt reports refuted claims as a NUMBER but not as text.
  Add quorum.refuted_claims: [ ... the actual sentences that were killed ... ].
  A tally cannot be reviewed; a manufactured kill is only visible as prose.""")
if refuted_texts and len(refuted_texts) != refuted:
    die(f"claims_refuted={refuted} but {len(refuted_texts)} refuted_claims listed -- "
        "the tally and the text disagree")

if refuted == 0 and not q.get("refutation_waived"):
    die("""the quorum refuted NOTHING.
  Every claim survived, which usually means the refuters were not adversarial.
  If that is genuinely the result, set quorum.refutation_waived to a reason
  string so the claim is on the record rather than implied by silence.""")

# LANE 1 -- CRUX (competitive survey).
#
# >=3 NAMED systems, matching the bar `provable-iac.md` already sets. This lane is
# what stops a "fix" that is worse than the industry default: on #390, Ansible has
# returned stdout/stderr/rc as separate fields for a decade, and one survey lane
# would have caught the gap years before a misfiled caching bug did.
crux = r["crux"]
systems = crux.get("systems", [])
if len(systems) < 3:
    die(f"CRUX lane surveyed {len(systems)} systems, floor is 3 named systems")
if len(set(systems)) != len(systems):
    die("CRUX systems are not distinct")
if not crux.get("verdict"):
    die("CRUX lane has no verdict -- a survey with no conclusion is not a survey")

# LANE 3 -- agy /teamwork (independent stack).
agy = r["agy_teamwork"]
if not agy.get("ran"):
    die("agy /teamwork lane did not run -- an independent stack must have reviewed this")
if not agy.get("verdict"):
    die("agy /teamwork ran but recorded no verdict")

# LANE 4 -- pmat mcp (mechanical).
#
# `analyze_vacuous_tests` is required by name, not merely "some pmat tool". The whole
# output of a quorum is a claim backed by a test; a test that cannot fail backs
# nothing. This tree already contained tautologies inside files named
# `falsification_*` -- passing, correctly named, and proving nothing.
pmat = r["pmat"]
tools = pmat.get("tools", [])
if "analyze_vacuous_tests" not in tools:
    die("pmat lane did not run analyze_vacuous_tests -- it is required by name.\n"
        "  A quorum's output is a claim backed by a test; a vacuous test backs nothing.")
vac = pmat.get("vacuous_tests_in_touched_paths")
if vac is None:
    die("pmat lane did not report vacuous_tests_in_touched_paths -- UNMEASURED is not a pass")
if int(vac) > 0 and not pmat.get("accepted"):
    die(f"pmat found {vac} vacuous test(s) in the touched paths and none are accepted.\n"
        "  Fix them, or name each in pmat.accepted with a reason. Silence is not a pass.")

# THE RULE WITH TEETH.
#
# "A passing test suite proves the tests pass, not that the fix works." The only
# check that outranks the whole panel is: revert the production hunk and watch
# the test go red for the right reason.
f = r["falsification"]
for field in ("test", "reverted", "observed_failure"):
    if not f.get(field):
        die(f"falsification is missing '{field}' -- the revert-the-hunk check is not optional")

test_file = f.get("test_file", "")
if not test_file or not os.path.exists(test_file):
    die(f"falsification names test_file '{test_file}', which does not exist in the tree.\n"
        "  A falsification against a test that is not here cannot have happened.")

# THE FREE-RIDER DEFENCE.
#
# Requiring only that the named test EXISTS and PASSES was the gate's worst hole,
# and an outside review found it before this shipped: an agent minimising effort
# points `test_file` at any pre-existing always-green test -- `tests/sanity.rs` --
# and the gate happily verifies it is green, then trusts the attestation that it
# went red. The whole falsification becomes free.
#
# A falsification test must be one this branch WROTE OR CHANGED. That is
# mechanically checkable against the diff, costs nothing, and closes the cheap
# path completely: to fake it now you must actually add a test to the diff, which
# is most of the work you were trying to avoid.
if test_file not in touched:
    die(f"""falsification test '{test_file}' is not touched by this branch.
  Every file this branch changes was checked; that test is not among them.
  A falsification must exercise the change under review, so its test has to be
  written or modified HERE. Pointing at a pre-existing green test proves nothing
  -- it is the cheapest way to fake this check, which is why it is blocked.""")

# Verify the half that is cheap to verify: the test passes WITH the fix.
# A receipt citing a test that does not currently pass is rejected outright.
target = f.get("cargo_test_target")
if not target:
    die("falsification is missing 'cargo_test_target' (e.g. the --test name)")

print(f"  verifying falsification test passes with the fix: {target}")
proc = subprocess.run(
    ["cargo", "test", "--test", target, "--quiet"],
    capture_output=True, text=True,
)
if proc.returncode != 0:
    tail = (proc.stdout + proc.stderr).strip().splitlines()[-15:]
    die("the falsification test does NOT pass on this tree:\n     "
        + "\n     ".join(tail))

print(f"✓ quorum receipt valid for {r['issue']}")
print(f"    lanes={len(lanes)} refuters={refuters} judges={judges} "
      f"confirmed={confirmed} refuted={refuted}")
print(f"    falsification: {f['test']}")
print(f"      reverted: {f['reverted']}")
print(f"      observed: {f['observed_failure']}")
PY

echo "✓ quorum gate passed"
