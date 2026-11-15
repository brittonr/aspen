# World commit delta

## ADDED Requirements

### Requirement: Runtime-spine logical execution

r[molten.world_snapshot.runtime_spine_restore] The executable runtime-spine logical profile MUST observe its running executable cohort, verify every closed component projection, reconstruct RuntimeState through its owner and persist/read back state through a capability-derived typed local store before activation. Supplied destination descriptors MUST NOT establish observed compatibility.

#### Scenario: Process restart

- GIVEN a complete admitted committed-turn runtime-spine image was activated and its restore process exited
- WHEN a fresh process reads the typed store and reconstructs the runtime owner
- THEN it MUST recover the same pending messages, assertions, observers, facets, logical time, RNG and effect sequence without dispatching unresolved requests

### Requirement: Fresh admission and honest failure

r[molten.world_snapshot.runtime_spine_activation] Activation MUST use fresh independently provisioned policy, capability, revocation, resource, generation and adapter facts, reread immediately before activation. Partial reconstruction MUST remain inactive. Failed activation writes MUST report uncertainty; receipt failure after activation MUST retain the completed activation fact. Restoration MUST NOT automatically delete retained reconstruction evidence or dispatch unresolved effects.

#### Scenario: Revocation during reconstruction

- GIVEN reconstruction has staged verified component state under one exact current admission
- WHEN revocation, generation or any signed current fact changes before activation
- THEN the attempt MUST remain inactive and retain its descriptor-isolated rows for reconciliation

#### Scenario: Receipt storage fails after activation

- GIVEN the runtime activation transaction completed
- WHEN canonical receipt storage or operator output publication fails
- THEN the public route MUST preserve the completed activation fact instead of reporting policy denial or proof of no activation
