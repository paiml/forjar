# Independent review — agy /teamwork

From the 1.24.0 pre-release review, on this pair:

> "#390-B — the parallel path skips `output_verify::post_apply_failure` entirely,
> so identical configs give opposite verdicts. HIGHER SEVERITY THAN #390 ITSELF
> and I verified the blast radius is a category larger than the parallel-gap
> proposal claimed: `src/resources/task.rs:118-120` falls through to
> `verdict::always_diverged("task=pending")` for any task with no
> `completion_check` and no `output_artifacts`, so folding this in flips EVERY
> plain `type: task` sitting in a multi-resource wave."

And on the capture half:

> "THE FIX MUST ALSO CALL `update_run_meta`: `run_log_types.rs:71-80` shows
> `summary.failed` is incremented only by `RunMeta::record_resource`, and
> `src/cli/logs.rs:106` drops any run where `failures_only && meta.summary.failed
> == 0`, so capture alone leaves `forjar logs --failures` blind."

ACTED ON: both land in the same PR. `update_run_meta` is a named follow-up rather
than silently omitted — see the receipt's known_limits.
