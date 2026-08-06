## Context

Molten already requires coordination service-registry updates to commit through the control plane. Default reads use normalized linearizable currentness evidence.

The active ChaosControl conformance package projects committed state-machine transitions inside deterministic KVM campaigns. A live black-box rail must use only public client behavior for its model verdict.

## Decisions

### 1. Use the service registry as the first register workload

**Choice:** One admitted service-registry key represents one register. A write replaces its endpoint ref. A read returns the current endpoint ref or the declared empty value.

The workload profile binds the exact key, initial value, endpoint value corpus, generator identity, seed, operation weights, client set, concurrency, operation bounds, and consistency model.

**Rationale:** The service registry already uses control-plane commits and linearizable reads.

### 2. Generate operations in the product core

**Choice:** A pure Molten generator consumes the admitted profile and emits a deterministic bounded plan of reads and writes plus complete choice records.

The generator does not call services, read clocks, inspect faults, or decide expected model results. OnixOS schedules the plan and the Molten adapter executes it.

**Rationale:** Product operation semantics remain with Molten while generic orchestration remains with OnixOS.

### 3. Exercise public product paths

**Choice:** The adapter invokes the production-shaped coordination request and read interfaces through the admitted node service transport.

It does not read terms, commit indexes, replica logs, Redb state, snapshots, or internal receipts to decide operation results. Those facts can appear only in diagnostic bundles.

**Rationale:** A black-box model must evaluate the service contract visible to clients.

### 4. Preserve operation and attempt identity

**Choice:** Every logical write or read receives one stable operation ID. Retries receive distinct attempt IDs and retain the original operation ID.

Acknowledged results map to `ok`. Definite pre-effect denials map to `fail`. Timeout, disconnect, or lost response maps to `info`.

**Rationale:** An uncertain client result can follow a committed operation.

### 5. Use the shared semantic-history contract

**Choice:** The adapter emits the pinned ChaosControl semantic-history v2 schema. One OnixOS control process assigns event order and controller time.

Molten binds each event to its profile, group, service, key, client session, operation, attempt, request, and public endpoint refs.

**Rationale:** Shared history semantics allow one checker without transferring Molten product ownership.

### 6. Keep live and deterministic profiles separate

**Choice:** Simulation, NixOS VM, ChaosControl KVM, and OnixOS live runs can share operation corpora and invariant names.

Every receipt retains a distinct environment profile, adapter identity, fault mechanism, time source, and non-claim set. No profile substitutes for another.

**Rationale:** Similar workloads do not create environmental evidence equivalence.

### 7. Start with a bounded crash-fault matrix

**Choice:** The initial matrix contains a no-fault control, process restart, temporary network partition, heal, bounded recovery, and final reads.

Clock and durability profiles remain unsupported until Molten and OnixOS define exact public semantics and target capabilities.

**Rationale:** The first live slice must remain reviewable and executable.

### 8. Require final reads from every endpoint

**Choice:** After heal, the adapter waits for declared service readiness and stable membership facts. It reads the selected key through every admitted public endpoint.

Missing or conflicting final observations block a valid verdict. Internal state cannot fill a missing public observation.

**Rationale:** Final reads expose divergence, lost acknowledged writes, and stalled recovery.

### 9. Treat linearizability as a finite model result

**Choice:** The runner invokes the pinned register checker over the complete admitted history. It optionally invokes the pinned Jepsen-compatible reference checker.

A native `invalid` verdict preserves the witness as bounded failure evidence. `unknown`, incomplete history, missing recovery observations, or checker disagreement blocks promotion.

**Rationale:** A pass cannot exceed the exact finite history and checker bounds.

### 10. Import external evidence without authority transfer

**Choice:** A pure Molten importer validates producer identities, cluster and artifact refs, workload profile, operation completeness, fault outcomes, recovery, checker cohort, verdict, witness, teardown, and non-claims.

The shell stores a canonical external live-reliability receipt in the Molten ledger. The receipt cannot satisfy authority, policy, resource, provenance, transport, retention, deployment, or release gates.

**Rationale:** Live observations are supporting evidence, not ambient trust.

### 11. Keep the functional core pure

**Choice:** Generator planning, workload projection, outcome mapping, profile admission, import validation, cross-profile comparison, and claim classification remain pure functions.

Service calls, OnixOS control, network access, files, external checkers, and ledger writes remain shells.

**Rationale:** Product semantics and evidence admission need infrastructure-free tests.

## Dependencies and blockers

Implementation requires these archived producer contracts:

- ChaosControl `add-linearizable-operation-histories`
- OnixOS `add-live-black-box-reliability-rail`
- OnixOS `onixos-molten-native-service`

The live profile also requires a production-shaped cross-process Molten coordination endpoint. A local in-process fixture cannot replace that dependency.

The active `add-chaoscontrol-consensus-conformance` package remains independent. It can share operation corpora after its own producer dependencies archive.

## Risks and trade-offs

- Service-registry writes test one narrow control-plane object. They do not establish all coordination semantics.
- Public transport failures can create many `info` outcomes. The profile bounds them explicitly.
- An observer adapter can misclassify responses. Direct public fixtures and reference-checker agreement reduce this risk.
- Live timing is not deterministic. Receipts record observed history and do not claim replay.
