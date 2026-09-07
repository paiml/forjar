# Crux lane — PMAT-162 — the field on generations, restore and undo

The CRUX panel for 1.26.0 (three lanes, conv-e1814b17, conv-67822dca, conv-5ace27f8; documentation memory, no network, every third-party figure [X]) surveyed Nix profile generations, Kubernetes rollout history and undo, Terraform state locking and workspaces, and Pulumi per-stack state for behaviour 7 and asked what the spec should copy. Majority: Nix profile generations (generations record their owner; undo replays only the invoking stack's subset); dissent: Kubernetes rollout history (lineages attached to the stack identity). The spec takes both — storage is an append-only log with parent pointers (Kubernetes-shaped), navigation is a per-lineage pointer that walks back (Nix-shaped).

## Panel lane 1 (FAIL)

ecific stack's generation history) |
| (8) outputs merge per stack | Terraform workspaces | Outputs are per workspace state file [X] | Outputs merge per stack in shared lock [V] | forjar merges within a shared state file | reject(merging outputs per stack provides isolation in single file) |
| (8) outputs merge per stack | Pulumi per-stack state | Stack outputs isolated per stack [X] | Outputs merge per stack in shared lock [V] | forjar merges within a shared state file | reject(merging outputs per stack provides isolation in single file) |
| (8) outputs merge per stack | Kubernetes | No global outputs map, resources are isolated [X] | Outputs merge per stack in shared lock [V] | N/A | reject(merging outputs per stack provides isolation in single file) |
| (9) status attributes machine | Kubernetes | ownerReferences attribute objects to controllers [X] | Attributes machine to stack that wrote it [V] | Matches Kubernetes | reject(owner attribution is standard for shared infrastructure) |
| (9) status attributes machine | Pulumi | Stack resources tracked per stack [X] | Attributes machine to stack that wrote it [V] | Matches Pulumi | reject(owner attribution is standard for shared infrastructure) |
| (9) status attributes machine | Terraform | State file tracks which workspace owns resource [X] | Attributes machine to stack that wrote it [V] | Matches Terraform | reject(owner attribution is standard for shared infrastructure) |

**GO-Spec Finding (Behaviour 7):**
The GO (generation ownership) spec should copy Nix's per-profile generations and Pulumi's per-stack states. Specifically, generations must record their owning stack ID, and `undo` must only restore the subset of the generation snapshot that belongs to the invoking stack, leaving other stacks' resources untouched.

## Panel lane 2 (FAIL)

do` targets a specific Deployment [X] | | Lacks K8s's scoped restore | |
| | Pulumi | Operations are scoped to the specific stack [X] | | Temporary safe refusal | |
| **(8) outputs merge per stack** | Terraform | Requires `terraform_remote_state` plumbing [X] | `merge_outputs` shifts old keys and merges new ones [V] | Shared map simplifies usage | reject(Shared outputs map simplifies cross-stack dependencies) |
| | Pulumi | Requires `StackReference` [X] | | Simpler than StackReference | |
| | Kubernetes | ConfigMaps accessed explicitly [X] | | Implicit cross-reference | |
| **(9) status attributes machines** | Kubernetes | `OwnerReferences` attribute resources [X] | `StackStamp.machines` tracks ownership [V] | Matches K8s explicit ownership | reject(Explicit ownership prevents cross-stack corruption) |
| | Terraform | Ownership implicit by state file [X] | | Explicit within shared state | |
| | Pulumi | Ownership implicit by stack state [X] | | Explicit within shared state | |

### GO Spec Recommendation (PMAT-162)
For behaviour (7), the Generation Ownership (GO) spec should copy **Nix profile generations**. Nix demonstrates that a single system can manage multiple independent timelines (profiles) side-by-side, where a rollback of Profile A replays only A's historical closure without touching Profile B's current state. Because forjar supports multiple stacks in a single state directory, the GO spec must isolate generations by stack ID and replay only the target stack's resources, upgrading forjar's whole-dir snapshot mechanism to a Nix-like stack-scoped restore. This is the ONE finding that should most change that spec, as it directly addresses the temporary `multi_stack_restore_refusal` safe-block. (Verdict FAIL because forjar currently lacks this required capability).

## Panel lane 3 (FAIL)

 Nix | Profile rollback scoped to generation [X] | (as above) | Forjar lacks Nix independent lineage | (as above) |
| (8) outputs merge per stack | Pulumi | StackReferences consume isolated outputs [X] | Outputs merge per stack instead of wholesale replace [V from CHANGELOG.md] | Forjar merges in one file | reject(Merging supports the shared-dir architecture correctly) |
| (8) outputs merge per stack | Terraform | Independent state file outputs [X] | (as above) | Forjar uses shared lock | (as above) |
| (8) outputs merge per stack | Kubernetes | Server-Side Apply merges fields by manager [X] | (as above) | Analogous to SSA field merging | (as above) |
| (9) status attributes ownership | Kubernetes | Managed Fields track manager [X] | Attributes each machine to stack that wrote it [V from CHANGELOG.md] | Analogous to K8s Managed Fields | reject(Accurately reflects multi-tenant ownership) |
| (9) status attributes ownership | Pulumi | Stack state URNs track ownership [X] | (as above) | Similar principle | (as above) |
| (9) status attributes ownership | Terraform | Only tracks one config per state [X] | (as above) | Forjar handles multi-tenant better | (as above) |

GO-SPEC FINDING (Behaviour 7): The GO spec (PMAT-162) should copy Kubernetes' rollout history [X]. Kubernetes tracks revision history natively on the owner object (e.g., ReplicaSet ownerReferences to a Deployment), meaning rollbacks inherently affect only that specific controller's managed resources without touching others in the namespace. Forjar should similarly attach generation lineages strictly to the stack identity (name) rather than utilizing global timestamps, ensuring `undo` only reads and replays the specific stack's generation records while entirely ignoring other stacks sharing the state directory.

