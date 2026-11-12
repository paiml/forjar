# Lanes — ResourceType Display spelling & cron block guards (PMAT-916 / forjar#943)

Two-round `LANES=3` review by `agy` (`Gemini 4 Carbon (thinking=high)`), distinct from the Claude author model:

- **Round 1 (`a4e28c3b`)**: `0/3 PASS` — Lane 1 & Lane 2 killed `intact_block_guard` (`src/resources/cron.rs:62`) for consuming adjacent `# forjar:<other>` / `#` / blank lines on truncated blocks and killed dead `== "Unknown"` branches (`src/cli/validate_deep.rs:106`, `src/cli/validate_quality.rs:16`) plus `CB-200` (`652 > 651`); Lane 3 demanded negative tests for mid-file truncated blocks.
- **Round 2 (`f7525fb6`)**:
  - **Lane 1 (Correctness & Hash Stability)**: `PASS` — verified `d==2{if($0~/^[ \t]*(#|$)/)bad=1;d=0;next}` in `src/resources/cron.rs:51`, removal of dead `"Unknown"` loops, and BLAKE3 `Debug` preservation in `src/cli/lineage.rs:168` and `src/cli/provenance.rs:199`.
  - **Lane 2 (Failure Modes & Ratchets)**: `PASS` — verified `crontab_check_guard` (`src/resources/cron.rs:37`), `fj445` (`10/10`), and `CB-200` restoration on `<host>`.
  - **Lane 3 (Falsification & Revert Proof)**: `PASS` — verified `resource_type_spelling` (`5/5`), `resource_type_cli_output` (`10/10`), and `FJ_943_REVERT_RC=101` on `<host>` (`<SCRATCH>`).
