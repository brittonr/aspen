## Context

`aspen-transport` currently mixes ALPN/protocol routing, iroh connection management, OpenRaft transport integration, sharding metadata, auth, trust, metrics, and runtime helpers. `aspen-rpc-core` currently owns handler abstractions but also depends on concrete Aspen shell types and domain crates. That makes it hard to answer a simple question: can another Rust project use Aspen's iroh protocol-handler/registry pattern without adopting the Aspen node?

This is a higher-risk extraction than coordination or branch/DAG. The right first step is a staged contract: document intended layers, move the smallest generic surface first, and prove app-specific context stays behind named features.

## Goals / Non-Goals

**Goals:**

- Identify and extract/gate generic iroh transport helpers from Aspen-specific transport concerns.
- Identify and extract/gate generic RPC handler registry abstractions from concrete Aspen service contexts.
- Preserve existing runtime behavior through compatibility features or adapter crates.
- Add deterministic dependency and source-audit checks for default reusable surfaces.
- Provide downstream fixtures proving generic usage.

**Non-Goals:**

- Removing iroh/irpc from transport adapter crates.
- Replacing OpenRaft networking.
- Rewriting all handlers.
- Making the full Aspen node runtime reusable.
- Publishing crates or splitting repositories.

## Decisions

### Decision 1: Split generic core from Aspen runtime adapters, not transport from iroh

**Choice:** Iroh/irpc remain allowed in transport adapter surfaces because they are the transport purpose. The forbidden coupling is to Aspen node/runtime/domain services by default.

**Rationale:** Aspen is intentionally Iroh-only. A reusable Aspen transport crate should help users build iroh protocols, not hide iroh behind an unrelated abstraction.

**Alternative:** Create a transport-agnostic abstraction first. Rejected because it conflicts with Aspen's Iroh-only architecture and produces abstraction without a current consumer.

### Decision 2: RPC core default must not own concrete service contexts

**Choice:** Default `aspen-rpc-core` should expose request dispatch traits, handler registry, operation metadata, metrics/error surfaces, and context traits. Concrete Raft, coordination, jobs, Forge, CI, hooks, blob, cluster, and testing contexts move behind named features or shell crates.

**Rationale:** Handler registry mechanics are reusable; Aspen service graph assembly is not.

**Alternative:** Leave all contexts in default RPC core. Rejected because every registry user inherits the whole Aspen app graph.

### Decision 3: Use compatibility features during migration

**Choice:** Keep existing consumer imports compiling through explicit runtime feature bundles while canonical reusable surfaces are introduced.

**Rationale:** This split touches central runtime code. Compatibility lets the migration land without a flag day.

**Alternative:** Immediate hard split. Rejected as too risky for cluster, handlers, and node bootstrap.

## Risks / Trade-offs

- **[Risk] Feature gating hides rather than removes coupling** → Mitigate with negative source audits and downstream fixtures that compile without runtime features.
- **[Risk] Runtime feature matrix becomes confusing** → Mitigate with manifest feature tables and representative consumer commands.
- **[Risk] Circular dependencies surface during split** → Mitigate by moving concrete contexts outward into adapter crates rather than adding back edges.
- **[Trade-off] This change may stop at `workspace-internal` readiness** → Acceptable; transport/RPC is high value but should not be marked ready until downstream fixtures and compatibility rails pass.
