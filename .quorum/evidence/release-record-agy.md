# Quorum evidence — PMAT-231 — agy

- Version: agy 1.2.0; six sandboxed lanes over two rounds through the delegate's fixed calling form, writes=false, three per round.
- Conversations: conv-01453612,conv-f478c764 conv-2292ace1,conv-4f8f5c5f conv-0fe0e63a,conv-41808662.
- Round 1 on fcbc5b7e: 0/3 PASS — a rounded crates.io timestamp, a miscounted cancellation, and two roadmap rows a patching bug had left with `notes: null`. Round 2 on f2c5cb58: 1/3 PASS — all ten claims confirmed by two lanes, and two lanes independently answered the hostile-reader question with the same finding: the receipt's gate table paraphrased gate R instead of quoting it.
- Every lane measured the live release, crates.io, docs.rs and the workflow runs rather than reading the receipt back to itself, and each said plainly which command it did not re-run.
- Both delegate dispatches hit the 30-turn cap after their lanes had written their files; the orchestrator read `<out_dir>/lane-*.json` and ran `lane-reduce.sh` itself.
- The hostile-reader question is the part of the brief that earned its keep: the three round-1 findings were about numbers, and the two round-2 findings were about a receipt describing itself more kindly than the record supports. A receipt is the only artifact this branch produces, so those are its defects, not cosmetic notes.
- Residue: `git status --porcelain` unchanged around every round; no lane wrote to the repository.
