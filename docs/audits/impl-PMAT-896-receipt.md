# Implementation receipt — PMAT-896 — python-ban train, part 1: dead splitters deleted

verdict: PARTIAL — the two dead Python splitters are deleted; the jq port of the two ratchet scripts is split out of this PR and is not landed.

## Identity

- ticket PMAT-896; kind: code; PR #959, branch `cf/896-python-ban-train`, merged with `origin/main` @ c4ce3a2c
- files changed against main: `pmat_split.py` (deleted, 415 lines), `split_files.py` (deleted, 405 lines), this receipt, the PMAT-896 roadmap row
- children: forjar#958 (dead splitters, this PR); forjar#749 (cb200-ratchet) and forjar#752 (comply-count) stay open

## Scope change (coordinator ruling, 2026-11-12 10:59Z)

The PR used to also port `scripts/cb200-ratchet.sh` and `scripts/ratchets/comply-count.sh` from Python to jq.
Those two scripts are run by CI, so the port changes what a gate accepts. It moves to its own PR, which must first
show a proof run: the ported scripts pass on the last shipped release and fail on a planted defect.
In this PR the two scripts and their two test files are byte-identical to main (`git diff origin/main -- scripts tests` is empty).
The port, its fail-closed fix from review round 1 and its tests are kept on the local branch `feat/896-ratchet-ports-jq` (f9116ff6).

## Evidence (lambda, git only; nothing compiled)

- `git diff --stat origin/main`: 4 files, the two deletions plus this receipt and the roadmap row.
- Files outside `.pmat-work/`, `docs/` and `CHANGELOG.md` that name `pmat_split` or `split_files`: 0 (`git grep -lE`). The names remain only in historical work records.
- Tracked `*.py` files: 4 on main, 2 on this branch (`git ls-files '*.py'`).

Not measured: any cargo target on this tree (no Rust source, script or workflow differs from main); CI on this head; the clean-room.
Open: the ratchet-port PR with its proof run (forjar#749, forjar#752).

IMPL-PMAT-896-RECEIPT-END
