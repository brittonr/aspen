# Testing Harness Delta: Iroh ALPN Registry Fixtures

## ADDED Requirements

### Requirement: ALPN registry fixtures cover valid and invalid routing records
r[molten.testing.iroh_alpn_registry_negative_fixtures] Molten SHOULD include positive fixtures for valid registry admission, handler install, replacement, and removal, plus negative fixtures for duplicate ALPN bytes, malformed encoding, wrong owner namespace, stale generation, unsupported lifecycle state, handler-profile mismatch, unsupported incoming ALPN, and attempts to use ALPN routing evidence as authority.

#### Scenario: Duplicate ALPN fixture denies
- GIVEN a fixture with two active registry entries using the same ALPN bytes
- WHEN registry validation runs
- THEN validation denies with a duplicate-ALPN diagnostic and produces no admitted live router map.

#### Scenario: ALPN-as-authority fixture denies
- GIVEN a fixture where a peer has a valid ALPN route and framed-stream receipt but lacks operation authority evidence
- WHEN downstream operation admission runs
- THEN admission denies before side effects and the fixture records ALPN routing as transport evidence only.

#### Scenario: Stale generation fixture preserves live map
- GIVEN a replacement fixture references an old router generation
- WHEN router admission evaluates the replacement
- THEN the decision denies and the expected live advertised ALPN map remains unchanged.