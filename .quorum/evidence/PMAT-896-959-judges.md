# Judges — Dead Python Splitter Removal (PMAT-896 / forjar#959)

Round count and heads: see `PMAT-896-959-lanes.md`. Every claim `C1..C4` and `R1..R2` met 3 independent refuters across Dead-Code Caller Graph, Failure Modes, and Gate A Contract Audit.

## CONFIRMED

1. [code] C1 — Root-level dead splitters `pmat_split.py` and `split_files.py` (`820` lines combined) have zero live callers across the repository.
   - evidence: `git grep -nE 'pmat_split|split_files'` at `103fa942d202dc525882c29e449166c28d925125` finds zero matches in `src/`, `scripts/`, `tests/`, `.github/`, `Makefile`, `Cargo.toml`, or `contracts/`, reducing tracked `*.py` files from `4` down to `2` (`scripts/bench_update.py` and `scripts/quorum_evidence.py`).

2. [diff] C2 — Gate-executing scripts, Rust sources, tests, and workflows are byte-identical to `origin/main` (`c4ce3a2c`).
   - evidence: `git diff c4ce3a2c5a4d770a79d4dfdcb6295cd438a9e7da...103fa942d202dc525882c29e449166c28d925125 -- src scripts tests .github Cargo.toml Cargo.lock` produces zero bytes of diff, confirming `scripts/cb200-ratchet.sh` and `scripts/ratchets/comply-count.sh` were cleanly split out per the coordinator ruling.

3. [test] C3 — `*.py` glob consumers and squash-merge `Pmat-Ticket:` trailer checks remain non-vacuous and consistent.
   - evidence: `scripts/cb200-ratchet.sh:54` continues scanning all `*.rs` sources plus the `2` remaining `scripts/*.py` files, and all 16 commits on `cf/896-python-ban-train` emit `Pmat-Ticket: PMAT-896` exclusively so `scripts/dogfood/harness.sh` (`PMAT-540` trailer arm) cannot mismatch after squash-merge.

4. [doc] C4 — `docs/roadmaps/roadmap.yaml:7130` and `docs/audits/impl-PMAT-896-receipt.md:1` satisfy Gate A and YAML schema invariants.
   - evidence: `dogfood_pr_tickets` resolves `PMAT-896` from PR `#959`'s body (`Ticket: PMAT-896`), `docs/audits/impl-PMAT-896-receipt.md:3` and `docs/audits/impl-PMAT-896-receipt.md:28` provide exactly one `verdict:` line and `IMPL-PMAT-896-RECEIPT-END`, and YAML validation on `yoga` (`/var/tmp/agy-fj959`) verified `377` valid roadmap entries.

## REFUTED

1. [scope] R1 — `PMAT-896` roadmap title and acceptance criteria span the full python-ban train, whereas PR `#959` ships only the dead-splitter deletion (`#958`).
   - corrected: `docs/roadmaps/roadmap.yaml:7134` sets `status: inprogress` rather than `completed`, and `docs/audits/impl-PMAT-896-receipt.md:3` records `verdict: PARTIAL`, explicitly deferring `#749` (`cb200-ratchet.sh`) and `#752` (`comply-count.sh`) to a separate proof-run PR from `feat/896-ratchet-ports-jq`.

2. [doc] R2 — `docs/audits/impl-PMAT-896-receipt.md:25` discloses that no Cargo target was compiled specifically for `103fa942` when the receipt was drafted.
   - corrected: All 3 lanes verified that zero bytes in `src/`, `scripts/`, `tests/`, `.github/`, `Cargo.toml`, or `Cargo.lock` differ from `origin/main` (`c4ce3a2c`), validated `docs/roadmaps/roadmap.yaml` parsing on `yoga`, and confirmed PR CI (`classify` green, remaining lanes queued) covers the branch head.
