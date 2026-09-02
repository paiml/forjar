# Independent review — agy /teamwork

From the 1.24.0 pre-release review:

> "#390-C — DELIBERATELY NOT TAKEN HERE, and this is the single biggest thing I am
> leaving on the table for the reporter, who is on a stateless CI runner: writing
> that key persists the string into `state.lock.yaml`, which is re-serialised and
> blake3-sidecarred on every run and commonly committed. Doing it before every
> `record_failure` call site is capped is what made Proposal 2 FATAL … After this
> fix all six ARE capped, so #390-C becomes a safe four-line change. It must go
> with the stale-row half, because filling `details["error"]` on a data source
> that reprints resources this run never executed turns an obviously-contentless
> stale row into a convincing wrong one."

ACTED ON precisely: both halves in one change, and the bound is the precondition.
