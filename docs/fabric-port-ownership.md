# Fabric port ownership

<!-- r[impl molten.modularity.fabric_boundary.inventory] -->
<!-- r[impl molten.modularity.fabric_boundary.ownership] -->
<!-- r[impl molten.modularity.fabric_boundary.docs] -->

## Result

Molten keeps selected fabric capability contracts outside adapter modules.
Pure decisions remain in `molten-core` or canonical core modules.
Application shells own effect order and uncertainty.
Adapters contain host mechanisms only.

## Inventory

| Family | Capability | Owner | Inputs | Outputs | Typed failures | Effects | Composition root |
|---|---|---|---|---|---|---|---|
| Membership | Snapshot observation | `fabric_membership/ports.rs` | Source profile and provider state | `MembershipProviderSnapshot` | unavailable or malformed observation | Reads a static, managed, or scripted provider | Membership bootstrap and simulation setup |
| Membership | Assignment persistence | `fabric_membership/ports.rs` | Current assignment and pure transition | Intent and commit refs | storage failure or uncertain outcome | Persists intent before role effects, then commits | Membership shell setup |
| Membership | Role lifecycle | `fabric_membership/ports.rs` | Admitted assignment | Role-effect ref | definite or uncertain role failure | Activates, drains, replaces, releases, fails, or quarantines a role | System-extension lifecycle setup |
| Time | Clock observation and wait | `fabric_time/ports.rs` | Admitted profile and target tick | Explicit tick observation | unavailable, malformed, or timeout | Reads a live clock or advances a virtual clock | Runtime and simulation time setup |
| Time | Cryptographic entropy | `fabric_time/ports.rs` | Bounded output buffer | Filled secret buffer | unavailable capability | Reads the operating-system entropy source | Runtime entropy setup |
| Transport | Command execution | `fabric_transport/ports.rs` | Pure admitted transport command | Canonical transition | transport failure or uncertain outcome | Runs deterministic or Iroh transport mechanisms | Registered transport effect-port setup |
| Durability | Command execution | `fabric_durability/ports.rs` | Pure admitted durable command | Canonical transition | storage failure or uncertain outcome | Runs Redb or deterministic durability mechanisms | Registered durable effect-port setup |

Pure profile checks, transitions, authority checks, command admission, and failure classification do not need ports.
They remain ordinary functions over explicit values.

## Required order

The membership shell validates authority before any protected effect.
It computes the transition, persists intent, runs the admitted role effect, and commits the outcome.
A denial does not persist intent or call the role lifecycle port.
A failure after an external effect becomes an uncertain outcome.
It does not become a policy denial.

Time, transport, and durability cores receive explicit observations or commands.
They do not construct live adapters.
Runtime and system-extension setup select concrete implementations.

## Compatibility baseline

Before migration, focused tests passed for three membership cases, 16 time cases, 18 transport cases, and seven durability cases.
The migration keeps the existing canonical Preserves values, BLAKE3 transition refs, receipt meanings, and adapter-neutral behavior.
The source audit has positive repository fixtures and negative fixtures for every prohibited dependency shape.

## Enforcement

The repository-owned fabric boundary audit rejects:

- port trait definitions in maintained `adapters.rs` modules;
- raw `String` failures in maintained `ports.rs` modules;
- host effects in declared core paths;
- adapter-owned policy functions;
- concrete live adapter construction in declared core paths.

The audit is a source-ownership check.
It does not prove type correctness, external behavior, or release eligibility.

## Active-change dependencies

This change preserves the contracts used by active consistency, transport, durability, simulation, and system-extension changes.
It does not complete those changes or transfer their authority.
Future live reliability work must use these ports without moving service semantics into generic fabric adapters.

## Non-claims

This boundary does not prove live transport correctness, durable storage, clock accuracy, entropy quality, authority correctness, simulation parity, or release readiness.
A canonical receipt records bounded supplied facts only.
