# Reference simulation admission cutover

## Existing authority and dependency audit

`molten_core::fabric_simulation::admit_simulated_world` validates the caller's `SimulatedWorldManifest` identity, positive bounds, node and workload shapes/counts, ports, faults, invariants and claims, then sorts a normalized clone. Its published `MAX_WORLD_NODES = 64` and `MAX_WORLD_WORKLOAD_STEPS = 4_096` are enforced at `validate_bounds` through `check_count`. `canonical_admit_simulated_world` invokes that core admission and hashes its admitted manifest. Neither API accepts nor reads host observations, so full existing admission can run before any reference host is built. No fabricated partial or count-only preflight is required.

## Minimal sequencing change

At entry to `prepare_world_for_manifest`, assign `let world = canonical_admit_simulated_world(manifest)?;`. Keep every following existing operation in order: reference profiles/descriptors/operations, system-extension tier admission, iteration over the original `manifest.nodes` and each service admission, executor creation, host creation and activation, duplicate-host check and admissions collection. Return the preadmitted `world` field without a second core admission. This changes only errors for nonadmitted manifests: core denial (including node/workload structural exceedance) precedes reference-host errors. For admitted manifests the core call succeeds first, leaving service/authority/host error order and simulation execution untouched.

## Observable regression and smoke

A forged 65-node manifest includes otherwise valid reference nodes plus an early unrecognized reference service; a forged 4,097-workload-step manifest uses a similarly unrecognized service after valid reference nodes. Before the cutover each reaches host creation and returns the service-selection error; afterward each returns the existing core `TooManyItems` diagnostic at 65/64 or 4,097/4,096 before selecting/activating that host. A within-count manifest with the same unsupported service still returns the original reference-service error. Smoke the compiled public `run_reference_world` API with these inputs and run the actual built-in `fabric-simulation run` CLI for the admitted fixture, which does not expose arbitrary manifest intake.

## Verification and constraints

Run focused red/green boundary tests, existing simulation unit/CLI tests, owner/composition fmt and Clippy, scoped nonincremental Octet with `status.json` warning counts, and strict Cairn. Do not invent a numeric budget, alter acceptance authority, accepted specs, waiver/policy/pin or retry the known-failing full strict Octet gate.
