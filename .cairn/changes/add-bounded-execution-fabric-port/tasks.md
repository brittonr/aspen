# Tasks: Add a bounded execution fabric port

## Phase 1: Component and contract

- [ ] [serial] Pin the reviewed Bounded Exec source, license, package, platform, API, and non-claim cohort through immutable Cargo and Nix inputs. r[molten.fabric_execution.component_pin]
- [ ] [serial] Define the application-owned execution port, typed failures, canonical request, lifecycle, outcome, status, and receipt schemas. r[molten.fabric_execution.port_contract] r[molten.fabric_execution.lifecycle]
- [ ] [serial] Add the execution fabric class, authority requirement, resource requirements, descriptor, registry binding, and profile validation. r[molten.fabric_execution.port_contract] r[molten.fabric_execution.authority]
- [ ] [parallel] Add typed Nickel profiles and positive and negative fixtures for environments, bounds, termination scope, platforms, and non-claims. r[molten.fabric_execution.request] r[molten.fabric_execution.environment] r[molten.fabric_execution.nonclaims]

## Phase 2: Pure core and shell plans

- [ ] [serial] Implement pure request, authority, resource, generation, lifecycle, output, and uncertainty admission over explicit in-memory facts. r[molten.fabric_execution.authority] r[molten.fabric_execution.request] r[molten.fabric_execution.generation] r[molten.fabric_execution.uncertainty]
- [ ] [serial] Define capability-rooted artifact, workspace, input, and output-resolution plans without canonical host paths or runtime handles. r[molten.fabric_execution.request] r[molten.fabric_execution.output]
- [ ] [parallel] Add positive and negative pure tests for complete requests, drift, missing authority, overbounds, invalid transitions, stale generations, and identity substitution. r[molten.fabric_execution.validation]

## Phase 3: Live and simulation adapters

- [ ] [serial] Implement the thin live adapter over Bounded Exec with cleared environment, explicit argv, bounded input and output, deadlines, cancellation, and teardown. r[molten.fabric_execution.environment] r[molten.fabric_execution.lifecycle]
- [ ] [serial] Publish retained output through the selected content-store port and preserve process results when publication fails. r[molten.fabric_execution.output]
- [ ] [serial] Implement unknown-outcome recording and exact-operation reconciliation without automatic retry. r[molten.fabric_execution.uncertainty]
- [ ] [parallel] Implement the deterministic simulation adapter over the same request and lifecycle contract. r[molten.fabric_execution.simulation]
- [ ] [parallel] Add shared adapter tests for accepted exits, rejected exits, input, output floods, timeout, cancellation, descendant-held pipes, teardown, start failure, and uncertainty. r[molten.fabric_execution.validation]

## Phase 4: Composition and closeout

- [ ] [serial] Register live and simulation implementations only at reviewed system-extension composition roots. r[molten.fabric_execution.port_contract]
- [ ] [serial] Add architecture checks for direct process calls outside the adapter, adapter-owned policy, raw string failures, mutable sibling dependencies, and hidden fallback. r[molten.fabric_execution.port_contract] r[molten.fabric_execution.component_pin]
- [ ] [parallel] Run one system-extension fixture through live and simulation profiles with equal canonical command and outcome contracts. r[molten.fabric_execution.simulation] r[molten.fabric_execution.validation]
- [ ] [serial] Document adoption, executable authority, platform behavior, output handling, recovery, operator status, and non-claims. r[molten.fabric_execution.nonclaims]
- [ ] [serial] Run formatting, focused and workspace tests, Clippy, Octet, Nickel, Cairn validation and gates, traceability, and relevant Nix checks. r[molten.fabric_execution.validation] r[molten.fabric_execution.nonclaims]

## Verification Coverage

- `Scenario: Reviewed component is selected` -> immutable dependency task
- `Scenario: An adapter defines the port contract` -> architecture checks
- `Scenario: Executable possession is the only authority` -> pure authority negative test
- `Scenario: A required limit is absent or overbound` -> profile and pure-core negative tests
- `Scenario: Request asks for inheritance or shell expansion` -> environment negative fixtures
- `Scenario: Cancellation races with completion` -> live adapter lifecycle tests
- `Scenario: Child floods an output stream` -> bounded capture test
- `Scenario: Completion belongs to a replaced generation` -> generation-fencing test
- `Scenario: Host fails after process start` -> uncertainty and recovery test
- `Scenario: Equal simulation inputs replay` -> deterministic adapter replay test
- `Scenario: Live and simulation adapters conform` -> shared conformance suite
- `Scenario: Process exits successfully` -> scoped status and documentation
