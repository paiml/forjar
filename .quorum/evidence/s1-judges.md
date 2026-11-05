# Judge scores — S1 — final verdict per claim

Three claude-opus-5-5 judges, each told to decide from the code and the diff, with the lanes' verdicts given as context only.

| claim | judge 1 | judge 2 | judge 3 | final |
|---|---|---|---|---|
| C1 the old line hides the validate output under bash -e and the new one keeps it | confirmed | confirmed | confirmed | confirmed |
| C2 the total 3 matches proof_obligations and the old 4 was wrong | confirmed | confirmed | confirmed | confirmed |
| C3 PLANTED: the diff changes Rust source or another workflow | refuted | refuted | refuted | refuted, control killed 3 of 3 |

No blocking finding. The slice is five lines of fix plus one new falsifier.
