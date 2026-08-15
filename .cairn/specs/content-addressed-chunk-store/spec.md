# Content Addressed Chunk Store Specification

## Purpose

Defines the `content-addressed-chunk-store` capability.

## Requirements

### Requirement: Chunk manifests are canonical
r[molten.chunk_store.manifest_model] Molten MUST represent chunked objects with canonical `chunk-manifest-v1` records that bind object kind, total length, chunker version, chunk refs, Merkle/root refs, metadata refs, policy refs, and evidence refs.

#### Scenario: Manifest identity is stable
- GIVEN identical object bytes, metadata, chunker parameters, policy refs, and evidence refs
- WHEN Molten renders the chunk manifest
- THEN the manifest has the same canonical content ref.

### Requirement: Chunk refs are canonical
r[molten.chunk_store.chunk_ref_model] Molten MUST represent chunks with canonical refs that bind hash, length, domain, chunker version, transform metadata, location hints, and evidence refs.

#### Scenario: Chunk ref binds fixed chunk metadata
- GIVEN a chunk produced by the fixed chunker
- WHEN Molten renders its chunk ref
- THEN the ref records length, domain, chunker version, transforms, and evidence independently of transport location.

### Requirement: Fixed v1 chunking is deterministic
r[molten.chunk_store.fixed_v1] Molten MUST provide deterministic fixed-size chunking as the first chunker version.

#### Scenario: Same bytes chunk identically
- GIVEN the same byte stream and fixed_v1 chunk size
- WHEN Molten chunks the stream twice
- THEN chunk boundaries and chunk refs are identical.

### Requirement: Transport ids are not object identity
r[molten.chunk_store.no_transport_identity] Molten MUST treat Iroh blob ids, filesystem paths, and storage locations as hints, not canonical Molten object identity.

#### Scenario: Iroh ticket preserves manifest identity
- GIVEN a manifest fetched from an Iroh-style ticket
- WHEN Molten imports the chunks
- THEN the manifest ref remains the canonical identity rather than the ticket or blob id.

### Requirement: Streaming verification is fail-closed
r[molten.chunk_store.streaming_verify] Molten MUST verify manifest hash/root, chunk hash/length, chunk order/proofs, and reconstructed total length during streaming fetch or read.

#### Scenario: Corrupt chunk is rejected
- GIVEN a stored chunk whose bytes do not match its ref
- WHEN Molten verifies or reads the manifest
- THEN the operation denies before returning object bytes.

### Requirement: Local index is durable
r[molten.chunk_store.redb_index] Molten MUST maintain a Redb-backed local index for manifests, chunks, availability, pins, partial fetch state, and receipts.

#### Scenario: Rebuild restores derived state
- GIVEN chunk store files and historical receipts
- WHEN Molten rebuilds the index
- THEN manifests, chunks, availability, pins, partial fetch state, and receipts are represented consistently.

### Requirement: Range reads verify relevant chunks
r[molten.chunk_store.range_reads] Molten MUST support fixed-size range reads by mapping byte ranges to chunk refs and verifying all relevant chunks before returning bytes.

#### Scenario: Range read returns verified slice
- GIVEN a manifest and byte range crossing chunk boundaries
- WHEN Molten performs a range read
- THEN only verified bytes in the requested range are returned.

### Requirement: Fetches are resumable
r[molten.chunk_store.resumable_fetch] Molten MUST support resumable fetch by requesting only missing chunks from a manifest.

#### Scenario: Missing chunks are fetched only once
- GIVEN a destination store that already has some manifest chunks
- WHEN Molten syncs from a source store
- THEN it fetches missing chunks and preserves existing verified chunks.

### Requirement: Pins protect GC
r[molten.chunk_store.pin_gc] Molten MUST track object-manifest and chunk-level pins and deny GC for chunks reachable from pinned or retained manifests.

#### Scenario: Pinned chunk is not removed
- GIVEN a pinned manifest or chunk
- WHEN GC evaluates removal
- THEN reachable chunks remain present and receipt diagnostics explain the retention.

### Requirement: Operations emit receipts
r[molten.chunk_store.receipts] Molten MUST emit receipts for manifest creation, chunk verification, fetch, range read, dedup hit, pin, unpin, GC, tombstone, and denial decisions.

#### Scenario: Denied operation has evidence
- GIVEN a chunk operation that fails validation
- WHEN Molten denies the operation
- THEN it emits a receipt binding the decision, diagnostics, manifest or chunk refs, and checks.

### Requirement: Confidentiality metadata is explicit
r[molten.chunk_store.confidentiality] Molten MUST support encryption/redaction policy metadata and avoid plaintext chunk hash leakage when confidentiality policy requires protected commitments.

#### Scenario: Protected commitment denies plaintext exposure
- GIVEN a manifest marked with protected-commitment confidentiality
- WHEN an operation would expose plaintext chunks without reveal authority
- THEN Molten denies and emits evidence instead of plaintext bytes.

### Requirement: Transform ordering is represented
r[molten.chunk_store.compression_modes] Molten MUST represent compression/encryption ordering explicitly in manifests and chunk refs.

#### Scenario: Unsupported transform denies safely
- GIVEN a manifest declaring an unsupported compression or encryption transform
- WHEN Molten reads or verifies plaintext bytes
- THEN the operation denies before exposing bytes.

### Requirement: Iroh adapter preserves identity
r[molten.chunk_store.iroh_adapter] Molten MUST map chunk and manifest fetch/store operations to Iroh blobs while preserving canonical manifest identity.

#### Scenario: Published blobs fetch to same manifest
- GIVEN a manifest published through the Iroh-style adapter
- WHEN another store fetches the ticket
- THEN the fetched manifest ref matches the original manifest ref.

### Requirement: Remote sync uses manifests
r[molten.chunk_store.remote_sync] Molten MUST use manifests for remote artifact sync missing-chunk calculation and resumable fetch.

#### Scenario: Remote sync resumes from manifest state
- GIVEN a remote manifest and a partial local store
- WHEN Molten syncs missing chunks
- THEN only unavailable chunks are fetched and indexed.

### Requirement: Typed storage large values use manifests
r[molten.chunk_store.typed_storage] Molten MUST store large typed-storage values as manifest refs and verify chunks before loading them.

#### Scenario: Typed storage load verifies chunks
- GIVEN a typed-storage record backed by a chunk manifest
- WHEN Molten loads the value
- THEN chunk hashes and manifest root are verified before the value is returned.

### Requirement: Replay snapshots use manifests
r[molten.chunk_store.replay_snapshots] Molten MUST use manifest refs for replay snapshots and logs and support partial chunk fetch for first-divergence debugging.

#### Scenario: Divergence debug fetches needed chunks
- GIVEN a replay snapshot manifest and missing local chunks
- WHEN Molten investigates first divergence
- THEN it fetches or reports only the chunks needed for the relevant snapshot or log range.

### Requirement: Catalog exposes chunk store state
r[molten.chunk_store.catalog] Molten MUST expose manifest/chunk availability, dedup ratio, and pin state through catalog and MCP views subject to visibility policy.

#### Scenario: Hidden refs are not rendered
- GIVEN a chunk catalog request with hidden refs
- WHEN Molten renders chunk availability, dedup, and pin summaries
- THEN hidden manifest or chunk refs are omitted from the catalog and MCP response.

### Requirement: Manifest identity tests cover stable refs
r[molten.chunk_store.identity_tests] Molten MUST test that fixed_v1 chunking produces stable manifest ids for identical bytes and different ids when bytes or chunker parameters change.

#### Scenario: Identity test catches chunker drift
- GIVEN changed chunker parameters
- WHEN the identity test computes the manifest ref
- THEN it differs from the original manifest ref.

### Requirement: Dedup tests cover shared chunks
r[molten.chunk_store.dedup_tests] Molten MUST test that chunks deduplicate across artifact, storage, and replay objects.

#### Scenario: Shared bytes produce dedup evidence
- GIVEN two objects sharing chunk bytes
- WHEN both are stored
- THEN dedup receipts or summaries show the shared chunk was not rewritten.

### Requirement: Verification tests reject invalid chunks
r[molten.chunk_store.verify_tests] Molten MUST test that corrupted, missing, reordered, or wrong-length chunks are rejected.

#### Scenario: Wrong length chunk denies
- GIVEN a chunk file with the wrong length
- WHEN Molten verifies its manifest
- THEN verification fails closed.

### Requirement: GC tests cover pin safety
r[molten.chunk_store.gc_tests] Molten MUST test that chunks reachable from pinned manifests cannot be deleted and become eligible after all pins are removed.

#### Scenario: Unpinned chunks become eligible
- GIVEN a manifest whose pins have been removed
- WHEN GC runs with valid deletion evidence
- THEN eligible chunks may be removed and tombstone evidence is emitted.

### Requirement: Property tests cover chunk invariants
r[molten.chunk_store.property_tests] Molten MUST add property tests for chunking determinism, range-read correctness, resumable fetch completeness, and no-dangling-chunk invariants.

#### Scenario: Generated sync leaves no missing chunks
- GIVEN generated bounded byte streams and partial destination stores
- WHEN property tests run resumable sync
- THEN the destination has no missing chunks for the synced manifest.

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

### Requirement: Chunk availability state is proof checked
r[molten.chunk_cache_state_proof.chunk_availability] Molten MUST prove that chunk manifests, chunk entries, availability indexes, partial fetch receipts, and missing scans agree before serving or reconstructing content.

#### Scenario: Corrupt fetched chunk denies read
- GIVEN a manifest whose fetched chunk bytes hash to the wrong chunk ref
- WHEN chunk reconstruction is requested
- THEN the read or fetch receipt decision is `deny`
- AND no reconstructed artifact ref is emitted.

### Requirement: Chunk GC requires exact retention gates
r[molten.chunk_cache_state_proof.retention_gc] Molten MUST prove that chunk and manifest GC removes content only when matching retention apply and execution gate refs bind the same object ref, object kind, action, and retention class.

#### Scenario: Missing apply ref denies chunk removal
- GIVEN an unpinned chunk candidate and no matching retention apply ref
- WHEN non-dry-run chunk GC is requested
- THEN GC decision is `deny`
- AND the chunk remains present or marked unavailable without deletion.

### Requirement: Chunk store responsibilities are semantically separated
r[molten.chunk_store.modularity.boundaries] Chunk store implementation SHOULD separate model types, canonical manifest codec, pure verification, filesystem storage, index adapter, Iroh exchange, retention integration, lineage evidence, and shell orchestration.

#### Scenario: Chunk module ownership is clear
- GIVEN chunk store code is reorganized
- WHEN reviewers inspect the module layout
- THEN each module has an identifiable responsibility such as model, codec, verify, fs_store, index, exchange, retention, lineage, or shell

### Requirement: Chunk refactors preserve content identity
r[molten.chunk_store.modularity.identity_preserving] Chunk store modularity refactors MUST preserve canonical manifest bytes, chunk refs, lineage refs, and parser decisions for existing artifact versions unless a separate schema change owns the break.

#### Scenario: Manifest ref remains stable
- GIVEN a representative valid chunk manifest fixture
- WHEN the manifest is reconstructed through the extracted codec boundary
- THEN its canonical bytes and BLAKE3 ref match the pre-migration fixture

#### Scenario: Tampered chunk denies verification
- GIVEN a manifest whose referenced chunk bytes are missing or tampered
- WHEN the pure verifier evaluates the manifest and byte summaries
- THEN verification fails before publish, import, GC, or lineage evidence is promoted

### Requirement: Chunk destructive paths consume retention admission
r[molten.chunk_store.modularity.retention_boundary] Chunk deletion, GC, unpin, destructive index mutation, or tombstone emission MUST consume admitted retention evidence or an explicit non-destructive plan before mutating local chunk state.

#### Scenario: Missing retention admission blocks chunk deletion
- GIVEN a chunk or manifest is locally present but retention admission is missing or denied
- WHEN the chunk store plans deletion or GC
- THEN the plan denies or omits destructive effects

### Requirement: Chunk boundary changes include positive and negative tests
r[molten.chunk_store.modularity.tests] Chunk store boundary refactors SHOULD include positive identity and verification tests plus negative tests for tampered bytes, missing chunks, malformed manifests, stale lineage, or missing retention admission.

#### Scenario: Chunk tests cover identity and denial
- GIVEN a chunk boundary is extracted
- WHEN reviewers inspect focused tests
- THEN valid content identity and at least one denied malformed or destructive path are covered

### Requirement: Local chunk-store filesystem authority is capability-rooted
r[molten.chunk_store.cap_std_boundary.root_wrappers] Molten MUST represent local artifact, chunk, retention, dataspace, and exchange filesystem authority as typed capability roots opened by the outer runtime or adapter shell.

#### Scenario: Valid local store path stays under the declared root
r[molten.chunk_store.cap_std_boundary.tests.positive]
- GIVEN Molten has opened an operator-declared local store root
- WHEN chunk-store, retention, dataspace, or exchange code reads or writes a valid relative path through the typed root
- THEN the operation MUST remain confined to that declared root while preserving canonical chunk and manifest identity behavior.

#### Scenario: Invalid local store path cannot escape the root
r[molten.chunk_store.cap_std_boundary.tests.negative]
- GIVEN input names `../` traversal, an absolute path, a missing capability root, a symlink escape attempt, or a remote locator presented as a local path
- WHEN Molten resolves the locator for local filesystem access through the typed root
- THEN Molten MUST fail closed before reading, writing, deleting, or exposing local artifact bytes outside the declared root.

### Requirement: Capability dependency stays outside identity cores
r[molten.chunk_store.cap_std_boundary.dependency] Molten MUST add `cap-std` only to modules that own local filesystem effects and MUST keep chunk, manifest, catalog, and identity cores independent of capability filesystem APIs.

#### Scenario: Manifest and catalog logic remains filesystem-neutral
r[molten.chunk_store.cap_std_boundary.conversion]
- GIVEN manifest, chunk-ref, catalog, and visibility-policy logic is reviewed
- WHEN cap-std adoption is complete
- THEN those cores MUST operate on in-memory content refs, DTOs, or validated relative locators and MUST NOT open ambient filesystem paths directly.

### Requirement: Capability boundary is documented without artifact overclaims
r[molten.chunk_store.cap_std_boundary.docs] Molten MUST document that capability roots bound local filesystem authority only and do not prove artifact truth, confidentiality, remote transport trust, Merkle correctness, or distributed runtime correctness.

#### Scenario: Closeout evidence includes local boundary validation
r[molten.chunk_store.cap_std_boundary.validation]
- GIVEN the cap-std artifact-store change is ready to archive
- WHEN focused chunk-store, retention, dataspace, exchange, Cairn validation, and gate checks run
- THEN the evidence MUST include positive and negative filesystem-boundary coverage and visible non-claim documentation.

### Requirement: Chunk-store traversal sync strategies
r[molten.chunk_store.traversal_sync_strategy] Molten SHOULD plan chunk-manifest sync with deterministic traversal strategies such as stem-first metadata fetch, leaf-only data fetch, peer-partitioned leaf fetch, and resumable missing-chunk fetch while preserving canonical manifest identity.

#### Scenario: Stem-first sync preserves manifest identity
- GIVEN a large chunk manifest whose branch metadata and leaf chunks can be fetched separately
- WHEN Molten plans a stem-first sync
- THEN the plan fetches only the selected metadata refs first
- AND the final manifest ref remains the canonical identity after leaves are fetched and verified.

#### Scenario: Leaf partitioning does not duplicate completed chunks
- GIVEN multiple candidate peers and a destination store with some verified chunks already present
- WHEN Molten plans partitioned leaf fetches
- THEN fetch effects are emitted only for missing chunks assigned by the deterministic plan
- AND already verified chunks are not fetched again.

#### Scenario: Traversal strategy denies on manifest drift
- GIVEN fetched metadata changes the expected manifest tree or chunk refs from the traversal descriptor
- WHEN Molten validates the sync plan against fetched refs
- THEN the sync denies before reconstructing or exposing object bytes.

### Requirement: Remote byte-source hints are not identity
r[molten.chunk_store.remote_byte_source_hints] Molten MAY record S3, HTTP, gateway, or other remote byte-source locations and outboard verification metadata as location hints, but MUST treat canonical manifests and chunk refs as object identity.

#### Scenario: Remote range read verifies before bytes are exposed
- GIVEN a remote byte-source hint for a visible manifest range
- WHEN an operator gateway reads that range
- THEN Molten verifies the relevant chunk refs, lengths, transforms, and reconstructed range before returning bytes
- AND the gateway receipt binds the source hint and verification evidence.

#### Scenario: Changed remote object denies readback
- GIVEN a remote source now returns bytes that do not match the expected chunk refs or manifest root
- WHEN Molten attempts readback or import
- THEN it emits a deny receipt
- AND no mismatched bytes are exposed, pinned, installed, or executed.

#### Scenario: Location hint cannot pin or delete content
- GIVEN a remote byte-source hint is present in a manifest or catalog record
- WHEN a subsystem evaluates retention, pinning, deletion, or import
- THEN it requires normal manifest, retention, authority, policy, and evidence gates
- AND the location hint alone grants no mutation authority.

### Requirement: Chunk traversal sync has positive and negative coverage
r[molten.chunk_store.traversal_sync_tests] Molten SHOULD test deterministic chunk traversal planning with positive cases for stem-first sync, leaf-only sync, partitioned fetch, and resumable missing chunks, plus negative cases for manifest drift, stale source hints, corrupt chunks, unsupported transforms, and duplicate or unexpected chunks.

#### Scenario: Resumable sync leaves no missing chunks
- GIVEN a destination store with a partial verified manifest and a deterministic fetch plan
- WHEN the sync completes successfully
- THEN the destination has all chunks required by the manifest
- AND the receipt records which chunks were already present and which were fetched.

#### Scenario: Unexpected chunk is rejected
- GIVEN a sender or remote source returns a chunk not requested by the deterministic plan
- WHEN Molten validates the response
- THEN the unexpected chunk is denied or ignored
- AND it is not indexed as verified content.

### Requirement: Store capability roots are operational authority
r[molten.chunk_store.cap_std_operational_roots] Molten MUST require typed capability roots, or narrow filesystem ports backed by them, at every artifact, chunk, retention, local dataspace, and local exchange operation that opens, lists, reads, writes, renames, or removes local filesystem objects. A type alias or unused root constructor MUST NOT count as capability adoption while the effectful operation still accepts an ambient root path.

#### Scenario: In-root store operation uses its supplied authority
- GIVEN the outer shell has opened a declared local store root
- WHEN a store adapter reads or mutates a validated relative locator
- THEN the adapter MUST perform the operation through the supplied capability root
- AND it MUST NOT reopen the child through the ambient filesystem namespace.

#### Scenario: Alias-only integration is rejected
- GIVEN a module exposes a capability-root alias but its production operation joins an ambient path and calls `std::fs`
- WHEN capability-boundary validation runs
- THEN validation MUST fail and identify the operation as not yet converted.

### Requirement: Ambient store authority is confined to bootstrap shells
r[molten.chunk_store.cap_std_ambient_boundary] Molten MUST confine creation or opening of operator-selected ambient store roots to reviewed CLI, runtime, or adapter bootstrap shells. Reusable store logic MUST accept existing authority and MUST NOT call ambient root-open, canonicalization, or direct `std::fs` child operations.

#### Scenario: Explicit operator root is opened once
- GIVEN an operator supplies a local store root to a command
- WHEN the command enters the store subsystem
- THEN the outer shell MAY create and open that root with explicit ambient authority
- AND all descendant operations MUST receive capability-derived authority.

#### Scenario: Locator cannot trigger ambient reacquisition
- GIVEN a manifest or remote envelope contains a content ref, URL, ticket, absolute path, or parent traversal
- WHEN reusable store logic evaluates it
- THEN the value MUST NOT be passed to an ambient root-open or direct filesystem API
- AND invalid local locator use MUST deny before local bytes are accessed.

### Requirement: Capability-relative enumeration does not leak host paths
r[molten.chunk_store.cap_std_relative_enumeration] Molten MUST enumerate store directories through the capability root and return bounded, deterministically ordered logical names or typed relative entries. Store callers MUST reopen selected entries through the same root and MUST NOT use host paths obtained from ambient directory entries.

#### Scenario: Stable in-root listing passes
- GIVEN a capability-rooted directory contains valid store entries
- WHEN the adapter lists and consumes those entries
- THEN it MUST sort bounded logical names deterministically
- AND each consumed entry MUST be reopened relative to the original capability.

#### Scenario: Symlinked entry cannot become an ambient reopen
- GIVEN an enumerated entry is a symlink or is replaced before it is consumed
- WHEN the adapter attempts to read or remove it
- THEN capability-relative resolution MUST prevent escape from the declared root
- AND the adapter MUST NOT fall back to an entry host path.

### Requirement: Path-oriented backends receive capability-acquired handles
r[molten.chunk_store.cap_std_backend_handles] Molten MUST open fixed backend files, including Redb files, beneath the relevant capability root and pass an already-open file handle or capability-preserving backend into the storage engine whenever the engine supports that interface. Backend setup MUST NOT reconstruct an ambient path from the capability root.

#### Scenario: Redb index opens from an in-root file handle
- GIVEN the chunk index uses a fixed reviewed database leaf under a chunk root
- WHEN the index is created or reopened
- THEN Molten MUST acquire the file through the chunk capability
- AND pass that acquired handle to the Redb file or backend constructor.

#### Scenario: Backend leaf substitution is denied
- GIVEN an attacker substitutes a symlink or non-regular object for the backend leaf
- WHEN backend acquisition runs
- THEN the operation MUST deny before the backend can access an object outside the declared root.

### Requirement: Converted adapters have a scoped ambient-filesystem regression gate
r[molten.chunk_store.cap_std_regression_gate] Molten MUST maintain a syntax-aware blocking gate for converted store adapter scopes that rejects direct ambient filesystem calls and ambient root reacquisition. The gate MUST have positive fixtures for prohibited adapter calls and negative fixtures for reviewed outer-shell bootstrap and adversarial test setup.

#### Scenario: Ambient call in converted adapter fails
- GIVEN a converted store adapter adds a direct `std::fs` read, write, listing, or removal call
- WHEN the structural authority gate runs
- THEN the gate MUST fail with a scoped ambient-filesystem diagnostic.

#### Scenario: Explicit bootstrap remains permitted
- GIVEN a reviewed CLI shell opens the operator-selected top-level root and immediately delegates to a typed adapter
- WHEN the structural authority gate runs
- THEN the bootstrap fixture MUST pass without permitting the same call in store internals.

### Requirement: Capability-store conversion has positive and negative evidence
r[molten.chunk_store.cap_std_conversion_validation] Molten MUST verify capability-rooted artifact, chunk, retention, dataspace, exchange, enumeration, and backend-handle behavior with positive tests and negative tests for traversal, absolute paths, locator confusion, symlink escape, replacement races, wrong-root handles, non-regular entries, and missing authority.

#### Scenario: Complete conversion evidence passes
- GIVEN all targeted store effects consume operational capability roots
- WHEN focused tests and structural gates run
- THEN valid in-root workflows MUST pass
- AND every declared invalid or escaping workflow MUST deny before out-of-root access or mutation.

#### Scenario: Missing negative coverage blocks closeout
- GIVEN positive store workflows pass but one declared escape or authority-confusion class lacks executable coverage
- WHEN the change is evaluated for archive
- THEN closeout MUST remain blocked with the missing negative class identified.

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
