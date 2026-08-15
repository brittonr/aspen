# Generation Retirement Specification

## Purpose

Defines the `generation-retirement` capability.

## Requirements

### Requirement: Every retirement-relevant runtime holder registers a root

r[molten.retirement.root_inventory] Molten MUST define a bounded root inventory covering active callbacks or execution units, sessions, tasks, cells and durable values, queues, timers, registries, effect handles, snapshots, rollback pins, retention pins, remote/cache pins, and other declared holders that can retain generation-scoped or generation-exclusive artifact refs.

#### Scenario: Instrumented generation has complete roots
- GIVEN every execution profile and holder class participating in a generation reports current roots and edges under one inventory snapshot
- WHEN retirement analysis begins
- THEN the inventory MAY be marked complete for that declared profile and snapshot.

#### Scenario: Native or remote holder cannot be observed
- GIVEN an execution profile may hold an old artifact but does not expose a registered root observation
- WHEN retirement analysis runs
- THEN the inventory MUST remain incomplete and the generation MUST NOT be classified retired.

### Requirement: Retirement reports explain deterministic reachability

r[molten.retirement.trace_report] Molten MUST emit canonical `generation-retirement-report-v1` evidence binding the inventory snapshot, binding revisions, generations, roots, edges or edge-set ref, generation attribution, reachable refs, stable pin paths, completeness, decision, diagnostics, and non-claims.

#### Scenario: Session pins an old implementation
- GIVEN an active protocol session root reaches an artifact exclusive to an old generation
- WHEN the report is constructed
- THEN it MUST classify the generation live and identify the session root and stable path to the artifact.

#### Scenario: Graph contains cycles and duplicate edges
- GIVEN runtime values and dependency closures form cyclic or duplicate reachability edges
- WHEN the report is computed repeatedly from equivalent input sets
- THEN it MUST terminate and produce identical decisions, reachable refs, paths, and diagnostics.

### Requirement: Retirement classification is conservative

r[molten.retirement.classification] Molten MUST report `retired` only from a complete root, edge, and attribution inventory with no reachable generation-scoped root or generation-exclusive artifact. Incomplete, stale, contradictory, or uninstrumented evidence MUST produce `unknown`, `incomplete`, or `live`, never `retired`.

#### Scenario: Shared unchanged artifact remains current
- GIVEN an artifact ref is shared by old and current generations while no old-generation-scoped root or old-exclusive artifact remains reachable
- WHEN complete attribution is evaluated
- THEN the shared ref alone MUST NOT keep the old generation live.

#### Scenario: Attribution is ambiguous
- GIVEN a reachable artifact cannot be classified shared or generation-exclusive
- WHEN retirement is evaluated
- THEN the report MUST remain incomplete and identify the attribution blocker.

### Requirement: Retirement evidence is not deletion authority

r[molten.retirement.gc_boundary] A retired generation report MUST remain lifecycle and reachability evidence only. Retention, reference-index completeness, remote clearance, deletion authority, policy, tombstone, apply, execute, and audit gates MUST still run before content removal.

#### Scenario: Retired generation contains a legal-hold artifact
- GIVEN retirement reachability reports no live runtime pin but retention evidence records a legal hold
- WHEN garbage collection is requested
- THEN deletion MUST deny or retain the artifact according to existing retention authority regardless of retirement status.

### Requirement: Deploy diagnostics expose unreachable and pinned changes

r[molten.retirement.deploy_diagnostics] Deployment and upgrade reports MUST diagnose stale binding plans, incompatible target contracts, successor artifacts unreachable from any declared late-bound re-entry point, incomplete root inventories, live pin paths, and semantic effect-operation mismatches without inventing success or deletion eligibility.

#### Scenario: New artifact is published but no re-entry reaches it
- GIVEN a binding update introduces a successor artifact but all declared long-lived work remains permanently pinned with no future late-bound boundary
- WHEN deploy diagnostics run
- THEN the report MUST identify the successor as unreachable for live work and require an explicit boundary, drain, or restart decision.
