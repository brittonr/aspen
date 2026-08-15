# Design: Separate fabric ports and adapters

## Context

Molten documents a primitive, adapter, system-extension, and workload ownership split. Several fabric modules do not yet express that split in source ownership.

For example, `src/fabric_membership/adapters.rs` defines ports, simulation implementations, policy checks, transition application, persistence order, role effects, and uncertainty results.

`src/fabric_time/adapters.rs` defines its clock and entropy ports beside host clock, sleep, and operating-system entropy code. Transport and durability follow a similar shape.

## Success Contract

Completion requires application-owned ports, pure fabric decisions, thin shells, and infrastructure-only adapters for the selected fabric families.

Core tests must run with ordinary values. Shell tests must prove that denial prevents protected effects.

Moving code without correcting dependency direction, raw errors, or policy ownership is false completion.

## Decisions

### Decision: Application modules own fabric ports

**Choice:** Define narrow ports beside the application shells that consume each capability.

Ports use Molten domain inputs, outputs, and typed errors. Vendor handles, paths, Iroh values, operating-system errors, and protocol objects stay outside these contracts.

**Rationale:** Application needs define capabilities. Concrete mechanisms do not define product contracts.

### Decision: Pure cores return decisions and effect plans

**Choice:** Keep profile admission, state transitions, authority-input validation, command validation, and uncertainty classification in pure functions.

The core receives current state, policy, authority facts, time facts, observations, and requests explicitly. It returns a decision, new state, events, or a typed effect plan.

**Rationale:** Domain meaning must remain deterministic and directly testable.

### Decision: Shells own ordering and uncertainty

**Choice:** Application shells load facts, call pure decisions, persist intent, execute approved effects, commit outcomes, and classify infrastructure failures.

Shells own retries, timeouts, cancellation, transactions, and ambiguous external outcomes. They must not duplicate policy.

**Rationale:** These concerns depend on external state and effect ordering.

### Decision: Adapters contain mechanism code only

**Choice:** Keep live clocks, sleeps, operating-system entropy, Iroh, files, stores, and deterministic simulation mechanisms in adapter modules.

A simulation adapter can hold scripted observations. It must not become a second policy implementation.

**Rationale:** Live and simulation mechanisms must obey the same application contract.

### Decision: Errors retain ownership

**Choice:** Core errors describe invalid values, rejected transitions, policy denial, and conflicts.

Port and adapter errors describe unavailable capabilities, malformed external observations, timeouts, storage failures, transport failures, and partial effects.

Raw `String` errors cannot cross maintained fabric port boundaries.

**Rationale:** Text-only errors merge domain and infrastructure meaning.

### Decision: Composition is visible

**Choice:** Select static, simulation, live, Iroh, entropy, and persistence adapters only in reviewed bootstrap or system-extension composition roots.

Core and application modules receive port implementations through explicit parameters.

**Rationale:** Concrete infrastructure choice must remain at the system edge.

### Decision: Migration follows fabric families

**Choice:** Migrate membership first, then time and entropy, transport, and durable state.

Each slice preserves canonical transition and receipt fixtures before the next slice starts.

**Rationale:** The selected modules have broad call graphs and active roadmap work.

## Required Flow

```text
fabric request or observation
  -> inbound adapter
  -> application shell
  -> pure fabric decision
  -> state, events, and effect plan
  -> application shell
  -> application-owned port
  -> concrete adapter
  -> observed result and receipt facts
```

## Test Design

Core tests cover valid and rejected profiles, transitions, authority facts, stale generations, conflicts, and deterministic replay.

Shell tests cover port-call order, intent-before-effect, denial without effects, commit and rollback, timeouts, and uncertain outcomes.

Adapter tests cover live and simulation conversion, malformed observations, transport or storage failure, and port conformance.

Source checks reject trait definitions in maintained adapter modules, raw string port errors, host effects in core scopes, and concrete adapter construction outside composition roots.

## Risks and Trade-offs

- Active fabric changes can touch the same modules. Migrate one family at a time and record dependencies.
- Typed errors add conversion code. They preserve domain and infrastructure meaning.
- Simulation can drift from live behavior. Shared port fixtures and explicit non-claims reduce false parity claims.
- A broad interface campaign can create useless ports. The inventory must retain only genuine external capabilities.

## Claim Boundary

This change establishes source ownership and dependency direction for selected fabric paths. It does not prove live behavior, authority, or release readiness.
