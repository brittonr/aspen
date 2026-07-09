## Context

Molten has accepted upgrade-session requirements and multiple systems that need coordinated evolution. The next hardening step is to make sessions consistently consume exact refs, dependency impact receipts, and subsystem-specific gates before cutover.

## Design

### Session artifact

An upgrade session contains:

- session id and purpose;
- affected refs and relation types;
- immutable plan artifact ref;
- task graph with required evidence markers;
- impact query receipt refs;
- compatibility/migration/protocol/replay/policy gate refs;
- rollback and cleanup strategy refs;
- non-claim boundary checks.

Task completion is mutable metadata recorded by receipts that point back to the plan. The plan hash remains stable.

### Cutover gates

Cutover decisions must bind evidence appropriate to the affected surface:

- aliases/name views require exact target refs and update capabilities;
- schemas and storage require compatibility or migration receipts;
- protocols require terminal protocol-session gate receipts;
- policy/effect/profile updates require handler and policy admission receipts;
- transcript rewrites require replay or receipt-oracle evidence;
- cleanup requires dependency/retention impact evidence.

### Functional core and shell

Pure cores validate session plans, task dependencies, evidence refs, and cutover readiness from in-memory records. Shells read registries/ledgers, persist task receipts, invoke subsystem gates, and render operator summaries.

### Non-goals

- Do not adopt UCM patch syntax, namespace behavior, typechecker, or codebase model.
- Do not replace Git, Cargo, Nix, Cairn changes, or human review.
- Do not let task checkboxes alone authorize cutover.
- Do not perform destructive cleanup without retention and dependency impact gates.