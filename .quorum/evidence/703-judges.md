# Judges — forjar#703: quinn-proto 0.11.18 and its lock guard

Lanes and their findings: see `703-lanes.md`. Line numbers are at head
c98fb5c6. The test measurements ran on a build host (cargo 1.99.0) with
`cargo test --locked --test lock_quinn_proto_advisories`; the advisory scans
read the committed lock files and changed no tracked file.

## CONFIRMED

1. [lock] C1 — The lock moves quinn-proto from 0.11.17 to 0.11.18 and changes exactly the version and checksum lines of that one package entry.
   - evidence: `git diff 9a37a2ff c98fb5c6 -- Cargo.lock` is one hunk of two changed lines: `Cargo.lock:2746` reads `version = "0.11.18"` and `Cargo.lock:2748` the new checksum `a9746dbd...`. Nothing else in the lock moves.
2. [audit] C2 — With the branch's lock the three quinn-proto advisories are gone, and two independent advisory sources agree on it.
   - evidence: cargo-audit 0.22.1 over the RustSec database at 64a0302d (2026-10-13), `--no-fetch -f` on each committed lock: main's lock `error: 3 vulnerabilities found!` (quinn-proto 0.11.17: RUSTSEC-2026-0338, -0340, -0343); the lock at `Cargo.lock:2746` reports no vulnerabilities, only the 3 allowed warnings main already had. OSV.dev `v1/query` for crates.io quinn-proto: 0.11.17 returns those three ids, 0.11.18 returns none. All three advisories list `patched = [">= 0.11.18"]`.
3. [no-exemption] C3 — The fix is a version move and not a silenced finding: no ignore or exemption entry is added anywhere.
   - evidence: `git diff --name-only 9a37a2ff c98fb5c6` lists Cargo.lock, docs/roadmaps/roadmap.yaml and tests/lock_quinn_proto_advisories.rs only; deny.toml and .cargo/audit.toml are not in the diff. The one lock hunk is the version and checksum at `Cargo.lock:2746` and `Cargo.lock:2748`.
4. [discriminates] C4 — The guard measures the bump: with main's lock under the same test file it goes red, and with the branch's lock it is green.
   - evidence: at 4a57e71e (c98fb5c6 with Cargo.lock checked out from 9a37a2ff, never pushed) the target exits 101: `committed_lock_has_patched_quinn_proto` at `tests/lock_quinn_proto_advisories.rs:75` FAILED, panicking at `tests/lock_quinn_proto_advisories.rs:80` with `Cargo.lock pins quinn-proto ["0.11.17"]; versions below 0.11.18 are affected by ...`; 1 passed, 1 failed. At head c98fb5c6: 2 passed, 0 failed, rc 0.
5. [both-directions] C5 — The guard cannot pass vacuously: an absent entry fails as well as a vulnerable, pre-release or unparseable one, and a lock carrying both a fixed and a vulnerable entry fails.
   - evidence: `tests/lock_quinn_proto_advisories.rs:95` (0.11.17), `tests/lock_quinn_proto_advisories.rs:96` (0.11.18-rc.1), `tests/lock_quinn_proto_advisories.rs:97` (garbage), `tests/lock_quinn_proto_advisories.rs:98` (no quinn-proto entry) and `tests/lock_quinn_proto_advisories.rs:100` (0.11.18 beside 0.11.17) all assert `is_err()`; `guard_refuses_vulnerable_absent_and_prerelease` passed in both runs of C4.
6. [scope] C6 — The ticket row says cargo-deny stays red on unsafe-libyaml, and at this head that is the only advisory error left, so the PR claims no more than it clears.
   - evidence: cargo-deny 0.19.0, `--locked --offline check advisories` at c98fb5c6: `advisories FAILED` with one error, `error[unmaintained]: unsafe-libyaml is unmaintained` (RUSTSEC-2024-0449), and no quinn-proto row; the lock entry it no longer reports is `Cargo.lock:2746`.

## REFUTED

1. [scope-test] R1 — Earlier local round r2, claude-sonnet-5-5: "PMAT-703's acceptance criteria and the roadmap row ask for a lock bump with no other change. A new regression test file is something the ticket does not ask for, and no lane reviewed it."
   - corrected: criterion 1 bounds the Cargo.lock lines ("no other Cargo.lock line changes"), not the files, and C1 shows it holds. A lock-only bump is invisible to the rest of the suite, so `tests/lock_quinn_proto_advisories.rs:75` is the guard that keeps a later lock refresh from bringing 0.11.17 back; it reads the lock with std only and adds no dependency. In the round at c98fb5c6 every lane (sonnet, haiku, agy) reviewed the test and named it in its summary, and the planted control was caught through it.
