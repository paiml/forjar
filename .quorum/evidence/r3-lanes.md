# Quorum evidence - r3 - lane summaries

Author models: Sonnet 5.5 and Opus 5.5. No lane used either id. Lane brief: the r3 delta (440 diff lines) against origin/epic/forjar-img-001, four author claims (a) to (d) to refute, no tools.

## argon lane (agy, gemini-4-argon-high)
VERDICT PASS. Four findings, all low or medium and none blocking: multi-key runs-on is not a product in runner_legs (latent, no workflow uses it); no test guards cache keys; stale_in narrows to fleet legs with an arch label; pinned check names keep X64 on a leg that no longer pins it (deliberate, commented). Claims (a) to (c) held; (d) held structurally.

## flash lane (agy, gemini-3.8-flash-high)
VERDICT PASS. Three low findings: non-Cartesian runner_legs, duplicate values when include and a top-level dimension share a key (benign, idempotent), ARCHES constant duplicated in a message. All four author claims held.

## sonnet5 lane (Claude Code, claude-sonnet-5, fallback because the gpt-oss agy bucket returned 429)
VERDICT PASS. Six findings, one medium: the check strings bake X64 into names for legs that no longer pin it (intentional). Claims (a) to (c) held; (d) not falsifiable from the diff alone.

## opus5 lane (Claude Code, claude-opus-5)
VERDICT FAIL on nine findings. Three were adjudicated away by measurement: the empty features name worry (check-runs of main match), restore-keys in four files (none exist), fail-open runs-on (every templated runs-on has a matrix). Residual accepted as follow-ups: a guard that every include leg defines check, and a product expansion in runner_legs. The FAIL was overturned by the judges, not ignored.

## Planted controls
1. Planted defect: the diff was altered so ci.yml, proofs.yml and the lint.yml matrix legs read runs-on [self-hosted] with the clean-room pool label removed. The argon lane returned VERDICT FAIL and named the dropped clean-room label in ci.yml and proofs.yml as its first finding. The detector fires.
2. Falsification mutations: reverting the two workflow files made one real-workflow test fail; flattening the legs and deleting the hosted-runner branch made both new fixture tests fail. All restored; 17 of 17 green.
