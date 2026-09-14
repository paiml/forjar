# PMAT-534 — the agy round

    lane=quorum width=3 writes=false sandbox=true
    out_dir=.../paiml-implement/agy/PMAT-534/<session>/ph1
    not_before=<dispatch instant>  timeout=25m

The clone under review was a FULL clone, not `git clone --shared`: a shared
clone's `objects/info/alternates` names an object store outside the sandbox, so
every git command a lane runs fails. Measured on an earlier round this session
and recorded as a memory.

Every lane was briefed NO WRITES first: no edit, no create, no delete, no
`git checkout`, no `cargo build`/`cargo test`, no `cp -r` out of the clone. A
lane that believed it needed to write was told to say what it would write and
what the outcome would be, as a claim the orchestrator re-executes. The clone
was clean after the round.

    ran: true          rounds: 1        lanes_per_round: 3
    verdict: 2/3 PASS, 1/3 FAIL — one real hole in the case set

The round's value was not its verdict. Two lanes passed the branch and one
found a guard that no case defended; re-measuring that finding is what turned up
the second, larger one, which no lane raised: the refusal ran before the arm
ever asked what the release IS.

A dispatch that had died on an API session limit earlier left the out_dir empty;
the round was re-dispatched with the same brief, clone and `not_before`, and
`lane-reduce` refuses a directory holding more lane JSONs than `--width` or
lanes older than `--not-before`, so a stale set could not have been read as this
round's.
