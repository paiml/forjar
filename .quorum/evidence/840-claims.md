# Quorum evidence — #835 (fix/deny-advisories-2026-10, PMAT-840) — adjudicated claims

degraded: same-family. Every author of this branch is Claude (Sonnet 5.5 and Opus 5.5) and every non-Claude family is unavailable, so the operator's same-family rule applies: 3 claim lanes and 3 judges ran claude-haiku-4-5 through Claude Code with no tools, each reading the 20873-byte diff and the five claims inline. One planted control (C5) was refuted 3 of 3 by the lanes and 3 of 3 by the judges.

## CONFIRMED — 4 claims survived refutation

1. [design] (C1) Cargo.toml moves the direct dependency zstd from "0.13" to "0.14", and the lock now carries zstd 0.14.3 with zstd-safe 8.2.0 where 0.14.0 and 8.0.0 stood.
   - evidence: the manifest line is Cargo.toml:116 (`zstd = "0.14"`); the lock still holds a separate zstd 0.13.3 at Cargo.lock:5042 for batuta-common and the 0.14.3 entry at Cargo.lock:5051, so the two majors coexist as the ignore reasons say.

2. [design] (C2) deny.toml and .cargo/audit.toml each gain exactly 14 ignore entries, thirteen zstd advisories (RUSTSEC-2026-0391 and 0395 through 0406) plus RUSTSEC-2026-0374 for basic-toml, each with a reason naming its dependency path and the audit date, and no wildcard.
   - evidence: tests/falsification_840_advisory_ignores_are_bounded.rs:231 counts 23 ids in deny.toml and tests/falsification_840_advisory_ignores_are_bounded.rs:236 counts 24 in audit.toml (the deny set plus the legacy -0097), and all three lanes read the same 14 reasons in the diff.

3. [design] (C3) contracts/blake3-state-v1.yaml changes only verification_summary.total_obligations, from 4 to 3, a one-line edit.
   - evidence: the hunk header sits at line 152 of the contract and shows one removed and one added line; no other line of that file is in the diff.

4. [design] (C4) The diff adds tests/falsification_840_advisory_ignores_are_bounded.rs and a PMAT-840 row in docs/roadmaps/roadmap.yaml whose status is planned.
   - evidence: the test file is new (258 lines) and its first test is tests/falsification_840_advisory_ignores_are_bounded.rs:64; the roadmap hunk adds 19 lines at the end of the file with `status: planned`.

## REFUTED — 1 claim did not survive

5. [planted] (C5) The diff modifies production Rust code under src/.
   - corrected: false. The changed paths are .cargo/audit.toml, Cargo.lock, Cargo.toml, contracts/blake3-state-v1.yaml, deny.toml, docs/roadmaps/roadmap.yaml and the new file under tests/; nothing under src/ appears in the diff stat.
