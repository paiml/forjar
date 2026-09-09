# Quorum evidence — PMAT-220 — the claims put to the lanes

1. The `exec_script` exemption is sound: that call is unreachable for a namespace.
2. `controller_answers_for` is the right predicate for each of the three converted sites.
3. Excluding a namespace from the build-I/O probe does not break anything downstream.
4. The falsification rules are non-vacuous.
5. The table-driven output-verification case distinguishes the fix from the bug.
6. The diff does nothing the ticket does not ask for.

Claims 1, 2, 5 and 6 were confirmed by all three lanes. Claim 3 was refuted, and settling the lanes' disagreement about it located a defect older than this ticket. Claim 4 was refuted three separate ways, each of them correct.
