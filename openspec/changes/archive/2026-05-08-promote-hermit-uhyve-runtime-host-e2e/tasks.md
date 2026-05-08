# Tasks: Promote Hermit/Uhyve runtime-host E2E

## Phase 1: Spec foundation

- [x] [serial] Define Hermit/Uhyve promotion proposal, design, tasks, and runtime-host delta spec.
- [x] [serial] Validate the spec-only package and global OpenSpec set before implementation.

## Phase 2: Runner/product path

- [x] [serial] Add a gated Hermit/Uhyve job runner or worker seam that launches declared Hermit artifacts through Aspen orchestration, not direct shell-only validation.
- [x] [depends:runner] Add a structured Hermit/Uhyve job payload and secret-safe receipt wrapper with image hash, engine, runner identity, lifecycle state, exit status, bounded serial output, and proof marker.
- [x] [depends:runner] Add product-path negative tests proving malformed images/payloads reach Aspen worker orchestration before failing with `ASPEN_HERMIT_UHYVE_RUNTIME_HOST_PRODUCT_PATH_GUARD`.
- [x] [depends:runner] Add an ignored/gated real Uhyve proof test that executes a declared Hermit image through Aspen orchestration and emits `ASPEN_HERMIT_UHYVE_RUNTIME_HOST_EXECUTED`.

## Phase 3: Harness/docs promotion

- [x] [depends:proof] Replace `runtime-host-hermit-gap` with a runnable gated Hermit/Uhyve product-path row only after the real proof test passes.
- [x] [depends:proof] Update runtime-host readiness docs and doc guardrails with Hermit/Uhyve proof commands, markers, and non-evidence examples.
- [x] [depends:proof] Run focused tests, harness export/check, strict OpenSpec validation, archive the completed change, commit, push, and verify clean state.
