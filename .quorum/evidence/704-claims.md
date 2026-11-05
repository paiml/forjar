# Claims — forjar#704: apply --dry-run previews the drift repairs the apply makes

Briefed to every lane with the ticket row and the full diff against main
9a37a2ff; head 302123a9. The fix commit carries `closes #704`.

- C1: one function, `mark_drifted`, turns a drift finding into planner input; the real apply persists its result and the dry run only plans with it.
- C2: `apply --dry-run` overlays the drift gate's findings on in-memory locks (`overlay_drift`) and still writes no lock (#310).
- C3: the ticket's acceptance test: after a tampered depends_on prerequisite, the `-r leaf` dry run lists it as Update, a second dry run plans the same, and a real `apply -r leaf` restores it.
- C4: the case table asserts, per row, that the dry run lists exactly the set the real apply then converges, read from the apply's own events.
- C5: the binary-driven falsifier reads only what a user reads (the built forjar's dry-run stdout, text and --json, and the state-dir bytes) and discriminates: with the `overlay_drift` call removed, rows a, b, d and e go red and c stays green.
- C6: the remaining hunks are plumbing: the gate's findings are passed to the dry-run output, and four existing tests gain the new argument.
