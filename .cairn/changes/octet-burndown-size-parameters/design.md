# Design: Octet burn-down, explicit input structs for long parameter lists

## Context

`too_many_parameters` flags functions and methods with six or more parameters, not counting `self`. The repository
convention (AGENTS) is to pass explicit input structs, never positional bags of same-typed arguments.

## Decisions

### Decision: Group the trailing related parameters, and keep the leading owner parameter

**Choice:** The owner or context parameter (for example the root, the state root, the request, or `&mut self`)
stays positional. The related values move into a struct with named fields, and output sinks (`&mut` collectors) stay
positional where they are the function's effect.

**Rationale:** Call sites name every grouped value, so same-typed arguments (`&str` references, `u64` ticks, and
`Option<String>` references) can no longer be swapped silently. The callee's output channels stay visible in its
signature.

### Decision: Destructure at the top of the body

**Choice:** Each function starts with `let XInput { a, b, .. } = input;`. Where that would push a file past the
length limit, the body uses `input.field` access directly (`verify_bundle`, `content_event`).

**Rationale:** The body logic does not change, which keeps the diff reviewable and behavior-preserving.

### Decision: Share a struct when the same group repeats

**Choice:** Groups that repeat use one struct:
- `DependencyEdgeInput` for artifact dependency edges.
- `RunArtifacts` for harness run artifacts.
- `ActionPorts` for replication execution ports.
- `ReplicaAdapterSet` for the five Raft replica adapters, with the `ConcreteReplicaAdapterSet` alias.
- `AdapterDelivery` for observability export deliveries.
- `EventHeader` for fabric-time canonical events.
- `FailureEvidence` for execution failure process and receipt evidence, with a `FailureEvidence::NONE` constant.

**Rationale:** One definition per concept, and callers build the value once and pass it along.

### Decision: Choose struct names that do not repeat module path segments

**Choice:** New struct names avoid words already in their module path, for example `RequestProgress` in
`coordination_delivery::simulation` and `AdmissionSet` in `system_extension::native_host::service`.

**Rationale:** `path_segment_repetition` must not grow. The first candidate run added five such sites, and the names
were changed until the family was level.

## Risks

- Downstream callers of the changed public functions must build the new structs. Every in-tree caller (library,
  the `molten` binary, and tests) is migrated in this change.
