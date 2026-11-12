# PMAT-896 (`forjar#959`) — Extracted Claim Manifest (`C1..C4`, `R1..R2`)

Base commit: `c4ce3a2c5a4d770a79d4dfdcb6295cd438a9e7da`
Head commit: `103fa942d202dc525882c29e449166c28d925125`
Branch: `cf/896-python-ban-train`

1. **C1 (`[code]`)**: Root-level ad-hoc splitters `pmat_split.py` (`415` lines) and `split_files.py` (`405` lines) are deleted with zero live callers across `src/`, `scripts/`, `tests/`, `.github/`, `Makefile`, `Cargo.toml`, and `contracts/`, reducing tracked `*.py` files from `4` to `2` (`scripts/bench_update.py`, `scripts/quorum_evidence.py`).
2. **C2 (`[diff]`)**: Following the coordinator scope split (`2026-11-12 10:59Z`), `src/`, `scripts/`, `tests/`, `.github/`, `Cargo.toml`, and `Cargo.lock` are byte-identical to `origin/main` (`c4ce3a2c`), keeping `scripts/cb200-ratchet.sh` and `scripts/ratchets/comply-count.sh` untouched in this PR.
3. **C3 (`[test]`)**: `scripts/cb200-ratchet.sh:54` (`git ls-files -z -- '*.rs' '*.py'`) remains non-vacuous after deleting `pmat_split.py` and `split_files.py`, and all 16 branch commits carry `Pmat-Ticket: PMAT-896` exclusively so squash-merge trailer resolution in `scripts/dogfood/harness.sh` stays aligned.
4. **C4 (`[doc]`)**: `docs/roadmaps/roadmap.yaml:7130-7151` (`PMAT-896`, `status: inprogress`) and `docs/audits/impl-PMAT-896-receipt.md:1-28` (`verdict: PARTIAL`, `IMPL-PMAT-896-RECEIPT-END`) satisfy Gate A ticket resolution (`dogfood_pr_tickets` via PR `#959` body) and YAML schema validation (`377` rows verified on `yoga`).
5. **R1 (`[scope]`)**: Although `docs/roadmaps/roadmap.yaml:7133,7141` titles `PMAT-896` as porting `cb200-ratchet.sh` and `comply-count.sh` to `jq`, `#959` lands only `#958` (dead splitter removal) and leaves `#749` and `#752` open on `feat/896-ratchet-ports-jq` pending a separate proof-run PR.
6. **R2 (`[doc]`)**: `docs/audits/impl-PMAT-896-receipt.md:25` records that no Cargo test target or clean-room CI run had yet completed at `103fa942` when written, relying on zero-byte diff identity against `origin/main` across `src/`, `scripts/`, `tests/`, and `.github/`.
