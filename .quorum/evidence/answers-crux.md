# Quorum evidence — PMAT-223 — CRUX

Two questions: in a world-class build system, is a cache hit decided from the inputs alone, and does a cache ever answer for a machine it never measured?

- **Make** — a target is rebuilt when it is MISSING or older than any prerequisite; the recipe text is not tracked, so a changed command over unchanged inputs does not rebuild, and `make -B` is the operator overriding the cache. That is the three-part shape this diff lands: outputs are part of the question, the command is not, and `--force` wins.
- **Bazel** — the action cache key carries the execution platform and every input digest; a cached result from one platform is never served to another, and a remote-cache hit whose outputs cannot be fetched is treated as a miss and re-executed rather than reported done. A digest is a fact about the tree it was computed from, and outputs are validated, not assumed.
- **Nix** — a derivation is keyed by `system`; a binary cache answers only for the system it built; and a store path is valid only while it is present — `nix-store --verify` rebuilds what is missing rather than trusting the database row that says it once existed.
- **ccache** — the hit is decided from the compiler, its arguments and the preprocessed source, and the object is served only if the cache can actually produce it; `CCACHE_RECACHE` forces a re-run. A memo that could not hand back the output would not be a hit.
- **Counter-example, rejected: an inputs-only memo** — a cache that answers `unchanged` from the input hash alone, never asking whether the output is still there and never asking which machine the hash was taken on. That is the shape forjar had, dead by accident of a mismatched base directory, and alive for exactly one measured run before this diff.

Verdict: accept (Make, Bazel, Nix and ccache all key a hit by where it was measured and validate the output before honouring it; the diff does the same and lets the operator override) and reject (the inputs-only memo, which reports done over a deleted artifact).
