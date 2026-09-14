# PMAT-555 — the crux comparison

`docs/audits/crux-1.30.0.md` holds one row per behaviour bullet in the
`[1.30.0]` CHANGELOG section: nine rows, each naming at least three of the 28
systems `scripts/dogfood/crux-reconcile.sh` surveys.

`GATE H PASS 9 of 9 behaviour bullet(s) under [1.30.0] reconciled in
docs/audits/crux-1.30.0.md, each naming >= 3 of the 28 surveyed systems`.

## Provenance, stated rather than inherited

The 1.29.0 comparison was surveyed by an agy quorum lane. This one was written
by the release orchestrator directly, from documentation memory, with no network
access and no live invocation of any reference system. Every claim about a
third-party system is marked `[X]`.

That is a weaker source than an independent survey, and the audit's own Method
section says so in its first paragraph rather than reusing the previous
release's wording. The difference between "a lane surveyed this" and "the author
wrote it" is a provenance claim, and an unstated provenance claim is how a false
record starts.

## The finding worth carrying forward

Two of the nine rows found forjar on the wrong side of a unanimous convention.
Every drift-capable system surveyed — Ansible's `unreachable`, Kubernetes' node
`Unknown`, Terraform's refusal to plan against an unreachable provider — already
had the third state forjar lacked. And every build system surveyed distinguishes
a requirement from a lock: `flake.lock`, `MODULE.bazel.lock`,
`.terraform.lock.hcl`. forjar's gate T had been reading the requirement and
calling it evidence.
