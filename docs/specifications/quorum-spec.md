# Quorum — the pre-PR refutation gate

Status: DRAFT v1 (2026-09-01 — NOT itself quorum-validated; see "Open questions")
Owner: PAIML Engineering
Extends: `provable-iac.md` ("quorum-validated design (≥3 world-class systems)", §Phasing)
Enforced by: `scripts/quorum-gate.sh`, called from `pre-push`

## What quorum is — and is NOT (honest framing)

Quorum is a gate on **claims**, not on merges.

It exists because a branch carries two different things: code, and *the story about
why the code is right*. CI checks the code. Nothing checked the story. #390 is the
worked example — a reporter, a maintainer, and a merged fix (#391) all operated for
days on a story ("forjar replays a cached transcript") that was false. Every test
was green the entire time, because the tests were never the problem.

Quorum is **not** PR-approval-by-vote. It is worth being blunt about this, because
the name invites the wrong reading:

| Quorum IS | Quorum is NOT |
|---|---|
| adversarial validation of claims and designs | N reviewers approving a PR |
| run *before* the branch is pushed | a GitHub branch-protection rule |
| a receipt bound to a specific diff | a standing team policy on merges |
| falsifiable — it can and does reject | advisory |

`main` is **not** branch-protected (`gh api repos/paiml/forjar/branches/main/protection`
→ `404 Branch not protected`) and `CODEOWNERS` routes every path to a single owner.
So there is no downstream reviewer who will catch an unrefuted claim. The push is the
last chokepoint before a PR exists. That is where the gate goes, and why it is a
pre-push hook rather than a CI job: a CI job runs *after* the PR is already open,
which is after the thing this gate is meant to prevent.

## Result vocabulary (never collapse these)

Borrowed deliberately from `provable-iac.md`'s PROVED/CHECKED/UNKNOWN discipline,
because the same failure mode applies — a gate that reports one when it means another
is worse than no gate.

- **VERIFIED** — the gate mechanically confirmed it on this tree, this run.
- **ATTESTED** — a human or agent asserts it; the gate records but cannot confirm it.
- **UNMEASURED** — the gate could not establish it. **This is a failure, not a pass.**

`scripts/cb200-ratchet.sh` records why the third one is spelled out: CB-200 once
reported Skip ("no `.pmat/context.db`") and that green was *the index's absence*
rather than the tree's quality. Every early exit in the quorum gate is therefore a
non-zero exit with a stated reason, never a silent `0`.

## The four mandatory lanes

Every quorum MUST run all four. They are not interchangeable and a missing one is
UNMEASURED, not clean — the gate rejects a receipt that omits any of them.

They are mandatory because each catches a class the others structurally cannot:

| Lane | Question it answers | Blind to |
|---|---|---|
| **CRUX** | what do competing tools do here? | our own bugs |
| **Adversarial** | is this claim actually true? | what we never thought to claim |
| **agy /teamwork** | does an independent stack agree? | things it also cannot see |
| **pmat mcp** | what do the metrics say, mechanically? | intent, design, taste |

### 1. CRUX — competitive survey

What do other tools in this space actually do? Named systems, checked, not recalled
from memory. `provable-iac.md:219` already fixes the form: ≥3 named prior-art systems
per feature.

This lane is what stops us from shipping a "fix" that is worse than the industry
default, and from claiming novelty that is not there. On #390 it is decisive and
cheap: Ansible returns `stdout`/`stderr`/`rc` as separate fields on every module
result, and `kubectl describe` surfaces both streams on `CrashLoopBackOff`. forjar
printed stderr only. That is not a subtle design trade-off — it is a gap against the
field, and one lane of competitive survey would have caught it years earlier than a
misfiled caching bug did.

CRUX is a survey of **behaviour**, not of marketing. Prefer running the other tool, or
reading its source, over reading its docs.

### 2. Adversarial — refutation by lens-diverse majority

The refutation stage described below. Non-negotiable, and the only lane empowered to
*kill* a claim outright.

### 3. agy /teamwork — independent second opinion

An independent agent stack reviews the same artifact. The value is that it does not
share our context, our priors, or our failure modes — the same reason the fan-out
lanes are kept blind to each other, applied one level up.

It must be given the artifact and the established facts, and told to attack rather
than summarize. A `/teamwork` run that returns a summary has not been used.

On #390 this lane independently reached the stdout/stderr root cause and separately
surfaced a second candidate defect (`check_script` not resolving `working_dir`) that
no other lane raised.

### 4. pmat mcp — mechanical review

The metrics lane, run through the pmat MCP tools rather than by eye:

- `analyze_vacuous_tests` — **the highest-value check for this methodology**, because
  a quorum's whole output is a claim backed by a test. A test that cannot fail is a
  claim backed by nothing.
- `quality_gate --strict` — complexity, dead code, SATD, security on the touched paths.
- `analyze_complexity`, `analyze_satd`, `analyze_dead_code` as the change warrants.

Run on 2026-09-01 against this tree it examined 17,796 tests and found ~300 vacuous
ones — including tautologies inside files named `falsification_*`, e.g.
`tests/falsification_overlay_interface_contract.rs:270`, which asserts
`hash_desired_state(&r) == hash_desired_state(&r)`. A falsification test that cannot
falsify is the exact disease this document exists to prevent, and it was already in
the tree, under the right filename, passing.

**Receipt rule:** if this lane flags anything in the paths the branch touches, the
receipt must either fix it or name it in `pmat.accepted` with a reason. Silence is
not a pass.

## The method

Three stages, run across the four lanes above. The shape matters more than the size —
a 3-lane quorum that killed a claim beats a 9-lane one that rubber-stamped.

### 1. Fan out — independent lanes

Split the question into lanes that **cannot see each other's reasoning**, and run
them concurrently. Independence is the whole point: agreement between blind lanes is
evidence, agreement between agents reading each other is an echo.

#390 ran seven: `output-path`, `replay-cache`, `script-gen`, `transport`,
`plan-freshness`, `timing-summary`, `live-repro`. Six read code; one was forbidden to
reason and required to run the real binary. That mix is intentional — see §Rule with
teeth.

### 2. Refute — by lens-diverse majority

Every surviving claim goes to **3 refuters with distinct lenses**, not 3 identical
skeptics. Redundancy catches noise; diversity catches failure modes.

The #390 lenses, as an example worth copying:

- **code-reading** — open every cited `file:line`; a citation that does not exist, or
  says something else, refutes the claim.
- **rival-explanation** — assume the claim is a plausible story that is wrong. What
  else explains the same observations equally well? If a rival fits, nothing is
  established.
- **reporter-facts** — hold the claim against every specific fact in the report. If it
  cannot account for one, it fails.

**Kill rule: any un-countered substantive objection kills the claim.** Not a vote.

This started as "≥2 of 3 refute it" and an outside review was right to reject that,
on two grounds worth recording because both are easy to get wrong again:

1. **Technical truth is not a democracy.** If one refuter produces a proof — a
   citation that does not say what was claimed, a counter-example that runs — a 2–1
   vote does not make the claim true. Majority rule actively discards the single
   most valuable output the panel can produce.
2. **LLM refuters do not fail independently.** Three instances of one model share a
   prior and collapse together, so "2 of 3 agreed" measures correlation, not
   corroboration. Byzantine-fault intuitions do not transfer.

IETF rough consensus is the prior art and it is explicit: consensus is reached by
**addressing substantive objections**, not by counting hands. A claim survives only
when every objection against it has been *answered*, and an objection is answered by
evidence, not by outvoting it.

"Substantive" means: cites a file/line, a command and its output, or a
counter-example. "I am not convinced" is not substantive and does not kill a claim —
otherwise the burden inverts and nothing survives.

Refuters still default to skepticism when unconvinced, so the burden of proof sits on
the claim.

This stage is load-bearing, not ceremonial. #390: **43 claims confirmed, 17 refuted.**
Among the dead was a claim *I* had written and believed — "a task's STDOUT is
invisible in every operator-visible surface" — refuted 3/3 because `forjar logs
--resource` does print it. The true claim was narrower: invisible on every
*apply-time* surface. A quorum that had confirmed the original would have shipped an
overclaim into the changelog and the issue reply.

### 3. Judge — competing designs, scored independently

Generate **≥3 independent designs from different angles**, attack each with a hostile
critic, then score with independent judges. `provable-iac.md` states the prior-art
form of this rule: validate against **≥3 world-class systems**, named per feature.

#390: four proposals (minimal / observability-first / contract / blast-radius), each
critiqued, then scored by 3 judges. Unanimous winner, with named grafts from the
losers folded in — the winner alone would have missed the mirror-image defect in
`verify_against_host`.

## The rule with teeth

Everything above is still opinion until this:

> **Revert the production hunk and watch the test go red for the right reason.**
> A test that still passes with the fix reverted is BLOCKING.

This is the one check that outranks the entire panel. It is not new — it is recorded
in the fleet's own notes after the 1.21.0 sweep, where every high-value finding came
from an agent told to refute, and two regressions were caught that a fully green
suite had passed.

Its companion: **build the attack, do not reason about whether one exists.** An agent
once claimed a plan seal defeated a re-sealing adversary; the verifier wrote that
adversary in ~15 lines against the public API and won.

#390's falsification: `exec_failure` was reverted to the pre-fix stderr-only string;
5 of 6 tests went red, and the pre-fix console output came back **byte-identical to
the text pasted in the issue**. That single result outranks all 43 confirmed claims,
because it is the difference between "three agents agreed" and "the defect is this
one."

One test stayed green under revert — `the_console_still_shows_the_stderr_that_already_worked`
— and that is correct: it guards against "fixing" #390 by swapping which stream gets
destroyed. A receipt should say which tests are expected to survive the revert, so a
reviewer can tell a guard from a dud.

## The receipt

The quorum's output is `.quorum/<branch>.json`, committed on the branch.

**The binding is the whole design.** `diff_sha256` ties the receipt to
`git diff <base>...` — the same change set GitHub shows in the PR. Without it, one
receipt would clear every future branch and an amended commit would keep a verdict
about code that no longer exists.

```json
{
  "issue": "#390 — a failed task's STDOUT never reached the operator",
  "diff_sha256": "<sha256 of git diff origin/main... excluding .quorum>",
  "base": "origin/main",
  "quorum": {
    "lanes": ["output-path", "replay-cache", "..."],
    "refuters_per_claim": 3,
    "kill_rule": "a claim dies if >=2 of 3 refute it",
    "judges": 3,
    "claims_confirmed": 43,
    "claims_refuted": 17,
    "refutation_waived": null
  },
  "crux": {
    "systems": ["Ansible — module result carries stdout/stderr/rc separately", "..."],
    "verdict": "what the field does that we do not, or vice versa"
  },
  "agy_teamwork": {
    "ran": true,
    "verdict": "independent agreement / dissent",
    "unique_findings": ["what only this lane surfaced"]
  },
  "pmat": {
    "tools": ["analyze_vacuous_tests", "quality_gate"],
    "vacuous_tests_in_touched_paths": 0,
    "quality_gate": "A (94.04), 0 blocking violations",
    "accepted": []
  },
  "falsification": {
    "test": "<what was done>",
    "test_file": "tests/falsification_390_....rs",
    "cargo_test_target": "falsification_390_...",
    "reverted": "<the exact production hunk>",
    "observed_failure": "<what went red, and why that is the right reason>",
    "still_green_when_reverted": "<tests expected to survive, and why>"
  }
}
```

### Floors

| Field | Floor | Rationale |
|---|---|---|
| `lanes` | ≥3, distinct | duplicate lanes are one lane |
| `refuters_per_claim` | ≥3 | a majority kill rule needs an odd panel |
| `judges` | ≥3 | matches `provable-iac.md`'s ≥3 systems |
| `claims_confirmed + claims_refuted` | >0 | adjudicating nothing is a formality |
| `claims_refuted` | >0 unless waived | see below |
| `crux.systems` | ≥3, distinct | `provable-iac.md:219` already sets this bar |
| `agy_teamwork.ran` | true | an independent stack must have looked |
| `pmat.tools` | must include `analyze_vacuous_tests` | a claim backed by a test that cannot fail is backed by nothing |
| `pmat.vacuous_tests_in_touched_paths` | 0, or listed in `accepted` | silence is not a pass |

**The anti-rubber-stamp rule, and its Goodhart problem.** A quorum that refutes
*nothing* is rejected, because a panel confirming 100% of what it was handed was
probably not adversarial.

An outside review attacked this hard and correctly: a numeric floor on kills invokes
Goodhart's Law. An agent that needs `claims_refuted > 0` to pass can manufacture a
throwaway claim ("the sky is green") purely to shoot it down. The counter measures
the theatre of refutation rather than the rigour of what survived.

The rule stays, but the count alone is not the signal. Two changes make gaming it
visible rather than free:

- **The receipt carries the claims as TEXT**, not just tallies. `claims_confirmed: 43`
  is a black box no human can review; `refuted_claims: [...]` with the actual
  sentences makes a manufactured kill obvious at a glance, and embarrassing.
- **Divergence is recorded** — whether the blind lanes disagreed *before* synthesis.
  Independent lanes reaching different conclusions is the real evidence of
  adversarial process; unanimity from the first pass is the thing to be suspicious of.

`refutation_waived` remains for the honest case where nothing was wrong. It is a
string on the record, reviewable, rather than a silence.

## What the gate verifies vs. attests

Stating this plainly, because a gate that overclaims is the failure mode this whole
document exists to prevent.

**VERIFIED** (mechanically, every push):
- the receipt exists and parses
- `diff_sha256` matches the tree — the receipt cannot be recycled or outrun by an edit
- lane / refuter / judge floors are met, lanes distinct
- something was refuted, or a waiver is on the record
- the named falsification test **exists** in the tree
- that test **passes right now**, with the fix in place

**ATTESTED** (recorded, not confirmed):
- that the same test went **red** when the fix was reverted

The red half is attested because verifying it means reverting production code and
rebuilding, which a pre-push hook must not do to a developer's working tree. The
green half *is* checked, and a receipt naming a test that does not exist or does not
pass is rejected — which closes the cheapest way to fake a falsification.

The gate does **not** re-run the quorum. That costs millions of tokens and minutes of
wall clock, and would only re-derive what the receipt records.

## Scope and exemptions

- `main` / `master` are exempt: there is no PR to gate, and a release commit has no
  new claims of its own.
- An empty diff against the base is exempt: nothing proposed, nothing to refute.
- A missing base ref is **UNMEASURED → exit 1**, not a pass.
- `.quorum/` is excluded from its own diff hash, or writing the receipt would
  invalidate the receipt.

## Bypass

`git push --no-verify`. It works, it is recorded in the reflog, and it is the right
call for an emergency. It is not the right call for "the quorum is inconvenient
today" — the gate takes seconds; the quorum is the part that costs, and skipping it is
how #390 stayed misdiagnosed across six builds and one merged fix.

## Failure modes this gate does not close

Named rather than hidden, per house discipline:

1. **A fabricated receipt.** An agent can write plausible numbers. Mitigated, not
   solved, by binding to the diff and by requiring a real, passing, named test.
2. **A vacuous falsification.** Reverting a hunk nothing depends on, against a test
   that would fail anyway. The `observed_failure` field must say *why it went red for
   the right reason*; that is reviewable but not machine-checkable.
3. **Lens collapse.** Three "distinct" lenses that ask the same question. Currently
   attested in `lens_diversity`, not enforced.
4. **A local hook enforces nothing on its own.** This is the honest framing, and an
   outside review was right to call the earlier wording an illusion: hooks live in
   untracked `.git/hooks/`, `git push --no-verify` always works, and with `main`
   unprotected a contributor can bypass the PR entirely. The tracked hook plus
   `make install-hooks` buys *fast local feedback*, not enforcement.

   **Enforcement requires both halves**, and shipping only the first is the mistake:
   (a) CI runs `scripts/quorum-gate.sh` on every PR, and (b) `main` gets branch
   protection with that check required. Until (b) exists, this gate is a good habit
   with a receipt, and this document should not claim otherwise.

## Open questions

- **This spec has not itself been through quorum.** Doing so would be the obvious
  first dogfood, and per its own §Judge should be validated against ≥3 prior-art
  systems (candidates: Rust RFC disposition, Kubernetes KEP approvers/reviewers split,
  IETF rough-consensus-and-running-code).
- **Should CI re-verify the receipt?** It would close failure mode 4 at the cost of
  making the gate advisory-until-PR, which is the property this design rejected.
  Probably: enforce locally, mirror in CI as defence in depth.
- **Are the floors right?** 3/3/3 is asserted, not measured. The honest answer is that
  no one has yet run a quorum small enough to fail them.
- **Does the diff binding hold against a real commit?** The stale-receipt path is the
  most important check in the gate and must be proven with a committed change, not a
  working-tree edit — `git diff <base>...` reads committed state, so a working-tree
  tamper is invisible to it *by design* (a pre-push hook only sees what will be
  pushed). Verify before trusting.
