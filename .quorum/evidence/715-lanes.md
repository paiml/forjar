# Lanes — forjar#715 / PMAT-698

One round, run by hand (quorum-review.sh was not used): three independent lanes on head 39fb79aa with the same brief and the same claims file, each read-only. The code was written by a claude-sonnet-5-5 session, and the only Claude lane is claude-opus-5-5, so no lane reviews its own model.

- agy lane 1: gemini-4-argon-high, via `agy --print --output-format json`, conversation a154c1c2 (full id in the receipt return value; scrubbed here). Answered C1-C4 CONFIRMED, P1 REFUTED. Its OVERALL line said FAIL, and that is only because P1, the planted claim, was refuted; it refuted no real claim.
- agy lane 2: gemini-3.8-flash-high, same tool, conversation f0f8eb5d. C1-C4 CONFIRMED, P1 REFUTED, OVERALL PASS.
- Claude lane: claude-opus-5-5 via `claude -p --model claude-opus-5-5` (the Agent tool was not available to the executing session), session 58a17227. C1-C4 CONFIRMED, P1 REFUTED, OVERALL PASS.
- A third agy lane (gemini-3.1-pro-high) was attempted first and never ran: agy rejected the model id as unknown, so it produced no verdict and is not counted.

agy was available and not quota-exhausted, so this round is not degraded same-family.
