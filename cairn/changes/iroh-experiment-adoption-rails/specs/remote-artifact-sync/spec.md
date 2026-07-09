# Remote Artifact Sync Delta: Deterministic Iroh Traversals

## ADDED Requirements

### Requirement: Deterministic traversal descriptors
r[molten.remote_sync.deterministic_traversal_descriptor] Molten MUST define deterministic traversal descriptors for remote artifact sync that bind traversal kind, root refs, optional visited refs, order, filters, inline policy, resource bounds, replay bounds, policy refs, and evidence refs.

#### Scenario: Artifact closure traversal is stable
- GIVEN the same artifact closure descriptor, local inventory summary, and traversal policy
- WHEN Molten computes a traversal plan twice
- THEN the selected refs, order, filters, and inline expectations are identical
- AND the descriptor can be hashed as canonical replay input.

#### Scenario: Unsupported traversal kind denies before network fetch
- GIVEN a remote sync request names an unknown traversal kind
- WHEN Molten validates the descriptor
- THEN it denies before opening Iroh streams, fetching blobs, or mutating local state.

#### Scenario: Inline policy is explicit
- GIVEN a traversal descriptor asks the sender to inline selected data
- WHEN Molten validates the descriptor
- THEN the inline policy names the allowed subject kinds or filters
- AND omitted or malformed inline policy falls back only to a reviewed safe default or denies according to policy.

### Requirement: Traversal missing-set planning is pure and receiver-driven
r[molten.remote_sync.traversal_missing_set] Molten MUST compute traversal missing sets and expected fetch refs from in-memory local registry and chunk summaries before fetching or installing remote content.

#### Scenario: Already-present ref is not fetched
- GIVEN a deterministic traversal contains refs already present and verified locally
- WHEN missing-set planning runs
- THEN already-present refs are recorded as present
- AND fetch effects are planned only for missing refs.

#### Scenario: Receiver refuses sender-pushed extra ref
- GIVEN a sender offers a ref not selected by the receiver's traversal descriptor or missing-set plan
- WHEN the receiver evaluates the response
- THEN Molten denies or ignores the extra ref before import
- AND emits diagnostics naming the unrequested ref.

#### Scenario: Non-deterministic traversal cannot gate replay
- GIVEN a traversal descriptor depends on unrecorded randomness, hash-map iteration order, live time, or ambient peer state
- WHEN Molten evaluates replay eligibility
- THEN deterministic pass evidence denies until the traversal input is made canonical and recorded.

### Requirement: External digest mappings require byte verification
r[molten.remote_sync.external_digest_mapping] Molten MUST validate any mapping from external non-BLAKE3 identifiers, such as IPFS CIDs, to Molten BLAKE3 content refs only after bytes verify against both the external digest algorithm and the Molten content ref.

#### Scenario: Valid CID mapping is admitted as compatibility metadata
- GIVEN bytes fetched for an external CID verify against the CID hash algorithm and the expected BLAKE3 content ref
- WHEN Molten records the mapping
- THEN the mapping is stored as compatibility metadata with evidence refs
- AND Molten-owned identity remains the canonical BLAKE3 ref.

#### Scenario: CID-to-BLAKE3 mismatch denies import
- GIVEN a sender claims that an external CID maps to a BLAKE3 ref but fetched bytes match only one side or neither side
- WHEN Molten validates the mapping
- THEN import denies before staging, install, or execution
- AND diagnostics report the failed digest boundary.

#### Scenario: Unsupported external hash algorithm fails closed
- GIVEN a traversal response references an external digest algorithm not supported by the reviewed compatibility policy
- WHEN Molten validates the response
- THEN the mapping and import deny before fetching unbounded additional data.

### Requirement: Traversal sync receipts bind request and response shape
r[molten.remote_sync.traversal_receipts] Molten MUST emit receipts for traversal descriptor validation, missing-set planning, response-frame verification, inline-data decisions, external-digest mapping, fetch completion, denial, and local admission.

#### Scenario: Successful traversal receipt binds replay inputs
- GIVEN a deterministic traversal sync completes
- WHEN Molten emits the sync receipt
- THEN the receipt binds descriptor ref, local inventory summary ref, missing-set ref, fetched refs, already-present refs, verification refs, admission refs, diagnostics, and replay eligibility.

#### Scenario: Malformed response has denial evidence
- GIVEN a sender response has a mismatched order, unexpected ref, invalid inline-data body, wrong digest, or unsupported mapping
- WHEN Molten evaluates the response
- THEN it emits a deny receipt
- AND no staged artifact is promoted from that response.
