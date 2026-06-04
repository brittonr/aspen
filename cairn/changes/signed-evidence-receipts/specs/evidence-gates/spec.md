# Evidence Gates Delta: signed evidence receipts

### Requirement: Signed envelopes preserve subject hashes
r[molten.evidence.signed_receipts.envelope] A signed receipt envelope MUST wrap a canonical receipt without changing the subject receipt's canonical ref.

#### Scenario: Signing a gate receipt preserves gate receipt ref
- GIVEN a canonical `<gate-receipt-v1 ...>`
- WHEN the receipt is signed
- THEN the signed envelope names the same gate receipt ref
- AND the signature covers the canonical bytes of that receipt

### Requirement: Signature verification is fail-closed
r[molten.evidence.signed_receipts.verify] Evidence profiles that require signatures MUST reject missing, malformed, unauthorized, or stale signatures before accepting pass evidence.

#### Scenario: Wrong key purpose is rejected
- GIVEN a signed receipt using a key authorized only for diagnostics
- WHEN a production gate profile requires pass-evidence signing
- THEN the receipt is rejected

#### Scenario: Mismatched subject ref is rejected
- GIVEN a signed envelope whose subject ref does not hash to the embedded receipt
- WHEN verification runs
- THEN verification fails closed

### Requirement: Receipt chains are explicit
r[molten.evidence.signed_receipts.chain] Signed receipt envelopes SHOULD name parent receipt refs when one receipt depends on another.

#### Scenario: Verify receipt chains to report gate receipt
- GIVEN a sealed repro verify receipt
- WHEN it is signed
- THEN the signed envelope names the embedded report gate receipt as a parent
- AND chain verification can reconstruct the gate-to-verify dependency
