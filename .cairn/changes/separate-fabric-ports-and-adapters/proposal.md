# Proposal: Separate fabric ports and adapters

## Why

Several Molten fabric `adapters.rs` modules define application contracts beside concrete implementations.

Examples include `MembershipPlacementProvider`, `AssignmentPersistence`, `ExtensionRoleLifecyclePort`, `TimerClockAdapter`, `TransportCommandShell`, `DurableCommandShell`, and `CryptographicEntropySource`.

Some methods return raw `String` errors. The membership adapter module also combines authority checks, domain transitions, persistence order, external role effects, and uncertainty classification.

This structure makes adapters own application ports. It also mixes pure policy, shell orchestration, simulation, and live infrastructure in the same modules.

## What Changes

- Add application-owned fabric port modules with narrow domain-oriented contracts.
- Replace raw string port failures with typed application or infrastructure errors.
- Move deterministic transition and validation logic into pure core modules.
- Move persistence order, retries, uncertainty, and effect execution into imperative shell modules.
- Keep live, deterministic simulation, and fixture implementations in adapter modules.
- Select concrete fabric adapters only at visible composition roots.
- Migrate membership, time, entropy, transport, and durable-state paths in bounded slices.
- Preserve canonical Preserves values, transition refs, receipt meanings, live and simulation behavior, and existing claim boundaries.
- Add architecture checks for adapter-owned traits, raw string port errors, policy duplication, and concrete adapter construction in core scopes.

## Impact

- **Core**: Fabric membership, time, entropy, transport, and durable-state validation and transitions.
- **Application**: Fabric port contracts, typed errors, shell orchestration, and effect plans.
- **Adapters**: Static, simulation, live clock, operating-system entropy, Iroh, and persistence implementations.
- **Composition**: System-extension and runtime bootstrap paths.
- **Testing**: Positive and negative core, shell, adapter, uncertainty, and dependency-direction tests.

## Non-goals

- Do not add ports for pure internal calculations.
- Do not move distributed-service semantics from system extensions into generic fabric code.
- Do not treat simulation parity as live correctness.
- Do not change Preserves schemas without a separate versioned change.
- Do not add Valence, Basalt, Kamacite, or another stack dependency without a required contract.
- Do not claim transport, durability, timing, entropy, authority, or release correctness.
