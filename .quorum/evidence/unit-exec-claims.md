# PMAT-560 — the claims put to the round

A `service` resource asked the host `is-active` and `is-enabled` and nothing
else. On yoga, 2026-09-15, both were true of `github-runner-ephemeral.service`
while the unit systemd had loaded ran `run-ephemeral-docker-v3.sh`, a file the
repository never declared, and the check reported converged.

The branch adds two optional fields, `exec_start` and `exec_sha256`, and a
fragment module that asks the MANAGER (`systemctl show -p ExecStart --value`)
which program the loaded unit starts and hashes the file at that LIVE path —
never the declared one. The fragments compose into the check (exit code), the
apply (the resource fails) and the state query (drift sees a swap). A service
declaring neither emits nothing new.

Two rounds of three sandboxed agy quorum lanes, review-only, against
self-contained clones, `--not-before` pinned, `out_dir` keyed by ticket AND
session id.

Round 1, at `8337cab5`, asked six things:

1. Does the emitted shell read the manager and hash the live path; could any
   path hash the declared one; could the awk misread a path with `;` or a
   space, or a unit with several `ExecStart=` lines?
2. Under the apply script's `set -euo pipefail`, is there an input that
   aborts with no marker, or one that exits 0 over the wrong program?
3. Undeclared services: nothing new in check, apply AND state query?
4. The golden-hash repin: is the CHANGELOG upgrade note accurate?
5. Does every contract falsifier cite a test its named mutation turns red?
6. Quote any sentence in the CHANGELOG, contract or book a reader could check
   and find false.

Round 2, at `921ed4b6`, re-asked 1, 2, 5 and 6 over the fixes round 1 forced
(the awk separator, the stderr marker, four contract and two book sentences)
and added the files round 1 did not read: `lifecycle_rules.rs`,
`known_fields.rs`, the resolver, recipe expansion and `observe`.
