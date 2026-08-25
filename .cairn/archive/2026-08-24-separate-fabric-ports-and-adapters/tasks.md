# Tasks: Separate fabric ports and adapters

## Phase 1: Inventory and baselines

- [x] [serial] I1 Inventory selected fabric traits, implementations, host effects, policy decisions, orchestration, concrete construction sites, and raw string errors. r[molten.modularity.fabric_boundary.inventory]
- [x] [serial] I2 Classify each candidate as pure core logic, application port, application shell, adapter, composition root, or unnecessary abstraction. r[molten.modularity.fabric_boundary.ownership]
- [x] [serial] V1 Record focused membership, time, entropy, transport, durability, simulation, canonical transition, and receipt baselines. r[molten.modularity.fabric_boundary.compatibility]

## Phase 2: Membership boundary

- [x] [serial] I3 Move membership snapshot, assignment persistence, and role lifecycle contracts into application-owned port modules with typed errors. r[molten.modularity.fabric_boundary.ports] r[molten.modularity.fabric_boundary.errors]
- [x] [serial] I4 Separate pure membership and assignment decisions from intent persistence, role effects, commit order, and uncertainty orchestration. r[molten.modularity.fabric_boundary.core] r[molten.modularity.fabric_boundary.shell]
- [x] [parallel] V2 Add positive and negative core, shell, static-provider, simulation-provider, persistence, role-effect, denial, and uncertain-outcome tests. r[molten.modularity.fabric_boundary.validation]

## Phase 3: Time and entropy boundary

- [x] [serial] I5 Move time and entropy capability contracts into application-owned ports with typed observations and errors. r[molten.modularity.fabric_boundary.ports] r[molten.modularity.fabric_boundary.errors]
- [x] [serial] I6 Keep clock reads, sleep, and operating-system entropy inside live adapters while pure scheduling and entropy decisions consume explicit facts. r[molten.modularity.fabric_boundary.core] r[molten.modularity.fabric_boundary.adapters]
- [x] [parallel] V3 Add live, virtual, scripted, timeout, backward-time, entropy-exhaustion, malformed-observation, and denial tests. r[molten.modularity.fabric_boundary.validation]

## Phase 4: Transport and durable-state boundary

- [x] [serial] I7 Move transport and durable-state command contracts into application-owned ports with typed failures. r[molten.modularity.fabric_boundary.ports] r[molten.modularity.fabric_boundary.errors]
- [x] [serial] I8 Keep transition policy in pure cores and move Iroh, store, transaction, retry, timeout, cancellation, and partial-effect behavior into shells and adapters. r[molten.modularity.fabric_boundary.core] r[molten.modularity.fabric_boundary.shell] r[molten.modularity.fabric_boundary.adapters]
- [x] [parallel] V4 Add positive and negative live, simulation, malformed-frame, storage-failure, timeout, cancellation, retry, commit, rollback, and uncertainty tests. r[molten.modularity.fabric_boundary.validation]

## Phase 5: Composition, enforcement, and closeout

- [x] [serial] I9 Select concrete fabric implementations only at reviewed runtime or system-extension composition roots. r[molten.modularity.fabric_boundary.composition]
- [x] [serial] I10 Add architecture checks for adapter-owned traits, raw string port errors, host effects in core scopes, duplicated policy, and concrete construction outside composition roots. r[molten.modularity.fabric_boundary.enforcement]
- [x] [serial] I11 Document port ownership, shell order, adapter roles, active-change dependencies, and claim boundaries. r[molten.modularity.fabric_boundary.docs]
- [x] [parallel] V5 Run focused suites, formatting, Clippy, Octet, Cairn validation and gates, nextest or Cargo fallback, and relevant Nix checks. r[molten.modularity.fabric_boundary.final_checks]

## Verification Coverage

- `Scenario: Inventory classifies one selected contract` -> I1, I2
- `Scenario: Adapter module defines an application port` -> I3, I5, I7, I10
- `Scenario: Port returns a raw string failure` -> I3, I5, I7, I10
- `Scenario: Pure decision returns an effect plan` -> I4, I6, I8, V2, V3, V4
- `Scenario: Authority denial prevents a role effect` -> I4, V2
- `Scenario: Live clock supplies an explicit observation` -> I6, V3
- `Scenario: Transport failure remains infrastructure-owned` -> I7, I8, V4
- `Scenario: Concrete adapter is selected in the core` -> I9, I10
- `Scenario: Canonical transition and receipt fixtures remain stable` -> V1, V5
- `Scenario: Boundary claims remain scoped` -> I11, V5
