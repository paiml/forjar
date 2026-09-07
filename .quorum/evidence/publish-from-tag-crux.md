# Crux lane — competitive survey — PMAT-165

One agy lane surveyed the field from documentation memory (sandboxed, no network); every third-party figure is [X] and asserted.

## Verdict (FAIL)

A competitive survey comparing forjar's publish-from-tag.sh script with 5 other systems was conducted and detailed in the artifact. The final verdict is FAIL because forjar relies on long-lived local credentials, violating the superior security pattern established by crates.io Trusted Publishing which uses short-lived OIDC tokens.

## Findings, as returned

- [asserted] — cargo publish --workspace handles dependency ordering natively and uses cargo's polling.
- [asserted] — cargo-release uses internal topo-sort and relies on cargo's polling.
- [asserted] — release-plz handles dependencies internally and uses its own index polling.
- [asserted] — cargo-workspaces (cargo ws publish) uses a fixed sleep interval.
- [asserted] — crates.io Trusted Publishing uses short-lived OIDC tokens.

## Orchestrator cross-check (asserted)

- `cargo publish --workspace` (cargo 1.90+): computes the publish order from the metadata graph and waits for the index between dependents; forjar's script does the same from `cargo metadata`, filtering dev-dependencies, and polls with `cargo info` rather than a fixed sleep.
- cargo-release: `cargo release publish` in dependency order with `publish.wait` polling the index; refuses a dirty tree unless `--allow-dirty`. Forjar refuses a dirty tree with no override at all.
- release-plz: publishes in order and polls the registry; runs in CI with a token. Forjar's publish is manual with the local credentials file by the operator directive.
- cargo-workspaces: `cargo ws publish` with a fixed `--publish-interval` sleep — the pattern forjar's bounded poll improves on.
- crates.io Trusted Publishing: short-lived OIDC tokens from GitHub Actions; the alternative the crux lane preferred and the operator rejected for this repository (no workflow publishes; no registry secret in GitHub).

The one deviation the lane scored as a FAIL is the credential location, which is the sanctioned path (D6); on ordering, index waiting and dirty-tree refusal forjar matches or exceeds every surveyed tool.
