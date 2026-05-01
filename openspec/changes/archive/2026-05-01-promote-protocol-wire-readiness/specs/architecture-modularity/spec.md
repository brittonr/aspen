## ADDED Requirements

### Requirement: Protocol wire readiness promotion [r[architecture-modularity.protocol-wire-readiness]]
The crate extraction inventory MUST mark the protocol/wire family as `extraction-ready-in-workspace` only when its manifest, policy entries, downstream fixture evidence, compatibility evidence, and dependency-boundary evidence are current and linked from the change verification index.

ID: architecture-modularity.protocol-wire-readiness

#### Scenario: Protocol wire readiness remains below publication [r[architecture-modularity.protocol-wire-readiness.no-publication-claim]]
- GIVEN the protocol/wire crates have reusable schema surfaces and fresh evidence
- WHEN the inventory and policy readiness are updated
- THEN the readiness state SHALL be `extraction-ready-in-workspace`
- AND no protocol/wire entry SHALL claim `publishable from monorepo` or `future repository split candidate`

ID: architecture-modularity.protocol-wire-readiness.no-publication-claim

#### Scenario: Runtime shells stay out of reusable protocol surfaces [r[architecture-modularity.protocol-wire-readiness.runtime-boundary]]
- GIVEN a downstream fixture imports canonical protocol crates directly
- WHEN dependency-boundary and negative grep evidence are generated
- THEN root app, handler, concrete transport, runtime auth, trust/secrets, SQL, UI, and binary shell crates MUST be absent from the reusable protocol default surfaces unless documented as explicit non-reusable consumers.

ID: architecture-modularity.protocol-wire-readiness.runtime-boundary
