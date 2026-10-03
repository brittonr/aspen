# Retention root and deletion evidence reference

Mode: Reference

Use this page to classify evidence in a candidate dossier without conflating roots, admissions, and lifecycle artifacts. The names below are source-level fields or CLI subcommands, not a complete wire schema. Canonical Preserves encoding and BLAKE3 identity govern artifacts; Rust structure layout and directory listing order do not.

This is a source-checked reference, not runtime verification. The [technical companion](../../technical/world-effects/distribution-retention-and-reachability.md) supplies conceptual background; the [distribution contract](../../world-distribution.md) governs world-root completeness and non-authority claims.

## Root observations and their owners

| Observation family | Responsible source of facts | Interpretation |
| --- | --- | --- |
| Current and competing heads | World/head owner | Competing admitted successors remain roots, not discarded losers |
| Active executions and task checkpoints | Execution/task owner | Ongoing work can retain otherwise old state |
| Replay, simulation, comparison pins | Respective workflow owner | Reproducibility dependencies remain distinct from active heads |
| Merge conflicts; promotion/reconciliation state | Merge and promotion owners | Unresolved transition state contributes roots |
| Rollback, legal, evidence, operator holds | Hold/policy owner | A cleanup preference does not cancel a hold |
| Remote leases | Peer/lease observation owner | Unresolved observations block completeness and retain named roots |
| Incomplete transfers | Distribution owner | Partial transfer state is not garbage by inference |

The projection's `missing_classes` records absent or unobserved classes. `unresolved_remote` records unresolved leases. `reference_index_complete` additionally depends on complete edge and attribution inventories. Explicitly observed empty classes are valid observations; omitted classes are not equivalent empty sets.

The world report contains `retained_refs`, `remote_refs`, `evidence_refs`, and a binding report. Its `observation_only`, `retention_authorized`, and `deletion_authorized` fields preserve the distinction between classification and authority. The handoff appends evidence to the existing workflow and combines completeness with logical AND.

## Destructive evidence fields

These fields belong to [`DestructiveEvidence`](../../../src/retention/parts/mod/p000/body.rs). CLI argument spellings are declared in [`RetentionEvidenceArgs`](../../../src/main/root.rs); this table is descriptive, not an executable recipe.

| Field | Planning use and required interpretation |
| --- | --- |
| `requester_ref` | Requester identity to which admissions are scoped |
| `policy_refs` | Policy admission inputs; a nonempty list alone does not establish valid policy |
| `authority_refs` | Authority admission inputs for the requested operation |
| `evidence_refs` | Supporting evidence inputs, distinct from authority |
| `retained_refs` | Known retaining dependencies; not deletion candidates |
| `remote_peer_refs` | Peer context for remote clearance coverage |
| `remote_refs` | Remote relationships requiring appropriate handling |
| `reference_index_refs` | Reference-index evidence bindings |
| `remote_gc_refs` | Remote-GC admission inputs |
| `remote_clearance_refs` | Clearance evidence, not interchangeable with GC plans |
| `is_reference_index_complete` | Completeness assertion requiring supporting evidence, not a discovery operation |

Admission kinds are `policy`, `authority`, `supporting-evidence`, `reference-index`, and `remote-gc`. Admission checks inspect kind, passing decision, currentness, revoked references, nonempty bound references, and matching requester/object/kind/class/action scope. Reference-index admissions additionally require completeness; remote-GC admissions must cover required remote references. These are checks on supplied records, not proof that every producer independently observed reality.

## Artifact and command boundaries

Subcommands below are under `molten test retention`, as declared by the [root command](../../../src/main/root/parts/command/p000/body.rs) and [retention command enum](../../../src/cli/workflow/retention/command.rs). Arguments and handlers are linked in Sources. No command in this table was executed for this batch.

| Artifact or view | Producer/consumer | Boundary |
| --- | --- | --- |
| Candidate explanation | `explain` | Collects matching known evidence; optional output writes a view, not permission |
| Pin and pin receipt | `pin`; retention store | Records protection and operation evidence |
| Evidence admission | `admit`; planning gates | Binds a kind and scope; acceptance still depends on admission checks |
| GC plan | `gc-plan` | Persists a dry-run decision and gates, without creating retention receipts or tombstones |
| GC apply | `gc-apply-plan` | Recomputes plan, checks drift/admissions, and may create receipt/tombstone |
| Execution gate | Owning destructive subsystem/library API | Distinct lifecycle stage; the retention enum has no generic `gc-execute` subcommand |
| GC audit | `gc-audit` | Reads an execution reference and related lifecycle evidence |
| Candidate bundle | `bundle-export`, `bundle-verify` | Portable evidence inspection; not current deletion authority |
| Artifact summary | `show` | Reads a supplied artifact; a summary is not a fresh eligibility evaluation |

## Worked interpretation and limits

Suppose a dossier contains a passing plan, a later apply denial with `retention-gc-apply-plan-drift`, and an old tombstone. The plan records the earlier dry-run inputs. The apply denial says recomputation did not preserve the plan identity. The unrelated or historical tombstone cannot fill that gap: lifecycle checks require matching plan, recomputed-plan, apply, execution, receipt, tombstone, and audit links and scope.

Likewise, a benchmark's planned deletion count is only structural observation. The [benchmark contract](../../world-benchmark-sharing-and-retention.md) prohibits retained categories from appearing as candidates but does not grant authority to execute any candidate. Inventory completeness, policy, remote clearance, and effect ownership remain separate review obligations.

## Sources

- [Handbook](../README.md)
- [Distribution contract](../../world-distribution.md)
- [Technical companion](../../technical/world-effects/distribution-retention-and-reachability.md)
- [World root projection](../../../crates/molten-core/src/world_distribution/retention.rs)
- [Evidence fields](../../../src/retention/parts/mod/p000/body.rs)
- [Admission checks](../../../src/retention/parts/mod/p011/body.rs)
- [CLI arguments](../../../src/cli/workflow/retention/command/ops.rs)
- [CLI handlers](../../../src/cli/workflow/retention/ops.rs)
- [Lifecycle link checks](../../../src/retention/parts/mod/p017/body.rs)
