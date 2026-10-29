# Quorum evidence — #794 (fix/tokio-rustsec-2026-0386) — adjudicated claims

Round design: 3 claim lanes (claude-opus-5-5; roles code, manifest side, adversary and waiver check), each also refuting every claim, then 3 independent claude-opus-5-5 judges. The author is Sonnet 5.5, so no lane used its model. One planted control (C3) was refuted 3 of 3 by lanes and 3 of 3 by judges.

## CONFIRMED — 2 claims survived refutation

1. [design] (C1) The Cargo.lock diff changes only the tokio package entry, its version 1.53.1 to 1.53.3 and its checksum, so no other crate version moves.
   - evidence: the two changed lines are Cargo.lock:3877 and Cargo.lock:3879, the dependencies block at Cargo.lock:3880 is unchanged, and the new checksum equals the sha256 of the cached tokio-1.53.3.crate (lane 3 and lane 1 compared it).

2. [design] (C2) tokio 1.53.3 is a patched version of RUSTSEC-2026-0386, so cargo-deny's unsound finding for the tokio entry no longer applies, and no deny.toml ignore was added.
   - evidence: the advisory file lists patched = [">= 1.51.6, < 1.52.0", ">= 1.53.3"] and Cargo.toml:105 requires tokio "1.35", which admits 1.53.3; the new falsifier tests/falsification_tokio_rustsec_2026_0386.rs:25 asserts the locked version is in the patched ranges.

## REFUTED — 1 claim did not survive

3. [planted] (C3) The diff modifies Rust source under src/ or deny.toml.
   - corrected: false. The only changed file besides the new test and the .quorum files is Cargo.lock (2 insertions, 2 deletions); git diff over src and deny.toml prints nothing.
