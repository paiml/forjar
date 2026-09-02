# Quorum evidence — #416 (CRUX audit E14) — adjudicated claims

## CONFIRMED — 7 claims survived refutation

1. [probe] (explains-symptom) `prove` counted UNKNOWN as passing and rendered it `[PASS]`.
   - evidence: src/cli/prove.rs:82 at base said it in its own words — "PROVED/CHECKED/UNKNOWN pass with the state in the detail" — and `passed` at src/cli/prove.rs:92 was `!(class == Hard && state == Falsified)`, so an unproven obligation counted toward "N/N proofs passed". Now `&& state != Unknown`, and the error names how many are UNKNOWN and how many FALSIFIED. Pinned by `prove_exits_nonzero_on_unknown` in tests/falsification_e14_claims_outrun_behaviour.rs, RED with the hunk reverted; the whole file is RED on main (0 of 3).

2. [probe] (explains-symptom) `provenance` labelled an unsigned, non-conformant attestation "SLSA Level 3", and the JSON payload carried an SLSA predicate type under the relabelled banner.
   - evidence: src/cli/provenance.rs:199 at base printed "SLSA Level 3 attestation chain"; the first cut relabelled the banner but src/cli/provenance.rs:149 still emitted `predicateType: https://slsa.dev/provenance/v1` — the agy lane caught that a consumer parsing the payload read the claim the text had just withdrawn. The predicate type is now forjar's own unsigned URI and the payload states `signed: false`, `slsa_level: null`. Pinned by `provenance_does_not_claim_slsa_level_3` (banner AND payload), RED with either reverted.

3. [design] Withdrawing `tripwire::chain` and `lock-audit-trail` was the smaller honest change, and no user lost a capability.
   - evidence: `append_event` never chained and src/tripwire/chain.rs had no production caller (src/tripwire/mod.rs:7 exported it to tests only), so `lock-audit-trail` reported on a chain that was never built — a security illusion, as the agy lane put it. Wiring the chain into the event-log write path is a product feature with its own design (what is the root, who verifies, what happens on a gap); it is #416's "either/or" and the smaller honest branch was taken [A]. `lock-history` remains the history verb; `lock-verify` and `lock-verify-sig` remain the tamper-evidence and authentication verbs, and the book says so where it used to teach the withdrawn one.

4. [agy] (explains-symptom) Deleting the verb left its `#[command(name = "lock-audit-trail")]` attribute attached to the NEXT variant, `LockRotateKeys`.
   - evidence: src/cli/commands/mod.rs:226 at base carried the attribute on its own variant; the first cut removed the variant and its args but not the attribute, so `LockRotateKeys` carried two `name` attributes. Taken: removed. Pinned by `lock_audit_trail_is_withdrawn`, which requires clap to reject the verb BY NAME — `Usage:` alone is what a still-present verb prints on a missing argument.

5. [agy] (explains-symptom) `prove -m m1` failed on another machine's UNKNOWN obligation once UNKNOWN failed the proof.
   - evidence: `structural_invariants` proved the WHOLE config regardless of the `-m` filter; harmless while UNKNOWN passed, a broken machine isolation the moment it did not. Taken: the scoped config is proved. Pinned by `prove_machine_filter_isolates_other_machines_unknowns` (m1 clean, m2 UNKNOWN: unscoped fails, `-m m1` passes), RED with the scoping reverted.

6. [design] The three pre-existing `fj1401` prove tests were bent to the defect, not to the fix, and now expect the honest outcome.
   - evidence: src/cli/tests_prove.rs:32, :110 and :148 at base asserted `is_ok()` over fixtures whose `package` obligations are UNKNOWN by construction — i.e. they asserted the [PASS]-on-UNKNOWN the ticket names. They now expect `Err` naming UNKNOWN. The agy lane put the question directly ("was that the honest outcome or a test bent to the fix?") and answered honest: a strict proof engine cannot pass an obligation it did not prove.

7. [design] The docs no longer claim what the code withdrew.
   - evidence: docs/book/src/07-cookbook.md no longer says "SLSA Level 3"; docs/book/src/01-getting-started.md no longer teaches `lock-audit-trail` and says why it is gone; the v2 quality spec's row 30 is relabelled. The remaining `SLSA Provenance Attestation` heading names the in-toto/SLSA STATEMENT SHAPE, not a conformance level, and sits above the line that says unsigned.

## REFUTED — 2 claims killed

1. [design] refuted 1/1 — Wiring `tripwire::chain` into the event-log write path is the fix #416 asks for.
   - corrected: #416 offers either wiring it in or removing what reports on it, and asks for the smaller honest change. Wiring in a tamper-evident chain needs a trust root, a verifier and a gap policy that the ticket does not specify; landing a chain without them would be a second "claim outrunning behaviour". The withdrawal is honest today; the chain is a feature ticket when someone wants it.

2. [design] refuted 1/1 — `[UNKNOWN]` should keep passing under `--json` so machine consumers see the state without a non-zero exit.
   - corrected: Terraform's `-detailed-exitcode` and Kani both fail the run on an undecided obligation and put the detail in the payload; an exit code that says "passed" over an unproven obligation is what a CI consumer cannot branch on. The state is in the JSON AND the exit is non-zero on every path — text, `--json`, and `-m` (now scoped).
