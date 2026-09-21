# agy lanes — the MSRV job left a toolchain override behind (PMAT-566)

Round count and heads: see `PMAT-566-lanes.md`.

`gemini-3.1-pro-high` and `gemini-3.1-pro-low`, `--sandbox`, JSON schema, the
commit messages and diff pasted inline. Both agy lanes confirmed the precedence
claim from rustup's documented order, and both refuted C4's unqualified wording.
agy-high's re-run added the one refutation the others missed: the commit's quote
showed the override was active but none of the failure it caused.

The first agy-high run is the lesson worth keeping. Asked to attack a claim about
tool behaviour, it tried to measure it — the right instinct — and the sandbox
denied the command and ended the turn with no verdict. A lane that can reason but
not run needs to be told so in the brief, or it spends its one turn on a tool call.
