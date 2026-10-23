# crux lane — PMAT-770 r2

- GitHub Actions self-hosted runners: label sets select runners; omitting the arch label accepts either.
- GitLab Runner tags: an untagged job runs on any runner accepting untagged jobs.
- Buildkite agent tags and queues: a missing arch tag means any agent in the queue.
Verdict: the base choice is a branching question, not a crux row; stacking a PR on its epic is ordinary practice.
