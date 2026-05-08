## ADDED Requirements

### Requirement: Testing Harness Public API Follow-up [r[testing-harness-extraction.public-api-followup]]

Aspen MUST review and, where needed, tighten the `aspen-testing` public API so reusable suite inventory and assertion helpers remain available without pulling adapter-specific runtime dependencies into the default surface.

#### Scenario: Reusable inventory API stays dependency-light [r[testing-harness-extraction.public-api-followup.inventory-api]]

- GIVEN downstream tests or tools need to parse suite inventory and validate manifest consistency
- WHEN they depend on the reusable harness API surface
- THEN they SHALL be able to use inventory parsing, validation diagnostics, and report summaries without importing VM, patchbay, madsim, runtime-app, or real-network adapters by default

#### Scenario: Adapter APIs are explicit [r[testing-harness-extraction.public-api-followup.adapter-boundaries]]

- GIVEN a harness consumer needs VM, patchbay, madsim, real-network, or runtime-host execution helpers
- WHEN it enables or imports that adapter surface
- THEN the dependency and feature boundary SHALL be explicit and documented rather than leaking through reusable defaults

#### Scenario: Runtime-host readiness checks use stable harness helpers [r[testing-harness-extraction.public-api-followup.runtime-host-readiness]]

- GIVEN runtime-host readiness checks inspect manifests, generated inventory, proof markers, and anti-overclaiming boundaries
- WHEN those checks use harness APIs
- THEN the APIs SHALL expose stable structured diagnostics rather than requiring each check to scrape ad hoc text output
