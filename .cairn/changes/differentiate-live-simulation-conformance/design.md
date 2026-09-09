# Design: Differentiate live and simulation conformance

## Context

The current differential calls `canonical_simulation_differential` with the same `trace_refs` vector for both profiles. That value is a valid structural summary of the declared contracts, but as an execution comparison it is tautological.

## Approach

- Define a bounded shared workload as an ordered list of port requests with declared permitted outcome classes per request, derived from the port contract's command and event schema set.
- Execute the workload twice through one host abstraction: once against the simulated adapter and once against the live adapter implementation selected for the profile.
- Normalize each observed outcome to its declared outcome class. Compare sequences by class equality and by declared ordering properties, such as per-key ordering where the contract declares it. Physical timing, latency, and non-canonical physical bytes do not participate.
- Introduce three differential result states: agreement, permitted-divergence with recorded class pair, and incomplete, when one side cannot execute a step or a behavior is unsupported. Only a live adapter that emits an outcome class outside its declared set is a differential failure.
- Keep the descriptor-level structural differential unchanged and add the execution differential as a second, separately identified artifact bound into the same evidence bundle.

## Alternatives considered

- Require byte-identical histories. Rejected: live transport and storage have legitimate physical variation that the contracts explicitly permit.
- Extend the existing single differential type with optional execution fields. Rejected: mixing structural and execution evidence in one artifact weakens identity; separate artifacts compose cleanly.

## Non-claims

- Agreement does not prove live equivalence beyond the exercised workload and declared properties.
- Differential evidence does not establish performance, durability, or release eligibility.

## Risks

- Live adapter availability varies by environment; the harness must record incomplete rather than fail, and CI selects available adapters explicitly.
