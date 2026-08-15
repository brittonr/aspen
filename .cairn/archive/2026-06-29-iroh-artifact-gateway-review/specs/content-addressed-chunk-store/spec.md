## ADDED Requirements

### Requirement: Operator gateway verified range readback
r[molten.operator_gateway.verified_range_read] Molten MUST verify chunk-store manifest identity, relevant chunk hashes, chunk lengths, transform support, and reconstructed byte ranges before any operator gateway response exposes bytes.

#### Scenario: Valid range returns verified bytes
- GIVEN a visible chunk manifest and a bounded byte-range request
- WHEN the operator gateway maps the byte range to chunk refs
- THEN every relevant chunk is verified before response bytes are emitted
- AND the gateway range receipt binds the manifest ref, normalized range, chunk refs, and verification checks.

#### Scenario: Corrupt chunk denies before response
- GIVEN a requested range whose backing chunk bytes do not match the chunk ref or declared length
- WHEN the operator gateway verifies the range
- THEN it emits a deny receipt with corrupt-chunk diagnostics
- AND no plaintext response bytes are exposed.

#### Scenario: Unsupported transform denies before response
- GIVEN a manifest range that requires an unsupported compression, encryption, or transform mode
- WHEN the operator gateway evaluates the range
- THEN it emits a deny receipt for unsupported transform
- AND the gateway does not expose transformed or plaintext bytes.
