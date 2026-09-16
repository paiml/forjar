# PMAT-574 — the crux comparison this release owes

`docs/audits/crux-1.31.0.md`, three rows, one per behaviour bullet, each naming
at least three of the 28 systems `scripts/dogfood/crux-reconcile.sh` surveys.
Written by the release orchestrator from documentation memory rather than
surveyed by an `agy` lane as 1.29.0's was; every third-party claim is marked
`[X]` and the audit's Method section says so in its first paragraph.

## The three rows, and what each comparison found

1. **drift declines over an empty scope (PMAT-564)** — Bazel, Ansible,
   Terraform, Kubernetes. Bazel treats a target pattern matching nothing as an
   error `[X]` and modern Ansible refuses a `--limit` that matches no host
   `[X]`; `kubectl get` over an empty selection prints `No resources found` and
   exits 0 `[X]`, which is the shape forjar had. forjar's defect was worse than
   the Kubernetes shape, because the empty selection was not empty: the run
   graded two resources from a manifest it had never been given.

2. **a service is converged only while the loaded unit runs the declared
   program (PMAT-560)** — systemd, Ansible, Puppet, Chef, Podman. systemd itself
   distinguishes the file on disk from the unit it has loaded `[X]`; the
   configuration managers all converge on `state`/`enabled` and manage the unit
   FILE separately `[X]`, which is exactly the gap; the container world already
   identifies what runs by DIGEST `[X]`, which is what `exec_sha256` is — hashed
   at the LIVE path, never the declared one.

3. **drift exits 1 on any DRIFTED line (PMAT-562)** — Terraform, Puppet,
   Kubernetes, Ansible. `terraform plan -detailed-exitcode` exits 2 on changes
   `[X]`, Puppet's `--detailed-exitcodes` exits 2 `[X]`, `kubectl diff` exits 1
   `[X]`; Ansible's `--check` exits 0 and makes you parse the recap `[X]`. The
   finding: the two systems with an opt-in flag both default to "changes are not
   an error" for a PLAN, whereas forjar's `drift` is a CHECK, whose only reason
   to run is the question the exit code now answers.

## What the comparison cannot show

That it is correct in detail. No reference system was invoked; every `[X]` is a
documentation-memory claim. The gate agrees about itself: it asserts the
reconciliation was WRITTEN and is COMPLETE over the bullets, and catches the
cheapest failure — a behaviour shipped with no comparison at all.
