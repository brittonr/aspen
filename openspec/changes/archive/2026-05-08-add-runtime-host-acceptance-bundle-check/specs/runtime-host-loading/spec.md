## ADDED Requirements

### Requirement: Runtime Host Acceptance Bundle Check [r[runtime-host-loading.acceptance-bundle-check]]

Aspen MUST provide a deterministic acceptance-bundle check that verifies promoted runtime-host readiness surfaces remain synchronized without requiring gated host execution by default.

#### Scenario: Promoted runtime-host surfaces agree [r[runtime-host-loading.acceptance-bundle-check.promoted-surfaces]]

- GIVEN the runtime-host readiness documentation, suite manifests, and generated inventory exist
- WHEN the acceptance-bundle check runs
- THEN it SHALL verify promoted product-path rows use product-path manifest names, appear in generated inventory, and are discoverable from operator-facing readiness documentation

#### Scenario: Proof markers are pinned [r[runtime-host-loading.acceptance-bundle-check.proof-markers]]

- GIVEN promoted WASM, Hyperlight, OCI-lowered WASM, microVM, and Hermit/Uhyve runtime-host rows exist
- WHEN the acceptance-bundle check evaluates their documentation and manifest anchors
- THEN it SHALL require the expected positive proof markers and guard/non-proof markers for each promoted row where applicable

#### Scenario: Build-only evidence remains non-proof [r[runtime-host-loading.acceptance-bundle-check.non-proof-boundary]]

- GIVEN prerequisite packages or checks such as `.#uhyve`, `.#hermit-uhyve-marker`, or marker metadata contracts pass
- WHEN the acceptance-bundle check evaluates readiness claims
- THEN it SHALL require wording or metadata that prevents package, fixture, metadata, admission-only, or fake-runner evidence from being treated as runtime-host proof
