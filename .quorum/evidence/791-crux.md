# crux lane — #791 — how other systems pin a job to a machine class

The question: should a CI job on a mixed-architecture pool name its architecture?

- GitHub Actions self-hosted runners: runs-on is a label set and the runner adds X64 or ARM64 itself; a job that omits the label accepts either architecture, which R1 now forbids for the clean-room pool.
- GitLab Runner tags: an untagged job may land on any runner that accepts untagged jobs; pinning needs an explicit tag.
- Buildkite agent queues and tags: a missing architecture tag means any agent in the queue; teams keep per-arch queues for steps whose output depends on the chip.
- Jenkins node labels: a job with no label expression runs anywhere; mixed fleets pin by label.

Verdict: pinning every clean-room job to X64 is the conservative common practice; the guard that fails an unpinned job keeps the omission from returning silently.
