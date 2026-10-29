C1. check_cost_limit resolves the selection before planning (src/cli/dispatch_apply.rs:107).
C2. apply_execute builds the selectors from args and passes them (src/cli/dispatch_apply_b.rs:422).
C3. Integration tests drive the real binary with and without a selector (tests/falsification_cost_limit_honors_selection.rs:45).