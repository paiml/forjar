# Independent review — agy /teamwork-preview — PMAT-208

## Verdict (PASS)

The implemented repair is the pragmatic and correct choice. It technically papers over a deeper architectural coupling—using the GitHub Release itself as an intermediate artifact store rather than using native CI artifacts—but a fully decoupled design is not worth the cost here. A decoupled pipeline would pass the `SHA256SUMS` file between jobs using `actions/upload-artifact` and `actions/download-artifact`, keeping the build independent of the release platform until a final publishing job. However, PMAT-166 explicitly designed the workflow to stage assets in a draft release to easily save partial progress. The current fix gracefully bridges the gap by using the authenticated `gh` CLI to read from the draft, introducing minimal complexity while being fully verified by a new falsification test.

## What became of it

It named the deeper shape correctly: the workflow uses the GitHub release as a staging area, so a job that makes assets reads assets. It judged a decoupled design — pass SHA256SUMS between jobs as CI artifacts, create the release last — not worth the cost, because the draft-as-staging is what PMAT-166 deliberately designed. That judgement is recorded rather than acted on: this branch repairs the coupling's one broken edge and leaves the design question open for 1.27, where the CRUX lane's cargo-dist posture is the candidate.
