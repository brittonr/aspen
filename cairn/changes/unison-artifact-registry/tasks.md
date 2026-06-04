## Phase 1: Artifact identity and canonicalization

- [ ] [serial] r[molten.artifacts.registry_model] Define artifact DTOs with artifact id, kind, canonical payload/content ref, dependency refs, schema refs, effect manifest ref, policy refs, and evidence refs.
- [ ] [serial] r[molten.artifacts.domain_hashing] Add domain-separated Blake3 hashing over canonical artifact representations.
- [ ] [serial] r[molten.artifacts.names_metadata] Model names, aliases, tags, and version channels as metadata assertions that point to immutable artifact ids.
- [ ] [parallel] r[molten.artifacts.no_unison_compat] Document that Unison/UCM are non-normative prior art and are not compatibility targets.

## Phase 2: Registry storage and dependency graph

- [ ] [serial] r[molten.artifacts.redb_index] Add a Redb-backed local index for artifact metadata, name metadata, dependency edges, and reverse dependencies.
- [ ] [serial] r[molten.artifacts.dependency_closure] Compute dependency closures and closure hashes from explicit artifact dependency edges.
- [ ] [parallel] r[molten.artifacts.iroh_payloads] Store large immutable artifact payloads through content references suitable for Iroh blobs.
- [ ] [parallel] r[molten.artifacts.semantic_queries] Add query APIs by artifact kind, schema/type refs, effects, capabilities, dependencies, and evidence refs.

## Phase 3: Policy, evidence, and docs

- [ ] [serial] r[molten.artifacts.installation_admission] Gate artifact installation through Nickel/Basalt/Trellis policy before registry mutation.
- [ ] [serial] r[molten.artifacts.installation_receipts] Emit and validate Cairn receipts for artifact installation, name changes, and dependency closure admission.
- [ ] [parallel] r[molten.artifacts.provenance_refs] Attach Octet/Valence provenance and review evidence refs to artifact metadata where available.
- [ ] [parallel] r[molten.artifacts.docs_transcripts] Represent docs and executable transcripts as registry artifacts referencing exact artifact ids and expected receipt/trace artifacts.

## Phase 4: Tests

- [ ] [serial] r[molten.artifacts.identity_tests] Add tests showing artifact ids are stable across names and unstable across canonical payload changes.
- [ ] [serial] r[molten.artifacts.name_move_tests] Add tests showing name/alias moves emit metadata receipts without changing artifact identity.
- [ ] [parallel] r[molten.artifacts.closure_tests] Add tests for dependency closure computation, missing dependency detection, and reverse dependency impact queries.
- [ ] [parallel] r[molten.artifacts.property_tests] Add Hegel property tests for canonical hash determinism and dependency graph invariants.
