# Quorum evidence — PMAT-222 — CRUX

The question: in a world-class system, can a measurement taken on one machine ever answer for another?

- **Bazel remote execution** — an action's cache key includes the execution platform's properties and the digests of every input as they would be on THAT platform; a cached result from one platform is never served to another, and the protocol carries the platform in the request. A digest is a fact about the tree it was computed from, and the key says which tree. That is the (machine, resource) key.
- **Nix** — a derivation is keyed by `system`; `x86_64-linux` and `aarch64-darwin` builds of the same expression are different store paths, and a binary cache answers only for the system it was built on. The same input hash under a different system is a different question, and Nix refuses to conflate them.
- **Ansible** — facts are gathered per host and live in `hostvars[inventory_hostname]`; a task on host B never reads host A's `ansible_facts` unless the play says so by name. A per-host fact is keyed by the host it was measured on, which is what this diff makes the probe do.
- **Counter-example, rejected: Make** — one process, one tree; `make` has no notion of a remote target and answers every rule from the filesystem in front of it. That is exactly the shape forjar had by accident: a controller-local hash presented as the state of a remote row.

Verdict: accept (Bazel, Nix, Ansible key a measurement by where it was taken; the diff does the same and names the rows that have no measurement) and reject (Make's single-tree answer for a multi-machine question).
