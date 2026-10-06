# Judges — forjar#410: seal a sandbox's output in-process

Lanes and their findings: see `410-lanes.md`.

## CONFIRMED

1. [seal] C1 — `seal_output` hashes `$out` with the store's own content hash, renames it to `<store>/<hex>/content`, and re-hashes the moved entry, failing by name if the rename crosses filesystems or the hash moved.
   - evidence: the hash at `src/core/store/sandbox_seal.rs:47`, the address at `src/core/store/sandbox_seal.rs:49`, the move at `src/core/store/sandbox_seal.rs:66` and the re-hash at `src/core/store/sandbox_seal.rs:68`; the test at `tests/falsification_410_seal_output.rs:23` asserts the address, that `$out` is gone and that the entry hashes to its name.
2. [refusal] C2 — A symlink or special file anywhere under `$out` is refused before anything is hashed or moved, because the directory hash skips symlinks and would return a hash that does not cover the store entry.
   - evidence: the recursive walk at `src/core/store/sandbox_seal.rs:83` and the refusal at `src/core/store/sandbox_seal.rs:94`; the test at `tests/falsification_410_seal_output.rs:52` plants a symlink and asserts the store was not written; turning the refusal into a no-op turns that test red.
3. [existing] C3 — An entry already at the address is re-hashed: equal leaves `$out` in place and reports it already present; unequal refuses and overwrites nothing.
   - evidence: the branch at `src/core/store/sandbox_seal.rs:51` and the refusal at `src/core/store/sandbox_seal.rs:53`; the tests at `tests/falsification_410_seal_output.rs:72` and `tests/falsification_410_seal_output.rs:89`; trusting the entry without re-hashing it turns the tamper test red.
4. [plan] C4 — Plan steps 8 and 9 carry no shell command, name neither the missing binary nor a literal HASH path, and say NOT EXECUTED in both the sandbox plan and the derivation plan.
   - evidence: `command: None` at `src/core/store/sandbox_exec.rs:216` and `src/core/store/sandbox_exec.rs:225`, the derivation steps at `src/core/store/derivation_exec.rs:180` and `src/core/store/derivation_exec.rs:188`, and the command scan at `tests/falsification_e07_sandbox_is_real_or_honest.rs:116`; all lanes confirmed it.
5. [refuse] C5 — `execute_sandbox_plan` still refuses by name, and the reason now names seccomp-bpf as the one missing mechanism instead of a hashing binary that is no longer needed.
   - evidence: the refusal at `src/core/store/sandbox_run.rs:36` and the count of unavailable steps, now one, at `tests/falsification_e07_sandbox_is_real_or_honest.rs:97`; the gemini and haiku lanes named it explicitly.

## REFUTED

1. [ran] R1 — With the mechanism written, plan steps 8 and 9 run in-process as part of a sandbox build.
   - corrected: the round 1 sonnet lane showed nothing calls `seal_output` while `execute_sandbox_plan` refuses at `src/core/store/sandbox_run.rs:36`; the descriptions at `src/core/store/sandbox_exec.rs:225` now say NOT EXECUTED, and e07 asserts it for both steps.
2. [pin] R2 — The earlier e06/e07 receipt's pin of exactly two NOT EXECUTABLE steps still describes this tree.
   - corrected: the round 2 sonnet lane caught it; that receipt binds the earlier diff it reviewed, and the live test at `tests/falsification_e07_sandbox_is_real_or_honest.rs:97` now pins one unavailable step with the reason in its message.
