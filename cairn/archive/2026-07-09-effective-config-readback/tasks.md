# Tasks: effective-config-readback

## Phase 1: Canonical readback core

- [x] [serial] r[molten.project.effective_config_readback.artifact] Define the effective-config artifact model with canonical Preserves encoding and BLAKE3 identity.
- [x] [serial] r[molten.project.effective_config_readback.source_trace] Implement pure source-trace normalization for profile values, CLI overrides, defaults, environment-resolved shell inputs, and ledger evidence refs.
- [x] [serial] r[molten.project.effective_config_readback.cli_core] Add pure validate, explain, diff, and fingerprint decision functions over explicit input records.

## Phase 2: CLI and docs

- [x] [parallel] r[molten.project.effective_config_readback.cli_core] Add config readback CLI shell commands that read inputs, call the core, write artifacts, and render diagnostics.
- [x] [parallel] r[molten.project.effective_config_readback.evidence_only] Document that readback artifacts are diagnostics/evidence only and do not replace subsystem gates.
- [x] [parallel] r[molten.project.effective_config_readback.source_trace] Document source classes and override-source rendering.

## Phase 3: Tests and validation

- [x] [parallel] r[molten.project.effective_config_readback.artifact] Add positive tests for stable fingerprints and canonical identity.
- [x] [parallel] r[molten.project.effective_config_readback.source_trace] Add negative tests for conflicting sources, hidden defaults in release mode, and stale profile refs.
- [x] [parallel] r[molten.project.effective_config_readback.cli_core] Add CLI tests that assert canonical artifacts before rendered output.
- [x] [parallel] r[molten.project.effective_config_readback.evidence_only] Add negative tests that readback-only evidence cannot authorize mutation or release gates.
- [x] [serial] r[molten.project.effective_config_readback.artifact] Run focused config/readback tests and Cairn proposal/design/tasks/spec gates.
