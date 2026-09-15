# PMAT-557 — instruments, and what each said

    bash scripts/dogfood/tagged.sh        on origin/main: GATE T FAIL, v1.30.0
                                          has no row. On this branch, in order:
                                          FAIL (the receipt had no end marker)
                                          -> FAIL (the copied cookbook commit
                                          locks 1.29.0) -> FAIL (PMAT-562 not
                                          labelled release:v1.31.0) -> PASS:
                                          8 tagged releases since v1.25.0,
                                          53 tickets carry their tag, 2 tickets
                                          from 2 PRs merged since v1.30.0 carry
                                          release:v1.31.0, due 2026-09-15T20:52:01Z
    bash scripts/release-goal.sh window v1.30.0   the row, byte-for-byte
    git for-each-ref refs/tags/v1.30.0    creatordate 2026-09-13T22:52:01+02:00
                                          = 20:52:01Z, the row's cut
    paiml/forjar-cookbook#21              forjar = "1.30", Cargo.lock 1.29.0 ->
                                          1.30.0 via cargo update --precise,
                                          cargo check --workspace --locked clean;
                                          squash-merged as 60acf9c9
    bash scripts/ratchets/comply-count.sh CB-2112 37 -> 34 (ceiling 35),
                                          CB-2114 34 (ceiling 34),
                                          CB-2115 53 -> 45 (ceiling 43; the rows
                                          for #560, #564, #565 land with their
                                          PRs and take it to 42)
    python3 yaml.safe_load roadmap.yaml   parses; 220 rows
    pmat work sync --check-only           the orphan list the minted rows answer
    kind-gate.sh (paiml-implement)        refuses: releases.yaml is outside the
                                          skill's triage rail (see the agy file)
    PRINT_HASH=1 bash scripts/quorum-gate.sh   the diff hash in the receipt
    per-row YAML comparison               yaml.safe_load of roadmap.yaml at
                                          origin/main and at HEAD, rows keyed by
                                          id, fields compared: 6 rows added,
                                          0 removed, 6 rows changed
                                          (PMAT-526/528/529/547/555/562),
                                          PMAT-545 and PMAT-527 identical — the
                                          instrument that settles which rows a
                                          hunk edits
