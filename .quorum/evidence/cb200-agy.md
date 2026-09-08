# Independent review — agy /teamwork-preview — PMAT-206

## Verdict (FAIL)

The proposed fix hides the staleness rather than surfacing it. By silently wiping the cache when `pmat comply check` fails to invalidate it, the CI gate masks a tool defect. While this automatic refresh costs an operator zero time in CI (as the build passes), it leaves the tool broken for local developers and doubles the gate's execution time by running the check twice. Refusing with UNMEASURED would be better: it would cost an operator immediate time to unblock CI (by manually wiping the cache or fixing the tool), but it would preserve the integrity of the gate and force the underlying bug to be addressed. 

Furthermore, the gate is now significantly weaker because it uses an unanchored regex (`grep -oE '/[^ \"]*/comply/index/[^ \"/]+'`) against the entire JSON output. Since the JSON includes source code snippets for violations, any source file containing a matching path string will cause the gate to run `rm -rf` against that path. The subsequent `case` statement safeguard is a tautology since the regex already guarantees the string contains `/comply/index/`.

## What became of it

It failed the first rewrite on two counts and both were acted on: the regex over comply's JSON that fed `rm -rf` is gone, replaced by a directory found by name under the cache root; and the staleness is now printed as a NOTE before the cache is removed, so the tool defect is surfaced rather than hidden. Its preference for refusing with UNMEASURED instead of refreshing was weighed and not taken: a gate that refuses on every CI run whose cache predates the checkout is a gate that gets bypassed; a gate that says the cache was stale, removes it and measures the tree is both true and actionable.
