# Content store adapter delta

## MODIFIED Requirements

### Requirement: Verification precedes availability and exposure
r[molten.content_store_adapter.verify_before_available] Molten MUST verify requested manifest identity, chunk hash, chunk length, chunk order, transform support, reconstructed length, and relevant range before returning bytes or marking content verified and available. Adapter success alone MUST NOT satisfy verification.

The optional object-storage shell MUST bound local bytes and entries independently of operation and remote inventory size, reverify local hits without network I/O and keep cap-exceeding cold reads remote-only.

#### Scenario: Full tier
- GIVEN a tier at its byte cap
- WHEN another valid remote object is fetched
- THEN verified bytes are returned without increasing residency.

#### Scenario: Valid content becomes available
- GIVEN every requested chunk matches the admitted manifest
- WHEN streaming verification completes
- THEN availability MAY transition to verified and the terminal receipt MUST bind the manifest and verified chunks.

#### Scenario: Corrupt chunk is rejected
- GIVEN an adapter returns bytes that do not match a requested chunk ref or length
- WHEN verification evaluates the chunk
- THEN the operation MUST deny before exposing reconstructed content or marking the chunk available.

### Requirement: Content operations are resource bounded
r[molten.content_store_adapter.streaming_bounds] Molten MUST enforce admitted bounds for total bytes, chunk count, chunk size, range size, concurrent operations, queued bytes, memory, deadline, retry, and cancellation on content adapter operations. Adapters MUST NOT require unbounded buffering or hidden queues.

The client MUST admit numeric endpoints, disable redirects/proxies, freeze credentials and endpoint for its lifetime, and fail excess outstanding requests before dispatch. It MUST bound connect/request time and body buffering.

#### Scenario: Invalid resolution policy
- GIVEN a hostname endpoint
- WHEN the client is constructed
- THEN admission fails without DNS or network I/O.

#### Scenario: Bounded stream completes
- GIVEN content and operation sizes remain within the admitted profile
- WHEN a stream is read and verified
- THEN progress and terminal events MUST remain within declared resource bounds.

#### Scenario: Oversized object is denied
- GIVEN a manifest or remote response exceeds the admitted byte or chunk bound
- WHEN adapter preflight or streaming validation detects it
- THEN the operation MUST deny or cancel before excess bytes are exposed or indexed.

### Requirement: Backend protection does not replace retention
r[molten.content_store_adapter.retention_boundary] Molten MUST treat backend tags, leases, or protection handles as adapter effects subordinate to canonical retention pins and deletion gates. Protect, unprotect, pin, unpin, GC, and delete operations MUST preserve separate authority, policy, retention, confidentiality, and evidence checks.

Canonical deletion admission MUST precede remote deletion; protected-object loss at the remote MUST fail a presence audit even if cached locally. Current fixture execution receipts and NVMe device facts MUST NOT be synthesized from source or temporary-directory tests.

#### Scenario: Backend unprotect cannot authorize deletion
- GIVEN a backend protection tag is removed
- WHEN content deletion eligibility is evaluated
- THEN deletion MUST still require normal retention admission, reference-index completeness, authority, and execution evidence.

#### Scenario: Pin does not grant read authority
- GIVEN a manifest is retained by a canonical pin
- WHEN an unauthorized caller requests its bytes
- THEN content read or reveal MUST deny despite the pin.
