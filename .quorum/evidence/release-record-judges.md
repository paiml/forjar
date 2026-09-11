# Quorum evidence — PMAT-231 — adjudicated claims

Two rounds of three sandboxed lanes, base pinned at aff6be71: round 1 on fcbc5b7e (0/3 PASS), round 2 on f2c5cb58 (1/3 PASS). Every lane measured the live release, crates.io, docs.rs and the workflow runs rather than reading the receipt back to itself, and every lane was asked to read the receipt as a hostile reader. Five findings across the two rounds, every one of them a defect in the RECORD — which is the only thing this branch contains, and the reason the rounds were worth running.

Citations: `docs/audits/release-1.28.0-receipt.md` is added by this branch, so a line number in it resolves at HEAD; `docs/roadmaps/roadmap.yaml:3292` resolves at the merge base and is `- id: PMAT-226`, the cut this release closes.

## CONFIRMED

1. [identity] EVERY IDENTITY FACT IS MEASURED AND EXACT — the tag is annotated (object 931ee2eb, tagger date 2026-09-10T16:07:14Z, on cdcc0e80); the release is published, not a draft, not a prerelease, at 2026-09-10T23:56:51Z with 14 assets; crates.io reports `created_at 2026-09-10T16:12:45.938977Z`, 3,822,649 bytes, not yanked; docs.rs reports `doc_status: true`.
- evidence: all three round-2 lanes re-read every source live and checked every digit including the fractional seconds and the byte count, after round 1 refuted the truncated timestamp. `docs/audits/release-1.28.0-receipt.md:12` is the crates.io row and `:14` the release row.

2. [assets] THE 14 ASSETS ARE EXACTLY THE SIX TARBALLS, THEIR SIX SIDECARS, `SHA256SUMS` AND `install.sh` — no more, and nothing from another version.
- evidence: confirmed 3/3 in both rounds from `gh release view v1.28.0 --json assets`, against `docs/audits/release-1.28.0-receipt.md:14`, which names all fourteen. The fourteenth, `install.sh`, is the one the first tag run never produced, and its presence is the observable proof that PMAT-230's fix did what it claimed.

3. [gates] GATE R EXITS 0 ON THE PUBLISHED RELEASE AND THE PUBLISHED-ARTIFACT GATES PASS — `scripts/dogfood/release-check.sh` returns 0, and `make dogfood-published VERSION=1.28.0` reported `GATE C PASS` and `GATE D PASS` against the crate crates.io serves, not against this tree.
- evidence: `docs/audits/release-1.28.0-receipt.md:30` is the first line of the gate's own output as this receipt quotes it; every round-2 lane re-ran gate R in its own clone; each said plainly that it did NOT re-run `make dogfood-published`, which installs from crates.io and takes ten minutes, and judged that half from `docs/audits/logs/PMAT-231-release-check.log`. The orchestrator ran it (exit 0).

4. [window] THE WINDOW CENSUS IS THE LEDGER ROW — ten PRs and ten tickets, PMAT-215 through PMAT-226, every one carrying `release:v1.28.0` on its roadmap row before the cut.
- evidence: confirmed 3/3 in both rounds against `docs/roadmaps/releases.yaml` and the labels; `docs/roadmaps/roadmap.yaml:3292` is PMAT-226, the last of them and the cut itself.

5. [defects-described-accurately] THE THREE FILED DEFECTS ARE DESCRIBED AS THE RECORD SHOWS THEM — PMAT-232's un-draft guarded by `needs.create-release.outputs.created`, with the assertion behind the same guard; PMAT-233's tap step logging an empty `GH_TOKEN` while its siblings log `***`, reaching `Cloning into '/tmp/tap'` and dying on `Invalid username or token`; PMAT-234's verdict line claiming `pre-tag` while origin serves the tag.
- evidence: confirmed 3/3 in round 2 from `.github/workflows/release.yml` and `gh run view 34530301814`; the receipt's own summary of them is `docs/audits/release-1.28.0-receipt.md:47` onward. Each row quotes the output it was written from rather than summarising it.

6. [rail] THE DIFF IS ON THE TRIAGE RAIL — `docs/audits/**` and `docs/roadmaps/roadmap.yaml` only, with no file under `src/`, `tests/` or `.github/`.
- evidence: confirmed 3/3 in both rounds from `git diff --name-only`. The rail that admits it is the one PMAT-226 widened, and `docs/roadmaps/roadmap.yaml:3292` is that ticket.

## REFUTED

7. [rounded-timestamp] ROUND 1, 2 OF 3 LANES: the receipt gave crates.io's `created_at` as `2026-09-10T16:12:45Z`.
- evidence: the API returns `2026-09-10T16:12:45.938977Z`. Two lanes refused the claim on the fractional seconds; one accepted it as equal. The stricter reading is the right one for a receipt whose whole value is that its numbers can be checked.
- corrected: the receipt now carries the full value (`docs/audits/release-1.28.0-receipt.md:12`), and round 2 confirmed every digit.

8. [miscounted-cancellations] ROUND 1, 3 OF 3 LANES: the receipt said "the first tag run had four jobs cancelled at 16:53:40Z".
- evidence: the tag run's attempt 1 had THREE cancelled jobs; the four others belonged to the pull-request CI run (3) and its Proofs run (1). The lanes found this by querying `gh api repos/paiml/forjar/actions/runs/<id>/attempts/1/jobs` — `gh run view --json jobs` reports only the LATEST attempt, so after a re-run it shows zero cancellations and the evidence looks like it never happened.
- corrected: `docs/audits/release-1.28.0-receipt.md:70` now says seven jobs across three runs with the per-run split, and records how to see them at all, which is the more useful fact than either count.

9. [null-notes] ROUND 1, 3 OF 3 LANES: the claim that every new roadmap row carries an `orch-basis:` token and a populated `notes:` field was false — PMAT-231's and PMAT-232's rows carried `notes: null`.
- evidence: `docs/roadmaps/roadmap.yaml`, read row by row by three lanes. The row patcher matched the string `notes: null\n`; `notes` is the LAST line of a row, so the trailing newline belongs to the next row's start and the replacement silently did nothing while the surrounding edit succeeded.
- corrected: both rows carry their notes, and round 2 confirmed all four rows for label, kind and notes together.

10. [paraphrased-gate] ROUND 2, 2 OF 3 LANES, UNPROMPTED: the receipt's gate table said gate R reported "the tag is on origin, the release is published, crates.io and docs.rs serve it". That is what the gate MEANT. What it PRINTED was `GATE R PASS pre-tag: … PENDING until the tag is cut: no docs/audits/crux-1.28.0.md …`.
- evidence: `docs/audits/logs/PMAT-231-release-check.log`, which the receipt itself links. Two lanes independently called it out as making the gate look better than the record supports; this repository's own rule is that a receipt quotes the `GATE` line a script printed and never paraphrases it.
- corrected: the line is quoted in full in `docs/audits/release-1.28.0-receipt.md`, with the reasoning that shows its arms passed — arm 1 emits four PENDING notes when the tag is absent and none of them is present — while its wording is false, which is PMAT-234. A third lane read the same row the same way and refuted the claim that quoted the line in abbreviated form; the fix is the same quotation.

11. [flattened-by-hand-steps] ROUND 2, 1 OF 3 LANES, UNPROMPTED: the verdict line said two steps were done by hand and "both are named below with the defect that made them necessary". Only one was made necessary by a defect.
- evidence: the body of the receipt already said that promotion from prerelease to full release is the operator's step by design, quoting the workflow's own words; the verdict line lumped it with the un-drafting the workflow refused to do.
- corrected: `docs/audits/release-1.28.0-receipt.md:3` now names the two steps separately and says the difference matters. A lane reading only the verdict would have been misled, which is the reader the verdict line exists for.

## A lane error, recorded rather than dropped

Round 2 lane 3 also refuted the claim that all four new rows carry a populated `notes:` field, reporting that "several PMAT rows lack a notes field entirely". Re-run by the orchestrator at f2c5cb58: PMAT-231, 232, 233 and 234 each carry `kind:`, `release:v1.29.0` and a non-null `notes:`. Other, older rows in the file do lack notes, which is what the lane most likely read; the claim was about these four.
