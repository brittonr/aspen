# Proposal: Differentiate live and simulation conformance

## Why

`reference_contract_differential` builds trace references from port descriptors and passes the same trace list as both sides of the comparison. The comparison therefore cannot diverge, and the documented live-networking and live-storage equivalence gap remains untested by construction.

FoundationDB combines simulation with live performance and hardware-based failure testing and reuses workload code between them. Aspen has the same need in miniature: a shared workload must run against both simulated and live adapters, and the comparison must check contract-permitted outcomes and histories, not identical physical timing.

## What Changes

- Add a shared workload definition that both a simulated adapter and a live adapter execute through their declared port contracts.
- Compare permitted outcome classes, semantic histories, and declared ordering properties instead of physical timing or byte identity.
- Record unsupported behaviors and incomplete observations as typed differential results rather than failures or passes.
- Bind both execution sides, the shared workload, permitted-outcome decisions, and non-claims into the differential evidence.
- Keep the existing descriptor-level differential as the cheap structural check and add the execution differential above it.

## Impact

- **Files**: `molten-core` simulation differential types and composition, live adapter conformance harness entry points, and tests.
- **Testing**: outcome-class agreement, permitted divergence, unsupported-behavior recording, incomplete-observation handling, and negative cases where a live adapter violates its declared outcome class.
- **Non-goals**: no claim that simulation replaces live testing, no performance or benchmark claim, and no proof of whole-system correctness from agreement.

## Dependencies

- `exercise-simulation-faults-causally` supplies a simulated side that executes faults causally; this change needs at least a fault-free shared workload, so it can proceed in parallel.
- Existing port contract command/event schema sets.
