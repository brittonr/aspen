# Design

## Separate consumer profiles

Keep `SimulatedWorldManifest` and its reference-world admission/projection unchanged. Add `InstalledWorldInput` containing that mechanism manifest plus installed node, workload and invariant-binding rows. Its mechanism workload must be empty and its invariants must contain the existing universal invariants without reference-system semantic rows. `admit_installed_world` validates these inputs against a separate caller-supplied admitted profile registry and returns the same scheduler-compatible `AdmittedSimulatedWorld`, tagged with private installed-admission metadata. Reference admission produces no installed tag.

An admitted extension profile binds a nominal immutable profile reference, nominal extension identity, the complete existing `ExtensionCoreIdentity`, finite node/step ceilings and exact required `(invariant reference, handler reference)` pairs. Observed node bindings select that profile. Each node's extension identity and both same-core identities must match it exactly. Workload rows bind a contiguous sequence, known node, exact selected profile and canonical request identity. Every selected profile requires each declared invariant once with the exact handler; unknown, missing and duplicate rows deny. Admission never invokes a caller callback or grants authority from a string.

The registry is loaded admitted configuration, not request-supplied policy. Reference syntax and profile equality are not cryptographic provenance proofs. Concrete shells must retain their real source/profile admission and current authority ports.

## Reuse and preservation

Factor only the common world identity, finite bounds, node, port, fault and claim checks. Keep the original reference workload checks and all three reference semantic oracle requirements on the original path. No generic consumer may enter the reference runner as an installed world. Existing reference canonical projections must remain byte-compatible; installed shells emit a separately versioned projection binding their admitted metadata.

Collaboration executes its actual `ScopeApplication`, real Basalt/UCAN verification, existing simulated transport and durable adapters under the generic scheduler. A recorded committed-log simulation profile does not claim to implement Raft; genuine Iroh/Redb multiprocess fixtures separately exercise live consistency. Invariants inspect the actual application states, not a copied product model. Handler identity binds the concrete maintained evaluator source. Mutating its expected identity or omitting an invariant is a negative admission case, not permission to skip evaluation.

## Concurrency and ownership

The collaboration worker owns this core prerequisite and reference-caller migration. Node recovery hosting and federated pull coordinate their consumer profiles without editing these shared core files concurrently. Parent owns common exports, signed producer artifacts, final checks and lifecycle closure. No tests, builds, formatters, gates or commits run mid-flight.
