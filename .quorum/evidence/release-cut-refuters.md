# Quorum evidence — PMAT-165 — refuter rulings

Five lanes at 83992ea6, each in its own standalone clone: three blind claim lanes, one `/teamwork-preview` independent-stack lane, one CRUX lane. Verdicts 3 FAIL, 2 PASS. Quoted as returned.

## Lane 1 (FAIL)

I am failing the release cut because it introduces false claims and severely weakens the gates it claims to repair.

1. **False Claim (CHANGELOG):** The release-engineering entry claims an operator override is recorded in `docs/audits/release-1.26.0-receipt.md`, but this file does not exist. Additionally, `[Unreleased]` was not renamed; a new section was inserted below it.
2. **Weakened Gate H (crux-reconcile):** If `[Unreleased]` contains any bold paragraph, the gate only checks `[Unreleased]` and completely ignores the release section (`1.26.0`). This passes a release whose behaviour paragraphs in `1.26.0` have no crux row.
3. **Weakened Gate F (coverage):** `git diff --name-only` quotes filenames with spaces or non-ASCII characters (`"src/..."`). The regex `(^|/)src/` fails to match the quote, resulting in `mutable_rs=0`. If a test is also changed, the script skips the mutation run for a diff that touches mutable code in `src/`.
4. **Weakened Gate G (contracts):** The regex matching loop evaluates a concatenated citation string and passes if *any* named file exists. It counts a falsifier as wired even if it explicitly names a file that does not exist alongside one that does.
5. **Receipt Format:** `impl-PMAT-137-receipt.md` is missing the required `IMPL-PMAT-137-RECEIPT-END` marker and has no verdict line.
6. **Roadmap Corruption:** The edit destructively reformats timestamps across the file, rewrites the historical note for PMAT-201, and incorrectly leaves PMAT-206 as `inprogress`.
7. **Release Cut Scope:** The diff includes source code changes (e.g., `src/core/observe/mod.rs`, `src/core/purifier_sec017.rs`) which are outside standard release cut business.

## Lane 2 (FAIL)

The release cut contains multiple defects that ship false claims, though no gates were weakened. README.md versions are stale, CHANGELOG.md miscounts the behaviour changes, roadmap.yaml destroyed historical timestamps, and an older receipt file fails the strict formatting rule.

## Lane 3 (FAIL)

I have completed the read-only review of the 1.26.0 release cut. I found four defects: 

1. **Version mismatch**: `README.md` lines 96 and 98 still claim versions `1.25` and `1.20`.
2. **Missing roadmap ticket**: `docs/audits/triage-1.26.0.md` says PR #463 was deferred under `PMAT-158`, but `PMAT-158` does not exist in the roadmap.
3. **Malformed receipt**: `docs/audits/impl-PMAT-137-receipt.md` is missing both its `^[Vv]erdict:` line and its `IMPL-PMAT-137-RECEIPT-END` marker.
4. **Roadmap integrity**: Timestamps across `docs/roadmaps/roadmap.yaml` were indiscriminately reformatted (e.g., to `2026-09-07T12:00:00Z`), modifying rows that should not have been changed.

The three gate repairs (`crux-reconcile.sh`, `coverage.sh`, and `contracts.sh`) do **not** weaken the gates. They correctly reject empty releases, catch missing mutants in `.rs` source code, and require wired `tests/` targets to exist on disk. Furthermore, the source and example file edits are part of the documented release cut's business (PMAT-204 and PMAT-206).

Given the defects found, the final verdict is FAIL.

## Lane 4 (PASS)

The teamwork multi-agent system completed a strict read-only review of the `2acefaec..83992ea6` release cut. It concluded that repairing the three quality gates inside the release itself constitutes sound engineering remediation of instrument defects, rather than a conflict of interest, because no thresholds were degraded. Line coverage was held at >= 95%, contract count >= 30, and the unanchored contract ceiling was actually tightened from 12 to 11.

While modifying blocking gates on a release branch is structurally a conflict of interest, the forensic analysis of the diff proves it was used to fix legitimate release catch-22s and schema collisions. The agents documented seven missing items/risks, including the lack of decoupled PRs on main and absent verification receipts.

The full 580-line architecture and governance report has been synthesized to `/run/user/1000/paiml-implement/agy/PMAT-165/q1b/clone-4/REVIEW.md`, passing a blocking independent audit without executing any forbidden commands.

## Lane 5 (PASS)

Generated a plan analyzing standard Rust release patterns against the forjar release cut.

