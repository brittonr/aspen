# Runtime Spine Delta: Canonical Content-Ref Helper Boundaries

### Requirement: Canonical refs are constructed by shared helpers
r[molten.runtime_spine.canonical_content_refs.helper_construction] Molten MUST construct canonical BLAKE3 content refs through shared content-ref helpers for raw bytes, computed BLAKE3 hash values, and validated lowercase hex inputs rather than duplicating `blake3:` string formatting at subsystem boundaries.

#### Scenario: Byte and hash helpers produce canonical refs
- GIVEN raw artifact, receipt, blob, or transport bytes
- WHEN Molten computes a canonical content ref for those bytes
- THEN the ref is formatted by the shared helper as `blake3:<64 lowercase hex chars>`
- AND callers do not hand-concatenate the scheme prefix.

#### Scenario: Hex helper validates before formatting
- GIVEN a lowercase 64-character BLAKE3 hex digest
- WHEN Molten reconstructs a canonical content ref from the digest
- THEN the shared helper validates the digest length and character set before returning the ref.

### Requirement: Filename readback validates refs
r[molten.runtime_spine.canonical_content_refs.filename_readback] Molten MUST convert ledger, chunk-store, ingress, and evidence filenames back into content refs only through validated hex/readback helpers and MUST fail closed for malformed names.

#### Scenario: Malformed filename does not synthesize a ref
- GIVEN a local materialized filename with a `blake3_` prefix but a malformed, uppercase, path-like, truncated, or overlong digest
- WHEN a store scans materialized content
- THEN Molten rejects or ignores the filename as malformed
- AND does not synthesize a plausible canonical ref from unchecked string concatenation.

#### Scenario: Valid filename remains identity-only
- GIVEN a valid materialized filename that converts to a canonical ref
- WHEN a protected operation requires local content
- THEN Molten still recomputes the ref from the stored bytes or canonical value before side effects.

### Requirement: Transitional aliases are scoped evidence only
r[molten.runtime_spine.canonical_content_refs.scoped_aliases] Molten MAY emit explicitly scoped alternate hash aliases such as Octet `b3:` evidence refs only for integrations that require them, but MUST NOT accept those aliases as canonical runtime content refs unless a future algorithm/model explicitly admits them.

#### Scenario: Octet alias is derived from canonical evidence
- GIVEN an Octet diagnostic artifact that records a `b3:` fingerprint alias
- WHEN Molten emits the alias
- THEN the alias is derived from a validated canonical hash helper path or equivalent checked bytes hashing
- AND the alias remains Octet evidence, not runtime content identity.

### Requirement: Subsystems avoid ad-hoc ref formatting
r[molten.runtime_spine.canonical_content_refs.no_ad_hoc_formatting] Molten subsystems SHOULD NOT hand-build canonical `blake3:` refs or strip/replace the canonical prefix outside the shared Preserves rail helper boundary.

#### Scenario: Ref construction cleanup preserves gate separation
- GIVEN a subsystem migrated from ad-hoc `blake3:` formatting to shared helper construction
- WHEN it parses, stores, or validates refs
- THEN parse failures and diagnostics come from the shared helper
- AND existing authority, policy, provenance, source-gate, retention, resource, transport, and replay gates remain separate from content-ref shape.

### Requirement: Cleanup validation evidence is recorded
r[molten.runtime_spine.canonical_content_refs.cleanup_tests] Molten MUST validate canonical content-ref cleanup with focused malformed-ref/readback tests and source gates before treating the cleanup as complete.

#### Scenario: Cleanup validation passes
- GIVEN content-ref helper cleanup across ledger, chunk-store, remote dataspace, Iroh exchange, Octet evidence, and related synthetic refs
- WHEN validation runs
- THEN focused content-ref tests, affected subsystem tests, clippy, full tests, and Octet gates pass or emit explicit denial evidence.
