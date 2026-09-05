# Quorum evidence — PMAT-159 — lane and refuter summaries

Three claim lanes read the same diff blind, an independent stack reviewed it
separately, three refuters attacked the consolidated dossier, and three judges
adjudicated. Verdicts are reproduced as each lane returned them.

## Claim lane A — transport (verdict PASS)

"Verified the sudo transport replacement uses mktemp and a trap correctly
without breaking stdin or exit codes. The timeout wrapper remains the only
surviving use of /dev/fd/."

Returned five claims with full prose. Its citations pointed at the function
signature rather than at the emitting line, which every refuter and every judge
picked up; the substance survived all three judges.

## Claim lane B — falsifier and tests (verdict PASS)

"The test suite correctly falsifies the defect. The emulation test runs the real
production wrapper under a fake sudo that replicates closefrom, verifying it
handles the missing fd correctly. The live-privilege test executes under real
sudo and properly guards against missing capabilities. The strictness test pins
the operational ordering. The textual assertions in tests_sudo.rs are
strengthened but remain vacuous."

Returned five bare assertions and no supporting prose. Both killed claims came
from this lane, and both were killed on the half of the sentence the lane had
not checked: the live test's guard is real but never demanded, and the
strictness claim described a file the branch had already rewritten.

## Claim lane C — briefed as crux (verdict PASS)

"The change correctly bypasses sudo's closefrom behavior by adopting a temp-file
transport instead of relying on an inherited file descriptor."

Returned ONE claim and NO survey, so it did not discharge the crux role it was
briefed for. Recorded as a lane shortfall rather than quietly re-labelled: the
survey that satisfies the crux lane came from refuter R3 and is in
`sudo-fd3-crux.md`.

## Refuter R1 — verdict FAIL, 7 sustained / 8 refuted

The most aggressive attacker and the only one that was itself caught inventing
evidence: it asserted that `tests_sudo.rs` spawns bash through a
`run_as_transport` helper to execute its assertions. J2 and J3 independently
read the file and found only `.contains()` calls; the helper exists in the two
new integration tests, not there. R1's one durable contribution is the citation
correction, which every judge accepted.

## Refuter R2 — verdict PASS, 16 sustained / 1 weakened

Sustained nearly everything and contributed the finding that killed B3: a
repository-wide search shows `FORJAR_REQUIRE_SUDO_TESTS` is set by no workflow,
Makefile or configuration file, so the live test's fail-closed branch never
engages where the gate runs.

## Refuter R3 — verdict do-not-implement-as-written, 14 sustained / 2 weakened / 1 refuted

Carried the competitive survey as well as the attack. Its four hardening
objections — attacker-controlled TMPDIR, no explicit umask, no sticky-bit check,
no cleanup on SIGKILL — were each ruled non-blocking by all three judges. See
`sudo-fd3-crux.md`.

## Judges

J1 PASS (7 confirmed, 10 narrowed, 0 refuted); J2 do-not-implement-as-written
(4 confirmed, 7 narrowed, 6 refuted); J3 do-not-implement-as-written (9
confirmed, 6 narrowed, 2 refuted). The two judge verdicts of
do-not-implement-as-written are about the CLAIM TEXT, not about the fix: both
scored "does the fix close the defect" 5 of 5. Majority over the seventeen
claims: 15 confirmed, 2 refuted.

J2's own closing line says "4 confirmed, 8 narrowed, 5 refuted", which does not
add to seventeen and disagrees with its own ruling table. The table is what was
counted here (4 confirmed, 7 narrowed, 6 refuted); a judge's summary sentence is
not evidence about its rulings when the rulings are printed next to it.
