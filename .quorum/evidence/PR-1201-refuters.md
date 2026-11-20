# PR-1201 refuter 1 (agy gemini-4-neon-high), verbatim
1.1	SURVIVED	docs/roadmaps/roadmap.yaml:8076-8129 `- id: PMAT-1176`, `- id: PMAT-1177`, `- id: PMAT-1178` match PR_BODY.md:10
1.2	SURVIVED	PR.stat:26 `25 files changed, 5542 insertions(+)` and ANGLE.diff:5633-5637 `diff --git a/docs/roadmaps/roadmap.yaml` (+54/-0, 0 deletions total)
1.3	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:42-44,65 `"blob": "f1c12849..."`, `"sha256": "6ffaaa7e..."`, `"bytes": 5601`, `"total_bytes": 16081`; all 18 sha256/blob/bytes and 6 total_bytes match
1.4	SURVIVED	PR_BODY.md:10,42 `Three commits, one per ticket, each with its Pmat-Ticket trailer.`; `./tree/.git` absent and ANGLE.diff has no commit headers
1.5	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:7 `"merge_commit": "65cf98d30675c46a13ea7133937e3953126cce42"`; all 6 SHAs match line 7 of receipts and docs/roadmaps/roadmap.yaml:8079,8097,8115
1.6	SURVIVED	PR_BODY.md:3-5 `Closes #1176`, `Closes #1177`, `Closes #1178`; 0 roadmap rows for 1169/1170/1171 in docs/roadmaps/roadmap.yaml:1-8129
1.7	SURVIVED	scripts/dogfood/quorum.sh:88 `receipt=".quorum/${DOGFOOD_SLUG}.json"` and scripts/dogfood/release-check.sh:390 `path=".quorum/${slug}.json"`
1.8	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:4-5 `"reviewed_after_merge": true`, `"reviewed_at": "2026-11-18T09:54:34Z"`; pre-merge history unverifiable without `.git`
1.9	SURVIVED	.quorum/fix-audit-chrono-0543.json:8-14 `"author_model": "Claude Haiku 5.5, Claude Sonnet 5.5"`, `"degraded": "same-family"`; matches all 6 receipts
1.10	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:26,67 `"refuters_per_claim": 3`, `"not_committed": "lane-1..3.md, refuter-1..3.json..."`
1.11	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:27-29 `"claims_total": 33`, `"claims_refuted": 2`; all 6 receipts match >=2-of-3 judge counts
1.12	SURVIVED	.quorum/feat-894-pool-interruption-orphans.json:19-21 `lane 1 ...: 20 claims`, `lane 2 ...: 14 claims`, `lane 3 ...: 15 claims`; all 18 judge files have identical contiguous 1..N sets
1.13	SURVIVED	scripts/dogfood/lib/receipt.sh:17 `any(test("waiv|override"))`; 0 matches across all 24 committed `.quorum/` files
1.14	SURVIVED	.quorum/evidence/PR-1126-judge-1.json:1-200 33 claims, 2 refuted >=2/3 (L1C12 3/3, L2C9 2/3), judge REFUTED counts 1/2/2
1.15	SURVIVED	.quorum/evidence/PR-1120-judge-1.json:1-242 40 claims, 1 refuted >=2/3 (L2C6 2/3), judge REFUTED counts 0/1/1
1.16	SURVIVED	.quorum/evidence/PR-1101-judge-1.json:1-272 45 claims, 2 refuted >=2/3 (L1C3 3/3, L3C3 3/3), judge REFUTED counts 2/2/2
1.17	SURVIVED	.quorum/evidence/PR-1140-judge-1.json:1-296 49 claims, 1 refuted >=2/3 (L3C14 3/3, L3C11 1/3), judge REFUTED counts 1/1/2
1.18	SURVIVED	.quorum/evidence/PR-1145-judge-1.json:1-266 44 claims, 3 refuted >=2/3 (L1C8, L1C16, L3C9 3/3), judge REFUTED counts 3/3/3
1.19	SURVIVED	.quorum/evidence/PR-1156-judge-1.json:1-416 69 claims, 1 refuted >=2/3 (L2C11 3/3), judge REFUTED counts 1/1/1
1.20	SURVIVED	PR_BODY.md:33 `#1176 has 73 claims and 3 refuted; #1177 has 94 and 3; #1178 has 113 and 4.`; sums 33+40=73/3, 45+49=94/3, 44+69=113/4 match
1.21	SURVIVED	.quorum/evidence/PR-1101-judge-1.json:5-6 `"verdict": "SURVIVED"`, `"reason": "no refuter refuted; cited line not checked"`; 811 SURVIVED + 29 REFUTED = 840, 798/811 start with `no refuter refuted`
1.22	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:1-69 identical top-level and `quorum` keys across all 6 receipts, no `"verdict"` key
1.23	SURVIVED	scripts/dogfood/lib/receipt.sh:33-41 `dogfood_receipt_status()` prints `ok` for all 6 receipts
1.24	SURVIVED	PR_BODY.md:39 `Receipt and judge files are byte-identical to the earlier review branch`; earlier branch absent from `./tree`
1.25	SURVIVED	.quorum/feat-894-pool-interruption-orphans.json:3 `"issue": "PR #1140: ... (#1115)"`; 0 `PMAT-*` or backfill ids (1169-1171, 1176-1178) in all 24 `.quorum/` files
1.26	SURVIVED	docs/roadmaps/roadmap.yaml:8076-8129 `yq` v4.52.2 parses 428 rows with 0 duplicate ids
1.27	SURVIVED	.quorum/fix-audit-chrono-0543.json:31 `judges REFUTED 3 of 3:` has 5 ` || `-delimited segments; segments 4 (321 chars) and 5 (551 chars) absent from PR-1156-judge-{1,2,3}.json:204
1.28	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:28 `"claims_confirmed": 31` (270 total across 6 receipts despite 729/811 SURVIVED votes recording `"cited line not checked"`)
2.1	SURVIVED	PR.stat:26 `25 files changed, 5542 insertions(+)` and ANGLE.diff:5633-5637 `diff --git a/docs/roadmaps/roadmap.yaml` (+54/-0, no source code)
2.2	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:37-65 all 18 sha256, git blob SHA-1, bytes, and 6 total_bytes match committed judge files
2.3	SURVIVED	PR_BODY.md:10,42 `Three commits, one per ticket`; `./tree/.git` absent and ANGLE.diff has no commit headers
2.4	SURVIVED	docs/roadmaps/roadmap.yaml:8076-8129 `PMAT-1176`/`1177`/`1178` titles and line 7 `"merge_commit"` of all 6 receipts match PR_BODY.md:12-14
2.5	SURVIVED	PR_BODY.md:3-5 `Closes #1176`, `Closes #1177`, `Closes #1178`; 0 rows with id/github_issue 1169/1170/1171 in docs/roadmaps/roadmap.yaml:1-8129
2.6	SURVIVED	scripts/dogfood/quorum.sh:73,88,100 and scripts/dogfood/release-check.sh:369,390,400 `dogfood_receipt_status` prints `ok` for all 6 receipts
2.7	SURVIVED	.quorum/fix-audit-chrono-0543.json:4-5,8-14 `"reviewed_after_merge": true`, `"reviewed_at": "2026-11-18T14:24:28Z"`, `"degraded": "same-family"` across all 6 receipts
2.8	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:26,67 `"refuters_per_claim": 3`, `"not_committed": "lane-1..3.md, refuter-1..3.json..."`
2.9	SURVIVED	.quorum/evidence/PR-1126-judge-1.json:1-200 all 18 judge files match PR_BODY.md:24-33 table rows, per-lane counts, and per-ticket sums
2.10	SURVIVED	scripts/dogfood/lib/receipt.sh:17 `any(test("waiv|override"))`; 0 matches for `waiv|overrid` across all 24 `.quorum/` files
2.11	SURVIVED	.quorum/evidence/PR-1156-judge-3.json:372 `"no refuter refuted; cited line checked against refuter evidence, not contradicted"`; 811 SURVIVED (729+67+2+13), 0 `verdict` keys in receipts
2.12	SURVIVED	PR_BODY.md:39 `byte-identical to the earlier review branch` unverifiable; .quorum/feat-894-pool-interruption-orphans.json:3 has `(#1115)` and 0 `PMAT-*`/batch ids
2.13	SURVIVED	docs/roadmaps/roadmap.yaml:1-8129 `yq '.roadmap | length'` prints `428`, all 428 `id` values unique
2.14	SURVIVED	.quorum/fix-audit-chrono-0543.json:24,31 `"judges": 3`, 5 ` || `-separated segments under `judges REFUTED 3 of 3:` vs 3 in PR-1156-judge-{1,2,3}.json:204
2.15	SURVIVED	.quorum/refactor-cloud-instance-parse-validate.json:28-29,65 `"claims_refuted": 1`, `"claims_digest": ".quorum/evidence/PR-1120-judge-1.json"` vs 0 REFUTED in PR-1120-judge-1.json:111-114 (and 1 vs 2 on #1126)
2.16	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:32 `judges REFUTED 2 of 3: Refuters split... The defect itself holds` puts dissenting SURVIVED judge-1 (PR-1126-judge-1.json:125) first, same on PR-1120-judge-1.json:113
2.17	SURVIVED	.quorum/evidence/PR-1156-judge-1.json:378 `Refuters split (r3 REFUTED, r1/r2 SURVIVED)` vs PR-1156-judge-2.json:378 `Refuter 2 REFUTED, 1 and 3 SURVIVED` and PR-1156-judge-3.json:378 `Refuter 2 REFUTED`
2.18	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:28,66 `"claims_confirmed": 31`, `"claims_digest": ".quorum/evidence/PR-1126-judge-1.json"` and scripts/quorum_evidence.py:186-189,267-276 (fails role/markdown checks; receipt.sh:27 checks only `len(.evidence.files) < 1`, never file contents or `claims_digest`)
2.19	SURVIVED	docs/roadmaps/roadmap.yaml:8080,8098,8116 `status: planned`, :8090-8091 `- kind:code` (no `release:<tag>`), scripts/dogfood/tagged.sh:461-467,481-495, scripts/quorum-gate.sh:486-488
2.20	SURVIVED	scripts/dogfood/harness.sh:98-105,214-217 `receipt="docs/audits/impl-${id}-receipt.md"`; no `impl-PMAT-1176/1177/1178-receipt.md` in `docs/audits/`
3.1	SURVIVED	PR.stat:26 `25 files changed, 5542 insertions(+)` and docs/roadmaps/roadmap.yaml:8076-8129 `- id: PMAT-1176` (0 deletions in ANGLE.diff)
3.2	SURVIVED	PR_BODY.md:10,42 `Three commits, one per ticket`; `./tree/.git` absent and ANGLE.diff has no commit headers
3.3	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:7 `"merge_commit": "65cf98d30675c46a13ea7133937e3953126cce42"` and docs/roadmaps/roadmap.yaml:8079,8097,8115 match PR_BODY.md:12-14
3.4	SURVIVED	PR_BODY.md:10,12-14 `.quorum/<branch>.json` and merge SHAs cannot be verified against upstream GitHub/git history without `.git`
3.5	SURVIVED	PR_BODY.md:3-5 `Closes #1176`, `Closes #1177`, `Closes #1178`; 0 roadmap rows for 1169/1170/1171 in docs/roadmaps/roadmap.yaml:1-8129
3.6	SURVIVED	scripts/dogfood/quorum.sh:88 `receipt=".quorum/${DOGFOOD_SLUG}.json"` and scripts/dogfood/release-check.sh:390 `path=".quorum/${slug}.json"`
3.7	SURVIVED	.quorum/feat-894-pool-interruption-orphans.json:4-5 `"reviewed_after_merge": true`, `"reviewed_at": "2026-11-18T13:38:45Z"`; self-reported with no run logs or git history in `./tree`
3.8	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:14 `"degraded": "same-family"` at line 14 of all 6 receipts
3.9	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:26,67 `"refuters_per_claim": 3`, `"not_committed": "lane-1..3.md, refuter-1..3.json..."`
3.10	SURVIVED	.quorum/fix-audit-chrono-0543.json:27-29 `"claims_total": 69`, `"claims_refuted": 1` and .quorum/evidence/PR-1140-judge-3.json:269 `"verdict": "REFUTED"`; all measured counts match PR_BODY.md:20,24-33
3.11	SURVIVED	scripts/dogfood/lib/receipt.sh:17 `any(test("waiv|override"))`; 0 matches for `waiv|overrid|bypass|skip_?quorum` across all 24 `.quorum/` files
3.12	SURVIVED	.quorum/evidence/PR-1101-judge-1.json:11-12 `"verdict": "SURVIVED"`, `"reason": "no refuter refuted; cited line not checked"`; 811/840 SURVIVED, 798/811 start with `no refuter refuted`
3.13	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:1-69 no `"verdict"` key in any of the 6 receipt JSON files
3.14	SURVIVED	scripts/dogfood/lib/receipt.sh:33-41 `dogfood_receipt_status()` prints `ok` for 6 of 6 receipts
3.15	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:42-44,65 all 18 sha256, git blob SHA-1, bytes, and 6 total_bytes match
3.16	SURVIVED	PR_BODY.md:39 `byte-identical to the earlier review branch`; earlier review branch absent from `./tree`
3.17	SURVIVED	docs/roadmaps/roadmap.yaml:8076-8129 `yq` v4.52.2 parses 428 rows with 428 unique `id`s
3.18	SURVIVED	docs/roadmaps/roadmap.yaml:8080,8098,8116 `status: planned` vs scripts/dogfood/tagged.sh:484-495 `fail "${t} ${where} and its roadmap row still reads status: ${DOGFOOD_ROW_STATUS}"`
3.19	SURVIVED	docs/roadmaps/roadmap.yaml:8090-8091,8108-8109,8126-8127 `- kind:code` (no `release:<tag>`) vs scripts/dogfood/tagged.sh:461-467 `fail "${t} merged since ${NEWEST} and its roadmap row does not carry release:${NEXT_TAG}"`
3.20	SURVIVED	docs/roadmaps/roadmap.yaml:8090-8093,6353-6356,4417-4434 `kind:code`, `epic: 764` (`BATCH-forjar-resource-converge`) vs `PMAT-601` `kind:triage` and docs/audits/dogfood-1.34.0-receipt.md:82 `debt ticket paiml/forjar#990 under epic #696`
3.21	SURVIVED	scripts/dogfood/harness.sh:214-217 `receipt="docs/audits/impl-${id}-receipt.md"` and scripts/dogfood/quorum.sh:87-91 `receipt=".quorum/${DOGFOOD_SLUG}.json"`; neither exists for PR #1201 / PMAT-1176..1178
3.22	SURVIVED	.quorum/fix-audit-chrono-0543.json:24,31 `"judges": 3`, 5 ` || `-delimited segments; 2 extra segments (880 chars / 884 UTF-8 bytes) absent from PR-1156-judge-{1,2,3}.json:204
3.23	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:32 and .quorum/refactor-cloud-instance-parse-validate.json:31 place dissenting SURVIVED judge-1 (PR-1126-judge-1.json:125, PR-1120-judge-1.json:113) first under `judges REFUTED 2 of 3:` with no verdict tag
3.24	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:28 `"claims_confirmed": 31` (270 total across 6 receipts despite 0 `CONFIRMED` verdicts and 729/811 `"cited line not checked"`)
3.25	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:29,66 `"claims_refuted": 2`, `"claims_digest": ".quorum/evidence/PR-1126-judge-1.json"` (1 REFUTED), scripts/quorum_evidence.py:186-189,259-276 `for need in ("claims", "lanes", "judges", "agy")`
3.26	SURVIVED	.quorum/evidence/PR-1156-judge-1.json:378 `r3 REFUTED, r1/r2 SURVIVED` vs PR-1156-judge-2.json:378 `Refuter 2 REFUTED, 1 and 3 SURVIVED` and PR-1156-judge-3.json:378 `Refuter 2 REFUTED`
3.27	REFUTED	docs/specifications/quorum-spec.md:173-199 `### 2. Refute` / `This started as "≥2 of 3 refute it"` rejects a 2-of-3 refuter vote, not a judge majority; scripts/dogfood/lib/receipt.sh:22 specifies `quorum.judges ... below the floor of 3: a majority needs three`, and .quorum/evidence/PR-1140-judge-1.json:270 (`All 3 refuters refuted only the side count... Refuters themselves say the filter point holds`) shows all 3 refuters agreed and Judges 1/2 answered the objection 2-1
VERDICT: FAIL
text_only: yes
refuter: 1
reviewer_model: gemini-4-neon-high
# PR-1201 refuter 2 (agy gemini-4-neon-high), verbatim
1.1	SURVIVED	docs/roadmaps/roadmap.yaml:8076-8129 `- id: PMAT-1176`, `- id: PMAT-1177`, `- id: PMAT-1178` added alongside 6 receipts
1.2	SURVIVED	PR.stat:1-26 `25 files changed, 5542 insertions(+)` and ANGLE.diff:5633-5637 0 deletions, 24 new `.quorum/` files + `roadmap.yaml` +54/-0
1.3	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:42-44,65 `"blob"`, `"sha256"`, `"bytes"`, `"total_bytes": 16081` all match 18/18 committed judge files
1.4	SURVIVED	PR_BODY.md:10,42 `Three commits, one per ticket` cannot be checked; `./tree` has no `.git` and `ANGLE.diff` has no commit headers
1.5	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:7 `"merge_commit": "65cf98d30675c46a13ea7133937e3953126cce42"` and line 7 of all 5 other receipts match PR_BODY.md:12-14 and roadmap.yaml:8079,8097,8115
1.6	SURVIVED	PR_BODY.md:3-5 `Closes #1176/#1177/#1178` and docs/roadmaps/roadmap.yaml:1-8129 contain 0 rows for 1169, 1170, or 1171
1.7	SURVIVED	scripts/dogfood/quorum.sh:88 `receipt=".quorum/${DOGFOOD_SLUG}.json"` and scripts/dogfood/release-check.sh:390 `path=".quorum/${slug}.json"` read these receipts
1.8	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:4-5 `"reviewed_after_merge": true`, `"reviewed_at": "2026-11-18T09:54:34Z"` self-reported; no git/run history in `./tree`
1.9	SURVIVED	.quorum/fix-audit-chrono-0543.json:8-14 `"author_model": "Claude Haiku 5.5, Claude Sonnet 5.5"`, `"lane_models": ["claude-opus-5-5", ...]`, `"degraded": "same-family"` matches all 6 receipts
1.10	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:26,67 `"refuters_per_claim": 3`, `"not_committed": "lane-1..3.md, refuter-1..3.json..."` across all 6 receipts
1.11	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:27-29 `"claims_total": 33`, `"claims_refuted": 2` and lines 27-29 of all 5 other receipts match >=2-of-3 REFUTED across 18 judge files
1.12	SURVIVED	.quorum/feat-894-pool-interruption-orphans.json:19-21 `20/14/15 claims` and lines 19-21 of all 5 other receipts match contiguous 1..N per-lane sets in all 18 judge files
1.13	SURVIVED	scripts/dogfood/lib/receipt.sh:17 `any(test("waiv|override"))` yields 0 matches across all 24 committed `.quorum/` files
1.14	SURVIVED	.quorum/evidence/PR-1126-judge-1.json:1-200 and judge-2/3 match 33 claims, 2 refuted (L1C12 3/3, L2C9 2/3), per-judge REFUTED 1/2/2
1.15	SURVIVED	.quorum/evidence/PR-1120-judge-1.json:1-242 and judge-2/3 match 40 claims, 1 refuted (L2C6 2/3), per-judge REFUTED 0/1/1
1.16	SURVIVED	.quorum/evidence/PR-1101-judge-1.json:1-272 and judge-2/3 match 45 claims, 2 refuted (L1C3, L3C3 3/3), per-judge REFUTED 2/2/2
1.17	SURVIVED	.quorum/evidence/PR-1140-judge-1.json:1-296 and judge-2/3 match 49 claims, 1 refuted (L3C14 3/3; L3C11 1/3 by judge-3), per-judge REFUTED 1/1/2
1.18	SURVIVED	.quorum/evidence/PR-1145-judge-1.json:1-266 and judge-2/3 match 44 claims, 3 refuted (L1C8, L1C16, L3C9 3/3), per-judge REFUTED 3/3/3
1.19	SURVIVED	.quorum/evidence/PR-1156-judge-1.json:1-416 and judge-2/3 match 69 claims, 1 refuted (L2C11 3/3), per-judge REFUTED 1/1/1
1.20	SURVIVED	PR_BODY.md:33 `#1176 has 73 claims and 3 refuted; #1177 has 94 and 3; #1178 has 113 and 4` matches exact sums (33+40/2+1, 45+49/2+1, 44+69/3+1)
1.21	SURVIVED	.quorum/evidence/PR-1101-judge-1.json:5-6 `"verdict": "SURVIVED"`, `"reason": "no refuter refuted; cited line not checked"` — 811 SURVIVED + 29 REFUTED = 840 (798 no-refuter, 729 unchecked, 69 checked, 13 split)
1.22	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:1-69 and all 5 other receipts share identical key sets with no `verdict` key
1.23	SURVIVED	scripts/dogfood/lib/receipt.sh:33-41 `dogfood_receipt_status()` prints `ok` for all 6 committed receipts
1.24	SURVIVED	PR_BODY.md:39 `byte-identical to the earlier review branch` cannot be verified without the earlier branch in `./tree`
1.25	SURVIVED	.quorum/feat-894-pool-interruption-orphans.json:3 `(#1115)` is the only non-PR issue number; 0 `PMAT-*` and 0 `1169/1170/1171/1176/1177/1178` across all 24 `.quorum/` files
1.26	SURVIVED	docs/roadmaps/roadmap.yaml:1-8129 parses cleanly with `yq` (428 rows, 428 unique `id`s)
1.27	SURVIVED	.quorum/fix-audit-chrono-0543.json:31 contains 5 ` || `-delimited segments under `judges REFUTED 3 of 3:`; segs 1-3 match PR-1156-judge-{1,2,3}.json:204 verbatim while segs 4 (321 chars) and 5 (551 chars) are absent from all judge files
1.28	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:28 `"claims_confirmed": 31` (270 total across 6 receipts), PR-1101-judge-1.json:6 `"cited line not checked"` (729/811), receipt.sh:14-31 does not require `claims_confirmed`, PR_BODY.md:33 `"not refuted" is not "confirmed correct"`
2.1	SURVIVED	PR.stat:1-26 `25 files changed, 5542 insertions(+)` and ANGLE.diff:5633-5637 confirm 24 new `.quorum/` files, +54/-0 in `roadmap.yaml`, 0 deletions, no source code changed
2.2	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:37-65 and all 5 other receipts match all 18 judge files on `sha256`, git `blob` SHA-1, `bytes`, and `total_bytes`
2.3	SURVIVED	PR_BODY.md:10,42 `Three commits` / `Pmat-Ticket` trailers unverifiable with no `.git` in `./tree` and no commit headers in `ANGLE.diff`
2.4	SURVIVED	docs/roadmaps/roadmap.yaml:8076-8129 and line 7 (`"merge_commit"`) of all 6 receipts match PR_BODY.md:12-14 pairings and 40-hex SHAs
2.5	SURVIVED	docs/roadmaps/roadmap.yaml:1-8129 has 0 rows for 1169/1170/1171 and PR_BODY.md:3-5 only has `Closes #1176/#1177/#1178`
2.6	SURVIVED	scripts/dogfood/quorum.sh:73,88,100, scripts/dogfood/release-check.sh:369,390,400, scripts/dogfood/lib/receipt.sh:14-41 read `.quorum/<slug>.json` and return `ok` for all 6 receipts
2.7	SURVIVED	.quorum/fix-audit-chrono-0543.json:4-5,8-14,25 and the same lines across all 5 other receipts record 2026-11-18 post-merge Opus 5.5 review and `"degraded": "same-family"`
2.8	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:26,67 `"refuters_per_claim": 3` / `"not_committed": "lane-1..3.md, refuter-1..3.json..."` across all 6 receipts with no lane/refuter files committed
2.9	SURVIVED	.quorum/evidence/PR-{1126,1120,1101,1140,1145,1156}-judge-{1,2,3}.json and lines 19-29 of all 6 receipts match every table cell, lane count, and ticket sum in PR_BODY.md:24-33
2.10	SURVIVED	.quorum/*.json and .quorum/evidence/PR-*-judge-*.json have 0 matches for `waiv|overrid`
2.11	SURVIVED	.quorum/evidence/PR-*-judge-*.json total 840 votes (811 SURVIVED, 29 REFUTED; 798 `"no refuter refuted"`, 729 unchecked, 67 not contradicted, 2 other checked) and 0 `verdict` keys in the 6 receipts
2.12	SURVIVED	PR_BODY.md:39 byte-identity to earlier review branch unverifiable without that branch; 0 `PMAT-*` or batch ticket ids (`1169..1171`, `1176..1178`) in the 24 `.quorum/` files
2.13	SURVIVED	docs/roadmaps/roadmap.yaml:1-8129 parses with `yq` (`428` rows, all 428 `id`s unique)
2.14	SURVIVED	.quorum/fix-audit-chrono-0543.json:24,31 (`"judges": 3`) has 5 ` || `-separated segments where segs 4 and 5 do not exist in PR-1156-judge-{1,2,3}.json:204, unlike the other 9 `refuted_claims` entries
2.15	SURVIVED	.quorum/refactor-cloud-instance-parse-validate.json:28-29,65 (`39/1`, `PR-1120-judge-1.json` has 40/0) and .quorum/fix-cloud-instance-validate-resolved.json:28-29,66 (`31/2`, `PR-1126-judge-1.json` has 32/1) disagree with their `claims_digest` target
2.16	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:32 and .quorum/refactor-cloud-instance-parse-validate.json:31 put Judge 1's `SURVIVED` text (PR-1126-judge-1.json:125, PR-1120-judge-1.json:113) first under `judges REFUTED 2 of 3:` with no verdict tag
2.17	SURVIVED	.quorum/evidence/PR-1156-judge-1.json:378 `r3 REFUTED, r1/r2 SURVIVED` contradicts PR-1156-judge-2.json:378 and PR-1156-judge-3.json:378 `Refuter 2 REFUTED` with `refuter-1..3.json` uncommitted
2.18	SURVIVED	.quorum/fix-audit-chrono-0543.json:28,65-66, PR_BODY.md:33, scripts/quorum_evidence.py:36,186-189,267-276, scripts/dogfood/lib/receipt.sh:14-31 confirm 270 `claims_confirmed` with 0 claim text, 729 unchecked + 67 not-contradicted in PR-1156-judge-3.json, and `receipt.sh` never opening evidence file contents or `claims_digest`
2.19	SURVIVED	docs/roadmaps/roadmap.yaml:8080,8098,8116 `status: planned` and `:8090-8091,8108-8109,8126-8127` `kind:code` (no `release:<next.tag>`) fail scripts/dogfood/tagged.sh:461-467,481-495 on merge; scripts/quorum-gate.sh:486-488 puts `.quorum/` and `roadmap.yaml` on the triage rail
2.20	SURVIVED	scripts/dogfood/harness.sh:98-105,214-217 `receipt="docs/audits/impl-${id}-receipt.md"` and `docs/audits/` lacking `impl-PMAT-1176/1177/1178-receipt.md` cause Gate A to fail once PR #1201 merges
3.1	SURVIVED	PR.stat:26 `25 files changed, 5542 insertions(+)` and docs/roadmaps/roadmap.yaml:8076-8129 confirm 6 receipts, 18 judge files, `roadmap.yaml` +54/-0, 0 deletions, no source code changed
3.2	SURVIVED	PR_BODY.md:10,42 commit count and `Pmat-Ticket` trailers cannot be verified without `.git` in `./tree` or commit headers in `ANGLE.diff`
3.3	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:7 and line 7 of all 5 other receipts plus docs/roadmaps/roadmap.yaml:8079,8097,8115 match PR_BODY.md:12-14 pairings and SHAs
3.4	SURVIVED	PR_BODY.md:10,12-14 live GitHub merge SHAs and `headRefName` slugs cannot be verified without git history or GitHub metadata in `./tree`
3.5	SURVIVED	PR_BODY.md:3-5 `Closes #1176/#1177/#1178` and docs/roadmaps/roadmap.yaml:1-8129 contain no rows for 1169, 1170, or 1171
3.6	SURVIVED	scripts/dogfood/quorum.sh:87-106 and scripts/dogfood/release-check.sh:389-405 read `.quorum/<slug>.json` via `scripts/dogfood/lib/receipt.sh`
3.7	SURVIVED	.quorum/feat-894-pool-interruption-orphans.json:4-5 and .quorum/fix-audit-chrono-0543.json:8 are self-reported metadata with no commit trailers or run logs in `./tree`
3.8	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:14 `"degraded": "same-family"` appears at line 14 of all 6 committed receipts
3.9	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:26,67 `"refuters_per_claim": 3` / `"not_committed": "lane-1..3.md, refuter-1..3.json..."` across all 6 receipts
3.10	SURVIVED	.quorum/fix-audit-chrono-0543.json:27-29 and all 18 judge files match every count, per-lane tally, per-judge REFUTED vote, and per-ticket sum in PR_BODY.md:20,24-33
3.11	SURVIVED	scripts/dogfood/lib/receipt.sh:17 `any(test("waiv|override"))` and regex scan `waiv|overrid|bypass|skip_?quorum` yield 0 hits across all 24 `.quorum/` files
3.12	SURVIVED	.quorum/evidence/PR-1101-judge-1.json:11-12 `"verdict": "SURVIVED"`, `"reason": "no refuter refuted; cited line not checked"` — 811 SURVIVED of 840 total, 798/811 (98.4%) start with `"no refuter refuted"`
3.13	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:1-69 and all 5 other receipts have no `"verdict"` key
3.14	SURVIVED	scripts/dogfood/lib/receipt.sh:33-41 `dogfood_receipt_status()` prints `ok` for 6 of 6 committed receipts
3.15	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:42-44,65 (`bytes` is on :44 rather than :43, minor range slip) and all 5 other receipts match 18/18 `sha256`, git `blob` SHA-1, `bytes`, and `total_bytes`
3.16	SURVIVED	PR_BODY.md:39 byte-identity to earlier review branch cannot be checked without that branch; 0 `PMAT-` or `#1169/#1170/#1171/#1176/#1177/#1178` in the 6 receipts
3.17	SURVIVED	docs/roadmaps/roadmap.yaml:1-8129 parses cleanly with `yq` (428 rows, 428 unique `id`s)
3.18	SURVIVED	docs/roadmaps/roadmap.yaml:8080,8098,8116 `status: planned` fails scripts/dogfood/tagged.sh:484-495 `case "$DOGFOOD_ROW_STATUS" in completed|cancelled)` on merge
3.19	SURVIVED	docs/roadmaps/roadmap.yaml:8090-8091,8108-8109,8126-8127 lack `release:<next.tag>`, failing scripts/dogfood/tagged.sh:461-467 on merge (docs/roadmaps/releases.yaml:33)
3.20	SURVIVED	docs/roadmaps/roadmap.yaml:8090-8093,8108-8111,8126-8129 label all three rows `kind:code` and `epic: 764` (`GH-764` at :6353-6356 is resource convergence, whereas `PMAT-601` at :4417-4434 uses `kind:triage` and `dogfood-1.34.0-receipt.md:82` places `#990` under `epic #696`)
3.21	SURVIVED	PR_BODY.md:1, scripts/dogfood/harness.sh:214-217, scripts/dogfood/quorum.sh:87-91 — PR #1201 adds neither `docs/audits/impl-PMAT-1176/1177/1178-receipt.md` nor a `.quorum/` receipt for PR #1201 itself
3.22	SURVIVED	.quorum/fix-audit-chrono-0543.json:24,31,66 appends 880 extra bytes (two additional `||`-delimited segments, 5 total for 3 judges) absent from PR-1156-judge-{1,2,3}.json:204
3.23	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:32 and .quorum/refactor-cloud-instance-parse-validate.json:31 place Judge 1's `SURVIVED` text (PR-1126-judge-1.json:125-126, PR-1120-judge-1.json:113-114) first under `judges REFUTED 2 of 3:` with no verdict tag
3.24	SURVIVED	.quorum/*.json:28 record 270 `"claims_confirmed"` while the 18 judge files emit 0 `CONFIRMED` verdicts and 729/811 `SURVIVED` votes state `"cited line not checked"`, contradicting PR_BODY.md:33
3.25	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:29,66 and .quorum/refactor-cloud-instance-parse-validate.json:29,65 point `claims_digest` at `judge-1.json` (only `roles: ["judges"]`, no claim text, failing scripts/quorum_evidence.py:186-189,259-276; and judge-1 disagrees with `claims_refuted`: 1 vs 2 on #1126, 0 vs 1 on #1120)
3.26	SURVIVED	.quorum/evidence/PR-1156-judge-1.json:378 `r3 REFUTED` contradicts PR-1156-judge-2.json:378 and PR-1156-judge-3.json:378 `Refuter 2 REFUTED`, unresolvable without uncommitted `refuter-1..3.json`
3.27	REFUTED	docs/specifications/quorum-spec.md:175,188-197 rejects a 2-of-3 refuter vote (`If one refuter produces a proof ... a 2-1 vote does not make the claim true`), not judge majority; receipts allow 1-of-3 refuters to kill (.quorum/fix-cloud-instance-validate-resolved.json:31 `Refuter-3 right, refuters 1/2 missed it`; .quorum/refactor-cloud-instance-parse-validate.json:31 `Refuters split (1 REFUTED; 2,3 SURVIVED)`), while 2-of-3 judge majority is required by scripts/dogfood/lib/receipt.sh:21-22 and scripts/dogfood/quorum.sh:27 (`quorum.judges >= 3 — a majority needs three`), and PR-1140-judge-1.json:270 / judge-2.json:270 explicitly answer why the core assertion survives despite the count slip
VERDICT: FAIL
text_only: yes
refuter: 2
reviewer_model: gemini-4-neon-high
# PR-1201 refuter 3 (agy gemini-4-neon-high), verbatim
1.1	SURVIVED	docs/roadmaps/roadmap.yaml:8076-8129 `- id: PMAT-1176`, `- id: PMAT-1177`, `- id: PMAT-1178` added alongside the 6 `.quorum/` receipts.
1.2	SURVIVED	PR.stat:26 `25 files changed, 5542 insertions(+)` and ANGLE.diff:5633 `diff --git a/docs/roadmaps/roadmap.yaml` (+54/-0, 24 new `.quorum/` files, 0 deletions, no source code).
1.3	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:42-44,65 `"blob": "f1c12849..."`, `"sha256": "6ffaaa7e..."`, `"bytes": 5601`, `"total_bytes": 16081`; all 18 sha256/blob/bytes and 6 total_bytes match committed judge files.
1.4	SURVIVED	PR_BODY.md:10 `Three commits, one per ticket, each with its Pmat-Ticket trailer.`; `./tree` has no `.git` directory and ANGLE.diff is a combined diff, so UNVERIFIED holds.
1.5	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:7 `"merge_commit": "65cf98d30675c46a13ea7133937e3953126cce42"` and line 7 of all 5 other receipts match PR_BODY.md:12-14 and docs/roadmaps/roadmap.yaml:8079,8097,8115.
1.6	SURVIVED	PR_BODY.md:3-5 `Closes #1176`..`#1178` and docs/roadmaps/roadmap.yaml:1-8129; 0 roadmap rows exist for 1169/1170/1171 and none is closed.
1.7	SURVIVED	scripts/dogfood/quorum.sh:88 `receipt=".quorum/${DOGFOOD_SLUG}.json"` and scripts/dogfood/release-check.sh:390,400 `path=".quorum/${slug}.json"`.
1.8	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:4-5 `"reviewed_after_merge": true`, `"reviewed_at": "2026-11-18T09:54:34Z"`; self-reported metadata only, pre-merge history/timing is external so UNVERIFIED holds.
1.9	SURVIVED	.quorum/fix-audit-chrono-0543.json:8-14 `"author_model": "Claude Haiku 5.5, Claude Sonnet 5.5"`, `"degraded": "same-family"` and lines 8-14,25 of all 5 other receipts match PR_BODY.md:20.
1.10	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:26,67 `"refuters_per_claim": 3`, `"not_committed": "lane-1..3.md, refuter-1..3.json..."`; only `-judge-{1,2,3}.json` files are committed.
1.11	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:27-29 `"claims_total": 33`, `"claims_refuted": 2` and all 5 other receipts match the >=2-of-3 REFUTED counts in all 18 judge files.
1.12	SURVIVED	.quorum/feat-894-pool-interruption-orphans.json:19-21 `lane 1 ...: 20 claims`..`lane 3 ...: 15 claims`; all 6 receipts match contiguous `1..N` per-lane counts in their 3 judge files and no lane files are committed.
1.13	SURVIVED	scripts/dogfood/lib/receipt.sh:17 `any(test("waiv|override"))`; 0 matches for `waiv` or `override` across all 24 added `.quorum/` files.
1.14	SURVIVED	.quorum/evidence/PR-1126-judge-1.json:1-200 (`PR-1126-judge-{1,2,3}.json`): 33 claims, L1C12 refuted 3/3, L2C9 refuted 2/3, judge REFUTED votes 1 / 2 / 2.
1.15	SURVIVED	.quorum/evidence/PR-1120-judge-1.json:1-242 (`PR-1120-judge-{1,2,3}.json`): 40 claims, L2C6 refuted 2/3, judge REFUTED votes 0 / 1 / 1.
1.16	SURVIVED	.quorum/evidence/PR-1101-judge-1.json:1-272 (`PR-1101-judge-{1,2,3}.json`): 45 claims, L1C3 and L3C3 refuted 3/3, judge REFUTED votes 2 / 2 / 2.
1.17	SURVIVED	.quorum/evidence/PR-1140-judge-1.json:1-296 (`PR-1140-judge-{1,2,3}.json`): 49 claims, L3C14 refuted 3/3, L3C11 refuted 1/3 by judge-3 only, judge REFUTED votes 1 / 1 / 2.
1.18	SURVIVED	.quorum/evidence/PR-1145-judge-1.json:1-266 (`PR-1145-judge-{1,2,3}.json`): 44 claims, L1C8/L1C16/L3C9 refuted 3/3, judge REFUTED votes 3 / 3 / 3.
1.19	SURVIVED	.quorum/evidence/PR-1156-judge-1.json:1-416 (`PR-1156-judge-{1,2,3}.json`): 69 claims, L2C11 refuted 3/3, judge REFUTED votes 1 / 1 / 1.
1.20	SURVIVED	PR_BODY.md:33 `#1176 has 73 claims and 3 refuted; #1177 has 94 and 3; #1178 has 113 and 4.` matches 33+40/2+1, 45+49/2+1, 44+69/3+1 across the 6 receipts and 18 judge files.
1.21	SURVIVED	.quorum/evidence/PR-1101-judge-1.json:5-6 `"verdict": "SURVIVED"`, `"reason": "no refuter refuted; cited line not checked"`; measured 840 votes = 811 SURVIVED + 29 REFUTED, 798/811 (98.4%) start with `no refuter refuted` (729 not checked, 69 checked, 13 other).
1.22	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:1-69 and all 5 other receipts carry identical key sets with no `verdict` field.
1.23	SURVIVED	scripts/dogfood/lib/receipt.sh:33-41 `dogfood_receipt_status()`; running it on all 6 `.quorum/*.json` receipts prints `ok` for 6 of 6.
1.24	SURVIVED	PR_BODY.md:39 `Receipt and judge files are byte-identical to the earlier review branch`; earlier review branch is absent from `./tree`, so UNVERIFIED holds.
1.25	SURVIVED	.quorum/feat-894-pool-interruption-orphans.json:3 `"issue": "PR #1140: ... (#1115)"`; 0 `PMAT-*` identifiers or backfill ticket ids (1169/1170/1171/1176/1177/1178) appear in any of the 24 `.quorum/` files.
1.26	SURVIVED	docs/roadmaps/roadmap.yaml:1-8129 parses cleanly with `yq v4.52.2` (`length` = 428, 0 duplicate ids).
1.27	SURVIVED	.quorum/fix-audit-chrono-0543.json:31 appends two extra `||`-delimited segments (321 chars, 551 chars) absent from `.quorum/evidence/PR-1156-judge-{1,2,3}.json:204`, unlike all 9 other `refuted_claims` entries which join only their 3 committed judge reasons verbatim.
1.28	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:28 `"claims_confirmed": 31` (270 total across 6 receipts), `.quorum/evidence/PR-1101-judge-1.json:6` `"cited line not checked"` (729/811 = 89.9%), `scripts/dogfood/lib/receipt.sh:14-31` does not check `claims_confirmed`, and `PR_BODY.md:33` notes `"not refuted" is not "confirmed correct"`.
2.1	SURVIVED	PR.stat:26 `25 files changed, 5542 insertions(+)` and ANGLE.diff:5633 `diff --git a/docs/roadmaps/roadmap.yaml` (+54/-0, 24 new `.quorum/` files, 0 deletions, no source code).
2.2	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:37-64 and all 18 judge files; every recorded `sha256`, git `blob` SHA-1, `bytes`, and `total_bytes` matches byte-for-byte.
2.3	SURVIVED	PR_BODY.md:10,42 `Three commits, one per ticket`; `./tree` has no `.git` and ANGLE.diff has no commit headers, so UNVERIFIED holds.
2.4	SURVIVED	docs/roadmaps/roadmap.yaml:8076-8129 and line 7 `"merge_commit"` of all 6 receipts match all 6 SHAs and ticket-to-PR pairings in PR_BODY.md:12-14.
2.5	SURVIVED	docs/roadmaps/roadmap.yaml:1-8129 has 0 rows with `id` or `github_issue` 1169/1170/1171 and PR_BODY.md:3-5 only closes `#1176`..`#1178`.
2.6	SURVIVED	scripts/dogfood/quorum.sh:73,88,100, scripts/dogfood/release-check.sh:369,390,400, and scripts/dogfood/lib/receipt.sh:14-41; `dogfood_receipt_status` prints `ok` for all 6 receipts.
2.7	SURVIVED	.quorum/fix-audit-chrono-0543.json:4-5,8-14 `"reviewed_after_merge": true`, `"reviewed_at": "2026-11-18T14:24:28Z"`, `"author_model": "Claude Haiku 5.5, Claude Sonnet 5.5"`, `"degraded": "same-family"` and lines 4-5,8-14 of all 5 other receipts match what is claimed (off-tree execution caveated).
2.8	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:26,67 `"refuters_per_claim": 3` and `"not_committed": "lane-1..3.md, refuter-1..3.json..."` (lines 66/67/68 across the 6 receipts); no lane or refuter files are committed.
2.9	SURVIVED	.quorum/evidence/PR-{1126,1120,1101,1140,1145,1156}-judge-{1,2,3}.json and lines 19-29 of all 6 receipts match every table cell in PR_BODY.md:24-31, every per-lane count, and every per-ticket sum in PR_BODY.md:33.
2.10	SURVIVED	All 6 `.quorum/*.json` receipts and all 18 judge files contain 0 matches for `waiv|overrid`.
2.11	SURVIVED	All 18 judge files total 840 entries (811 SURVIVED, 29 REFUTED; 798/811 start with `"no refuter refuted"`: 729 `"cited line not checked"`, 67 `"cited line checked against refuter evidence, not contradicted"`, 2 `"cited line checked: ..."`) and all 6 receipts have 0 `verdict` keys.
2.12	SURVIVED	PR_BODY.md:39 `byte-identical to the earlier review branch` is UNVERIFIED without that branch; sub-claim of 0 `PMAT-*`/batch ticket ids holds (`.quorum/feat-894-pool-interruption-orphans.json:3` has `(#1115)`).
2.13	SURVIVED	docs/roadmaps/roadmap.yaml:1-8129 parses with `yq` (428 rows, 428 unique `id` values).
2.14	SURVIVED	.quorum/fix-audit-chrono-0543.json:24,31 `"judges": 3` has 5 ` || `-separated segments under `judges REFUTED 3 of 3:`, where segments 4 and 5 do not appear in `.quorum/evidence/PR-1156-judge-{1,2,3}.json:204`, unlike the other 9 `refuted_claims` entries.
2.15	SURVIVED	.quorum/refactor-cloud-instance-parse-validate.json:28-29,65 (`39`/`1`, `PR-1120-judge-1.json` has 40/0, L2C6 `:113` `SURVIVED`) and `.quorum/fix-cloud-instance-validate-resolved.json:28-29,66` (`31`/`2`, `PR-1126-judge-1.json` has 32/1, L2C9 `:125` `SURVIVED`).
2.16	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:32 and `.quorum/refactor-cloud-instance-parse-validate.json:31` both place judge-1's dissenting `SURVIVED` text (`.quorum/evidence/PR-1126-judge-1.json:125`, `PR-1120-judge-1.json:113`) first under `judges REFUTED 2 of 3:` with no per-judge label or verdict indicator.
2.17	SURVIVED	.quorum/evidence/PR-1156-judge-1.json:378 `"Refuters split (r3 REFUTED, r1/r2 SURVIVED)"` contradicts `PR-1156-judge-2.json:378` `"Refuter 2 REFUTED, 1 and 3 SURVIVED"` and `PR-1156-judge-3.json:378` `"Refuter 2 REFUTED ...; refuters 1,3 SURVIVED"`, and `refuter-1..3.json` are uncommitted (`.quorum/fix-audit-chrono-0543.json:66`).
2.18	SURVIVED	Line 28 of all 6 receipts records 270 `"claims_confirmed"` despite 729/811 `"cited line not checked"` and 67 `"cited line checked against refuter evidence, not contradicted"`, commits vote-only JSON failing `scripts/quorum_evidence.py:36,186-189,267-276`, and `scripts/dogfood/lib/receipt.sh:14-31` never opens `evidence.files` contents or reads `claims_digest`.
2.19	SURVIVED	docs/roadmaps/roadmap.yaml:8080,8098,8116 `status: planned` and `:8090-8091,8108-8109,8126-8127` `- kind:code` with no `release:<next.tag>` fail Gate T (`scripts/dogfood/tagged.sh:461-467,481-495`) on merge, and `scripts/quorum-gate.sh:486-488` places `.quorum/` and `docs/roadmaps/roadmap.yaml` on the triage rail.
2.20	SURVIVED	scripts/dogfood/harness.sh:98-105,214-217 `receipt="docs/audits/impl-${id}-receipt.md"` and `docs/audits/` contains no `impl-PMAT-1176-receipt.md` (nor 1177/1178), so Gate A fails once PR #1201 merges.
3.1	SURVIVED	PR.stat:26 `25 files changed, 5542 insertions(+)` and docs/roadmaps/roadmap.yaml:8076-8129; 24 new `.quorum/` files + 1 modified `roadmap.yaml`, 0 deletion lines in ANGLE.diff, no source code.
3.2	SURVIVED	PR_BODY.md:10,42 `Three commits, one per ticket`; `./tree` has no `.git` and ANGLE.diff has no commit headers, so UNVERIFIED holds.
3.3	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:7 `"merge_commit": "65cf98d3..."` and all 5 other receipts match PR_BODY.md:12-14 and docs/roadmaps/roadmap.yaml:8079,8097,8115.
3.4	SURVIVED	PR_BODY.md:10,12-14; `./tree` has no `.git` or GitHub PR metadata to confirm upstream merge commits or `headRefName` slugs, so UNVERIFIED holds.
3.5	SURVIVED	PR_BODY.md:3-5 `Closes #1176`..`#1178` and docs/roadmaps/roadmap.yaml:1-8129 (0 rows for 1169/1170/1171; live GitHub state external).
3.6	SURVIVED	scripts/dogfood/quorum.sh:87-106 `receipt=".quorum/${DOGFOOD_SLUG}.json"` and scripts/dogfood/release-check.sh:389-405 `path=".quorum/${slug}.json"`.
3.7	SURVIVED	.quorum/feat-894-pool-interruption-orphans.json:4-5 `"reviewed_after_merge": true`, `"reviewed_at": "2026-11-18T13:38:45Z"` and `.quorum/fix-audit-chrono-0543.json:8`; self-reported only with no run logs or trailers in `./tree`, so UNVERIFIED holds.
3.8	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:14 `"degraded": "same-family"` holds at line 14 of all 6 committed receipts.
3.9	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:26,67 `"refuters_per_claim": 3` and `"not_committed": "lane-1..3.md, refuter-1..3.json..."` across all 6 receipts.
3.10	SURVIVED	.quorum/fix-audit-chrono-0543.json:27-29 (`69`/`68`/`1`) and `.quorum/evidence/PR-1140-judge-3.json:269` (`"verdict": "REFUTED"`); all 18 judge files match PR_BODY.md:20,24-33 counts, per-lane counts, per-judge votes, and per-ticket totals.
3.11	SURVIVED	scripts/dogfood/lib/receipt.sh:17 `any(test("waiv|override"))`; 0 matches for `waiv|overrid|bypass|skip_?quorum` across all 24 committed `.quorum/` files.
3.12	SURVIVED	.quorum/evidence/PR-1101-judge-1.json:11-12 `"verdict": "SURVIVED"`, `"reason": "no refuter refuted; cited line not checked"`; 811 SURVIVED of 840 total votes, 798/811 (98.4%) start with `"no refuter refuted"`.
3.13	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:1-69 and all 5 other receipts carry no `"verdict"` key.
3.14	SURVIVED	scripts/dogfood/lib/receipt.sh:33-41 `dogfood_receipt_status()` prints `ok` for all 6 receipts in `./tree/.quorum/`.
3.15	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:42-44,65 `"blob": "f1c12849..."`, `"sha256": "6ffaaa7e..."`, `"bytes": 5601`, `"total_bytes": 16081`; all 18 sha256/blob/bytes and 6 total_bytes match.
3.16	SURVIVED	PR_BODY.md:39 `byte-identical to the earlier review branch`; no earlier branch exists in `./tree`, so UNVERIFIED holds.
3.17	SURVIVED	docs/roadmaps/roadmap.yaml:1-8129 parses with `yq v4.52.2` with 428 rows and 428 unique `id` values.
3.18	SURVIVED	docs/roadmaps/roadmap.yaml:8080,8098,8116 `status: planned` vs scripts/dogfood/tagged.sh:484-495 `case "$DOGFOOD_ROW_STATUS" in completed|cancelled) ;; ... *) fail ...`; Gate T arm T7 fails once PR #1201 merges.
3.19	SURVIVED	docs/roadmaps/roadmap.yaml:8090-8091,8108-8109,8126-8127 carry only `- kind:code` (no `release:<tag>`) vs scripts/dogfood/tagged.sh:461-467 and docs/roadmaps/releases.yaml:33; Gate T arm T4 fails on merge.
3.20	SURVIVED	docs/roadmaps/roadmap.yaml:8090-8093,8108-8111,8126-8129 (`- kind:code`, `epic: 764`) vs `:6353-6356` (`GH-764` resource-convergence epic), `:4417-4434` (`PMAT-601` uses `kind:triage`), `docs/audits/dogfood-1.34.0-receipt.md:82` (`#990` under epic `#696`), and `scripts/quorum-gate.sh:486-488`.
3.21	SURVIVED	scripts/dogfood/harness.sh:214-217 `receipt="docs/audits/impl-${id}-receipt.md"` and scripts/dogfood/quorum.sh:87-91 `receipt=".quorum/${DOGFOOD_SLUG}.json"`; PR #1201 adds neither `impl-PMAT-1176/1177/1178-receipt.md` nor a `.quorum/` receipt for PR #1201 itself.
3.22	SURVIVED	.quorum/fix-audit-chrono-0543.json:24,31,66 (`"judges": 3`, 5 `||`-delimited segments with 880 extra chars / 884 UTF-8 bytes absent from `.quorum/evidence/PR-1156-judge-{1,2,3}.json:204`, `"not_committed": "lane-1..3.md, refuter-1..3.json"`).
3.23	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:32 and `.quorum/refactor-cloud-instance-parse-validate.json:31` place Judge 1's dissenting `SURVIVED` text (`.quorum/evidence/PR-1126-judge-1.json:125-126`, `PR-1120-judge-1.json:113-114`) first under `judges REFUTED 2 of 3:` with no verdict tag.
3.24	SURVIVED	Line 28 of all 6 receipts records 270 `"claims_confirmed"` while all 18 judge files emit 0 `CONFIRMED` verdicts and 729/811 (89.9%) `SURVIVED` votes state `"cited line not checked"`, contradicting `PR_BODY.md:33`.
3.25	SURVIVED	.quorum/fix-cloud-instance-validate-resolved.json:29,66 (`claims_refuted: 2` vs `PR-1126-judge-1.json` having 1 `REFUTED`), `.quorum/refactor-cloud-instance-parse-validate.json:29,65` (`claims_refuted: 1` vs `PR-1120-judge-1.json` having 0 `REFUTED`), and `scripts/quorum_evidence.py:186-189,259-276` (`roles: ["judges"]` only, no markdown claim prose).
3.26	SURVIVED	.quorum/evidence/PR-1156-judge-1.json:378 `"r3 REFUTED, r1/r2 SURVIVED"` vs `PR-1156-judge-2.json:378` `"Refuter 2 REFUTED, 1 and 3 SURVIVED"` and `PR-1156-judge-3.json:378` `"Refuter 2 REFUTED ...; refuters 1,3 SURVIVED"`, with `refuter-1..3.json` uncommitted (`.quorum/fix-audit-chrono-0543.json:66`).
3.27	REFUTED	docs/specifications/quorum-spec.md:173-199 (`### 2. Refute`: `If one refuter produces a proof ... a 2–1 vote does not make the claim true`) rejects majority voting among refuters, not judges; scripts/dogfood/lib/receipt.sh:21-22 (`below the floor of 3: a majority needs three`) and scripts/dogfood/quorum.sh:27 explicitly define a 3-judge majority, single refuters did kill claims in `.quorum/fix-cloud-instance-validate-resolved.json:31` (`Refuter-3 right, refuters 1/2 missed it`) and `.quorum/refactor-cloud-instance-parse-validate.json:31` (`Refuters split (1 REFUTED; 2,3 SURVIVED)`), and `.quorum/evidence/PR-1140-judge-1.json:270` / `PR-1140-judge-2.json:270` answered the count objection as an incidental parenthetical while ruling the main assertion stands.
VERDICT: FAIL
text_only: yes
refuter: 3
reviewer_model: gemini-4-neon-high
