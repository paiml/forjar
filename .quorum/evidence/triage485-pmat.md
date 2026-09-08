# Quorum evidence — PMAT-210 — mechanical lane

`pmat work validate`: passes. Every id named on this branch — PMAT-209, PMAT-210, PMAT-211 — resolves in `docs/roadmaps/roadmap.yaml`. One pre-existing warning, PMAT-200 has no acceptance criteria, which this branch neither introduces nor touches.

`analyze_vacuous_tests` over the touched paths: not applicable and not claimed. The branch adds no test, because it adds no code; the touched paths are two Markdown files and one YAML file.

## The probes, and their provenance

Binary under test resolved through `scripts/dogfood/lib/binary.sh`, reporting `forjar 1.26.0`, matching the manifest version. Control binary unpacked from `forjar-1.25.2-x86_64-unknown-linux-gnu.tar.gz`, downloaded from the v1.25.2 release, `sha256 ca633235aa95ac3c306ad1d78b6d24ede567c4b8953e722c3da739a44c6d75c4`, matching its line in that tag's `SHA256SUMS`; it reports `forjar 1.25.2`.

Every run: `plan -f <file> --state-dir <temp>` first, then `apply -f <same file> --state-dir <temp> --yes`, on `$(hostname)` only, against a temp state dir and a temp target path under the session scratchpad. No SSH. No host state written.

| probe | binary | content | apply | apply again | drift |
|---|---|---|---|---|---|
| A | 1.26.0 | literal block with `$HOME` | 1 converged, 0 unchanged, 0 failed | 0 converged, 1 unchanged | No drift detected |
| B | 1.26.0 | `{{params.user}}` and `${LD_LIBRARY_PATH:-}` inside the content | 1 converged, 0 unchanged, 0 failed | — | No drift detected |
| C | 1.25.2 | probe B's content | 1 converged, 0 unchanged, 0 failed | 0 converged, 1 unchanged | No drift detected |

One incidental measurement: `FORJAR_STATE_DIR` is not read. Set on the first attempt, forjar still resolved `state` relative to the cwd — the repository's own state directory. The apply refused there on that directory's integrity check and wrote nothing; `drift` read the host's real resources and reported them. `--state-dir` is the flag that works. Noted, not ticketed.
