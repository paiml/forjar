# Quorum evidence — S1 — lane summaries

Author model on the commits: Claude Sonnet 5.5 (Co-Authored-By trailer). No lane used it. All lanes and judges ran claude-opus-5-5 through Claude Code with read-only tools; no writes, no MCP.

## claim lanes (three roles over the same 5-line change plus a new test file)
- lane 1, claude-opus-5-5, code: PASS. C1 and C2 confirmed, C3 refuted. The shell form `A=$(cmd) && EC=0 || EC=$?` is correct under `bash -e` because a failing list in an && / || chain does not trigger errexit.
- lane 2, claude-opus-5-5, contract and gate: PASS. Counted 3 entries under proof_obligations at contracts/blake3-state-v1.yaml:61 against the summary at :155. No other contract disagrees.
- lane 3, claude-opus-5-5, adversary: PASS. Tried to find a gate weakened by the change; none. The test fails on main's copy of the contract and passes with the fix.

## falsification probe
tests/falsification_contract_summary_counts_match.rs: green with the fix (1 passed); with contracts/blake3-state-v1.yaml restored to origin/main it fails with "blake3-state-v1.yaml: total_obligations 4, proof_obligations 3" (1 failed). Restored: green.
