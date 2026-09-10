# Quorum evidence — PMAT-224 — CRUX

The question: in a world-class review system, what does a gate do with a change class it cannot measure the way it measures the others — refuse the shape, wave it through, or name a shape and say what it did not check?

- **Gerrit** — a change carries labels per review dimension (`Code-Review`, `Verified`); a project's submit rule names which labels a given change type needs, and a change that needs no `Verified` vote (a docs-only change under a `NoBlock` rule) is submittable WITHOUT one, with the missing label visible on the change page rather than silently satisfied. The rule is per kind, and the absence is displayed.
- **Bazel / Copybara-style monorepo presubmits** — a docs-only CL matches a path-based trigger set that runs no test targets; the presubmit reports "no affected targets" as a distinct result, never as "all tests passed". A class with nothing to run is reported as such.
- **Debian's NEW queue and `Rules-Requires-Root`** — a package that needs no build verification of a given kind declares it in the control file, and the tooling prints the declaration; an undeclared absence is a reject, a declared one is a pass with the declaration in the log.
- **Rust's own CI (`bors` / rust-lang)** — a PR labelled `rollup=always` or touching only `src/doc` runs a reduced set, and the reduced set is named in the merge commit's check list; a maintainer can see which jobs did not run.
- **Counter-example, rejected: the waiver** — a gate that accepts a free-text reason in place of a shape. It is reviewable, which is why it existed, but it teaches every author whose change does not fit that the reason field IS the gate. Ten waived receipts sit in `.quorum/` on main; the `CIT_RE` comment names the mechanism.

Verdict: accept (Gerrit, monorepo presubmits, Debian and bors all give a change class its own rule and PRINT what was not run) and reject (the waiver, which reports an unmeasured check with the same word as a measured one).
