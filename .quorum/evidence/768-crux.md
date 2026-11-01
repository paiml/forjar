# crux lane — #768 / PR 771 — how other systems let a job choose its machine class

The question: should a CI job that is chip-independent name an architecture, or be allowed to land on any pool machine?

- GitHub Actions self-hosted runners: `runs-on` is a set of labels and a job lands on any runner carrying all of them; the platform adds `X64`/`ARM64` automatically and a workflow that omits them accepts either. This PR relies on exactly that.
- GitLab Runner tags: a job runs on a runner holding every tag it lists; an untagged job may run on any runner that accepts untagged jobs, so an unpinned job is allowed to land on any architecture and the pool owner must decide that deliberately.
- Buildkite agent queues and tags: a step targets a queue or agent tags; a missing architecture tag means any agent in the queue, and teams keep per-arch queues for artifact-producing steps while sharing a default queue for lint and test.
- Jenkins node labels: a job with no label expression runs anywhere; mixed-architecture fleets pin release jobs by label and leave verification jobs unpinned.

Verdict: the common practice is the one this PR adopts, with the pin kept on artifact producers and dropped on verification. What the other systems lack, and this PR adds, is a test that fails when an unpinned job is not on a written list, so a future job cannot become architecture-agnostic by omission.
