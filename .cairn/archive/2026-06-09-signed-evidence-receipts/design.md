# Design: signed evidence receipts

## Envelope

A signed receipt envelope should wrap any canonical receipt value without changing that receipt's content hash:

`<signed-receipt-v1 "molten.evidence.signed-receipt.v1" <subject ...> <signer ...> <signature ...> <parents [...]> <checks [...]>>`

The subject binds:

- receipt schema id;
- canonical receipt ref;
- byte encoding profile;
- evidence class (`gate`, `repro-verify`, `redaction-transform`, `reveal`, `runtime`).

The signer binds:

- node/operator identity ref;
- signing key ref;
- key purpose;
- trust root or fixture ref;
- revocation epoch or status evidence when available.

## Signature verification

Verification must operate over canonical Preserves bytes of the subject receipt. It must fail closed on unknown algorithms, unsupported keys, stale revocation epochs, mismatched receipt refs, or a signer whose purpose does not authorize the evidence class.

## Chains

Receipts can name parent receipt refs. For example, a repro verify receipt can parent the embedded report gate receipt; an unpack receipt can parent the verify receipt; a distributed exchange receipt can parent the published signed receipt.

## Local development mode

Local unsigned receipts remain useful for diagnostics. A suite/policy/gate profile decides whether unsigned receipts are acceptable. Production evidence profiles should require signed envelopes and explicit trust roots.
