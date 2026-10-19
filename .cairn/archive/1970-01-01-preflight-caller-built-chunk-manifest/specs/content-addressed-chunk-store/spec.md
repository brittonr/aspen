# Caller-built chunk manifest intake bound

## ADDED Requirements

### Requirement: Public chunk sync and readback enforce parser-owned manifest chunk count

r[molten.chunk_store.caller_built_manifest_count_preflight] The public traversal sync planner and remote-range readback MUST deny a caller-built `ChunkManifest` containing more than 65,536 chunks at their shared identity-admission boundary, before allocating count-dependent collections or performing port I/O. They MUST retain existing validation precedence and successful behavior for admitted manifests, including exactly 65,536 chunks. This requirement reuses the parser-owned `MAX_CHUNK_STORE_CHUNKS`; it does not grant authority to a remote byte-source hint or replace hash/range validation.

#### Scenario: Exact bound remains eligible

- GIVEN a manifest with exactly 65,536 syntactically valid chunk refs and a valid first-chunk byte slice
- WHEN the caller requests a range covering the first chunk
- THEN readback proceeds through existing chunk-hash and range checks and exposes the verified bytes.

#### Scenario: Extra chunk is rejected before range collection

- GIVEN a caller-built manifest with 65,537 chunk entries
- WHEN the caller requests remote-range readback
- THEN admission rejects the count before collecting required chunk refs or copying caller bytes.

#### Scenario: Traversal rejects oversized manifest before other input checks

- GIVEN a caller-built manifest with 65,537 chunks and an unsupported traversal strategy
- WHEN the caller plans traversal sync
- THEN the manifest count denial wins before strategy validation or plan construction
- AND an admitted manifest with that unsupported strategy retains the existing strategy denial.

### Requirement: Remote range readback bounds and borrows supplied chunks

r[molten.chunk_store.remote_supplied_chunk_preflight] The public `verify_remote_range_readback` helper MUST reject more than `MAX_CHUNK_STORE_CHUNKS` supplied `(ref, bytes)` tuples before allocating a lookup or processing byte buffers, even when the tuples have duplicate or irrelevant refs. It MUST retain exact-bound eligibility, last-duplicate-wins lookup, and required-range error precedence for admitted inputs. The lookup MUST borrow refs and bytes rather than copying supplied chunk buffers. Verified output and required-chunk byte processing MUST remain inside the existing `MAX_CHUNK_STORE_OBJECT_BYTES` limit; no new limit is introduced.

#### Scenario: Exact supplied tuple count is admitted

- GIVEN 65,536 supplied tuples including many irrelevant duplicates and one four-byte required chunk
- WHEN the public helper reads that chunk's range
- THEN it verifies and exposes exactly the same four bytes without cloning irrelevant buffers.

#### Scenario: Excess supplied tuple is denied before lookup

- GIVEN 65,537 supplied tuples
- WHEN the public helper reads a valid range
- THEN it reports the existing source-owned chunk-count limit rather than collecting and cloning any tuples.

#### Scenario: Duplicate precedence and admitted range errors are retained

- GIVEN two supplied buffers for one required ref
- WHEN their order is reversed
- THEN only the last buffer determines verified pass or denied bytes
- AND an admitted tuple count with an overflowing requested range still rejects the range before supplied lookup.
