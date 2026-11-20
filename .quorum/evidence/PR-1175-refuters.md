# PR-1175 refuter 1 (agy gemini-4-neon-high), verbatim
1.1	SURVIVED	PR.stat:49-50 `docs/roadmaps/roadmap.yaml | 54 +++++` / `49 files changed, 9010 insertions(+)` and ANGLE.diff match 48 byte-identical `.quorum/` JSONs + `roadmap.yaml` (+54, -0)
1.2	SURVIVED	docs/roadmaps/roadmap.yaml:8022-8075 `- id: PMAT-1166`..`epic: 764` (`acceptance_criteria: []`, single `epic: 764` at :8057, `yq` rc=0, 0 duplicate keys) and Cargo.toml:3 `version = "1.34.2"`
1.3	SURVIVED	.quorum/feat-cloud-instance-ami-pin.json:37-64 `"sha256": "d9ced9b3..."`, `"bytes": 5300`, `"total_bytes": 15205` — all 36/36 sha256, git blob, bytes, and 12/12 total_bytes match committed files
1.4	SURVIVED	scripts/dogfood/quorum.sh:73,88,100, scripts/dogfood/release-check.sh:369,390,400, scripts/dogfood/lib/receipt.sh:29,33 `else "ok"` — `dogfood_receipt_status` prints `ok` on 12/12 receipts
1.5	SURVIVED	.quorum/feat-cloud-instance-ami-pin.json:4-14,19-21,24,26 `"reviewed_after_merge": true`, `"degraded": "same-family"`, `"refuters_per_claim": 3` and scripts/quorum-models.json:3-4; 7 at 08:29:17Z, 5 at 09:30:20Z, all 12 head/merge_commit distinct 40-hex
1.6	SURVIVED	.quorum/fix-876-572-quorum-receipt.json:27-32 `"claims_total": 38`, `"claims_confirmed": 36`, `"claims_refuted": 2` — across all 36 judge files: 448 total, 16 refuted (thirteen 3/3, three 2/3, zero 1/3), verbatim ` || `-joined reasons
1.7	SURVIVED	.quorum/evidence/PR-1097-judge-1.json:3-4,81-82,147-148 `"lane": 1`..`3` reproduces all 36/36 `quorum.lanes` counts across all 12 receipts, contradicting PR_BODY.md:11 `per-lane claim counts ... cannot be re-derived`
1.8	SURVIVED	scripts/dogfood/lib/receipt.sh:17 `any(test("waiv|override"))` — recursive key and raw-text scan across all 48 `.quorum/` JSON files finds 0 occurrences
1.9	SURVIVED	PR_BODY.md:15-28 matches all 12 receipts on totals/refuted counts and 15/16 snippets; .quorum/fix-858-e02-controlmaster.json:32 `"Closes #1070"` is paraphrased as `"Closes" ticket 1070` at PR_BODY.md:21
1.10	SURVIVED	.quorum/evidence/PR-1097-judge-1.json:5-6 `"verdict": "SURVIVED", "reason": "no refuter refuted; cited line not checked"` — 1344 votes (1299 SURVIVED: 1231 not checked, 50 checked, 18 detailed; 45 REFUTED), 0 `verdict` keys
1.11	SURVIVED	scripts/dogfood/lib/receipt.sh:25 `elif num(.quorum.claims_refuted) < 1` — none of the 12 PRs at PR_BODY.md:34 has a receipt or evidence file among 170 `.quorum/` entries and 730 `.quorum/evidence/` files
1.12	SURVIVED	docs/roadmaps/roadmap.yaml:8022-8075 `acceptance_criteria: []` — no batch-to-PR map or unreceipted round artifacts exist in `./tree` or `ANGLE.diff` (only 2 timestamps: 7 at 08:29:17Z, 5 at 09:30:20Z)
1.13	SURVIVED	PR_BODY.md:5,11,42,44 — `./tree` has no `.git` history and all 36 judge JSONs contain 0 model-name strings or timestamps
1.14	SURVIVED	.quorum/fix-876-572-quorum-receipt.json:31 `documented as "Repository scripts that run car | judges REFUTED` — 8/16 `refuted_claims` across 7 receipts are hard-truncated at 111-122 chars with `lane-1..3.md` uncommitted
1.15	SURVIVED	.quorum/fix-876-572-quorum-receipt.json:29,66 `"claims_refuted": 2`, `"claims_digest": ".quorum/evidence/PR-1106-judge-1.json"` vs .quorum/evidence/PR-1106-judge-1.json:21-23,159-161 `"verdict": "SURVIVED"` (38 SURVIVED, 0 REFUTED)
2.1	SURVIVED	PR.stat:49-50 `49 files changed, 9010 insertions(+)`, docs/roadmaps/roadmap.yaml:8022-8075 (`PMAT-1166`..`1168`), CHANGELOG.md:10-88 lists all 12 PRs under `[1.34.2]`
2.2	SURVIVED	.quorum/feat-cloud-instance-ami-pin.json:41-64 `"blob"`, `"sha256"`, `"bytes": 5300`, `"total_bytes": 15205` — all 36 evidence entries and 12 totals match committed files byte-for-byte
2.3	SURVIVED	scripts/dogfood/lib/receipt.sh:14-31 `DOGFOOD_RECEIPT_JQ` and .quorum/fix-1065-check-checksum.json:4-14,26 — 12/12 print `ok`, distinct 40-hex `head`/`merge_commit`, 0 `waiv|override` keys
2.4	SURVIVED	.quorum/evidence/PR-1106-judge-2.json:21-23,159-161 and PR-1106-judge-3.json:21-23,159-161 `"verdict": "REFUTED"` — all 12 receipts and PR_BODY.md:15-28 match judge >=2/3 tallies (448/432/16)
2.5	SURVIVED	.quorum/evidence/PR-1036-judge-1.json:3-4 `"lane": 1, "claim": 1` and .quorum/fix-cloud-instance-lookup-hardening.json:19-21 — grouping judge JSONs by `"lane"` reproduces 36/36 per-lane counts, refuting PR_BODY.md:11
2.6	SURVIVED	.quorum/evidence/PR-1036-judge-2.json:5-6 `"verdict": "SURVIVED", "reason": "no refuter refuted; cited line not checked"` — 1344 votes (45 REFUTED, 1299 SURVIVED: 1231 not checked, 50 checked, 18 custom), 0 `verdict` keys
2.7	SURVIVED	docs/roadmaps/roadmap.yaml:8022-8075 `acceptance_criteria: []`, single `epic: 764` at :8057 — `yq . docs/roadmaps/roadmap.yaml` exits 0 with 0 duplicate keys and 0 PR numbers in the 3 rows
2.8	SURVIVED	CHANGELOG.md:58-85 lists all 12 PRs from PR_BODY.md:34 under `[1.34.2]`, and none has a receipt or evidence file under `.quorum/`
2.9	SURVIVED	.quorum/feat-cloud-instance-ami-pin.json:66 `"not_committed": "lane-1..3.md, refuter-1..3.json..."` — without `.git`, PR metadata, or committed lane/refuter files, those PR_BODY.md:7,11,42,44 claims are unverified
2.10	SURVIVED	.quorum/fix-876-572-quorum-receipt.json:28-29,66 `"claims_confirmed": 36`, `"claims_refuted": 2`, `"claims_digest": ".quorum/evidence/PR-1106-judge-1.json"` vs PR-1106-judge-1.json:21-24,159-162 (0 REFUTED, 38 SURVIVED)
2.11	SURVIVED	.quorum/feat-cloud-instance-ami-pin.json:38-40,65 `"roles": ["judges"]` and scripts/quorum_evidence.py:186-189,359-364 `for need in ("claims", "lanes", "judges", "agy"):` / `missing 'base_commit'` — all 12 omit `base_commit`, required roles, and markdown claim sections
2.12	REFUTED	.quorum/fix-876-572-quorum-receipt.json:31 + fix-1074-typed-errors-image-cmd.json:31 + fix-858-e02-controlmaster.json:31 + fix-858-untyped-ratchet-5.json:31 + fix-1067-apply-live-version.json:31-32 + fix-858-remaining-tests.json:31 + fix-cloud-instance-1025-followups.json:31 — the 8 truncated entries span 7 receipts, not "across 6 receipts"
2.13	SURVIVED	.quorum/evidence/PR-1104-judge-1.json:279-282 `Lane gave UNVERIFIED, not TRUE` and PR-1107-judge-1.json:243-246 — 377/432 (87.3%) all-3 unchecked and 2 UNVERIFIED claims are all tallied as `"claims_confirmed"`, which scripts/dogfood/lib/receipt.sh:14-31 does not require
2.14	SURVIVED	docs/roadmaps/roadmap.yaml:8022-8075 `status: planned` / `- kind:code`, scripts/dogfood/lib/window.sh:411-434, tagged.sh:461-467,481-495, harness.sh:214-216 — missing `release:*` (T4), `status: planned` (T7), and no `docs/audits/impl-PMAT-116{6,7,8}-receipt.md` (Gate A)
3.1	SURVIVED	PR.stat:49-50 `49 files changed, 9010 insertions(+)` and docs/roadmaps/roadmap.yaml:8022-8075 match 12 receipts + 36 judge files + `roadmap.yaml` (+54, -0)
3.2	SURVIVED	.quorum/fix-cloud-instance-lookup-hardening.json:37-64 `"bytes": 5446`, `"total_bytes": 15652` — all 36 evidence sha256/blob/bytes and 12 total_bytes match committed files
3.3	SURVIVED	scripts/dogfood/quorum.sh:73,88-106 and scripts/dogfood/release-check.sh:369,390-401 both read `.quorum/<slug>.json` through `scripts/dogfood/lib/receipt.sh`
3.4	SURVIVED	.quorum/feat-cloud-instance-ami-pin.json:4-14,24,26 and scripts/quorum-models.json:3-4 `"claude-opus-5-5": "claude"` / `"claude-sonnet-5-5": "claude"` — all 12 carry those fields and 0 `waiv|overrid` keys
3.5	SURVIVED	.quorum/fix-876-572-quorum-receipt.json:27-29 and PR_BODY.md:15-28 — all 12 receipts and table cells match >=2/3 REFUTED counts (448/432/16: thirteen 3/3, three 2/3, zero 1/3)
3.6	SURVIVED	.quorum/evidence/PR-1036-judge-1.json:3-4,225-226 `"lane": 1, "claim": 1`..`"lane": 3, "claim": 13` reproduces all 36/36 `quorum.lanes` counts across all 12 receipts, refuting PR_BODY.md:11
3.7	SURVIVED	.quorum/evidence/PR-1036-judge-2.json:5-6 `"verdict": "SURVIVED", "reason": "no refuter refuted; cited line not checked"` — 1231 not checked + 50 checked + 15 substantive + 3 dissents = 1299 SURVIVED, 45 REFUTED, 0 `verdict` keys
3.8	SURVIVED	PR.stat:37-48 and PR_BODY.md:34 — none of the 12 listed PRs has a receipt among the 169 files directly under `.quorum/`
3.9	SURVIVED	docs/roadmaps/roadmap.yaml:8022-8075 `acceptance_criteria: []` — no `.git` history, no lane/refuter files, no PR lists, and only two timestamps (7 at 08:29:17Z, 5 at 09:30:20Z)
3.10	SURVIVED	scripts/dogfood/lib/receipt.sh:33-41 (`dogfood_receipt_status` prints `ok` on 12/12) and docs/roadmaps/roadmap.yaml:8040-8057 (`yq` rc=0, 0 duplicate keys)
3.11	SURVIVED	.quorum/fix-1074-typed-errors-image-cmd.json:28 `"claims_confirmed": 46` and scripts/dogfood/lib/receipt.sh:14-31 — 377/432 (87.3%) all-3 unchecked including 100% of unchallenged claims across the 7 listed PRs
3.12	SURVIVED	.quorum/fix-876-572-quorum-receipt.json:31 `run car | judges REFUTED` and PR_BODY.md:17-28 — 8/16 across 7 receipts truncated at 111-122 chars, PR_BODY.md:21 alters `"Closes #1070"` (.quorum/fix-858-e02-controlmaster.json:32) to `"Closes" ticket 1070`
3.13	REFUTED	.quorum/feat-cloud-instance-ami-pin.json:3,16,38-40 `"issue"`, `"quorum"`, `"roles": ["judges"]` vs scripts/quorum-gate.sh:461 and scripts/quorum_evidence.py:186 — all 12 receipts include 2/7 required fields (`issue`, `quorum`) and 1/4 required roles (`judges`), so "omit every required field and evidence role" is false
3.14	SURVIVED	.quorum/evidence/PR-1106-judge-1.json:21-24,159-162 and PR-1071-judge-2.json:165-168 — all 3 dissenting `SURVIVED` reasons are dropped from `quorum.refuted_claims` and PR_BODY.md:17,28 omits that these 3 were 2-of-3 splits
3.15	SURVIVED	docs/roadmaps/roadmap.yaml:8022-8075 `status: planned` / `- kind:code` and scripts/dogfood/tagged.sh:461-467,481-495 — `dogfood_pr_tickets` (window.sh:411-434) pulls `PMAT-1166..1168` from PR_BODY.md:1,47, failing T4 and T7
3.16	SURVIVED	scripts/dogfood/harness.sh:98-99,214-217, quorum.sh:88-91, release-check.sh:390-405 — no `docs/audits/impl-PMAT-1166-receipt.md` (or 1167/1168) and no `.quorum/` receipt for PR #1175 exist in `./tree`
3.17	SURVIVED	docs/roadmaps/roadmap.yaml:8022-8075 (`epic: 764`, `- kind:code`, `acceptance_criteria: []`) vs roadmap.yaml:6353-6369 (`GH-764` resource-convergence epic, `release: 1.35.0`) and roadmap.yaml:4417-4436 (`PMAT-601`, `kind:triage`)
3.18	SURVIVED	.quorum/feat-cloud-instance-ami-pin.json:8-14 `"degraded": "same-family"`, docs/specifications/quorum-spec.md:197-199, scripts/quorum-store-gate.sh:27-28,158-166, scripts/dogfood/lib/receipt.sh:14-31 — judges 2-3 checked 0/427 unchallenged claims and `lib/receipt.sh` ignores `degraded`/`lane_models`
VERDICT: FAIL
text_only: yes
refuter: 1
reviewer_model: gemini-4-neon-high
# PR-1175 refuter 2 (agy gemini-4-neon-high), verbatim
1.1	SURVIVED	PR.stat:49-50 "docs/roadmaps/roadmap.yaml | 54 +++++" / "49 files changed, 9010 insertions(+)" (48 new .quorum JSONs byte-identical in ANGLE.diff and ./tree)
1.2	SURVIVED	docs/roadmaps/roadmap.yaml:8022-8075 "- id: PMAT-1166".."epic: 764" (yq exit 0, 0 dup keys, single epic: 764 at :8057) and Cargo.toml:3 version = "1.34.2"
1.3	SURVIVED	.quorum/feat-cloud-instance-ami-pin.json:41-43,64 "blob": "6fc890c9..." "bytes": 5300 "total_bytes": 15205 (36/36 sha256/blob/bytes and 12/12 total_bytes match)
1.4	SURVIVED	scripts/dogfood/lib/receipt.sh:29,33 else "ok" / dogfood_receipt_status() (sourced at quorum.sh:73, release-check.sh:369; 12/12 print ok)
1.5	SURVIVED	.quorum/feat-cloud-instance-ami-pin.json:4-14,24,26 "reviewed_after_merge": true, "degraded": "same-family", "judges": 3, "refuters_per_claim": 3 (all 12 match; 7 at 08:29:17Z, 5 at 09:30:20Z)
1.6	SURVIVED	.quorum/fix-876-572-quorum-receipt.json:27-29 "claims_total": 38, "claims_confirmed": 36, "claims_refuted": 2 (448 total, 16 refuted: 13 3/3, 3 2/3, 0 1/3; 16/16 reasons verbatim)
1.7	SURVIVED	.quorum/evidence/PR-1097-judge-1.json:3-4,81-82,147-148 "lane": 1/2/3 reproduces all 36/36 per-lane counts in quorum.lanes across 12 receipts, contradicting PR_BODY.md:11
1.8	SURVIVED	scripts/dogfood/lib/receipt.sh:17 any(test("waiv|override")) — 0 key or raw-text matches across all 48 committed .quorum/ JSON files
1.9	SURVIVED	PR_BODY.md:17-28 and .quorum/fix-858-e02-controlmaster.json:32 'the PR body says "Closes #1070"' vs PR_BODY.md:21 '"Closes" ticket 1070' (15/16 exact prefixes; 448/432/16 match)
1.10	SURVIVED	.quorum/evidence/PR-1097-judge-1.json:5-6 "verdict": "SURVIVED", "reason": "no refuter refuted; cited line not checked" (1231/1299 SURVIVED, 45 REFUTED, 0 verdict keys in receipts)
1.11	SURVIVED	scripts/dogfood/lib/receipt.sh:25 elif num(.quorum.claims_refuted) < 1 (0 of the 12 PRs appear in ANGLE.diff or ./tree/.quorum/ across 170 entries and 730 evidence files)
1.12	SURVIVED	docs/roadmaps/roadmap.yaml:8022-8075 acceptance_criteria: [] (no PR-to-batch mapping or unreceipted review evidence in ANGLE.diff or ./tree)
1.13	SURVIVED	PR_BODY.md:42,44 "4 commits (1 per batch, then 1 fix)" (no .git in ./tree; 0 model-name or timestamp strings in the 36 judge JSONs)
1.14	SURVIVED	.quorum/fix-876-572-quorum-receipt.json:31 'documented as "Repository scripts that run car | judges REFUTED' (8 of 16 entries across 7 receipts truncated at 111-122 chars)
1.15	SURVIVED	.quorum/fix-876-572-quorum-receipt.json:29,66 "claims_refuted": 2, "claims_digest": ".quorum/evidence/PR-1106-judge-1.json" vs .quorum/evidence/PR-1106-judge-1.json:23,161 "verdict": "SURVIVED" (38 SURVIVED, 0 REFUTED)
2.1	SURVIVED	PR.stat:49-50 "49 files changed, 9010 insertions(+)" and CHANGELOG.md:10-88 "## [1.34.2] - 2026-11-17" (all 12 PRs listed under [1.34.2])
2.2	SURVIVED	.quorum/feat-cloud-instance-ami-pin.json:41-43,64 "sha256": "d9ced9b3..." "bytes": 5300 "total_bytes": 15205 (all 36 evidence files and 12 total_bytes match)
2.3	SURVIVED	scripts/dogfood/lib/receipt.sh:14-31 and .quorum/fix-1065-check-checksum.json:4-14,26 "reviewed_after_merge": true, "degraded": "same-family", "refuters_per_claim": 3 (12/12 ok, 0 waiv/override keys)
2.4	SURVIVED	.quorum/evidence/PR-1106-judge-2.json:23,161 "verdict": "REFUTED" and PR_BODY.md:15-28 (448 total, 432 not refuted, 16 refuted: 13 3/3, 3 2/3)
2.5	SURVIVED	.quorum/evidence/PR-1036-judge-1.json:3-4 "lane": 1, "claim": 1 reproduces .quorum/fix-cloud-instance-lookup-hardening.json:19-21 (36/36 per-lane counts re-derivable)
2.6	SURVIVED	.quorum/evidence/PR-1036-judge-2.json:5-6 "verdict": "SURVIVED", "reason": "no refuter refuted; cited line not checked" (1344 votes: 45 REFUTED, 1299 SURVIVED, 1231 not checked; 0 verdict keys)
2.7	SURVIVED	docs/roadmaps/roadmap.yaml:8022-8075 acceptance_criteria: [], notes: null, epic: 764 at :8057 (yq exit 0, 0 duplicate keys, no PR numbers listed)
2.8	SURVIVED	PR_BODY.md:34 "They have no receipt in this PR" and CHANGELOG.md:43-85 (0 receipts/evidence for those 12 PRs under .quorum/; all 12 appear under [1.34.2])
2.9	SURVIVED	.quorum/feat-cloud-instance-ami-pin.json:66 "not_committed": "lane-1..3.md, refuter-1..3.json" (no .git history or lane/refuter files in merge tree)
2.10	SURVIVED	.quorum/fix-876-572-quorum-receipt.json:28-29,66 "claims_confirmed": 36, "claims_refuted": 2, "claims_digest": ".quorum/evidence/PR-1106-judge-1.json" vs .quorum/evidence/PR-1106-judge-1.json:23,161 "verdict": "SURVIVED" (0 REFUTED)
2.11	SURVIVED	.quorum/feat-cloud-instance-ami-pin.json:38-40,65 "roles": ["judges"], "claims_digest": ".quorum/evidence/PR-1097-judge-1.json" vs scripts/quorum_evidence.py:36,186-189,359-362 (missing base_commit, missing claims/lanes/agy roles, 0 SEC_RE matches)
2.12	REFUTED	.quorum/fix-1067-apply-live-version.json:31-32 plus 6 other receipts (fix-876-572-quorum-receipt.json:31, fix-1074-typed-errors-image-cmd.json:31, fix-858-e02-controlmaster.json:31, fix-858-untyped-ratchet-5.json:31, fix-858-remaining-tests.json:31, fix-cloud-instance-1025-followups.json:31): the 8 truncated entries span 7 distinct receipts, not 6
2.13	SURVIVED	.quorum/evidence/PR-1104-judge-1.json:282 "Lane gave UNVERIFIED, not TRUE" and .quorum/evidence/PR-1107-judge-1.json:246 "Lane's verdict is UNVERIFIED, not TRUE" (377/432 all-3 unchecked; lib/receipt.sh ignores claims_confirmed)
2.14	SURVIVED	docs/roadmaps/roadmap.yaml:8026,8044,8062 "status: planned" and scripts/dogfood/tagged.sh:461-467,481-495 / scripts/dogfood/harness.sh:214-216 (no release:* label, status planned, no impl-PMAT-{1166,1167,1168}-receipt.md)
3.1	SURVIVED	PR.stat:49-50 "49 files changed, 9010 insertions(+)" and docs/roadmaps/roadmap.yaml:8022-8075 "- id: PMAT-1166".."epic: 764"
3.2	SURVIVED	.quorum/fix-cloud-instance-lookup-hardening.json:37-64 "bytes": 5446, "total_bytes": 15652 (36/36 sha256/blob/bytes and 12/12 total_bytes match)
3.3	SURVIVED	scripts/dogfood/quorum.sh:73,88 receipt=".quorum/${DOGFOOD_SLUG}.json" and scripts/dogfood/release-check.sh:369,390,400 path=".quorum/${slug}.json"
3.4	SURVIVED	.quorum/feat-cloud-instance-ami-pin.json:4-14,24,26 "degraded": "same-family", "author_model": "Claude Sonnet 5.5" and scripts/quorum-models.json:3-4 (distinct 40-hex SHAs, 0 waiv|overrid keys)
3.5	SURVIVED	PR_BODY.md:15-28 and .quorum/fix-876-572-quorum-receipt.json:27-29 "claims_total": 38, "claims_confirmed": 36, "claims_refuted": 2 (448/432/16; 13 3/3, 3 2/3, 0 1/3)
3.6	SURVIVED	.quorum/evidence/PR-1036-judge-1.json:3-4,225-226 "lane": 1, "claim": 1.."lane": 3, "claim": 13 reproduces .quorum/fix-cloud-instance-lookup-hardening.json:19-21 (36/36 per-lane counts)
3.7	SURVIVED	.quorum/evidence/PR-1036-judge-2.json:5-6 "verdict": "SURVIVED", "reason": "no refuter refuted; cited line not checked" (1231+50+15+3=1299 SURVIVED, 45 REFUTED, 0 verdict keys)
3.8	SURVIVED	PR_BODY.md:34 "They have no receipt in this PR" — 0 of those 12 PRs exist among the 169 files under .quorum/ at merge commit
3.9	SURVIVED	docs/roadmaps/roadmap.yaml:8022-8075 acceptance_criteria: [] (no .git, no lane/refuter files, 7 receipts at 08:29:17Z and 5 at 09:30:20Z)
3.10	SURVIVED	scripts/dogfood/lib/receipt.sh:33-41 dogfood_receipt_status() prints ok on 12/12; docs/roadmaps/roadmap.yaml:8040-8057 passes yq exit 0 with 0 duplicate keys
3.11	SURVIVED	.quorum/fix-1074-typed-errors-image-cmd.json:28 "claims_confirmed": 46 vs PR_BODY.md:30 '"not refuted" is not "confirmed correct"' (377/432 all-3 unchecked; 100% unchallenged unchecked across all 7 named PRs)
3.12	SURVIVED	.quorum/fix-876-572-quorum-receipt.json:31 'documented as "Repository scripts that run car | judges REFUTED', scripts/quorum_evidence.py:251-252 "a severed citation is unreviewable", PR_BODY.md:21 '"Closes" ticket 1070'
3.13	REFUTED	scripts/quorum-gate.sh:461 'for field in ("issue", "diff_sha256", "quorum", ...)' and scripts/quorum_evidence.py:186 'for need in ("claims", "lanes", "judges", "agy"):' — all 12 receipts include "issue" and "quorum" (.quorum/feat-cloud-instance-ami-pin.json:3,16) and "roles": ["judges"] (:38-40), so "omit every required field and evidence role" is false
3.14	SURVIVED	.quorum/evidence/PR-1106-judge-1.json:23-24,161-162 "verdict": "SURVIVED" and .quorum/evidence/PR-1071-judge-2.json:167-168 "The test-gap defect stands." dropped from .quorum/fix-876-572-quorum-receipt.json:31-32, .quorum/fix-894-pool-scale-decision.json:31, PR_BODY.md:17,28
3.15	SURVIVED	docs/roadmaps/roadmap.yaml:8026,8044,8062 "status: planned" and scripts/dogfood/tagged.sh:461-467,481-495 "its roadmap row still reads status: ${DOGFOOD_ROW_STATUS}" (no release:* label)
3.16	SURVIVED	scripts/dogfood/harness.sh:214-216 receipt="docs/audits/impl-${id}-receipt.md" and scripts/dogfood/quorum.sh:88-90 receipt=".quorum/${DOGFOOD_SLUG}.json" (neither harness nor quorum receipt for PR #1175 exists)
3.17	SURVIVED	docs/roadmaps/roadmap.yaml:8032,8037,8039 acceptance_criteria: [], - kind:code, epic: 764 vs :6353-6369 GH-764 (release: 1.35.0) and :4417-4436 PMAT-601 (- kind:triage)
3.18	SURVIVED	.quorum/feat-cloud-instance-ami-pin.json:8-15 "degraded": "same-family" / "3 Opus 5.5 reviewer lanes, 3 Opus 5.5 refuters per claim, 3 Opus 5.5 judges" and scripts/dogfood/lib/receipt.sh:14-31 (ignores degraded/lane_models; judge-2/3 checked 0/427 unchallenged claims)
VERDICT: FAIL
text_only: yes
refuter: 2
reviewer_model: gemini-4-neon-high
# PR-1175 refuter 3 (agy gemini-4-neon-high), verbatim
1.1	SURVIVED	docs/roadmaps/roadmap.yaml:8022-8075 + ANGLE.diff: 49 files changed, 9010 insertions, 0 deletions (12 receipts + 36 judge JSONs byte-identical to ./tree + roadmap.yaml +54)
1.2	SURVIVED	docs/roadmaps/roadmap.yaml:8022-8075 + Cargo.toml:3 `version = "1.34.2"`; adds PMAT-1166/1167/1168 with `acceptance_criteria: []`, yq exit 0, 0 duplicate keys, single `epic: 764` at :8057
1.3	SURVIVED	.quorum/feat-cloud-instance-ami-pin.json:36-64 `"bytes": 5300` / `"total_bytes": 15205`; all 36 sha256/blob/bytes and 12 total_bytes match committed judge files 1-to-1
1.4	SURVIVED	scripts/dogfood/quorum.sh:73,88,100 + scripts/dogfood/release-check.sh:369,390,400 + scripts/dogfood/lib/receipt.sh:29,33 `else "ok"`; all 12 receipts return `ok`
1.5	SURVIVED	.quorum/feat-cloud-instance-ami-pin.json:4-14,19-26 `"reviewed_after_merge": true`, `"degraded": "same-family"`, `"refuters_per_claim": 3` + scripts/quorum-models.json:3-4; all 12 receipts match (7 at 08:29:17Z, 5 at 09:30:20Z)
1.6	SURVIVED	.quorum/fix-876-572-quorum-receipt.json:27-32 `"claims_total": 38`, `"claims_refuted": 2`; all 36 judge files match 448 total, 16 refuted (13 3/3, 3 2/3, 0 1/3) and verbatim ` || `-joined reasons
1.7	SURVIVED	.quorum/evidence/PR-1097-judge-1.json:3-4,81-82,147-148 `"lane": 1|2|3` + .quorum/feat-cloud-instance-ami-pin.json:19-21; grouping any judge file by `"lane"` reproduces all 36/36 per-lane counts, falsifying PR_BODY.md:11
1.8	SURVIVED	scripts/dogfood/lib/receipt.sh:17 `any(test("waiv|override"))`; 0 key or raw-text matches for `waiv`/`override` across all 48 `.quorum/` JSON files
1.9	SURVIVED	PR_BODY.md:15-28 + .quorum/fix-858-e02-controlmaster.json:32 `the PR body says "Closes #1070"`; all 12 rows match 448/432/16, 15/16 exact/prefix snippets, #1072 L3#9 paraphrased
1.10	SURVIVED	.quorum/evidence/PR-1097-judge-1.json:5-6 `"verdict": "SURVIVED", "reason": "no refuter refuted; cited line not checked"`; 1344 votes = 45 REFUTED + 1299 SURVIVED (1231 + 50 + 18), 0 receipts have `verdict`
1.11	SURVIVED	scripts/dogfood/lib/receipt.sh:25 `elif num(.quorum.claims_refuted) < 1`; none of the 12 PRs in PR_BODY.md:34 appears in the 170 `.quorum/` entries or 730 `.quorum/evidence/` files
1.12	SURVIVED	docs/roadmaps/roadmap.yaml:8022-8075 `acceptance_criteria: []`; no file in ./tree contains batch-to-PR mappings or GitHub metadata for the 24-PR total or 1/6/5 split
1.13	SURVIVED	PR_BODY.md:5,11,42,44 + 36 `.quorum/evidence/PR-*-judge-*.json` files (0 model-name or timestamp strings); no `.git` in ./tree so branch names, pre-squash history, and execution are unverified
1.14	SURVIVED	.quorum/fix-1074-typed-errors-image-cmd.json:31 `5 sites of sla | judges REFUTED` + 6 other receipts; 8 of 16 `refuted_claims` entries across 7 receipts are hard-truncated at 111-122 chars with `lane-1..3.md` uncommitted
1.15	SURVIVED	.quorum/fix-876-572-quorum-receipt.json:29,66 `"claims_refuted": 2`, `"claims_digest": ".quorum/evidence/PR-1106-judge-1.json"` + .quorum/evidence/PR-1106-judge-1.json:21-23,159-161 (38 SURVIVED, 0 REFUTED)
2.1	SURVIVED	docs/roadmaps/roadmap.yaml:8022-8075 + CHANGELOG.md:10-88 `## [1.34.2] - 2026-11-17`; 49 files (+9010, 0 source files: 12 receipts, 36 judges, roadmap.yaml +54) and all 12 PRs appear in CHANGELOG.md:10-88
2.2	SURVIVED	.quorum/feat-cloud-instance-ami-pin.json:41-64 `"blob": "6fc890c9..."`, `"bytes": 5300`, `"total_bytes": 15205`; all 36 sha256/blob/bytes and 12 total_bytes match committed files
2.3	SURVIVED	scripts/dogfood/lib/receipt.sh:14-31 + .quorum/fix-1065-check-checksum.json:4-14,26; `dogfood_receipt_status` prints `ok` for all 12 receipts, distinct 40-hex `head != merge_commit`, 0 `waiv|override` keys
2.4	SURVIVED	.quorum/fix-876-572-quorum-receipt.json:27-29 + .quorum/evidence/PR-1106-judge-2.json:21-23,159-161 `"verdict": "REFUTED"`; all 12 receipts and PR_BODY.md:15-28 match 448/432/16 (13 3/3, 3 2/3)
2.5	SURVIVED	.quorum/evidence/PR-1036-judge-1.json:3-5 `"lane": 1, "claim": 1` + .quorum/fix-cloud-instance-lookup-hardening.json:19-21; all 36/36 per-lane counts are directly re-derivable from judge files
2.6	SURVIVED	.quorum/evidence/PR-1036-judge-2.json:5-6 `"verdict": "SURVIVED", "reason": "no refuter refuted; cited line not checked"`; 1344 votes (45 REFUTED, 1299 SURVIVED: 1231 + 50 + 18), 0 `verdict` keys
2.7	SURVIVED	docs/roadmaps/roadmap.yaml:8022-8075 `acceptance_criteria: []`, `epic: 764` at :8057; `yq . docs/roadmaps/roadmap.yaml` exits 0, 0 duplicate keys, 3 rows list no PR numbers
2.8	SURVIVED	CHANGELOG.md:43-85 + ./tree/.quorum/; all 12 listed PRs appear in CHANGELOG.md:43-85 and none has a receipt or evidence file in PR #1175 or `.quorum/`
2.9	SURVIVED	.quorum/feat-cloud-instance-ami-pin.json:66 `"not_committed": "lane-1..3.md, refuter-1..3.json..."`; without `.git`, GitHub metadata, or lane/refuter files those historical claims are unverified
2.10	SURVIVED	.quorum/fix-876-572-quorum-receipt.json:28-29,66 `"claims_refuted": 2`, `"claims_digest": "...PR-1106-judge-1.json"` + .quorum/evidence/PR-1106-judge-1.json:21-24,159-162 (0 REFUTED, 38 SURVIVED)
2.11	SURVIVED	.quorum/feat-cloud-instance-ami-pin.json:38-40,65 `"roles": ["judges"]`, `"claims_digest": "...PR-1097-judge-1.json"` + scripts/quorum_evidence.py:36,186-189,267-276,359-364; all 12 receipts fail `quorum_evidence.py`
2.12	REFUTED	.quorum/fix-1067-apply-live-version.json:31-32 + 6 other receipts (`fix-876-572-quorum-receipt.json:31`, `fix-1074-typed-errors-image-cmd.json:31`, `fix-858-e02-controlmaster.json:31`, `fix-858-untyped-ratchet-5.json:31`, `fix-858-remaining-tests.json:31`, `fix-cloud-instance-1025-followups.json:31`); the 8 truncated entries span 7 receipts, not "across 6 receipts"
2.13	SURVIVED	.quorum/evidence/PR-1104-judge-1.json:279-282 `Lane gave UNVERIFIED, not TRUE` + .quorum/evidence/PR-1107-judge-1.json:243-246 `Lane's verdict is UNVERIFIED, not TRUE`; 377/432 (87.3%) all-3 not checked, 50 J1-only on 09:30:20Z receipts, `lib/receipt.sh` omits `claims_confirmed`
2.14	SURVIVED	docs/roadmaps/roadmap.yaml:8022-8075 `status: planned` + scripts/dogfood/tagged.sh:461-467,481-495 + scripts/dogfood/harness.sh:214-216 + scripts/dogfood/lib/window.sh:411-434; no `release:*` label, `status: planned`, and no `docs/audits/impl-PMAT-{1166,1167,1168}-receipt.md`
3.1	SURVIVED	docs/roadmaps/roadmap.yaml:8022-8075 + ANGLE.diff; 49 files, 9010 insertions, 0 deletions (12 receipts + 36 judge JSONs + roadmap.yaml +54)
3.2	SURVIVED	.quorum/fix-cloud-instance-lookup-hardening.json:37-64 `"bytes": 5446` / `"total_bytes": 15652`; all 36 sha256/blob/bytes and 12 total_bytes match committed files
3.3	SURVIVED	scripts/dogfood/quorum.sh:73,88-106 `receipt=".quorum/${DOGFOOD_SLUG}.json"` + scripts/dogfood/release-check.sh:369,390-401 `status="$(receipt_status "$blob")"`; Gate E and Gate R Arm 5 both read via `lib/receipt.sh`
3.4	SURVIVED	.quorum/feat-cloud-instance-ami-pin.json:4-14,24,26 + scripts/quorum-models.json:3-4; all 12 receipts match with distinct 40-hex `head`/`merge_commit`, 2026-11-18 timestamps, same-family Opus 5.5 on Sonnet 5.5, 0 `waiv|overrid` keys
3.5	SURVIVED	PR_BODY.md:15-28 + .quorum/fix-876-572-quorum-receipt.json:27-29 + .quorum/evidence/PR-1106-judge-2.json:21-23; all 12 receipts and Results cells match 448/432/16 (13 3/3, 3 2/3, 0 1/3)
3.6	SURVIVED	.quorum/evidence/PR-1036-judge-1.json:3-4,225-226 `"lane": 1, "claim": 1`..`"lane": 3, "claim": 13` + .quorum/fix-cloud-instance-lookup-hardening.json:19-21; per-lane counts are directly re-derivable, falsifying PR_BODY.md:11
3.7	SURVIVED	.quorum/evidence/PR-1036-judge-2.json:5-6 `"verdict": "SURVIVED", "reason": "no refuter refuted; cited line not checked"`; 1344 votes = 45 REFUTED + 1299 SURVIVED (1231 + 50 + 15 custom + 3 dissent), 0 `"verdict"` keys
3.8	SURVIVED	PR_BODY.md:34 + ./tree/.quorum/ (169 files); none of the 12 listed PRs has a receipt in PR #1175 or `.quorum/`
3.9	SURVIVED	docs/roadmaps/roadmap.yaml:8022-8075 + 12 receipts (7 at `2026-11-18T08:29:17Z`, 5 at `2026-11-18T09:30:20Z`); no `.git`, no lane/refuter files, no PR lists in roadmap.yaml so those claims are unverified
3.10	SURVIVED	scripts/dogfood/lib/receipt.sh:33-41 + docs/roadmaps/roadmap.yaml:8040-8057; `dogfood_receipt_status` prints `ok` for all 12 receipts, `yq` exits 0, 0 duplicate keys in roadmap.yaml
3.11	SURVIVED	.quorum/fix-1074-typed-errors-image-cmd.json:28 `"claims_confirmed": 46` + scripts/dogfood/lib/receipt.sh:14-31; 377/432 (87.3%) all-3 not checked including 100% of unchallenged claims on 7 PRs, and `lib/receipt.sh` never reads `claims_confirmed`
3.12	SURVIVED	.quorum/fix-876-572-quorum-receipt.json:31 + 6 other receipts + PR_BODY.md:17-28 (`check.rs:63`, `include_st`, `the sh`, `ratche`, `"Closes" ticket 1070`); 8 of 16 entries across 7 receipts truncated at 111-122 chars and further truncated/altered in PR_BODY.md
3.13	REFUTED	.quorum/feat-cloud-instance-ami-pin.json:3,16,38-40 `"issue": "PR #1097:..."`, `"quorum": {`, `"roles": ["judges"]` vs scripts/quorum-gate.sh:461 + scripts/quorum_evidence.py:186; all 12 receipts include 2 of 7 required fields (`issue`, `quorum`) and 1 of 4 required roles (`judges`), so "omit every required field and evidence role" is false
3.14	SURVIVED	.quorum/evidence/PR-1106-judge-1.json:21-24,159-162 `Defect as a whole not shown false: SURVIVED.` + .quorum/evidence/PR-1071-judge-2.json:165-168 + .quorum/fix-876-572-quorum-receipt.json:31-32 + .quorum/fix-894-pool-scale-decision.json:31; all three 2-of-3 splits drop the dissenting `SURVIVED` judge's reason and PR_BODY.md:17,28 omits the 2-of-3 split
3.15	SURVIVED	docs/roadmaps/roadmap.yaml:8022-8075 `status: planned` + scripts/dogfood/lib/window.sh:411-434 + scripts/dogfood/tagged.sh:461-467,481-495; title names PMAT-1166/1167/1168 so Gate T4 (missing `release:<next.tag>`) and Gate T7 (`status: planned`) fail
3.16	SURVIVED	scripts/dogfood/harness.sh:98-99,214-217 + scripts/dogfood/quorum.sh:88-91 + scripts/dogfood/release-check.sh:390-405; PR #1175 commits neither `docs/audits/impl-PMAT-1166-receipt.md` (nor 1167/1168) nor a `.quorum/<branch>.json` receipt for itself
3.17	SURVIVED	docs/roadmaps/roadmap.yaml:8022-8075 `acceptance_criteria: []`, `- kind:code`, `epic: 764` + :6353-6369 `GH-764` (`release: 1.35.0`) + :4417-4436 `PMAT-601` (`- kind:triage`) + PR_BODY.md:42,44,52
3.18	SURVIVED	.quorum/feat-cloud-instance-ami-pin.json:8-14 + docs/specifications/quorum-spec.md:197-199 + scripts/quorum-store-gate.sh:27-28,158-166 + scripts/dogfood/lib/receipt.sh:14-31; `judge-2`/`judge-3` checked 0 of 427 unchallenged claims across 24 files and `lib/receipt.sh` ignores `degraded`/`lane_models`
VERDICT: FAIL
text_only: yes
refuter: 3
reviewer_model: gemini-4-neon-high
