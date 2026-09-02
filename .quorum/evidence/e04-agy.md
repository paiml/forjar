# Independent review — agy /teamwork (plan mode) — #406

Verdict as delivered: the branch "effectively intercepts the raw transport
streams and redacts resolved secrets before they reach the state directory",
but "the base64 redaction strategy is flawed against line-wrapped outputs,
and portions of the falsification suite can pass vacuously." Six claims put
to the reviewer; four confirmed, two charged.

TAKEN (changed the branch):

> "`no_state_file_contains_the_resolved_secret` lacks an assertion that
> transcripts were actually written … will fail to find a leak (passing
> vacuously)."

Correct for the sequential case — the `--parallel` twin already asserted a
non-empty transcript set. The sequential case now asserts it too.

REFUTED (did not survive the code):

> "`is_b64_char` does not allow newline characters, so if codegen line-wraps
> the base64 output, the secret is split across two chunks that decode
> independently and bypass the substring match."

codegen never line-wraps: `codegen::file` encodes with
`base64::engine::general_purpose::STANDARD` (no wrapping engine anywhere in
`src/resources` or `src/core/codegen`), and emits one `echo '<blob>' |
base64 -d`. A THIRD encoding produced by a task's own `base64 -w76` is a
real limit of redaction — which is what `sensitive: true` and the ciphertext
suppression exist for — and is recorded under known_limits, not fixed by
guessing every wrap width.

CONFIRMED BY THE REVIEWER (attacked and survived):

> "`capture_exec_output` intercepts and redacts the `script`, `stdout`, and
> `stderr` streams, which correctly propagates to the `.log`, `.json`, and
> `.script` files across both schedulers."

> "by using `serde_yaml_ng::to_string(resource)` the collector naturally
> serializes all populated fields (including flattened ones like
> `ArchiveSpec`) and perfectly mirrors the executor's template parsing."

> "`git add state ':(exclude)state/*/runs/*'` legitimately utilizes Git's
> pathspec syntax to exclude both tracked and untracked transcript files."

> "`capture_exec_output` short-circuits on `transcript.suppress` … but
> `meta.yaml` generation is driven separately via `update_meta_resource`,
> which retains the `ResourceRunStatus`."

Its CRUX conclusion placed the design ABOVE the industry default: Ansible,
Chef and Salt suppress only when the user remembers a flag; Terraform
redacts known values; forjar redacts what it can name AND suppresses what it
cannot, automatically for ciphertext.

Two moves landed after the review to get the branch through the repo's own
TDG hook rather than around it (`LifecycleRules` back beside its field; a
named `LogHeader` for the run-log writer); neither changes behaviour and
both are covered by the existing unit and falsification suites.
