# Lanes — `check` unmeasured exit 4 & `lock-prune` stack scoping (PMAT-659 / forjar#902)

Three-lane `LANES=3` review by `agy` (`Gemini 4 Carbon (thinking=high)`), distinct from the Claude author model, at head `2a3ac2e3f18ca39e4ae65ed873d9e1f638c89bd9`:

- **Lane 1 (Transport Exit-Code Classification & Aggregate Exit Contract)**: `PASS` — verified `src/cli/check.rs:140` guards `exit_code == 255` with `transport::is_ssh_transport(machine)`, preserves drift precedence (`fail` exit 1 ahead of `unmeasured` exit 4), and passes `falsification_659_check_unreachable_is_unmeasured` (`4/4`, `rc=101` on perturbation).
- **Lane 2 (`lock-prune` Pre-Write Atomicity & Shared-Machine Refusal)**: `PASS` — verified `src/cli/lock_core.rs:368` validates every declared machine via `scope::refuse_if_owned_by_other` before mutating any lock file (`falsification_661_lock_prune_scoped_to_config` `2/2`).
- **Lane 3 (Falsification Anti-Vacuity, JSON Schema & Gate A Traceability)**: `PASS` — verified `src/cli/check_json.rs:28`, `CHANGELOG.md:8`, `dogfood-guards` (`9/9`), and full `src/cli/lock_core.rs` revert kill (`FJ_902_REVERT_RC=101`) on `<host>` (`<SCRATCH>`).
