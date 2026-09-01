# Independent review — agy /teamwork

The pre-release review of forjar 1.24.0 named this defect the highest-severity item
the #390 investigation turned up, called it security-adjacent, and recommended it
ship SEPARATELY and fast rather than bundled behind a message-formatting change:

> "#390-E — HIGHEST SEVERITY ON THIS LIST and security-adjacent; file it today and
> ship it separately — bundling a codegen correctness fix behind a message-formatting
> change is how it gets held up in review."

That recommendation is why this is its own branch and PR rather than a rider.

The same review confirmed 1.24.0 shipped a HEDGE rather than a fix:
`failure_text::nested_shell_caveat` refuses to claim "the command itself exited 0"
for any resource declaring `timeout:` or `sudo:`, and cites this issue. That hedge
becomes unnecessary once this lands, but is deliberately left in place: it is
correct for anyone still running 1.24.0.
