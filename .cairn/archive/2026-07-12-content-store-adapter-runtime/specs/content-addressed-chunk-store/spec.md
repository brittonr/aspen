## ADDED Requirements

### Requirement: Content identity remains primitive-owned
r[molten.content_store_adapter.identity_boundary] Molten MUST retain canonical chunk and manifest refs, BLAKE3 hashes, lengths, ordering, transforms, metadata, policy, and evidence as content primitives independent of storage and transport adapters. Backend ids, tickets, tags, provider ids, object keys, and paths MUST NOT replace canonical Molten identity.

#### Scenario: Two adapters preserve one manifest ref
- GIVEN identical verified content is stored through local and live content adapters
- WHEN each adapter reports availability
- THEN both results MUST bind the same canonical manifest and chunk refs
- AND MAY bind different backend locator hints.

### Requirement: Content adapters expose canonical bounded operations
r[molten.content_store_adapter.port_contract] Molten MUST define versioned adapter contracts for streaming put, streaming get, verified range read, availability query, import, export, protection handle, cancellation, and bounded status. Commands and events MUST use canonical ids and values rather than backend runtime objects.

#### Scenario: Streaming get uses canonical events
- GIVEN an extension holds an admitted content-store binding and manifest ref
- WHEN it requests a bounded streaming get
- THEN the adapter MUST emit correlated canonical chunk or byte-range events and a terminal outcome.

#### Scenario: Unsupported range read denies
- GIVEN a selected adapter profile does not declare range support
- WHEN a range command is submitted
- THEN validation MUST deny before adapter I/O instead of silently reading the whole object.

### Requirement: Content operations are resource bounded
r[molten.content_store_adapter.streaming_bounds] Molten MUST enforce admitted bounds for total bytes, chunk count, chunk size, range size, concurrent operations, queued bytes, memory, deadline, retry, and cancellation on content adapter operations. Adapters MUST NOT require unbounded buffering or hidden queues.

#### Scenario: Bounded stream completes
- GIVEN content and operation sizes remain within the admitted profile
- WHEN a stream is read and verified
- THEN progress and terminal events MUST remain within declared resource bounds.

#### Scenario: Oversized object is denied
- GIVEN a manifest or remote response exceeds the admitted byte or chunk bound
- WHEN adapter preflight or streaming validation detects it
- THEN the operation MUST deny or cancel before excess bytes are exposed or indexed.

### Requirement: Verification precedes availability and exposure
r[molten.content_store_adapter.verify_before_available] Molten MUST verify requested manifest identity, chunk hash, chunk length, chunk order, transform support, reconstructed length, and relevant range before returning bytes or marking content verified and available. Adapter success alone MUST NOT satisfy verification.

#### Scenario: Valid content becomes available
- GIVEN every requested chunk matches the admitted manifest
- WHEN streaming verification completes
- THEN availability MAY transition to verified and the terminal receipt MUST bind the manifest and verified chunks.

#### Scenario: Corrupt chunk is rejected
- GIVEN an adapter returns bytes that do not match a requested chunk ref or length
- WHEN verification evaluates the chunk
- THEN the operation MUST deny before exposing reconstructed content or marking the chunk available.

### Requirement: Partial and uncertain outcomes are explicit
r[molten.content_store_adapter.partial_state] Molten MUST distinguish accepted, streaming, verified, durable where supported, cancelled, retryable, failed, and uncertain outcomes and MUST persist bounded verified partial state for resumable operations. A disconnect or process failure MUST NOT be normalized to definite success or definite absence without supporting evidence.

#### Scenario: Interrupted fetch resumes verified chunks
- GIVEN a fetch verifies a subset of manifest chunks before cancellation
- WHEN a later request resumes under the same compatible manifest and policy
- THEN the plan MUST request only still-missing chunks
- AND MUST revalidate retained partial-state identity.

### Requirement: Backend protection does not replace retention
r[molten.content_store_adapter.retention_boundary] Molten MUST treat backend tags, leases, or protection handles as adapter effects subordinate to canonical retention pins and deletion gates. Protect, unprotect, pin, unpin, GC, and delete operations MUST preserve separate authority, policy, retention, confidentiality, and evidence checks.

#### Scenario: Backend unprotect cannot authorize deletion
- GIVEN a backend protection tag is removed
- WHEN content deletion eligibility is evaluated
- THEN deletion MUST still require normal retention admission, reference-index completeness, authority, and execution evidence.

#### Scenario: Pin does not grant read authority
- GIVEN a manifest is retained by a canonical pin
- WHEN an unauthorized caller requests its bytes
- THEN content read or reveal MUST deny despite the pin.

### Requirement: Live and simulated adapters preserve one contract
r[molten.content_store_adapter.live_sim_conformance] Molten MUST provide capability-rooted local, Redb-indexed, live Iroh content, and deterministic simulation profiles through the same canonical operations, transitions, bounds, cancellation, failure classes, identity rules, and non-claims. Adapter-specific capabilities MUST be explicit and versioned.

#### Scenario: Shared fixture matches
- GIVEN a no-fault bounded content fixture runs through live loopback and simulation profiles with equivalent declared capabilities
- WHEN canonical observations are compared
- THEN manifest identity, verification transitions, partial state, and terminal outcomes MUST fall within the same allowed trace set.

### Requirement: Content adapters have positive and negative conformance
r[molten.content_store_adapter.final_validation] Molten MUST test every admitted content adapter with positive store, stream, range, resume, protection, restart, and cancellation cases and negative corruption, truncation, reordering, unexpected chunk, stale ticket, unsupported transform, root escape, retention denial, overload, and secret-leak cases.

#### Scenario: Non-conforming adapter is rejected
- GIVEN an adapter bypasses verification, leaks backend handles, loses terminal events, exceeds bounds, or treats transport success as authority
- WHEN shared conformance and profile admission run
- THEN production admission MUST deny with the failed invariant.
