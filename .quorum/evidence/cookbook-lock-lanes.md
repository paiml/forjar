# PMAT-537 — the lanes

One round, three sandboxed agy quorum lanes, width 3, `writes=false`, on the
diff at `50e8c8cb` with `--not-before` pinned to the dispatch instant.

| lane | conversation | exit | duration | verdict |
|---|---|---|---|---|
| 1 | conv-a99a2141 | 0 | 234s | FAIL |
| 2 | conv-3b67e241 | 0 | 357s | FAIL |
| 3 | conv-f46b801e | 0 | 389s | FAIL |

## Six for six, and still FAIL

Every numbered claim was confirmed by every lane — the parser attacked with
five malformed locks, the exactly-equal decision weighed against two
alternatives, the fail-closed directions probed for a hole, the cases checked
for vacuity, the cookbook bump verified from GitHub, and the after-the-tag
re-pointing judged on its merits.

All three still refused, on the standing instruction, for the same sentence:
**the receipt described a log file that did not contain what it said.** They
opened the file.

That is the fourth round in this release run where the instruction found what
the claims did not, and the sharpest instance: a change can be right in every
respect a reviewer was asked about and still ship a false sentence about
itself. A reviewer who checks the artifact rather than the argument is the only
one who catches it.

## The no-write rule

The brief opened with it and named the four earlier rounds in this session
whose lanes wrote into the repository root. The tree was clean afterwards.
