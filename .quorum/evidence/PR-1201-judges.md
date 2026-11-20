# PR-1201 judge 1 (agy gemini-4-neon-high), verbatim
1.1	SURVIVED	docs/roadmaps/roadmap.yaml:8076-8129 `- id: PMAT-1176`, `- id: PMAT-1177`, `- id: PMAT-1178` added alongside 6 receipts
1.2	SURVIVED	PR.stat:26 `25 files changed, 5542 insertions(+)` and ANGLE.diff:5633 `diff --git a/docs/roadmaps/roadmap.yaml` (+54/-0, no source code)
1.3	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:42-44,65 `"blob": "f1c12849..."`, `"sha256": "6ffaaa7e..."`, `"bytes": 5601`, `"total_bytes": 16081` match committed judge files
1.4	SURVIVED	PR_BODY.md:10,42 `Three commits, one per ticket, each with its Pmat-Ticket trailer.` unverifiable without `.git` in `./tree`
1.5	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:7 `"merge_commit": "65cf98d30675c46a13ea7133937e3953126cce42"` and docs/roadmaps/roadmap.yaml:8079,8097,8115 match PR_BODY.md:12-14
1.6	SURVIVED	PR_BODY.md:3-5 `Closes #1176`, `Closes #1177`, `Closes #1178`; 0 roadmap rows for 1169/1170/1171 in docs/roadmaps/roadmap.yaml:1-8129
1.7	SURVIVED	scripts/dogfood/quorum.sh:88 `receipt=".quorum/${DOGFOOD_SLUG}.json"` and scripts/dogfood/release-check.sh:390 `path=".quorum/${slug}.json"`
1.8	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:4-5 `"reviewed_after_merge": true`, `"reviewed_at": "2026-11-18T09:54:34Z"` self-reported; pre-merge history unverifiable in `./tree`
1.9	SURVIVED	.quorum/fix-audit-chrono-0543.json:8-14 `"author_model": "Claude Haiku 5.5, Claude Sonnet 5.5"`, `"degraded": "same-family"` matches all 6 receipts
1.10	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:26,67 `"refuters_per_claim": 3`, `"not_committed": "lane-1..3.md, refuter-1..3.json..."`
1.11	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:27-29 `"claims_total": 33`, `"claims_refuted": 2`; all 6 receipts match >=2-of-3 judge counts
1.12	SURVIVED	.quorum/feat-894-pool-interruption-orphans.json:19-21 `lane 1 ...: 20 claims`, `lane 2 ...: 14 claims`, `lane 3 ...: 15 claims`; all 18 judge files match
1.13	SURVIVED	scripts/dogfood/lib/receipt.sh:17 `any(test("waiv|override"))`; 0 matches across all 24 committed `.quorum/` files
1.14	SURVIVED	.quorum/evidence/PR-1126-judge-1.json:1-200 (`PR-1126-judge-{1,2,3}.json`): 33 claims, 2 refuted >=2/3, judge REFUTED votes 1/2/2
1.15	SURVIVED	.quorum/evidence/PR-1120-judge-1.json:1-242 (`PR-1120-judge-{1,2,3}.json`): 40 claims, 1 refuted >=2/3, judge REFUTED votes 0/1/1
1.16	SURVIVED	.quorum/evidence/PR-1101-judge-1.json:1-272 (`PR-1101-judge-{1,2,3}.json`): 45 claims, 2 refuted >=2/3, judge REFUTED votes 2/2/2
1.17	SURVIVED	.quorum/evidence/PR-1140-judge-1.json:1-296 (`PR-1140-judge-{1,2,3}.json`): 49 claims, 1 refuted >=2/3, judge REFUTED votes 1/1/2
1.18	SURVIVED	.quorum/evidence/PR-1145-judge-1.json:1-266 (`PR-1145-judge-{1,2,3}.json`): 44 claims, 3 refuted >=2/3, judge REFUTED votes 3/3/3
1.19	SURVIVED	.quorum/evidence/PR-1156-judge-1.json:1-416 (`PR-1156-judge-{1,2,3}.json`): 69 claims, 1 refuted >=2/3, judge REFUTED votes 1/1/1
1.20	SURVIVED	PR_BODY.md:33 `#1176 has 73 claims and 3 refuted; #1177 has 94 and 3; #1178 has 113 and 4.` matches sums across all 6 receipts
1.21	SURVIVED	.quorum/evidence/PR-1101-judge-1.json:5-6 `"verdict": "SURVIVED"`, `"reason": "no refuter refuted; cited line not checked"` (811 SURVIVED, 798/811 start with `no refuter refuted`)
1.22	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:1-69 and all 5 other receipts carry no `"verdict"` key
1.23	SURVIVED	scripts/dogfood/lib/receipt.sh:33-41 `dogfood_receipt_status()` prints `ok` for all 6 committed receipts
1.24	SURVIVED	PR_BODY.md:39 `Receipt and judge files are byte-identical to the earlier review branch` unverifiable without earlier branch in `./tree`
1.25	SURVIVED	.quorum/feat-894-pool-interruption-orphans.json:3 `"issue": "PR #1140: ... (#1115)"`; 0 `PMAT-*` or backfill ticket ids in the 24 `.quorum/` files
1.26	SURVIVED	docs/roadmaps/roadmap.yaml:8076-8129 parses cleanly with `yq` with 428 rows and 0 duplicate ids
1.27	SURVIVED	.quorum/fix-audit-chrono-0543.json:31 `judges REFUTED 3 of 3:` concatenates 5 ` || `-delimited segments; segments 4 and 5 are absent from PR-1156-judge-{1,2,3}.json:204
1.28	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:28 `"claims_confirmed": 31` (270 total across 6 receipts despite 729/811 `"cited line not checked"` and PR_BODY.md:33 `"not refuted" is not "confirmed correct"`)
2.1	SURVIVED	PR.stat:26 `25 files changed, 5542 insertions(+)` and ANGLE.diff:5633 `diff --git a/docs/roadmaps/roadmap.yaml` (+54/-0, 0 deletions, no source code)
2.2	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:37-65 and all 18 judge files match byte-for-byte on `sha256`, git `blob` SHA-1, `bytes`, and `total_bytes`
2.3	SURVIVED	PR_BODY.md:10,42 `Three commits, one per ticket` unverifiable without `.git` in `./tree` or commit headers in `ANGLE.diff`
2.4	SURVIVED	docs/roadmaps/roadmap.yaml:8076-8129 (`PMAT-1176`/`1177`/`1178`) and line 7 `"merge_commit"` of all 6 receipts match PR_BODY.md:12-14
2.5	SURVIVED	docs/roadmaps/roadmap.yaml:1-8129 has 0 rows for 1169/1170/1171 and PR_BODY.md:3-5 only has `Closes #1176`, `#1177`, `#1178`
2.6	SURVIVED	scripts/dogfood/quorum.sh:73,88,100 and scripts/dogfood/lib/receipt.sh:14-41 read `.quorum/<slug>.json` and print `ok` for all 6 receipts
2.7	SURVIVED	.quorum/fix-audit-chrono-0543.json:4-5,8-14 `"reviewed_after_merge": true`, `"reviewed_at": "2026-11-18T14:24:28Z"`, `"degraded": "same-family"` across all 6 receipts
2.8	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:26,67 `"refuters_per_claim": 3`, `"not_committed": "lane-1..3.md, refuter-1..3.json..."` across all 6 receipts
2.9	SURVIVED	.quorum/evidence/PR-1126-judge-1.json:1-200 and all 18 judge files match every table cell, lane count, and ticket sum in PR_BODY.md:24-33
2.10	SURVIVED	scripts/dogfood/lib/receipt.sh:17 `any(test("waiv|override"))`; 0 matches for `waiv|overrid` across all 24 `.quorum/` files
2.11	SURVIVED	.quorum/evidence/PR-1156-judge-3.json:372 `"no refuter refuted; cited line checked against refuter evidence, not contradicted"`; 811 SURVIVED total and 0 `verdict` keys in receipts
2.12	SURVIVED	PR_BODY.md:39 `byte-identical to the earlier review branch` unverifiable without that branch; 0 `PMAT-*` or batch ticket ids in the 24 `.quorum/` files
2.13	SURVIVED	docs/roadmaps/roadmap.yaml:1-8129 parses with `yq` (428 rows, all 428 `id` values unique)
2.14	SURVIVED	.quorum/fix-audit-chrono-0543.json:24,31 `"judges": 3` has 5 ` || `-separated segments under `judges REFUTED 3 of 3:` with segments 4 and 5 absent from PR-1156-judge-{1,2,3}.json:204
2.15	SURVIVED	.quorum/refactor-cloud-instance-parse-validate.json:28-29,65 (`claims_refuted: 1`, `claims_digest` points to PR-1120-judge-1.json:113 `SURVIVED`, 0 REFUTED) and fix-cloud-instance-validate-resolved.json:28-29,66 (2 vs 1)
2.16	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:32 and .quorum/refactor-cloud-instance-parse-validate.json:31 put Judge 1's dissenting `SURVIVED` text (PR-1126-judge-1.json:125, PR-1120-judge-1.json:113) first under `judges REFUTED 2 of 3:`
2.17	SURVIVED	.quorum/evidence/PR-1156-judge-1.json:378 `Refuters split (r3 REFUTED, r1/r2 SURVIVED)` contradicts PR-1156-judge-2.json:378 `Refuter 2 REFUTED, 1 and 3 SURVIVED` and PR-1156-judge-3.json:378 `Refuter 2 REFUTED`
2.18	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:28,66 `"claims_confirmed": 31`, `"claims_digest": ".quorum/evidence/PR-1126-judge-1.json"` and scripts/quorum_evidence.py:186-189,267-276 (`receipt.sh:14-31` never reads file contents or `claims_digest`)
2.19	SURVIVED	docs/roadmaps/roadmap.yaml:8080,8098,8116 `status: planned` and :8090-8091 `- kind:code` (no `release:<next.tag>`) fail scripts/dogfood/tagged.sh:461-467,481-495 on merge; scripts/quorum-gate.sh:486-488 classifies `.quorum/` and `roadmap.yaml` as triage
2.20	SURVIVED	scripts/dogfood/harness.sh:98-105,214-217 `receipt="docs/audits/impl-${id}-receipt.md"`; no `impl-PMAT-1176/1177/1178-receipt.md` exists in `docs/audits/`
3.1	SURVIVED	PR.stat:26 `25 files changed, 5542 insertions(+)` and docs/roadmaps/roadmap.yaml:8076-8129 `- id: PMAT-1176` (0 deletions in ANGLE.diff, no source code)
3.2	SURVIVED	PR_BODY.md:10,42 `Three commits, one per ticket` unverifiable without `.git` in `./tree` or commit headers in `ANGLE.diff`
3.3	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:7 `"merge_commit": "65cf98d30675c46a13ea7133937e3953126cce42"` and docs/roadmaps/roadmap.yaml:8079,8097,8115 match PR_BODY.md:12-14
3.4	SURVIVED	PR_BODY.md:10,12-14 live GitHub merge SHAs and `headRefName` slugs cannot be verified without git history or GitHub metadata in `./tree`
3.5	SURVIVED	PR_BODY.md:3-5 `Closes #1176`, `Closes #1177`, `Closes #1178` and docs/roadmaps/roadmap.yaml:1-8129 (0 rows for 1169/1170/1171)
3.6	SURVIVED	scripts/dogfood/quorum.sh:87-106 `receipt=".quorum/${DOGFOOD_SLUG}.json"` and scripts/dogfood/release-check.sh:389-405 `path=".quorum/${slug}.json"`
3.7	SURVIVED	.quorum/feat-894-pool-interruption-orphans.json:4-5 `"reviewed_after_merge": true`, `"reviewed_at": "2026-11-18T13:38:45Z"`; self-reported metadata with no run logs or git history in `./tree`
3.8	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:14 `"degraded": "same-family"` holds at line 14 of all 6 committed receipts
3.9	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:26,67 `"refuters_per_claim": 3`, `"not_committed": "lane-1..3.md, refuter-1..3.json..."` across all 6 receipts
3.10	SURVIVED	.quorum/fix-audit-chrono-0543.json:27-29 (`69`/`68`/`1`) and .quorum/evidence/PR-1140-judge-3.json:269 `"verdict": "REFUTED"` match all counts and totals in PR_BODY.md:20,24-33
3.11	SURVIVED	scripts/dogfood/lib/receipt.sh:17 `any(test("waiv|override"))`; 0 matches across all 24 committed `.quorum/` files
3.12	SURVIVED	.quorum/evidence/PR-1101-judge-1.json:11-12 `"verdict": "SURVIVED"`, `"reason": "no refuter refuted; cited line not checked"`; 811/840 SURVIVED, 798/811 start with `"no refuter refuted"`
3.13	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:1-69 and all 5 other receipts carry no `"verdict"` key
3.14	SURVIVED	scripts/dogfood/lib/receipt.sh:33-41 `dogfood_receipt_status()` prints `ok` for 6 of 6 committed receipts
3.15	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:42-44,65 (`sha256`, git `blob` SHA-1, `bytes`, `total_bytes`) match all 18 judge files
3.16	SURVIVED	PR_BODY.md:39 `byte-identical to the earlier review branch` cannot be verified without that earlier branch in `./tree`
3.17	SURVIVED	docs/roadmaps/roadmap.yaml:1-8129 parses cleanly with `yq` (428 rows, 428 unique `id` values)
3.18	SURVIVED	docs/roadmaps/roadmap.yaml:8080,8098,8116 `status: planned` fails scripts/dogfood/tagged.sh:484-495 `fail "${t} ${where} and its roadmap row still reads status: ${DOGFOOD_ROW_STATUS}"` on merge
3.19	SURVIVED	docs/roadmaps/roadmap.yaml:8090-8091,8108-8109,8126-8127 lack `release:<next.tag>`, failing scripts/dogfood/tagged.sh:461-467 on merge
3.20	SURVIVED	docs/roadmaps/roadmap.yaml:8090-8093,8108-8111,8126-8129 label all 3 rows `kind:code` and `epic: 764` despite touching only `.quorum/` and `roadmap.yaml` (`scripts/quorum-gate.sh:486-488`)
3.21	SURVIVED	scripts/dogfood/harness.sh:214-217 `receipt="docs/audits/impl-${id}-receipt.md"` and scripts/dogfood/quorum.sh:87-91 `receipt=".quorum/${DOGFOOD_SLUG}.json"`; neither exists for PR #1201 / PMAT-1176..1178
3.22	SURVIVED	.quorum/fix-audit-chrono-0543.json:24,31,66 `"judges": 3`, 5 ` || `-delimited segments where segments 4 and 5 are absent from PR-1156-judge-{1,2,3}.json:204
3.23	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:32 and .quorum/refactor-cloud-instance-parse-validate.json:31 place Judge 1's dissenting `SURVIVED` text (PR-1126-judge-1.json:125, PR-1120-judge-1.json:113) first under `judges REFUTED 2 of 3:` with no verdict tag
3.24	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:28 `"claims_confirmed": 31` (270 total across 6 receipts despite 0 `CONFIRMED` verdicts and 729/811 `"cited line not checked"`, contradicting PR_BODY.md:33)
3.25	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:29,66 (`claims_refuted: 2` vs PR-1126-judge-1.json having 1 REFUTED), refactor-cloud-instance-parse-validate.json:29,65 (1 vs 0), and scripts/quorum_evidence.py:186-189,259-276 (`roles: ["judges"]` only, no claim prose)
3.26	SURVIVED	.quorum/evidence/PR-1156-judge-1.json:378 `r3 REFUTED, r1/r2 SURVIVED` contradicts PR-1156-judge-2.json:378 `Refuter 2 REFUTED, 1 and 3 SURVIVED` and PR-1156-judge-3.json:378 `Refuter 2 REFUTED`, with `refuter-1..3.json` uncommitted
3.27	REFUTED	docs/specifications/quorum-spec.md:173-199 (`### 2. Refute`: `If one refuter produces a proof ... a 2–1 vote does not make the claim true`) rejects a 2-of-3 refuter vote, not a judge majority; scripts/dogfood/lib/receipt.sh:21-22 (`below the floor of 3: a majority needs three`) and scripts/dogfood/quorum.sh:27 define the 3-judge majority, single refuters killed claims in .quorum/fix-cloud-instance-validate-resolved.json:31 and .quorum/refactor-cloud-instance-parse-validate.json:31, and PR-1140-judge-1.json:270 / judge-2.json:270 explicitly answered the count objection while ruling the core assertion holds
VERDICT: FAIL
text_only: yes
judge: 1
reviewer_model: gemini-4-neon-high
# PR-1201 judge 2 (agy gemini-4-neon-high), verbatim
1.1	SURVIVED	docs/roadmaps/roadmap.yaml:8076-8129 `- id: PMAT-1176`, `- id: PMAT-1177`, `- id: PMAT-1178` added alongside 6 receipts
1.2	SURVIVED	PR.stat:26 `25 files changed, 5542 insertions(+)` and ANGLE.diff:5633 `diff --git a/docs/roadmaps/roadmap.yaml` (+54/-0, 0 deletions, no source code)
1.3	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:42-44,65 `"blob": "f1c12849..."`, `"sha256": "6ffaaa7e..."`, `"bytes": 5601`, `"total_bytes": 16081` (all 18 judge files match)
1.4	SURVIVED	PR_BODY.md:10,42 `Three commits, one per ticket, each with its Pmat-Ticket trailer.`; no `.git` in `./tree` and no commit headers in `ANGLE.diff`
1.5	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:7 `"merge_commit": "65cf98d30675c46a13ea7133937e3953126cce42"` and docs/roadmaps/roadmap.yaml:8079,8097,8115 match PR_BODY.md:12-14
1.6	SURVIVED	PR_BODY.md:3-5 `Closes #1176`, `Closes #1177`, `Closes #1178` and docs/roadmaps/roadmap.yaml:8076-8129 (0 rows for 1169/1170/1171)
1.7	SURVIVED	scripts/dogfood/quorum.sh:88 `receipt=".quorum/${DOGFOOD_SLUG}.json"` and scripts/dogfood/release-check.sh:390 `path=".quorum/${slug}.json"`
1.8	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:4-5 `"reviewed_after_merge": true`, `"reviewed_at": "2026-11-18T09:54:34Z"`; pre-merge history unverifiable without `.git`
1.9	SURVIVED	.quorum/fix-audit-chrono-0543.json:8-14 `"author_model": "Claude Haiku 5.5, Claude Sonnet 5.5"`, `"degraded": "same-family"` across all 6 receipts
1.10	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:26,67 `"refuters_per_claim": 3`, `"not_committed": "lane-1..3.md, refuter-1..3.json..."`
1.11	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:27-29 `"claims_total": 33`, `"claims_refuted": 2`; all 6 receipts match >=2-of-3 judge counts
1.12	SURVIVED	.quorum/feat-894-pool-interruption-orphans.json:19-21 `lane 1 ...: 20 claims`, `lane 2 ...: 14 claims`, `lane 3 ...: 15 claims`; all 18 judge files match
1.13	SURVIVED	scripts/dogfood/lib/receipt.sh:17 `any(test("waiv|override"))`; 0 matches across all 24 committed `.quorum/` files
1.14	SURVIVED	.quorum/evidence/PR-1126-judge-1.json:1-200 (and judge-2/3): 33 claims, 2 refuted (L1C12 3/3, L2C9 2/3), per-judge REFUTED 1/2/2
1.15	SURVIVED	.quorum/evidence/PR-1120-judge-1.json:1-242 (and judge-2/3): 40 claims, 1 refuted (L2C6 2/3), per-judge REFUTED 0/1/1
1.16	SURVIVED	.quorum/evidence/PR-1101-judge-1.json:1-272 (and judge-2/3): 45 claims, 2 refuted (L1C3, L3C3 3/3), per-judge REFUTED 2/2/2
1.17	SURVIVED	.quorum/evidence/PR-1140-judge-1.json:1-296 (and judge-2/3): 49 claims, 1 refuted (L3C14 3/3, L3C11 1/3), per-judge REFUTED 1/1/2
1.18	SURVIVED	.quorum/evidence/PR-1145-judge-1.json:1-266 (and judge-2/3): 44 claims, 3 refuted (L1C8, L1C16, L3C9 3/3), per-judge REFUTED 3/3/3
1.19	SURVIVED	.quorum/evidence/PR-1156-judge-1.json:1-416 (and judge-2/3): 69 claims, 1 refuted (L2C11 3/3), per-judge REFUTED 1/1/1
1.20	SURVIVED	PR_BODY.md:33 `#1176 has 73 claims and 3 refuted; #1177 has 94 and 3; #1178 has 113 and 4.` matches sums 33+40/2+1, 45+49/2+1, 44+69/3+1
1.21	SURVIVED	.quorum/evidence/PR-1101-judge-1.json:5-6 `"verdict": "SURVIVED"`, `"reason": "no refuter refuted; cited line not checked"` (811 SURVIVED, 798/811 no-refuter)
1.22	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:1-69 and all 5 other receipts share identical key sets with no `verdict` key
1.23	SURVIVED	scripts/dogfood/lib/receipt.sh:33-41 `dogfood_receipt_status()` prints `ok` for all 6 receipts
1.24	SURVIVED	PR_BODY.md:39 `Receipt and judge files are byte-identical to the earlier review branch`; earlier branch absent from `./tree`
1.25	SURVIVED	.quorum/feat-894-pool-interruption-orphans.json:3 `"issue": "PR #1140: ... (#1115)"`; 0 `PMAT-*` or backfill ticket ids across all 24 `.quorum/` files
1.26	SURVIVED	docs/roadmaps/roadmap.yaml:8076-8129 parses cleanly with `yq` (428 rows, 0 duplicate ids)
1.27	SURVIVED	.quorum/fix-audit-chrono-0543.json:31 `judges REFUTED 3 of 3:` contains 5 ` || `-delimited segments; segments 4 and 5 absent from PR-1156-judge-{1,2,3}.json:204
1.28	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:28 `"claims_confirmed": 31` (270 total across 6 receipts despite 729/811 `"cited line not checked"` and PR_BODY.md:33)
2.1	SURVIVED	PR.stat:26 `25 files changed, 5542 insertions(+)` and ANGLE.diff:5633 `diff --git a/docs/roadmaps/roadmap.yaml` (+54/-0, 0 deletions, no source code)
2.2	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:37-65 and all 5 other receipts match all 18 judge files on `sha256`, git `blob` SHA-1, `bytes`, and `total_bytes`
2.3	SURVIVED	PR_BODY.md:10,42 `Three commits, one per ticket`; `./tree` has no `.git` and `ANGLE.diff` has no commit headers
2.4	SURVIVED	docs/roadmaps/roadmap.yaml:8076-8129 and line 7 `"merge_commit"` of all 6 receipts match PR_BODY.md:12-14 pairings and SHAs
2.5	SURVIVED	PR_BODY.md:3-5 `Closes #1176/#1177/#1178` and docs/roadmaps/roadmap.yaml:1-8129 (0 rows for 1169/1170/1171)
2.6	SURVIVED	scripts/dogfood/quorum.sh:73,88,100, scripts/dogfood/release-check.sh:369,390,400, scripts/dogfood/lib/receipt.sh:14-41 return `ok` for all 6 receipts
2.7	SURVIVED	.quorum/fix-audit-chrono-0543.json:4-5,8-14 `"reviewed_after_merge": true`, `"reviewed_at": "2026-11-18T14:24:28Z"`, `"degraded": "same-family"` across all 6 receipts
2.8	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:26,67 `"refuters_per_claim": 3`, `"not_committed": "lane-1..3.md, refuter-1..3.json..."` across all 6 receipts
2.9	SURVIVED	.quorum/evidence/PR-1126-judge-1.json:1-200 and all 18 judge files match PR_BODY.md:24-33 table rows, per-lane counts, and per-ticket sums
2.10	SURVIVED	scripts/dogfood/lib/receipt.sh:17 `any(test("waiv|override"))`; 0 matches across all 6 receipts and 18 judge files
2.11	SURVIVED	.quorum/evidence/PR-1156-judge-3.json:372 `"no refuter refuted; cited line checked against refuter evidence, not contradicted"` (811/840 SURVIVED; 0 `verdict` keys in receipts)
2.12	SURVIVED	PR_BODY.md:39 byte-identity to earlier review branch unverifiable without that branch; 0 `PMAT-*`/batch ticket ids in `.quorum/` (`.quorum/feat-894-pool-interruption-orphans.json:3` has `(#1115)`)
2.13	SURVIVED	docs/roadmaps/roadmap.yaml:1-8129 parses cleanly with `yq` (428 rows, all 428 `id`s unique)
2.14	SURVIVED	.quorum/fix-audit-chrono-0543.json:24,31 `"judges": 3`, 5 ` || `-delimited segments under `judges REFUTED 3 of 3:` vs 3 in PR-1156-judge-{1,2,3}.json:204
2.15	SURVIVED	.quorum/refactor-cloud-instance-parse-validate.json:28-29,65 (`39`/`1`, `PR-1120-judge-1.json:113` `SURVIVED`, 40/0) and .quorum/fix-cloud-instance-validate-resolved.json:28-29,66 (`31`/`2`, `PR-1126-judge-1.json:125` `SURVIVED`, 32/1)
2.16	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:32 and .quorum/refactor-cloud-instance-parse-validate.json:31 put Judge 1's `SURVIVED` text (PR-1126-judge-1.json:125, PR-1120-judge-1.json:113) first under `judges REFUTED 2 of 3:` with no verdict tag
2.17	SURVIVED	.quorum/evidence/PR-1156-judge-1.json:378 `Refuters split (r3 REFUTED, r1/r2 SURVIVED)` contradicts PR-1156-judge-2.json:378 `Refuter 2 REFUTED, 1 and 3 SURVIVED` and PR-1156-judge-3.json:378 `Refuter 2 REFUTED`
2.18	SURVIVED	.quorum/fix-audit-chrono-0543.json:28,65-66 `"claims_confirmed": 68`, PR_BODY.md:33, scripts/quorum_evidence.py:186-189,267-276, scripts/dogfood/lib/receipt.sh:14-31
2.19	SURVIVED	docs/roadmaps/roadmap.yaml:8080,8098,8116 `status: planned` and `:8090-8091` `- kind:code` (no `release:<next.tag>`) fail scripts/dogfood/tagged.sh:461-467,481-495; scripts/quorum-gate.sh:486-488 puts `.quorum/` and `roadmap.yaml` on triage rail
2.20	SURVIVED	scripts/dogfood/harness.sh:98-105,214-217 `receipt="docs/audits/impl-${id}-receipt.md"`; no `impl-PMAT-1176/1177/1178-receipt.md` exists under `docs/audits/`
3.1	SURVIVED	PR.stat:26 `25 files changed, 5542 insertions(+)` and docs/roadmaps/roadmap.yaml:8076-8129 (24 new `.quorum/` files, 1 modified `roadmap.yaml`, 0 deletions, no source code)
3.2	SURVIVED	PR_BODY.md:10,42 `Three commits, one per ticket`; `./tree` has no `.git` and `ANGLE.diff` has no commit headers
3.3	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:7 `"merge_commit": "65cf98d30675c46a13ea7133937e3953126cce42"` and docs/roadmaps/roadmap.yaml:8079,8097,8115 match PR_BODY.md:12-14
3.4	SURVIVED	PR_BODY.md:10,12-14 `.quorum/<branch>.json` slugs and upstream merge commits cannot be verified without git history or GitHub metadata in `./tree`
3.5	SURVIVED	PR_BODY.md:3-5 `Closes #1176/#1177/#1178` and docs/roadmaps/roadmap.yaml:1-8129 (0 rows for 1169/1170/1171)
3.6	SURVIVED	scripts/dogfood/quorum.sh:87-106 `receipt=".quorum/${DOGFOOD_SLUG}.json"` and scripts/dogfood/release-check.sh:389-405 `path=".quorum/${slug}.json"`
3.7	SURVIVED	.quorum/feat-894-pool-interruption-orphans.json:4-5 `"reviewed_after_merge": true`, `"reviewed_at": "2026-11-18T13:38:45Z"`; self-reported metadata with no run logs or trailers in `./tree`
3.8	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:14 `"degraded": "same-family"` at line 14 of all 6 committed receipts
3.9	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:26,67 `"refuters_per_claim": 3`, `"not_committed": "lane-1..3.md, refuter-1..3.json..."` across all 6 receipts
3.10	SURVIVED	.quorum/fix-audit-chrono-0543.json:27-29 (`69`/`68`/`1`) and .quorum/evidence/PR-1140-judge-3.json:269 `"verdict": "REFUTED"`; all 18 judge files match PR_BODY.md:20,24-33
3.11	SURVIVED	scripts/dogfood/lib/receipt.sh:17 `any(test("waiv|override"))`; 0 matches across all 24 `.quorum/` files
3.12	SURVIVED	.quorum/evidence/PR-1101-judge-1.json:11-12 `"verdict": "SURVIVED"`, `"reason": "no refuter refuted; cited line not checked"` (811/840 SURVIVED, 798/811 no-refuter)
3.13	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:1-69 and all 5 other receipts carry no `"verdict"` key
3.14	SURVIVED	scripts/dogfood/lib/receipt.sh:33-41 `dogfood_receipt_status()` prints `ok` for all 6 receipts
3.15	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:42-44,65 `"blob"`, `"sha256"`, `"bytes"`, `"total_bytes": 16081`; all 18 judge files match
3.16	SURVIVED	PR_BODY.md:39 `byte-identical to the earlier review branch`; earlier review branch absent from `./tree`
3.17	SURVIVED	docs/roadmaps/roadmap.yaml:1-8129 parses cleanly with `yq` (428 rows, 428 unique `id`s)
3.18	SURVIVED	docs/roadmaps/roadmap.yaml:8080,8098,8116 `status: planned` vs scripts/dogfood/tagged.sh:484-495 `fail "${t} ${where} and its roadmap row still reads status: ${DOGFOOD_ROW_STATUS}"`
3.19	SURVIVED	docs/roadmaps/roadmap.yaml:8090-8091,8108-8109,8126-8127 lack `release:<next.tag>`, failing scripts/dogfood/tagged.sh:461-467 (`docs/roadmaps/releases.yaml:33`)
3.20	SURVIVED	docs/roadmaps/roadmap.yaml:8090-8093 `- kind:code` / `epic: 764` (`GH-764` at :6353-6356 is resource convergence) vs `PMAT-601` at :4417-4434 (`kind:triage`) and docs/audits/dogfood-1.34.0-receipt.md:82 (`#990 under epic #696`)
3.21	SURVIVED	scripts/dogfood/harness.sh:214-217 `receipt="docs/audits/impl-${id}-receipt.md"` and scripts/dogfood/quorum.sh:87-91 `receipt=".quorum/${DOGFOOD_SLUG}.json"`; neither exists for PR #1201 / PMAT-1176..1178
3.22	SURVIVED	.quorum/fix-audit-chrono-0543.json:24,31,66 `"judges": 3`, 5 ` || `-delimited segments (2 extra segments absent from PR-1156-judge-{1,2,3}.json:204)
3.23	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:32 and .quorum/refactor-cloud-instance-parse-validate.json:31 place Judge 1's `SURVIVED` text (PR-1126-judge-1.json:125, PR-1120-judge-1.json:113) first under `judges REFUTED 2 of 3:` with no verdict tag
3.24	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:28 `"claims_confirmed": 31` (270 total across 6 receipts despite 0 `CONFIRMED` verdicts and 729/811 `"cited line not checked"`, contradicting PR_BODY.md:33)
3.25	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:29,66 (`claims_refuted: 2` vs `PR-1126-judge-1.json` 1 REFUTED), .quorum/refactor-cloud-instance-parse-validate.json:29,65 (`1` vs `0`), and scripts/quorum_evidence.py:186-189,259-276 `for need in ("claims", "lanes", "judges", "agy")`
3.26	SURVIVED	.quorum/evidence/PR-1156-judge-1.json:378 `Refuters split (r3 REFUTED, r1/r2 SURVIVED)` contradicts PR-1156-judge-2.json:378 `Refuter 2 REFUTED, 1 and 3 SURVIVED` and PR-1156-judge-3.json:378 `Refuter 2 REFUTED`
3.27	REFUTED	docs/specifications/quorum-spec.md:175,188-199 (`### 2. Refute`: `If one refuter produces a proof ... a 2–1 vote does not make the claim true`) rejects ≥2-of-3 refuter voting, not judge majority; scripts/dogfood/lib/receipt.sh:21-22 (`below the floor of 3: a majority needs three`) and scripts/dogfood/quorum.sh:27 mandate a 3-judge majority, single refuters killed claims (.quorum/fix-cloud-instance-validate-resolved.json:31 `Refuter-3 right, refuters 1/2 missed it`; .quorum/refactor-cloud-instance-parse-validate.json:31 `Refuters split (1 REFUTED; 2,3 SURVIVED)`), and .quorum/evidence/PR-1140-judge-1.json:270 / judge-2.json:270 explicitly answered the count slip as non-fatal to the main assertion
VERDICT: FAIL
text_only: yes
judge: 2
reviewer_model: gemini-4-neon-high
# PR-1201 judge 3 (agy gemini-4-neon-high), verbatim
1.1	SURVIVED	docs/roadmaps/roadmap.yaml:8076-8129 `- id: PMAT-1176`, `- id: PMAT-1177`, `- id: PMAT-1178` added alongside the 6 receipts (no refuter refuted)
1.2	SURVIVED	PR.stat:26 `25 files changed, 5542 insertions(+)` and ANGLE.diff:5633-5637 `diff --git a/docs/roadmaps/roadmap.yaml` (+54/-0, 0 deletions; no refuter refuted)
1.3	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:42-44,65 `"blob"`, `"sha256"`, `"bytes": 5601`, `"total_bytes": 16081`; all 18 judge files match (no refuter refuted)
1.4	SURVIVED	PR_BODY.md:10,42 `Three commits, one per ticket, each with its Pmat-Ticket trailer.`; UNVERIFIED holds without `.git` or commit headers (no refuter refuted)
1.5	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:7 `"merge_commit": "65cf98d30675c46a13ea7133937e3953126cce42"` and roadmap.yaml:8079,8097,8115 match PR_BODY.md:12-14 (no refuter refuted)
1.6	SURVIVED	PR_BODY.md:3-5 `Closes #1176`, `Closes #1177`, `Closes #1178`; 0 roadmap rows for 1169/1170/1171 in docs/roadmaps/roadmap.yaml:1-8129 (no refuter refuted)
1.7	SURVIVED	scripts/dogfood/quorum.sh:88 `receipt=".quorum/${DOGFOOD_SLUG}.json"` and scripts/dogfood/release-check.sh:390 `path=".quorum/${slug}.json"` (no refuter refuted)
1.8	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:4-5 `"reviewed_after_merge": true`, `"reviewed_at": "2026-11-18T09:54:34Z"`; pre-merge history UNVERIFIED without `.git` (no refuter refuted)
1.9	SURVIVED	.quorum/fix-audit-chrono-0543.json:8-14 `"author_model": "Claude Haiku 5.5, Claude Sonnet 5.5"`, `"degraded": "same-family"` matches all 6 receipts (no refuter refuted)
1.10	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:26,67 `"refuters_per_claim": 3`, `"not_committed": "lane-1..3.md, refuter-1..3.json..."` (no refuter refuted)
1.11	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:27-29 `"claims_total": 33`, `"claims_refuted": 2`; all 6 receipts match >=2-of-3 judge counts (no refuter refuted)
1.12	SURVIVED	.quorum/feat-894-pool-interruption-orphans.json:19-21 `20 claims`, `14 claims`, `15 claims`; all 6 receipts match per-lane counts in judge files (no refuter refuted)
1.13	SURVIVED	scripts/dogfood/lib/receipt.sh:17 `any(test("waiv|override"))`; 0 matches across all 24 added `.quorum/` files (no refuter refuted)
1.14	SURVIVED	.quorum/evidence/PR-1126-judge-1.json:1-200 and judge-2/3 match `33 | 2 | 1 / 2 / 2` in PR_BODY.md:26 (no refuter refuted)
1.15	SURVIVED	.quorum/evidence/PR-1120-judge-1.json:1-242 and judge-2/3 match `40 | 1 | 0 / 1 / 1` in PR_BODY.md:27 (no refuter refuted)
1.16	SURVIVED	.quorum/evidence/PR-1101-judge-1.json:1-272 and judge-2/3 match `45 | 2 | 2 / 2 / 2` in PR_BODY.md:28 (no refuter refuted)
1.17	SURVIVED	.quorum/evidence/PR-1140-judge-1.json:1-296 and judge-2/3 match `49 | 1 | 1 / 1 / 2` in PR_BODY.md:29 (no refuter refuted)
1.18	SURVIVED	.quorum/evidence/PR-1145-judge-1.json:1-266 and judge-2/3 match `44 | 3 | 3 / 3 / 3` in PR_BODY.md:30 (no refuter refuted)
1.19	SURVIVED	.quorum/evidence/PR-1156-judge-1.json:1-416 and judge-2/3 match `69 | 1 | 1 / 1 / 1` in PR_BODY.md:31 (no refuter refuted)
1.20	SURVIVED	PR_BODY.md:33 `#1176 has 73 claims and 3 refuted; #1177 has 94 and 3; #1178 has 113 and 4.` matches per-ticket sums (no refuter refuted)
1.21	SURVIVED	.quorum/evidence/PR-1101-judge-1.json:5-6 `"verdict": "SURVIVED"`, `"reason": "no refuter refuted; cited line not checked"`; 811/840 SURVIVED, 798/811 no-refuter (no refuter refuted)
1.22	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:1-69 and all 5 other receipts carry no `"verdict"` key (no refuter refuted)
1.23	SURVIVED	scripts/dogfood/lib/receipt.sh:33-41 `dogfood_receipt_status()` prints `ok` for all 6 receipts (no refuter refuted)
1.24	SURVIVED	PR_BODY.md:39 `Receipt and judge files are byte-identical to the earlier review branch`; UNVERIFIED holds without earlier branch in `./tree` (no refuter refuted)
1.25	SURVIVED	.quorum/feat-894-pool-interruption-orphans.json:3 `"issue": "PR #1140: ... (#1115)"`; 0 `PMAT-*` or backfill ticket ids across all 24 `.quorum/` files (no refuter refuted)
1.26	SURVIVED	docs/roadmaps/roadmap.yaml:8076-8129 parses cleanly with `yq` with 428 rows and 0 duplicate ids (no refuter refuted)
1.27	SURVIVED	.quorum/fix-audit-chrono-0543.json:31 `judges REFUTED 3 of 3:` contains 5 ` || `-delimited segments while PR-1156-judge-{1,2,3}.json:204 account for only segments 1-3
1.28	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:28 `"claims_confirmed": 31` (270 total across 6 receipts) despite 729/811 SURVIVED votes recording `"cited line not checked"`, scripts/dogfood/lib/receipt.sh:14-31 not requiring `claims_confirmed`, and PR_BODY.md:33 `"not refuted" is not "confirmed correct"`
2.1	SURVIVED	PR.stat:26 `25 files changed, 5542 insertions(+)` and ANGLE.diff:5633-5635 `diff --git a/docs/roadmaps/roadmap.yaml` (+54/-0, 0 deletions, no source code; no refuter refuted)
2.2	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:37-65 and all 18 judge files match on `sha256`, git `blob`, `bytes`, and `total_bytes` (no refuter refuted)
2.3	SURVIVED	PR_BODY.md:10,42 `Three commits, one per ticket`; UNVERIFIED holds with no `.git` in `./tree` or commit headers in `ANGLE.diff` (no refuter refuted)
2.4	SURVIVED	docs/roadmaps/roadmap.yaml:8076-8129 `id: PMAT-1176/1177/1178` and line 7 `"merge_commit"` of all 6 receipts match PR_BODY.md:12-14 (no refuter refuted)
2.5	SURVIVED	docs/roadmaps/roadmap.yaml:1-8129 (0 rows for 1169/1170/1171) and PR_BODY.md:3-5 `Closes #1176`, `Closes #1177`, `Closes #1178` (no refuter refuted)
2.6	SURVIVED	scripts/dogfood/quorum.sh:73,88,100, scripts/dogfood/release-check.sh:369,390,400, scripts/dogfood/lib/receipt.sh:14-41 read `.quorum/<slug>.json` and print `ok` for all 6 receipts (no refuter refuted)
2.7	SURVIVED	.quorum/fix-audit-chrono-0543.json:4-5,8-14 `"reviewed_after_merge": true`, `"reviewed_at": "2026-11-18T14:24:28Z"`, `"degraded": "same-family"` across all 6 receipts (no refuter refuted)
2.8	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:26,67 `"refuters_per_claim": 3`, `"not_committed": "lane-1..3.md, refuter-1..3.json..."` across all 6 receipts (no refuter refuted)
2.9	SURVIVED	.quorum/evidence/PR-{1126,1120,1101,1140,1145,1156}-judge-{1,2,3}.json and lines 19-29 of all 6 receipts match PR_BODY.md:24-33 (no refuter refuted)
2.10	SURVIVED	.quorum/*.json and .quorum/evidence/PR-*-judge-*.json contain 0 matches for `waiv|overrid` (no refuter refuted)
2.11	SURVIVED	.quorum/evidence/PR-*-judge-*.json total 840 entries (811 SURVIVED, 29 REFUTED; 798 `"no refuter refuted"`) and all 6 receipts have 0 `verdict` keys (no refuter refuted)
2.12	SURVIVED	PR_BODY.md:39 `byte-identical to the earlier review branch` is UNVERIFIED without that branch; 0 `PMAT-*`/batch ids in the 24 `.quorum/` files (no refuter refuted)
2.13	SURVIVED	docs/roadmaps/roadmap.yaml:1-8129 parses cleanly with `yq` (428 rows, all 428 `id`s unique; no refuter refuted)
2.14	SURVIVED	.quorum/fix-audit-chrono-0543.json:24,31 `"judges": 3` has 5 ` || `-delimited segments under `judges REFUTED 3 of 3:`, with segments 4 and 5 absent from PR-1156-judge-{1,2,3}.json:204
2.15	SURVIVED	.quorum/refactor-cloud-instance-parse-validate.json:28-29,65 (`39`/`1`, `claims_digest` points to PR-1120-judge-1.json with 40/0, L2C6 `:113` `SURVIVED`) and .quorum/fix-cloud-instance-validate-resolved.json:28-29,66 (`31`/`2` vs PR-1126-judge-1.json 32/1, L2C9 `:125` `SURVIVED`)
2.16	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:32 and .quorum/refactor-cloud-instance-parse-validate.json:31 put Judge 1's dissenting `SURVIVED` text (PR-1126-judge-1.json:125-126, PR-1120-judge-1.json:113-114) first under `judges REFUTED 2 of 3:` with no per-judge verdict label
2.17	SURVIVED	.quorum/evidence/PR-1156-judge-1.json:378 `Refuters split (r3 REFUTED, r1/r2 SURVIVED)` contradicts PR-1156-judge-2.json:378 `Refuter 2 REFUTED, 1 and 3 SURVIVED` and PR-1156-judge-3.json:378 `Refuter 2 REFUTED`, with `refuter-1..3.json` uncommitted (.quorum/fix-audit-chrono-0543.json:66)
2.18	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:28,66-67 (`claims_confirmed`, `claims_digest`, `not_committed`), scripts/quorum_evidence.py:186-189,267-276 (`for need in ("claims", "lanes", "judges", "agy")`), and scripts/dogfood/lib/receipt.sh:14-31 confirm 270 `claims_confirmed` with vote-only JSON failing `quorum_evidence.py`
2.19	SURVIVED	docs/roadmaps/roadmap.yaml:8080,8098,8116 `status: planned` and `:8090-8091,8108-8109,8126-8127` `- kind:code` (no `release:<next.tag>`) fail scripts/dogfood/tagged.sh:461-467,481-495 on merge, and scripts/quorum-gate.sh:486-488 classifies `.quorum/` and `roadmap.yaml` as triage
2.20	SURVIVED	scripts/dogfood/harness.sh:98-105,214-217 `receipt="docs/audits/impl-${id}-receipt.md"` and scripts/dogfood/lib/window.sh:411-434 resolve `PMAT-1176` first from PR_BODY.md:1, while `docs/audits/impl-PMAT-1176-receipt.md` (and 1177/1178) does not exist
3.1	SURVIVED	PR.stat:26 `25 files changed, 5542 insertions(+)` and docs/roadmaps/roadmap.yaml:8076-8129 `- id: PMAT-1176` (0 deletions, no source code; no refuter refuted)
3.2	SURVIVED	PR_BODY.md:10,42 `Three commits, one per ticket`; UNVERIFIED holds with no `.git` in `./tree` and no commit headers in ANGLE.diff (no refuter refuted)
3.3	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:7 `"merge_commit": "65cf98d30675c46a13ea7133937e3953126cce42"` and docs/roadmaps/roadmap.yaml:8079,8097,8115 match PR_BODY.md:12-14 (no refuter refuted)
3.4	SURVIVED	PR_BODY.md:10,12-14 live upstream merge SHAs and `headRefName` slugs cannot be verified without `.git` or GitHub metadata (no refuter refuted)
3.5	SURVIVED	PR_BODY.md:3-5 `Closes #1176 / Closes #1177 / Closes #1178` and docs/roadmaps/roadmap.yaml:1-8129 (0 rows for 1169/1170/1171; no refuter refuted)
3.6	SURVIVED	scripts/dogfood/quorum.sh:87-106 `receipt=".quorum/${DOGFOOD_SLUG}.json"` and scripts/dogfood/release-check.sh:389-405 `path=".quorum/${slug}.json"` (no refuter refuted)
3.7	SURVIVED	.quorum/feat-894-pool-interruption-orphans.json:4-5 `"reviewed_after_merge": true`, `"reviewed_at": "2026-11-18T13:38:45Z"` self-reported; UNVERIFIED holds without logs/git history (no refuter refuted)
3.8	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:14 `"degraded": "same-family"` holds at line 14 of all 6 receipts (no refuter refuted)
3.9	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:26,67 `"refuters_per_claim": 3` and `"not_committed": "lane-1..3.md, refuter-1..3.json..."` across all 6 receipts (no refuter refuted)
3.10	SURVIVED	.quorum/fix-audit-chrono-0543.json:27-29 (`69`/`68`/`1`) and .quorum/evidence/PR-1140-judge-3.json:269 `"verdict": "REFUTED"` match all counts in PR_BODY.md:20,24-33 (no refuter refuted)
3.11	SURVIVED	scripts/dogfood/lib/receipt.sh:17 `any(test("waiv|override"))`; 0 matches for `waiv|overrid|bypass|skip_?quorum` across all 24 `.quorum/` files (no refuter refuted)
3.12	SURVIVED	.quorum/evidence/PR-1101-judge-1.json:11-12 `"verdict": "SURVIVED"`, `"reason": "no refuter refuted; cited line not checked"`; 811/840 SURVIVED, 798/811 start with `"no refuter refuted"` (no refuter refuted)
3.13	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:1-69 and all 5 other receipts carry no `"verdict"` key (no refuter refuted)
3.14	SURVIVED	scripts/dogfood/lib/receipt.sh:33-41 `dogfood_receipt_status()` prints `ok` for 6 of 6 committed receipts (no refuter refuted)
3.15	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:42-44,65 `"blob"`, `"sha256"`, `"bytes": 5601`, `"total_bytes": 16081`; all 18 judge files match (no refuter refuted)
3.16	SURVIVED	PR_BODY.md:39 `byte-identical to the earlier review branch`; UNVERIFIED holds without earlier branch in `./tree` (no refuter refuted)
3.17	SURVIVED	docs/roadmaps/roadmap.yaml:1-8129 parses cleanly with `yq` (428 rows, 428 unique `id` values; no refuter refuted)
3.18	SURVIVED	docs/roadmaps/roadmap.yaml:8080,8098,8116 `status: planned` vs scripts/dogfood/tagged.sh:484-495 `case "$DOGFOOD_ROW_STATUS" in completed|cancelled) ;; ... *) fail ...` causes Gate T arm T7 to fail on merge
3.19	SURVIVED	docs/roadmaps/roadmap.yaml:8090-8091,8108-8109,8126-8127 carry only `- kind:code` (no `release:<next.tag>`) vs scripts/dogfood/tagged.sh:461-467 `*" release:${NEXT_TAG} "*) ... *) fail ...`, breaking Gate T arm T4 on merge
3.20	SURVIVED	docs/roadmaps/roadmap.yaml:8090-8093,8108-8111,8126-8129 (`- kind:code`, `epic: 764`) vs `:6353-6356` (`GH-764` `BATCH-forjar-resource-converge`), `:4417-4434` (`PMAT-601` `- kind:triage`), docs/audits/dogfood-1.34.0-receipt.md:82 (`#990 under epic #696`), and scripts/quorum-gate.sh:486-488
3.21	SURVIVED	scripts/dogfood/harness.sh:214-217 `receipt="docs/audits/impl-${id}-receipt.md"` and scripts/dogfood/quorum.sh:87-91 `receipt=".quorum/${DOGFOOD_SLUG}.json"`; PR #1201 adds neither `impl-PMAT-1176/1177/1178-receipt.md` nor a `.quorum/` receipt for PR #1201 itself
3.22	SURVIVED	.quorum/fix-audit-chrono-0543.json:24,31,66 `"judges": 3`, 5 ` || `-delimited segments (2 extra segments absent from PR-1156-judge-{1,2,3}.json:204), `"not_committed": "lane-1..3.md, refuter-1..3.json..."`
3.23	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:32 and .quorum/refactor-cloud-instance-parse-validate.json:31 put Judge 1's dissenting `SURVIVED` text (PR-1126-judge-1.json:125-126, PR-1120-judge-1.json:113-114) first under `judges REFUTED 2 of 3:` with no verdict tag
3.24	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:28 `"claims_confirmed": 31` (270 total across all 6 receipts) despite 0 `CONFIRMED` verdicts and 729/811 `"cited line not checked"`, contradicting PR_BODY.md:33 `"not refuted" is not "confirmed correct"`
3.25	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:29,66 (`claims_refuted: 2`, `claims_digest` points to PR-1126-judge-1.json with 1 REFUTED), .quorum/refactor-cloud-instance-parse-validate.json:29,65 (`1` vs 0 in PR-1120-judge-1.json), and scripts/quorum_evidence.py:186-189,259-276 (`for need in ("claims", "lanes", "judges", "agy")`)
3.26	SURVIVED	.quorum/evidence/PR-1156-judge-1.json:378 `Refuters split (r3 REFUTED, r1/r2 SURVIVED)` contradicts PR-1156-judge-2.json:378 `Refuter 2 REFUTED, 1 and 3 SURVIVED` and PR-1156-judge-3.json:378 `Refuter 2 REFUTED`, with `refuter-1..3.json` uncommitted (.quorum/fix-audit-chrono-0543.json:66)
3.27	REFUTED	docs/specifications/quorum-spec.md:173-199 (`### 2. Refute`: `If one refuter produces a proof ... a 2–1 vote does not make the claim true`) rejects ≥2-of-3 majority voting among refuters, not among judges; scripts/dogfood/lib/receipt.sh:21-22 (`below the floor of 3: a majority needs three`) and scripts/dogfood/quorum.sh:27 require a 3-judge majority, single refuters did kill claims in .quorum/fix-cloud-instance-validate-resolved.json:31 (`Refuter-3 right, refuters 1/2 missed it`) and .quorum/refactor-cloud-instance-parse-validate.json:31 (`Refuters split (1 REFUTED; 2,3 SURVIVED)`), and .quorum/evidence/PR-1140-judge-1.json:270 / PR-1140-judge-2.json:270 explicitly answered the count objection while finding the core filter assertion survived
VERDICT: FAIL
text_only: yes
judge: 3
reviewer_model: gemini-4-neon-high
