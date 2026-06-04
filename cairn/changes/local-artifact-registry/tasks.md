## Phase 1: Canonical artifact model

- [x] [serial] r[molten.local_artifacts.artifact_dto] Define canonical `artifact-v1` DTOs with kind, domain separator, inline/content payload, schema refs, dependency refs, effect manifest ref, policy refs, evidence refs, and checks.
- [x] [serial] r[molten.local_artifacts.domain_hashing] Compute artifact refs from domain-separated canonical artifact envelopes rather than mutable names, paths, or raw source text.
- [x] [serial] r[molten.local_artifacts.name_pointer_dto] Define canonical name/alias/tag/channel pointer DTOs that point to immutable artifact refs and carry previous refs plus receipt refs.
- [x] [parallel] r[molten.local_artifacts.no_ucm_compat] Document that Unison/UCM are non-normative prior art and not compatibility targets for Molten registry identity or CLI workflows.

## Phase 2: Local registry index

- [x] [serial] r[molten.local_artifacts.redb_index] Add a Redb-backed local registry for artifact envelopes, summaries, metadata pointers, dependency edges, reverse dependencies, schema/effect indexes, and receipt refs.
- [x] [serial] r[molten.local_artifacts.index_rebuild] Make the Redb index rebuildable from canonical artifact and pointer records without trusting stale derived tables.
- [x] [parallel] r[molten.local_artifacts.large_payload_refs] Support large artifact payloads through chunk/content refs and verify manifests before installation or viewing.
- [x] [parallel] r[molten.local_artifacts.semantic_indexes] Index artifacts by kind, schema refs, effect manifest refs, dependency refs, policy refs, and evidence refs for later catalog/MCP use.

## Phase 3: Dependency closure, impact, and receipts

- [x] [serial] r[molten.local_artifacts.dependency_closure] Compute deterministic dependency closures with ordered refs, missing-dependency diagnostics, and closure hashes.
- [x] [serial] r[molten.local_artifacts.reverse_impact] Compute impact sets from reverse-dependency edges and prove monotonicity as dependents are installed.
- [x] [serial] r[molten.local_artifacts.install_receipts] Emit and parse receipts for install pass/deny, dependency-closure admission, index mutation, and missing-dependency denial.
- [x] [serial] r[molten.local_artifacts.name_move_receipts] Emit and parse receipts for name/alias/tag/channel pointer changes that bind old/new refs without mutating artifact content.

## Phase 4: CLI and upgrade integration

- [x] [serial] r[molten.local_artifacts.cli_install_view] Add `molten test artifact install`, `list`, and `view` commands that always print full artifact refs.
- [x] [serial] r[molten.local_artifacts.cli_names_deps_impact] Add `name set/show`, `deps`, `closure`, and `impact` CLI commands over the local registry.
- [x] [serial] r[molten.local_artifacts.upgrade_impact_hook] Wire upgrade sessions to use registry-backed impact queries when a registry root is provided, with the current ledger scan as fallback.
- [x] [parallel] r[molten.local_artifacts.cleanup_safety_hook] Extend upgrade cleanup checks to consult registry pointers, reverse dependencies, receipts, and dependency closures before admitting deletion.

## Phase 5: Tests and properties

- [x] [serial] r[molten.local_artifacts.identity_tests] Add tests proving artifact refs are stable across names and change when payload, kind, domain, or dependencies change.
- [x] [serial] r[molten.local_artifacts.name_move_tests] Add tests proving name moves emit receipts and do not mutate artifact content or dependency edges.
- [x] [serial] r[molten.local_artifacts.closure_impact_tests] Add tests for closure computation, missing dependency denial, reverse-dependency impact, and upgrade integration.
- [x] [parallel] r[molten.local_artifacts.property_tests] Add Hegel properties for canonical hash determinism, closure idempotence, reverse-edge consistency, impact monotonicity, and no-name-identity.
