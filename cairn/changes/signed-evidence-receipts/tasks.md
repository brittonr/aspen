# Tasks: signed-evidence-receipts

- [x] [serial] r[molten.evidence.signed_receipts.envelope] Define canonical signed receipt envelope schema without changing subject receipt hashes.
- [x] [serial] r[molten.evidence.signed_receipts.key_fixtures] Add signer identity/key/trust-root fixtures for local harness and CLI verification.
- [x] [serial] r[molten.evidence.signed_receipts.verify] Implement fail-closed signature verification for configured evidence profiles.
- [x] [serial] r[molten.evidence.signed_receipts.chain] Bind parent receipt refs for report gate, bundle gate, verify, unpack, redaction, and reveal receipts.
- [x] [parallel] r[molten.evidence.signed_receipts.cli] Add CLI commands to sign and verify receipt envelopes.
- [x] [parallel] r[molten.evidence.signed_receipts.tests] Add tests for wrong signer purpose, stale revocation epoch, mismatched subject ref, unsupported algorithm, and missing trust root.
