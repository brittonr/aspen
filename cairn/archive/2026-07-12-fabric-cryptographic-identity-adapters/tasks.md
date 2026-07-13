## Phase 1: Pure cryptographic identity boundary

- [x] [serial] Define canonical crypto adapter descriptors, opaque key handles, key purposes, signature domains, operation requests, outcomes, and profile admission in a pure core. r[molten.crypto_identity.adapter_contract] r[molten.crypto_identity.purpose_domain_separation]
- [x] [serial] Refactor identity and federation verification decisions to consume canonical cryptographic outcomes and payload refs without secret material or adapter runtime types. r[molten.crypto_identity.canonical_signature_binding] r[molten.crypto_identity.redaction]
- [x] [parallel] Add positive and negative pure tests for purpose separation, canonical payload binding, stale handles, revocation, rotation, malformed records, and redaction. r[molten.crypto_identity.purpose_domain_separation] r[molten.crypto_identity.rotation_revocation] r[molten.crypto_identity.redaction]

## Phase 2: Production adapter shells

- [x] [serial] Add an Ed25519-compatible production key-store and signer adapter using admitted cryptographic entropy, capability-rooted restricted persistence or managed backend handles, and public-only receipts. r[molten.crypto_identity.production_key_lifecycle] r[molten.crypto_identity.adapter_contract]
- [x] [parallel] Construct live Iroh endpoints and federation signatures from admitted key handles while keeping transport, federation, delegation, evidence, and authority purposes separately scoped. r[molten.crypto_identity.purpose_domain_separation] r[molten.crypto_identity.canonical_signature_binding]
- [x] [parallel] Reclassify deterministic BLAKE3 signatures and public-input-derived endpoint secrets as fixture-only profiles and deny them from production startup and federation gates. r[molten.crypto_identity.fixture_profile_boundary]

## Phase 3: Rotation, readback, and conformance

- [x] [serial] Implement generation-fenced rotation, overlap, revocation, drift, backend-unavailable, and restart behavior through pure plans plus thin adapter effects. r[molten.crypto_identity.rotation_revocation] r[molten.crypto_identity.production_key_lifecycle]
- [x] [parallel] Add bounded redacted operator readback for public identity, purpose, profile, generation, backend class, permissions, currentness, and latest receipts. r[molten.crypto_identity.redaction]
- [x] [parallel] Run shared positive and negative adapter conformance, including restart stability, wrong-purpose, wrong-key, malformed signature, unsafe permission, unavailable backend, stale generation, revoked key, and secret-leak fixtures. r[molten.crypto_identity.adapter_conformance]

## Phase 4: Validation

- [x] [serial] Run focused identity, Iroh endpoint, federation signature, evidence signature, adapter, and negative-security tests. r[molten.crypto_identity.adapter_conformance]
- [x] [serial] Run formatting, Clippy, Cairn validation, proposal/design/tasks gates, and the smallest relevant Nix checks before sync and archive. r[molten.crypto_identity.adapter_conformance]
