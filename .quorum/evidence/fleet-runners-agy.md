# PMAT-547 — the agy round

    lane=quorum width=3 writes=false sandbox=true
    out_dir=.../paiml-implement/agy/PMAT-547/<session>/ph1
    not_before=<dispatch instant>  timeout=25m

The clone under review was a FULL clone, not `git clone --shared`: a shared
clone's `objects/info/alternates` names an object store outside the sandbox, so
every git command a lane runs fails. Measured on an earlier round this session
and recorded as a memory.

Every lane was briefed NO WRITES first: no edit, no create, no delete, no
`git checkout`, no `cargo build`/`cargo test`, no `cp -r` out of the clone. A
lane that believed it needed to write was told to say what it would write and
what the outcome would be, as a claim the orchestrator re-executes. That
constraint is visible in the results: five of lane 2's nine findings are
`asserted`, because a lane that cannot run the parser can only reason about it.
Four of those five were true.

    ran: true          rounds: 1        lanes_per_round: 3
    verdict: 3/3 FAIL — a suite this branch broke, four holes in its own parser,
             a glibc measurement of the wrong machine, and two numbers that were
             a script's report rather than a measurement

The round's value is concentrated in one finding no amount of re-reading the
diff would have produced: the branch broke
`tests/falsification_hosted_jobs_do_not_cache_target` by emptying its
denominator, and the orchestrator had run eight workflow-reading suites without
running that one. A lane that enumerates the suites rather than trusting the
list it was handed is what caught it.

Lane 1 was briefed to ask the question that actually decides this change —
"will these 33 jobs run on the fleet?" — and returned the cross-container glibc
finding, which is the only finding in the round that a green test suite could
never have surfaced, because the step it concerns runs only on a tag.
