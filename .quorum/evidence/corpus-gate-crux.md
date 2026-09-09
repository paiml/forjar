# Quorum evidence — PMAT-217 — CRUX

This ticket changes no product behaviour, so the survey asks the question it actually raises: **when a project vendors a crate whose test suite needs data the host does not ship, what happens to those tests?** Three systems, all claims `[X]` — asserted from documentation memory, not measured against a running instance.

| system | what it does with tests that need absent data |
|---|---|
| Rust itself, vendoring via `cargo vendor` | vendored sources are built, not tested. The host runs its own suite and the vendored crate's tests are simply never invoked, so the question does not arise and nothing records that they were skipped [X] |
| Go, vendored modules under `vendor/` | `go test ./...` deliberately excludes `vendor/`, the same answer by exclusion rather than by decision [X] |
| Debian and similar distribution packaging | the closest analogue: a package that cannot run part of its suite disables it explicitly in `debian/rules`, and the disabling is in the packaging diff where a reviewer sees it. The reason is written next to the switch [X] |

**What forjar does, and how it differs.** It keeps the tests compiled and running on every build, gated behind a named feature, each printing its own reason, and it ratchets the count EXACTLY in both directions so an exclusion cannot be added or removed silently. That is stricter than all three: the first two make the question disappear, and the third records the decision but not the count.

The cost of being stricter is what this ticket paid. An exact figure in five places rots unless something checks it, and nothing did outside a full gate F run that cannot complete on this host at all (PMAT-216). The rule added here is that check. accept(keep the exact two-way ratchet; add the invariant that makes it survivable)

The distinction worth carrying to the next survey: none of the three surveyed systems has a notion of a test that decides whether to assert by looking at what else is on the disk, which is the defect this ticket removed. That shape appears to be specific to a monorepo-adjacent layout where siblings are sometimes present, and it is worth asking how Bazel or Nix workspaces prevent it.
