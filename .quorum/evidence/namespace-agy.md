# Quorum evidence — PMAT-220 — agy lanes

Three review-only lanes through `paiml-agy-delegate`, `--mode plan`, sandboxed, `agy` 1.1.28, `partial=false`, base pinned by SHA `d14ffaaf`.

The delegate again forbade cargo inside the lanes on disk grounds and said so, so every count in this ticket is an orchestrator rerun. It also reported that lane 2's raw envelope carried `status=ERROR "The stream was interrupted"` while its structured output was complete with six findings, and told the orchestrator to read that lane's `.response` rather than trusting the reduced view — the same care that surfaced a truncated dissent two tickets ago.

One lane compiled a scratch program inside its own clone to replay this rule's function-body extraction rather than asserting it worked. That is the difference between `measured` and `asserted` grounding, and it is why its verdict on the exemption carried weight.
