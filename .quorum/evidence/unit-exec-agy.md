# PMAT-560 — the agy rounds

    lane=quorum width=3 writes=false sandbox=true
    schema=quorum-lane-schema.json timeout=25m
    out_dir=.../paiml-implement/agy/PMAT-560/<session>/ph4   (round 1)
    out_dir=.../paiml-implement/agy/PMAT-560/<session>/ph4r2 (round 2)

Composed by the paiml-agy-delegate through `agy-lane.sh --repo-root
<checkout>`; each lane in a self-contained sandbox clone, tree witness
verified before, isolation asserted after.

Round 1: the brief named no mode, so the delegate composed `--mode plan`
(the one mode whose default schema is the lane schema the brief gave) and
recorded that choice. It hit its 30-turn cap before reducing; it was resumed
ONCE through SendMessage, read the three lane files already on disk, ran
`lane-reduce.sh`, and returned. No lane was relaunched.

Round 2: the brief named `mode=plan`, a turn budget, and a single
lane-group launch; the delegate returned in 14 tool uses. It noted that a
literal `sleep 120` poll exceeds the tool's 120 s timeout (one poll exited
143; the lanes were unaffected because `lane-group.sh` owns them).

Both rounds: `agreed=false, partial=true`, in each case because the 503 lane
was voided; round 1 also split FAIL/PASS among the counted lanes. Every
finding was re-executed or re-read on this host before anything was changed.
The operator interrupted the session once between rounds; nothing in flight
was lost.
