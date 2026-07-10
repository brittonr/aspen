# Testing Harness Delta: Iroh Identity Fixtures

### Requirement: Iroh identity fixtures cover restart stability and fail-closed cases
r[molten.testing.iroh_identity_positive_negative_fixtures] Molten SHOULD include positive fixtures for first-boot generation, restart with the same state root preserving endpoint public identity, explicit-key precedence, managed secret-backend load, admitted rotation, and redacted receipts, plus negative fixtures for malformed key metadata, missing required backend, unsafe permissions, unadmitted endpoint drift, stale rotation evidence, private-key receipt leakage, and endpoint identity used as authority.

#### Scenario: Restart fixture preserves endpoint ref
- GIVEN a fixture performs first boot generation and then restarts using the same state-root identity metadata
- WHEN identity resolution runs for the restart
- THEN the endpoint public identity ref matches the first boot receipt and no rotation evidence is required.

#### Scenario: Secret leakage fixture fails
- GIVEN a startup or rotation receipt contains private endpoint key bytes or raw credential material
- WHEN identity receipt validation runs
- THEN the fixture fails with a redaction diagnostic before the receipt can be accepted as startup evidence.

#### Scenario: Endpoint-as-authority fixture denies
- GIVEN a fixture attempts to satisfy operation authority with only an endpoint public identity ref
- WHEN operation admission evaluates the request
- THEN admission denies and records that endpoint identity is transport evidence only.