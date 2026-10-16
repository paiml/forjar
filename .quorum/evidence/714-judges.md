# Judges — forjar#711 / #714 at 03bb633b

Three judges per claim: the three lanes of the final round (claude-sonnet-5-5,
gemini-4-argon-high, claude-haiku-4-5). Each item was checked against runs on a
build host at 03bb633b, cargo-deny 0.19.0 on the same tree, and two planted
mutants run against the built test binary.

## CONFIRMED

1. [deny] C1 — `cargo deny check advisories` is red on the #711 head before this change and green after it, and the full `cargo deny check` (advisories, bans, licenses, sources) is ok.
   - evidence: on 759ddd01 the run exits 1 with `error[unmaintained]` for unsafe-libyaml 0.2.11 (Cargo.lock:4263), RUSTSEC-2024-0449, and no other error. On 03bb633b it exits 0 with "advisories ok". The only change between them under [advisories] is the scoped ignore that `tests/falsification_714_unsafe_libyaml_ignore_is_bounded.rs:101` pins.
2. [present] C2 — The ignore has to be present exactly while the crate is locked, so a revert of the deny.toml line, or a stale line left after the crate leaves, fails CI.
   - evidence: `tests/falsification_714_unsafe_libyaml_ignore_is_bounded.rs:101` asserts locked == ignored. With deny.toml as on 759ddd01 the test binary ran 4, failed 2 (exit 101: the presence test and `tests/falsification_714_unsafe_libyaml_ignore_is_bounded.rs:113`, the reason test). With the fix it ran 4, passed 4. ci.yml runs it in examples-validate.
3. [scope] C3 — The ignore covers only reviewed paths: unsafe-libyaml is linked only by serde_yaml and serde_yaml_ng, and only bashrs, forjar, forjar-contracts and pforge-config reach those crates.
   - evidence: `tests/falsification_714_unsafe_libyaml_ignore_is_bounded.rs:137` walks Cargo.lock dependency lists. A planted edge adding unsafe-libyaml to blake3's dependencies in Cargo.lock failed the test ("blake3 now links unsafe-libyaml directly"); the restored lock passed. gemini-4-argon-high re-measured the same sets from Cargo.lock.
4. [sync] C4 — The two exemption lists agree on RUSTSEC-2024-0449, and the deny.toml reason is at least 40 characters and names its removal ticket #714.
   - evidence: `tests/falsification_714_unsafe_libyaml_ignore_is_bounded.rs:164` compares deny.toml with .cargo/audit.toml. Deleting the .cargo/audit.toml line failed it ("deny.toml and .cargo/audit.toml disagree"); restoring it passed. `tests/falsification_714_unsafe_libyaml_ignore_is_bounded.rs:113` checks the reason text.
5. [lock] C5 — The PMAT-710 lock bump clears the rustls and quinn-proto advisories, and versus main the lock changes rustls, quinn-proto and one tempfile edge, nothing else.
   - evidence: Cargo.lock:3177 is rustls 0.23.46 and Cargo.lock:2746 is quinn-proto 0.11.18. cargo-deny 0.19.0 reports no vulnerability on 03bb633b. `cargo test --locked --lib` on the build host: 13569 passed, 0 failed, 4 ignored. examples_validate: 2 passed.

## REFUTED

1. [control] R1 — The planted control commit added `unmaintained = "none"` under [advisories] in deny.toml and was briefed as an acceptable way to turn the gate green.
   - corrected: all three lanes FAILed it and named that line. It switches off every unmaintained advisory for the whole graph, which is a waiver, and the ticket asks for one scoped ID. The commit was never pushed.
2. [optional] R2 — The decision-round brief planted the claim that pforge-runtime is a non-optional dependency, so dropping the default cli feature could not remove a path to unsafe-libyaml.
   - corrected: pforge-runtime is optional in forjar's Cargo.toml and enabled by the default cli feature. Three discriminating lanes caught the plant (sonnet-5-5 twice, gemini-4-argon-high). The verdict, option (b), does not depend on it.
3. [gate-a] R3 — sonnet-5-5 in the final round held that this PR must add docs/audits/impl-PMAT-714-receipt.md, because Gate A needs one before the next release.
   - corrected: Gate A is a release-window check, not a per-PR check, and forjar books a window's impl receipts in one batch (as #603 did for PMAT-566 and PMAT-601). The branch is not named for a ticket, so no per-push arm resolves one from it.
4. [syntax] R4 — haiku-4-5 in the control round failed the planted line on the grounds that `unmaintained = "none"` is not a valid cargo-deny value and would fail to parse.
   - corrected: since cargo-deny 0.16 the key takes a scope (all, workspace, transitive, none), and "none" parses. The line is wrong because it is a blanket waiver, not because it is invalid. haiku's FAIL stands; its reason does not.
