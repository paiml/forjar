# Triage — release 1.26.0 (2026-09-07)

Every open pull request and issue on paiml/forjar at the cut, with one disposition each: **completed** (the merged PR that closed it), **rejected** (with the rationale), or **deferred** (to a named release, with a roadmap ticket). Every quorum finding minted during the release program is listed with its ticket id and the PR that carried the fix. Written by the release orchestrator; the dispositions of open issues were posted on each issue through the triage skill's check-then-write path. Nothing here closes an issue: closes require a ledger quorum, and none was run for this table.

## Pull requests open at the start of the cut

| PR | Title | Disposition |
|---|---|---|
| #463 | ci: timeout-minutes on every job (BSE-001 wave 1, PMAT-158) | closed with five-whys (draft, workflows-only, no receipt; re-cut from the released main under PMAT-158 in 1.27) |
| #474 | ci: bump softprops/action-gh-release 3.0.2 → 3.0.3 (nightly.yml) | closed with five-whys; pin carried by PMAT-190 in 1.27 |
| #475 | ci: bump actions/checkout 4 → 7 (quorum.yml) | closed with five-whys; pin carried by PMAT-190 in 1.27 |
| #476 | feat(dogfood): forjar-dogfood is a release blocker with eight gates (PMAT-163) | merged (quorum receipt `.quorum/PMAT-163-forjar-dogfood.json`); nine merge-review rounds, every finding fixed, none waived |
| #477 | fix(purifier): a path containing 666 or 777 is not a chmod mode (PMAT-204) | merged (`.quorum/PMAT-204-chmod-path-false-positive.json`); found by #476's coverage job, nine adversarial rounds |
| #478 | docs(audits): CRUX audit for 1.26.0 (PMAT-164) | merged (`.quorum/PMAT-164-crux-1.26.0.json`) |
| #481 | spec(state): generation ownership and stack-scoped restore, v10 (PMAT-162) | merged (`.quorum/PMAT-162-generation-ownership-spec.json`); implementation deferred to 1.27 |
| #479 | ci(release): the clean-room gate creates the GitHub prerelease (PMAT-166) | merged (`.quorum/PMAT-166-release-prerelease.json`) |
| #480 | build: publish-from-tag (PMAT-165) | merged (`.quorum/PMAT-165-publish-from-tag.json`) |
| #482 | fix(cb200): the ratchet grades this tree, and three functions come back under the ceiling (PMAT-206) | merged (`.quorum/PMAT-206-cb200-back-under-the-ceiling.json`); opened during the cut when gate B refused it |

The five-whys for each closed PR is the closing comment on that PR; the common root is one rule of the release program: every PR merged into main carries a three-lane quorum receipt, and none of the three had one or was worth the round it would take.

## Issues open at the cut

| Issue | Title (short) | Class | Disposition | Ticket | Release |
|---|---|---|---|---|---|
| #471 | apply --only-machine X -m Y runs nothing and exits 0 | S2 | deferred: the two selectors must resolve through one selection or be refused by name; found by the PMAT-160 teamwork lane and out of that ticket's scope by its own text | PMAT-192 | 1.27 |
| #470 | apply --refresh-only ignores -r / -g / --subset | S2 | deferred: route the refresh through `apply_selection::resolve_selection`; same provenance as #471 | PMAT-191 | 1.27 |
| #462 | Homebrew publishing needs a HOMEBREW_TAP_TOKEN secret | — | deferred: an operator decision; the release program adds no publish secret (§3) | PMAT-200 | 1.27 |
| #456 | build.rs bakes CARGO_MANIFEST_DIR at compile time | S2 | deferred: clean-room builds from a fresh checkout, so no release artifact is affected; the hazard is local target/ reuse | PMAT-194 | 1.27 |
| #452 | forjar-contracts vendored unit tests fail in the workspace | S1 | completed in 1.26.0 by #476 (PMAT-169): the 43 corpus tests are parked behind the `aprender-corpus` feature under an exact ignored-count ratchet; running them with the corpus in a CI lane is the follow-up | PMAT-169 / PMAT-193 | 1.26.0 / 1.27 |
| #445 | cron: a hand-edited entry under a forjar marker is kept and a fresh block appended | S2 | deferred | PMAT-195 | 1.27 |
| #435 | apply --canary-machine skips apply_pre_checks | S2 | deferred: pre-existing since #362's quorum found it; the canary path is not on this release's changed surface | PMAT-196 | 1.27 |
| #434 | Policy rules should declare an explicit, unique id | enhancement | deferred | PMAT-197 | 1.27 |
| #433 | ResourceType printed in its Debug spelling at ~50 CLI sites | S3 | deferred | PMAT-198 | 1.27 |
| #432 | apply --plan-file runs no live-drift probe and the summary does not say so | S2 | deferred | PMAT-199 | 1.27 |
| #417 | E15: typed error taxonomy adopted at 1 of ~1,542 sites | epic | deferred: CRUX-0 stage S6; the CHANGELOG records the deferral | PMAT-152 | 1.27+ |
| #415 | E12: plan/check/--dry-run are lock-relative and cannot consult a host | epic | deferred: CRUX-0 stage S5 | PMAT-150 | 1.27+ |
| #414 | E11: no facts model | epic | deferred, blocked on E12 | PMAT-149 | 1.27+ |
| #413 | E10: cut the CLI surface | epic | deferred: CRUX-0 stage S5; the dogfood surface ledger (gate C) now measures the surface every run | PMAT-148 | 1.27+ |
| #411 | E08: three SSH sessions per converged resource | epic | deferred: CRUX-0 stage S5 | PMAT-146 | 1.27+ |
| #410 | E07: derivation sandbox names binaries that do not exist | epic | deferred: CRUX-0 stage S5 | PMAT-145 | 1.27+ |

Class: S1 blocks a release, S2 ships with a ticket, S3 is cosmetic. The epic rows PMAT-145..154 and the umbrella PMAT-137 were carried onto main's roadmap from the local roadmap branch in the release-cut PR, so every id above resolves in `docs/roadmaps/roadmap.yaml`.

## Filed after the cut

| Issue | Title (short) | Class | Disposition | Ticket | Release |
|---|---|---|---|---|---|
| #485 | a file resource whose content is byte-identical is reported DRIFTED | S1 | deferred: filed by the operator at 16:04Z on 2026-09-08, after the tag, the publish and the fleet pin. It reproduces on three resources across two machines against 1.25.2, and it is what keeps the infra drift tripwire from ever being green. It is not in the 1.26.0 window and a published release cannot absorb it; it is the first ticket of 1.27 | PMAT-209 | 1.27 |

An S1 arriving after the cut does not retroactively make the cut unsound: the row above is outside the window every gate in this release measured. It is recorded here so the board is not read as empty, and it is the first item of the next release, not a loose end of this one.

## Quorum findings minted during the program

| Ticket | Class | Carried by | Status |
|---|---|---|---|
| PMAT-168, 171, 172, 174, 175, 176, 177, 182, 183 | S1/S2 (state stamp, shared state dir) | #473 | completed |
| PMAT-169, 178, 179, 180, 181 | S1/S2/S3 (dogfood gates) | #476 | completed |
| PMAT-170 | S1 (release.yml and binary-release.yml raced on a v* tag) | #479 | completed |
| PMAT-173 | S2 (spec v2 findings) | #481 | completed (absorbed into the spec) |
| PMAT-184, 185, 186, 187 | S1/S2/S3 (publish-from-tag) | #480 | completed |
| PMAT-188 | S3 (crux audit citations) | #478 | completed |
| PMAT-201 | S1 (dogfood gates A and E were not mechanical; found by the #476 merge review, then a vacuous empty-window pass found by the delta quorum) | #476 | completed |
| PMAT-204 | S2 (a file path containing 666 or 777 made apply refuse its own script; five regressions caught and fixed across nine rounds) | #477 | completed |
| PMAT-205 | S3 (the conda store-hash test flaked once under the full library run) | — | deferred to 1.27 |
| PMAT-206 | S1 (the release's own merges took CB-200 from 651 to 654 and gate B refused the cut; the ratchet was also grading a stale cache) | #482 | completed |
| PMAT-207 | S3 (an older implementation receipt carries neither a verdict line nor its END marker) | — | deferred to 1.27 |
| PMAT-158 | CI (timeout-minutes on every job; PR #463's ticket, carried onto the roadmap so the row above resolves) | — | deferred to 1.27 |
| PMAT-189 | S3 (README `forjar make clean` KNOWN_BROKEN in gate D) | — | deferred to 1.27 |
| PMAT-202 | CI (a required check running `pmat comply check`, so CB-2100 can be re-enabled; measured still failing when enabled) | — | deferred to 1.27 |
| PMAT-203 | S2 (the CB-200 ratchet re-based to pmat 3.39's grader: 651 on main, 628 at the August baseline commit under the same grader; bring it under 628) | — | deferred to 1.27 |
| PMAT-162 | spec-first (generation ownership) | #481 (spec) | implementation in 1.27 |

No S1 finding was open at the cut. One was filed four hours after it — #485, above — and is deferred to 1.27 as PMAT-209.

TRIAGE-1.26.0-END
