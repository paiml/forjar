# Quorum evidence — PMAT-163 — refuter rulings

Three refuter lanes (conv-75378465, conv-fe4744ba, conv-b551e150; 420–469 s; per-lane standalone clones) ruled on all 35 dossier ids at 029925b1 with none missing and none invented. Every disposition D1–D7 and every measured claim F1–F4 survived; the four round-1 complaints the fix commits addressed were refuted as stale; three narrowings (the ratchet variable is IGNORED_EXPECTED, docs.sh prints a NOTE rather than passing silently, gate C diffs the ledger rather than regenerating it) and one new observation (the corpus feature is enabled in no CI workflow — D8) were recorded. The unanimous FAIL verdicts are the brief's REFUTED-implies-FAIL rule applied to stale complaints; no lane named a live defect.

## Refuter 1 (verdict FAIL)

L1-F1: SURVIVES
L1-F2: SURVIVES
L1-F3: NARROWED
L1-F4: SURVIVES
L1-F5: SURVIVES
L1-F6: SURVIVES
L2-F1: SURVIVES
L2-F2: SURVIVES
L2-F3: NARROWED
L2-F4: SURVIVES
L2-F5: SURVIVES
L2-F6: SURVIVES
L3-F1: REFUTED
T-F1: REFUTED
T-F2: REFUTED
T-F3: SURVIVES
T-F4: SURVIVES
T-F5: NARROWED
T-F6: REFUTED
T-F7: REFUTED
X-F1: SURVIVES
X-F2: SURVIVES
X-F3: SURVIVES
X-F4: SURVIVES
D1: SURVIVES
D2: SURVIVES
D3: SURVIVES
D4: SURVIVES
D5: SURVIVES
D6: SURVIVES
D7: SURVIVES
F1: SURVIVES
F2: SURVIVES
F3: SURVIVES
F4: SURVIVES

Findings:
- R1-F1 [asserted] scripts/dogfood/coverage.sh:45:45 — L1-F3 (proposed fix: The aprender-corpus tests are gated behind a not(feature = "aprender-corpus") ignore attribute rather than deleted, ensuring they compile on every run, print their ignore reason, and are bounded by the IGNORED_EXPECTED and APRENDER_ANNOTATIONS ratchets.)
- R1-F2 [asserted] scripts/dogfood/coverage.sh:105:105 — L2-F3 (proposed fix: aprender-corpus tests are excluded with a cfg_attr ignore, ensuring they are compiled and reported as ignored (tracked by IGNORED_EXPECTED) rather than being silently deleted or hidden.)
- R1-F3 [cited] contracts/forjar-dogfood-coverage-v1.yaml:202:202 — L3-F1 (proposed fix: FALSIFY-DF-006 maps the rule to exactly_one_skill_claims_the_name at HEAD, meaning the test matches the rule exactly.)
- R1-F4 [asserted] scripts/dogfood/docs.sh:197:197 — T-F1 (proposed fix: Gate theater is prevented because Gate C (surface.sh) regenerates docs/audits/surface_audit.csv from the built artifact and fails the run via diff -u if it was stale, long before docs.sh reads it.)
- R1-F5 [asserted] scripts/dogfood/release-check.sh:203:203 — T-F2 (proposed fix: At HEAD, release-check.sh uses `gh pr list --state merged` instead of `git log --merges` and fails explicitly if an empty PR set occurs over commits that landed.)
- R1-F6 [asserted] scripts/dogfood/docs.sh:256:256 — T-F5 (proposed fix: docs.sh executes blocks, so dead verbs fail execution. However, a dead verb listed in KNOWN_BROKEN bypasses the exit code check and passes, explicitly noting the known breakage in the output (it does not pass silently).)
- R1-F7 [asserted] scripts/dogfood/release-check.sh:114:114 — T-F6 (proposed fix: At HEAD, release-check.sh fails and exits 1 explicitly asking to fetch tags if the tag exists on the remote but is missing locally, instead of passing with PENDING.)
- R1-F8 [asserted] scripts/dogfood/coverage.sh:102:102 — T-F7 (proposed fix: coverage.sh checks if ignored > IGNORED_EXPECTED and also explicitly checks if ignored < IGNORED_EXPECTED to enforce shrinkage. Slack cannot be used to silently ignore new failures without updating the expected value.)

## Refuter 2 (verdict FAIL)

L1-F1: SURVIVES
L1-F2: SURVIVES
L1-F3: SURVIVES
L1-F4: SURVIVES
L1-F5: SURVIVES
L1-F6: SURVIVES
L2-F1: SURVIVES
L2-F2: SURVIVES
L2-F3: SURVIVES
L2-F4: SURVIVES
L2-F5: SURVIVES
L2-F6: SURVIVES
L3-F1: REFUTED
T-F1: REFUTED
T-F2: REFUTED
T-F3: SURVIVES
T-F4: SURVIVES
T-F5: REFUTED
T-F6: REFUTED
T-F7: REFUTED
X-F1: SURVIVES
X-F2: SURVIVES
X-F3: SURVIVES
X-F4: SURVIVES
D1: SURVIVES
D2: SURVIVES
D3: SURVIVES
D4: SURVIVES
D5: NARROWED
D6: SURVIVES
D7: SURVIVES
F1: SURVIVES
F2: SURVIVES
F3: SURVIVES
F4: SURVIVES

Findings:
- R2-F1 [cited] contracts/forjar-dogfood-coverage-v1.yaml:202 — L3-F1
- R2-F2 [cited] Makefile:150 — T-F1
- R2-F3 [cited] scripts/dogfood/release-check.sh:203 — T-F2
- R2-F4 [cited] scripts/dogfood/docs.sh:256 — T-F5
- R2-F5 [cited] scripts/dogfood/release-check.sh:95 — T-F6
- R2-F6 [cited] scripts/dogfood/coverage.sh:105 — T-F7
- R2-F7 [cited] scripts/dogfood/surface.sh:267 — D5 (proposed fix: gate C measures the surface from the built binary earlier in the same make dogfood run and fails if the CSV differs)

## Refuter 3 (verdict FAIL)

L1-F1: SURVIVES
L1-F2: SURVIVES
L1-F3: NARROWED
L1-F4: SURVIVES
L1-F5: SURVIVES
L1-F6: SURVIVES
L2-F1: SURVIVES
L2-F2: SURVIVES
L2-F3: NARROWED
L2-F4: SURVIVES
L2-F5: SURVIVES
L2-F6: SURVIVES
L3-F1: REFUTED
T-F1: REFUTED
T-F2: REFUTED
T-F3: SURVIVES
T-F4: SURVIVES
T-F5: REFUTED
T-F6: REFUTED
T-F7: NARROWED
X-F1: SURVIVES
X-F2: SURVIVES
X-F3: SURVIVES
X-F4: SURVIVES
D1: SURVIVES
D2: SURVIVES
D3: SURVIVES
D4: SURVIVES
D5: SURVIVES
D6: SURVIVES
D7: SURVIVES
F1: SURVIVES
F2: SURVIVES
F3: SURVIVES
F4: SURVIVES

Findings:
- R3-F1 [asserted] scripts/dogfood/coverage.sh:105 — L1-F3 (proposed fix: The aprender-corpus tests are bounded by the exact IGNORED_EXPECTED and APRENDER_ANNOTATIONS ratchets, not IGNORED_CEILING.)
- R3-F2 [asserted] scripts/dogfood/coverage.sh:105 — L2-F3 (proposed fix: The ignored count is tracked by the exact IGNORED_EXPECTED ratchet, not an IGNORED_CEILING.)
- R3-F3 [cited] contracts/forjar-dogfood-coverage-v1.yaml:199 — L3-F1 (proposed fix: FALSIFY-DF-006 correctly cites tests/falsification_dogfood_skill_is_named.rs::exactly_one_skill_claims_the_name.)
- R3-F4 [asserted] scripts/dogfood/surface.sh:267 — T-F1 (proposed fix: docs.sh does read the CSV, but gate C (surface.sh) verifies the CSV against the built artifact in the same run and fails the build if it differs, so the CSV is never stale when docs.sh executes.)
- R3-F5 [asserted] scripts/dogfood/release-check.sh:203 — T-F2 (proposed fix: release-check.sh uses gh pr list --state merged to find PRs, which accurately enumerates squash-merged PRs.)
- R3-F6 [asserted] scripts/dogfood/docs.sh:252 — T-F5 (proposed fix: A dead verb listed in KNOWN_BROKEN does not silently pass; it prints a loud 'NOTE known-broken' and increments the BROKEN count, functioning as an explicit skip.)
- R3-F7 [asserted] scripts/dogfood/release-check.sh:95 — T-F6 (proposed fix: release-check.sh uses git ls-remote --tags origin to explicitly check for remote tags, preventing a missing local tag from masking a missing release.)
- R3-F8 [asserted] scripts/dogfood/coverage.sh:105 — T-F7 (proposed fix: coverage.sh explicitly enforces shrinkage with `if [ \"$ignored\" -lt \"$IGNORED_EXPECTED\" ]`. However, the #452 gating still hides genuinely failing corpus tests because the aprender-corpus feature is never enabled in CI, meaning those tests are unconditionally ignored.)

