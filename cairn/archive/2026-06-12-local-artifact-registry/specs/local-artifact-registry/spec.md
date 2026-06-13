# local artifact registry Delta Spec

## ADDED Requirements

### Requirement: System MUST Define canonical `artifact-v1` DTOs with kind, domain separator, inline/content payload, schema refs, dependency refs, effect manifest ref, policy refs, evidence refs, and checks
r[molten.local_artifacts.artifact_dto] The system MUST Define canonical `artifact-v1` DTOs with kind, domain separator, inline/content payload, schema refs, dependency refs, effect manifest ref, policy refs, evidence refs, and checks.

### Requirement: System MUST Compute artifact refs from domain-separated canonical artifact envelopes rather than mutable names, paths, or raw source text
r[molten.local_artifacts.domain_hashing] The system MUST Compute artifact refs from domain-separated canonical artifact envelopes rather than mutable names, paths, or raw source text.

### Requirement: System MUST Define canonical name/alias/tag/channel pointer DTOs that point to immutable artifact refs and carry previous refs plus receipt refs
r[molten.local_artifacts.name_pointer_dto] The system MUST Define canonical name/alias/tag/channel pointer DTOs that point to immutable artifact refs and carry previous refs plus receipt refs.

### Requirement: System MUST Document that Unison/UCM are non-normative prior art and not compatibility targets for Molten registry identity or CLI workflows
r[molten.local_artifacts.no_ucm_compat] The system MUST Document that Unison/UCM are non-normative prior art and not compatibility targets for Molten registry identity or CLI workflows.

### Requirement: System MUST Add a Redb-backed local registry for artifact envelopes, summaries, metadata pointers, dependency edges, reverse dependencies, schema/effect indexes, and receipt refs
r[molten.local_artifacts.redb_index] The system MUST Add a Redb-backed local registry for artifact envelopes, summaries, metadata pointers, dependency edges, reverse dependencies, schema/effect indexes, and receipt refs.

### Requirement: System MUST Make the Redb index rebuildable from canonical artifact and pointer records without trusting stale derived tables
r[molten.local_artifacts.index_rebuild] The system MUST Make the Redb index rebuildable from canonical artifact and pointer records without trusting stale derived tables.

### Requirement: System MUST Support large artifact payloads through chunk/content refs and verify manifests before installation or viewing
r[molten.local_artifacts.large_payload_refs] The system MUST Support large artifact payloads through chunk/content refs and verify manifests before installation or viewing.

### Requirement: System MUST Index artifacts by kind, schema refs, effect manifest refs, dependency refs, policy refs, and evidence refs for later catalog/MCP use
r[molten.local_artifacts.semantic_indexes] The system MUST Index artifacts by kind, schema refs, effect manifest refs, dependency refs, policy refs, and evidence refs for later catalog/MCP use.

### Requirement: System MUST Compute deterministic dependency closures with ordered refs, missing-dependency diagnostics, and closure hashes
r[molten.local_artifacts.dependency_closure] The system MUST Compute deterministic dependency closures with ordered refs, missing-dependency diagnostics, and closure hashes.

### Requirement: System MUST Compute impact sets from reverse-dependency edges and prove monotonicity as dependents are installed
r[molten.local_artifacts.reverse_impact] The system MUST Compute impact sets from reverse-dependency edges and prove monotonicity as dependents are installed.

### Requirement: System MUST Emit and parse receipts for install pass/deny, dependency-closure admission, index mutation, and missing-dependency denial
r[molten.local_artifacts.install_receipts] The system MUST Emit and parse receipts for install pass/deny, dependency-closure admission, index mutation, and missing-dependency denial.

### Requirement: System MUST Emit and parse receipts for name/alias/tag/channel pointer changes that bind old/new refs without mutating artifact content
r[molten.local_artifacts.name_move_receipts] The system MUST Emit and parse receipts for name/alias/tag/channel pointer changes that bind old/new refs without mutating artifact content.

### Requirement: System MUST Add `molten test artifact install`, `list`, and `view` commands that always print full artifact refs
r[molten.local_artifacts.cli_install_view] The system MUST Add `molten test artifact install`, `list`, and `view` commands that always print full artifact refs.

### Requirement: System MUST Add `name set/show`, `deps`, `closure`, and `impact` CLI commands over the local registry
r[molten.local_artifacts.cli_names_deps_impact] The system MUST Add `name set/show`, `deps`, `closure`, and `impact` CLI commands over the local registry.

### Requirement: System MUST Wire upgrade sessions to use registry-backed impact queries when a registry root is provided, with the current ledger scan as fallback
r[molten.local_artifacts.upgrade_impact_hook] The system MUST Wire upgrade sessions to use registry-backed impact queries when a registry root is provided, with the current ledger scan as fallback.

### Requirement: System MUST Extend upgrade cleanup checks to consult registry pointers, reverse dependencies, receipts, and dependency closures before admitting deletion
r[molten.local_artifacts.cleanup_safety_hook] The system MUST Extend upgrade cleanup checks to consult registry pointers, reverse dependencies, receipts, and dependency closures before admitting deletion.

### Requirement: System MUST Add tests proving artifact refs are stable across names and change when payload, kind, domain, or dependencies change
r[molten.local_artifacts.identity_tests] The system MUST Add tests proving artifact refs are stable across names and change when payload, kind, domain, or dependencies change.

### Requirement: System MUST Add tests proving name moves emit receipts and do not mutate artifact content or dependency edges
r[molten.local_artifacts.name_move_tests] The system MUST Add tests proving name moves emit receipts and do not mutate artifact content or dependency edges.

### Requirement: System MUST Add tests for closure computation, missing dependency denial, reverse-dependency impact, and upgrade integration
r[molten.local_artifacts.closure_impact_tests] The system MUST Add tests for closure computation, missing dependency denial, reverse-dependency impact, and upgrade integration.

### Requirement: System MUST Add Hegel properties for canonical hash determinism, closure idempotence, reverse-edge consistency, impact monotonicity, and no-name-identity
r[molten.local_artifacts.property_tests] The system MUST Add Hegel properties for canonical hash determinism, closure idempotence, reverse-edge consistency, impact monotonicity, and no-name-identity.

