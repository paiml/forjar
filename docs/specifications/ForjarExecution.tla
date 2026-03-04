---------------------------- MODULE ForjarExecution ----------------------------
\* FJ-042: TLA+ specification of the forjar execution model.
\*
\* This specification model-checks the plan-apply protocol for:
\*   - Safety: no resource is applied out of dependency order
\*   - Liveness: all reachable resources eventually converge
\*   - Idempotency: applying a converged system is a no-op
\*   - Termination: the apply loop terminates in bounded steps
\*
\* Run with: tlc ForjarExecution.tla
\* Or use the TLA+ Toolbox / VS Code extension.
\*
\* Model parameters:
\*   RESOURCES: {"pkg-nginx", "file-config", "svc-nginx"}
\*   DEPENDENCIES: {<<"file-config", "pkg-nginx">>, <<"svc-nginx", "file-config">>}

EXTENDS Naturals, Sequences, FiniteSets, TLC

CONSTANTS
    RESOURCES,          \* Set of resource names
    DEPENDENCIES        \* Set of <<dependent, dependency>> pairs

VARIABLES
    state,              \* Function: resource -> {"pending", "converged", "failed"}
    applied,            \* Set of resources that have been applied in this round
    plan,               \* Sequence of resources in topological order
    phase               \* "planning" | "applying" | "done"

vars == <<state, applied, plan, phase>>

\* ---------- Type invariant ----------

TypeOK ==
    /\ state \in [RESOURCES -> {"pending", "converged", "failed"}]
    /\ applied \subseteq RESOURCES
    /\ phase \in {"planning", "applying", "done"}

\* ---------- Dependency helpers ----------

\* Resource r's dependencies (things r depends on)
DepsOf(r) == {d \in RESOURCES : <<r, d>> \in DEPENDENCIES}

\* A resource is ready when all its dependencies are converged
Ready(r) == \A d \in DepsOf(r) : state[d] = "converged"

\* ---------- Topological sort (simplified: pick any ready resource) ----------

PickReady == {r \in RESOURCES : r \notin applied /\ Ready(r) /\ state[r] # "converged"}

\* ---------- Initial state ----------

Init ==
    /\ state = [r \in RESOURCES |-> "pending"]
    /\ applied = {}
    /\ plan = <<>>
    /\ phase = "planning"

\* ---------- Planning phase ----------

PlanStep ==
    /\ phase = "planning"
    /\ phase' = "applying"
    /\ UNCHANGED <<state, applied, plan>>

\* ---------- Apply phase ----------

ApplyResource ==
    /\ phase = "applying"
    /\ PickReady # {}
    /\ \E r \in PickReady :
        /\ state' = [state EXCEPT ![r] = "converged"]
        /\ applied' = applied \cup {r}
        /\ UNCHANGED <<plan, phase>>

\* ---------- Completion ----------

Complete ==
    /\ phase = "applying"
    /\ PickReady = {}
    /\ phase' = "done"
    /\ UNCHANGED <<state, applied, plan>>

\* ---------- Idempotent re-apply (no-op when converged) ----------

IdempotentReapply ==
    /\ phase = "done"
    /\ \A r \in RESOURCES : state[r] = "converged"
    \* Re-applying is a no-op: state doesn't change
    /\ UNCHANGED vars

\* ---------- Next-state relation ----------

Next ==
    \/ PlanStep
    \/ ApplyResource
    \/ Complete
    \/ IdempotentReapply

\* ---------- Fairness ----------

Fairness == WF_vars(ApplyResource) /\ WF_vars(Complete)

Spec == Init /\ [][Next]_vars /\ Fairness

\* ---------- Safety properties ----------

\* S1: No resource is applied before its dependencies
SafetyDependencyOrder ==
    \A r \in applied : \A d \in DepsOf(r) : d \in applied

\* S2: Once converged, state does not regress to pending
SafetyNoRegression ==
    [][
        \A r \in RESOURCES :
            state[r] = "converged" => state'[r] = "converged"
    ]_state

\* ---------- Liveness properties ----------

\* L1: Every resource eventually converges (assuming no failures)
LivenessAllConverge ==
    <>(\A r \in RESOURCES : state[r] = "converged")

\* L2: The apply phase eventually completes
LivenessTermination ==
    <>(phase = "done")

\* ---------- Idempotency property ----------

\* I1: In done state with all converged, re-running apply produces no state change
IdempotencyProperty ==
    [](
        (phase = "done" /\ \A r \in RESOURCES : state[r] = "converged")
        => [][UNCHANGED state]_state
    )

================================================================================
