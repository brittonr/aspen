# Dag Sync Specification

## Purpose

Defines the `dag-sync` capability.

## Requirements

### Requirement: Generic DAG records are canonical and domain-neutral
r[molten.dag_sync.model] Molten MUST define canonical DAG node, edge, root, traversal-bound, inventory, sync-request, sync-plan, response, progress, and receipt records that do not encode job, Forge, commit, merge, or transport-specific semantics in the base graph model.

#### Scenario: Artifact and job DAGs share the model
- GIVEN two extensions describe graphs with canonical node and edge refs
- WHEN generic DAG validation runs
- THEN both MAY use the same traversal and sync primitives
- AND their domain metadata and admission remain extension-owned.

### Requirement: DAG traversal is pure, deterministic, and bounded
r[molten.dag_sync.traversal_core] Molten MUST implement cycle detection, duplicate detection, edge validation, visited-set transitions, deterministic traversal and topological order, missing-set calculation, and completion checks as pure functions with explicit node, edge, depth, byte, step, and diagnostic bounds.

#### Scenario: Acyclic traversal is stable
- GIVEN the same valid bounded graph and traversal profile
- WHEN traversal planning runs repeatedly
- THEN node order, missing refs, visited-state ref, and plan ref MUST match.

#### Scenario: Cycle denies before fetch
- GIVEN a graph descriptor contains a dependency cycle
- WHEN validation or traversal planning runs
- THEN it MUST deny before transport or content effects.

### Requirement: Traversal strategies are explicit
r[molten.dag_sync.strategy_profiles] Molten MUST represent full, stem-first, leaf-only, resumable missing-ref, and deterministic peer-partitioned strategies as versioned profile inputs where supported. Strategy ordering, peer assignment, bounds, and completion criteria MUST contribute to plan and replay identity.

#### Scenario: Stem-first plan fetches metadata first
- GIVEN a graph separates branch metadata from leaf content
- WHEN a stem-first strategy is selected
- THEN the plan MUST request only admitted metadata refs before leaf requests are derived.

#### Scenario: Unknown strategy denies
- GIVEN a request names an unsupported traversal strategy or version
- WHEN plan validation runs
- THEN it MUST deny rather than fall back to full traversal.

### Requirement: DAG synchronization is receiver-driven
r[molten.dag_sync.receiver_driven] Molten MUST require the receiver to derive requested node and content refs from its verified local state and active plan. Responses MUST bind request, traversal epoch, peer, service generation, and exact requested refs. Unsolicited, duplicate-conflicting, stale, or over-bound responses MUST NOT become verified graph state.

#### Scenario: Requested nodes advance progress
- GIVEN a receiver requests missing nodes under an active plan
- WHEN matching verified responses arrive
- THEN progress MAY advance and MUST bind the newly available refs.

#### Scenario: Unexpected node is rejected
- GIVEN a sender returns a node not present in the active request
- WHEN response validation runs
- THEN the node MUST be denied or ignored without indexing it as verified.

### Requirement: DAG bytes use content adapters
r[molten.dag_sync.content_adapter_boundary] Molten MUST carry bounded graph metadata and canonical content refs through the DAG protocol while moving large node bodies and payloads through admitted content-store and content-exchange adapters. Content verification, transforms, confidentiality, retention, and range behavior MUST remain owned by those adapters and primitives.

#### Scenario: Large leaf uses a content ref
- GIVEN a leaf payload exceeds the protocol metadata bound
- WHEN synchronization plans the leaf
- THEN the plan MUST request its canonical content ref through a compatible content adapter.

#### Scenario: Corrupt content blocks graph completion
- GIVEN a fetched node body fails content verification
- WHEN progress evaluation runs
- THEN the graph MUST remain incomplete and no domain admission receipt may use that node.

### Requirement: DAG synchronization resumes safely
r[molten.dag_sync.resume_fencing] Molten MUST persist bounded progress over verified refs and MUST bind resume to root, graph schema, strategy, policy, traversal epoch, peer assignment, and service generation. Stale or incompatible progress MUST deny or restart from a reviewed plan without treating old assignments as current.

#### Scenario: Compatible resume skips verified refs
- GIVEN a prior run verified part of the same graph under compatible inputs
- WHEN synchronization resumes
- THEN the new plan MUST omit already verified refs and preserve their verification evidence.

#### Scenario: Stale traversal epoch is rejected
- GIVEN graph root or strategy changes after progress was recorded
- WHEN old progress is supplied
- THEN resume MUST deny or create a new traversal epoch before fetching.

### Requirement: Synchronization does not grant domain admission
r[molten.dag_sync.domain_boundary] Molten MUST treat DAG sync completion as verified availability evidence only. Installation, execution, publication, merge, registry mutation, protocol activation, and authority decisions MUST require their owning extension's normal schema, policy, capability, provenance, resource, and evidence gates.

#### Scenario: Synced executable remains uninstalled
- GIVEN executable artifact bytes and dependencies synchronize successfully
- WHEN no execution or installation admission is supplied
- THEN the artifact MUST remain available but uninstalled and unexecuted.

### Requirement: DAG sync has live and simulated conformance
r[molten.dag_sync.final_validation] Molten MUST run the same traversal, strategy, request/response, progress, resume, and domain-boundary core under live and deterministic-simulation adapters and MUST test positive complete/resume flows plus negative cycles, unknown edges, duplicate nodes, bound exhaustion, unexpected responses, corruption, cancellation, partitions, stale epochs, unauthorized peers, and domain overclaims.

#### Scenario: Live and simulation traces agree
- GIVEN equivalent bounded no-fault profiles
- WHEN one DAG sync fixture runs under live loopback and simulation
- THEN canonical traversal and progress traces MUST fall within the same allowed set.

#### Scenario: Direct adapter import fails conformance
- GIVEN a DAG extension bypasses content or transport ports to access backend internals
- WHEN fabric conformance runs
- THEN activation or validation MUST deny the bypass.
