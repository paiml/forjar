# PMAT-542 — the agy round

One round, three quorum lanes, `writes=false`, sandboxed, dispatched in one
message. `out_dir` keyed by ticket AND session id, `--not-before` pinned to the
dispatch instant so a stale lane set cannot be reduced into a verdict.

Each lane got a different brief — is the selection sound; the workflow as GitHub
runs it; the tests and the documents number by number — and all three carried
the standing instruction to quote any sentence a reader could check and find
false. The round's most valuable finding came from that instruction: two lanes
quoted the same sentence about what the gates read, and both were right.

Two operational notes:

- **A FULL clone, not `--shared`.** The previous round's third lane could run no
  git command at all, because a shared clone's `objects/info/alternates` points
  into the parent repository and the sandbox does not reach it. A full clone of
  this repository is 168 MB and every lane here ran git without trouble.
- **The delegate hit its 30-turn limit before writing a receipt** — the tenth
  time in this session. All three `lane-*.json` were already on disk with
  `exit 0` and were read directly; nothing was taken from a summary that was
  never written.
