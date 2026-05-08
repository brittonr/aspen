## ADDED Requirements

### Requirement: Reproducible Hermit Uhyve Marker Fixture [r[runtime-host-loading.hermit-uhyve-marker-fixture]]
Aspen MUST provide a reproducible, source-provenanced Hermit marker fixture before treating Hermit/Uhyve proof reproduction as self-contained.

#### Scenario: Marker fixture package is source provenanced [r[runtime-host-loading.hermit-uhyve-marker-fixture.source-provenance]]
- GIVEN the Hermit/Uhyve runtime-host row is reproduced on a capable host
- WHEN the marker fixture is selected for the gated proof
- THEN the fixture SHALL come from a reproducible Aspen flake package rather than an untracked `/tmp` binary
- AND the package SHALL record source revision, target triple, expected marker, and output image path as non-secret metadata

#### Scenario: Fixture package is not runtime-host proof [r[runtime-host-loading.hermit-uhyve-marker-fixture.not-proof]]
- GIVEN `.#hermit-uhyve-marker` builds successfully or its metadata contract check passes
- WHEN runtime-host readiness is reported
- THEN Aspen SHALL treat the package result as prerequisite fixture evidence only
- AND it SHALL NOT satisfy the Hermit/Uhyve row unless a real Uhyve run through `JobManager` and `WorkerPool` emits `ASPEN_HERMIT_UHYVE_RUNTIME_HOST_EXECUTED` in the product-visible receipt

#### Scenario: Gated proof prefers packaged fixture [r[runtime-host-loading.hermit-uhyve-marker-fixture.gated-proof]]
- GIVEN a host has real Uhyve, virtualization support, and the packaged marker fixture
- WHEN the ignored Hermit/Uhyve product-path test is run deliberately
- THEN the documented command SHALL be able to derive `ASPEN_UHYVE` from `.#uhyve` and `ASPEN_HERMIT_UHYVE_IMAGE` from `.#hermit-uhyve-marker`
- AND the proof SHALL retain the existing marker enforcement and failure behavior for successful Uhyve exits that omit the expected marker
