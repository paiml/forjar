# PMAT-549 — the claims put to the rounds

forjar#549: `forjar drift` reported a host it could not reach as drifted. A
query that never came back (a transport error, or ssh's own exit 255) became a
`MISSING` finding. It was counted in `drift_count`, alerted on, and made
`--tripwire` exit 1. The branch gives an unanswered query its own state,
UNMEASURED:
- neither clean nor drifted;
- counted in the census and listed apart from `findings`;
- exit 4 (connection) under `--tripwire` when nothing else is wrong.

There were two rounds. In each, three sandboxed agy lanes reviewed a
standalone clone, then three judges ruled under a binding kill rule. Round one
reviewed 830ff969, the first commit. Round two reviewed 37096656, after the
fixes round one forced.

ROUND ONE — eight claims and the CRUX, lanes assigned 1-3, 4-6, 7-8.

1. The reader rule. `Err`, or exit 255 from an SSH transport, is UNMEASURED;
   any other exit status is the target's answer. A remote script that exits
   255 itself is read as unmeasured, which errs toward "not known", never
   toward clean.
2. Every FIRST query each detector makes goes through `unmeasured::read`. The
   directory listing's second query still mapped a failure to "no finding",
   and was disclosed as a residual.
3. The census invariant in_scope == inspected + skipped + unmeasured holds for
   every report the CLI and MCP produce, and no report is built without
   `DriftReport::new`.
4. CLI JSON keeps `drift_count == findings.len()` and adds
   `unmeasured_count == unmeasured.len()`. Text output never prints
   `No drift detected.` while anything is unmeasured. With drift and unmeasured
   both present, the tripwire error is the drift one.
5. The MCP verb lists unmeasured resources in `unmeasured` and `unchecked`,
   not in `findings`, and `drifted` is false when only unmeasured remain.
6. `apply` behaviour is unchanged: it consumes every finding, unmeasured ones
   included.
7. The tests discriminate. M1 to M4 were measured on the committed tree. Name
   any claimed property that no test would catch if it were reverted.
8. Compatibility. paiml/infra's drift-tripwire.sh "will now see unmeasured
   resources outside findings".

CRUX. Does UNMEASURED plus exit 4 match how monitoring and IaC systems
separate "could not measure" from "failed"?

ROUND TWO — the same subjects, re-worded to what the branch then claimed;
each lane given a focus and told to judge the rest more briefly.

C1. The reader rule, including a remote script that exits 255 over SSH.
C2. Every query a drift detector sends goes through `unmeasured::read`, the
    directory listing included. A refused listing is an ERROR finding and an
    unanswered one is UNMEASURED. No drift path, the lockless one included,
    turns either into "no finding".
C3. The census puts every in-scope resource in exactly one of inspected,
    skipped or unmeasured, for every report, the lockless path included. Every
    report is built by `DriftReport::new`.
C4. Round one's claim 4, with the tripwire exit for drift and unmeasured
    together stated as exit 1.
C5. Round one's claim 5. The E05 contract in src/mcp/types.rs already tells an
    agent to read `drifted` together with `unchecked`.
C6. `apply` still treats an unmeasured resource as observed drift and plans to
    reconcile it; only the printed detail changed. `--alert-cmd`,
    `policy.notify.on_drift` and `--auto-remediate` act on drift only.
C7. Every new or changed test fails if the behaviour it covers is reverted.
    Name any property that no test would catch.
C8. Every sentence of the CHANGELOG entry and of FALSIFY-VE-025 is true of the
    code, including the upgrade note for `--json` consumers.

Every lane was told to refute. The kill rule, binding on every judge:
- One verified counterexample kills a claim however many lanes confirmed it.
  It must be a cited line reproduced with `sed -n` at HEAD.
- An absence ("no test asserts X") is verified only by the judge's own search
  over all of src/ and tests/.
- A judgment question inside a claim is answered in the reason and does not
  decide the ruling.
