# PMAT-540 — the agy round

One round, three quorum lanes, `writes=false`, sandboxed, against a FULL
standalone clone, dispatched in one message with `out_dir` keyed by ticket AND
session id and `--not-before` pinned to the dispatch instant.

Each lane had a different brief — the arm and the floor; the cases and the
fixture; the documents and the numbers — and all three carried the standing
instruction to quote any sentence a reader could check and find false.

The instruction earned the round again. Lane 2 quoted *"a squash message ends
with whatever bullets GitHub assembled"*, read the real message, and found the
`---------` separator and the `Co-authored-by:` block that GitHub actually
appends. Following that one sentence to the fixture is what uncovered the real
defect: the fixture did not reproduce the shape, so git parsed its trailers and
no case could have caught an arm rewritten to use git's parser.

The delegate hit its 30-turn limit before writing a receipt — the eleventh time
in this session, filed as paiml-implement#141 with the measured shape and four
suggestions. All three lane JSONs were on disk with `status: SUCCESS` and were
read directly; nothing was taken from a summary that was never written.
