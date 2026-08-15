# Tasks: nickel-envelope-payload-contracts

## Phase 1: Typed envelope contracts

- [x] [serial] r[molten.nickel_envelope_payload_contracts.schema_payload_coupling] Add schema-specific plugin contract and grant envelope contracts in Nickel.
- [x] [parallel] r[molten.nickel_envelope_payload_contracts.identity_binding] Add export-identity predicates that bind envelope identity to payload extension id/version or plugin id/operation.

## Phase 2: Fixture migration

- [x] [serial] r[molten.nickel_envelope_payload_contracts.fixture_migration] Migrate plugin extension contract and grant envelope fixtures to the typed envelope contracts.
- [x] [parallel] r[molten.nickel_envelope_payload_contracts.negative_envelopes] Add negative fixtures for wrong payload type, identity mismatch, wrong schema id, unsupported source, and missing metadata.

## Phase 3: Drift and validation

- [x] [serial] r[molten.nickel_envelope_payload_contracts.schema_payload_coupling] Regenerate drift-gated generated JSON and run the contract export drift gate.
- [x] [serial] r[molten.nickel_envelope_payload_contracts.fixture_migration] Run `nix run path:$PWD#cairn -- validate --root .`.
