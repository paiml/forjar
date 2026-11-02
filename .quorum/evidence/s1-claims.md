# Quorum evidence — S1 (fix/s1-contract-count-proofs-bash-e) — adjudicated claims

Round design: 3 claim lanes (claude-opus-5-5; roles code, contract and gate, adversary), each also refuting every claim, then 3 independent claude-opus-5-5 judges. The author is Sonnet 5.5, so no lane used its model. One planted control (C3) was injected and refuted 3 of 3 by lanes and 3 of 3 by judges.

## CONFIRMED — 2 claims survived refutation

1. [design] (C1) Under the shell `bash -e` that GitHub Actions runs `run:` steps with, the old line `OUT=$(pv validate "$c" 2>&1); EC=$?` exits the step at the failed assignment before `EC=$?` and before any output is printed, so a failing validate showed no reason; the new `&& EC=0 || EC=$?` keeps the step alive and records the exit code.
   - evidence: the changed line is .github/workflows/proofs.yml:346 and the comment above it, .github/workflows/proofs.yml:336, records that no CI job had run `pv validate` before, so the silent failure was never seen.

2. [design] (C2) blake3-state-v1 lists exactly 3 entries under proof_obligations, so the summary total of 3 now matches and pv rule VS-COUNT-001 passes; the old value 4 counted the four FALSIFY-B3 ids, which are falsification tests and not obligations.
   - evidence: the list starts at contracts/blake3-state-v1.yaml:61 and the corrected total is contracts/blake3-state-v1.yaml:155; the new falsifier tests/falsification_contract_summary_counts_match.rs:22 compares the two for every contract and fails on main's copy with the line "blake3-state-v1.yaml: total_obligations 4, proof_obligations 3".

## REFUTED — 1 claim did not survive

3. [planted] (C3) The diff changes Rust source or a workflow other than proofs.yml.
   - corrected: false. The diff touches proofs.yml, blake3-state-v1.yaml and the new test file only; no file under src/ and no other workflow is in it. The test file is a new addition that reads YAML and edits nothing.
