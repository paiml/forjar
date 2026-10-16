# Judges — forjar#415: plan --refresh consults the host before diffing

Lanes and their findings: see `415-lanes.md`.

## CONFIRMED

1. [refresh] C1 — `plan --refresh` runs the drift detectors per machine in scope and marks each entry they measured as changed `Drifted` in memory; the planner, unchanged, plans `Drifted` as an update.
   - evidence: the detector call at `src/cli/plan_refresh.rs:63`, the mark at `src/cli/plan_refresh.rs:79` and the call site at `src/cli/plan.rs:159`; `tests/falsification_415_plan_refresh_consults_the_host.rs:97` edits a file after apply and asserts plain plan has `to_update: 0` while `--refresh` has `to_update: 1`.
2. [write] C2 — A refreshed plan writes nothing: the lock bytes under the state dir and the file on the target are byte-identical before and after, so `drift` and `apply --refresh` stay the commands that change state.
   - evidence: `tests/falsification_415_plan_refresh_consults_the_host.rs:123` compares every byte under the state dir before and after the refreshed plan, and the next assertion reads the target back; the unreachable-host test does the same for its lock at `tests/falsification_415_plan_refresh_consults_the_host.rs:188`.
3. [unmeasured] C3 — A query the target did not answer is unmeasured, not drift: the entry is planned exactly as the lock-relative plan plans it, counted as unconsulted, and the disclosure names `forjar drift`.
   - evidence: the skip at `src/cli/plan_refresh.rs:75` and the disclosure at `src/cli/plan_refresh.rs:104`; `tests/falsification_415_plan_refresh_consults_the_host.rs:242` asserts the count at TEST-NET-3, and removing the skip turns the test red with `refresh.drifted` 1.
4. [seal] C4 — `plan --refresh --out` is refused by name, because a sealed plan is re-checked against the locks on disk, which a refresh never writes, so the file could only be rejected by `apply --plan-file`.
   - evidence: the refusal at `src/cli/plan.rs:139`, and `tests/falsification_415_plan_refresh_consults_the_host.rs:165` asserts the stderr names `plan --refresh --out` and that no plan file was written; the r9 sonnet lane and the agy lane both named the refusal in their verdicts.
5. [quantifier] C5 — The machine surface states which quantifier ran: `lock_relative` is false and a `refresh` object is present iff `--refresh` ran, and the lock-relative sentence is not printed on a refreshed plan.
   - evidence: `src/cli/plan_json.rs:81` and `src/cli/plan_json.rs:92`; `tests/falsification_415_plan_refresh_consults_the_host.rs:150` asserts the text output lacks the sentence that says the plan did not contact any machine.

## REFUTED

1. [scope] R1 — #415 asks for plan, check and --dry-run all to consult a host before they report, so a diff that delivers `plan --refresh` alone leaves two thirds of the ticket undone.
   - corrected: the issue's Fix and Success criterion name `plan --refresh` alone, quoted verbatim in the contract SCOPE note; `check` already runs its scripts on the target and `apply --dry-run --refresh` already refreshes first. PMAT-150 is retitled to that scope.
2. [census] R2 — Counting only the findings the target did not answer already covers every entry a refreshed plan still plans from the lock alone, so its disclosure names every blind spot the plan has.
   - corrected: observed entries a detector skipped were planned in silence; the census now counts the union at `src/tripwire/drift/census.rs:231`, and `src/tripwire/drift/tests_refresh_census.rs:56` pins it at 3, going red at 1 when the skipped arm is removed.
3. [wording] R3 — Every entry that a refreshed plan reports as not measured carries observed state in the lock, so the disclosure may say so in the same sentence that gives the count.
   - corrected: an unanswered query may carry only a locked content hash. The disclosure at `src/cli/plan_refresh.rs:110` drops the claim, and `tests/falsification_415_plan_refresh_consults_the_host.rs:252` asserts it is gone for exactly that entry.
4. [untested] R4 — The disclosure that fires iff something went unmeasured is already covered by the existing falsifiers, so the unconsulted path of a refreshed plan needs no test of its own.
   - corrected: no test reached it; `tests/falsification_415_plan_refresh_consults_the_host.rs:188` now drives the binary at an unroutable address and asserts the count, the disclosure and unchanged lock bytes, and it is red when the unmeasured skip is removed.
5. [binding] R5 — The contract binding for `print_plan_json` in `contracts/binding.yaml` still names the real signature of the function after this diff adds the refresh outcome to it.
   - corrected: the diff adds a fourth parameter, `refreshed`, and the binding said three; `contracts/binding.yaml` now names `refreshed: Option<&RefreshOutcome>`, as a sonnet lane asked in round 5.
