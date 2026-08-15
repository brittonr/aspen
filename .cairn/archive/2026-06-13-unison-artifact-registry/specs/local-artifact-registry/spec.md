# Local Artifact Registry Delta Spec

## Requirements

### Requirement: Artifact registry DTOs MUST bind immutable artifact identity and metadata
r[molten.artifacts.registry_model] Molten MUST define artifact DTOs with artifact id, kind, canonical payload/content ref, dependency refs, schema refs, effect manifest ref, policy refs, and evidence refs.
r[molten.local_artifacts.artifact_dto] Molten MUST define canonical `artifact-v1` DTOs with kind, domain separator, inline/content payload, schema refs, dependency refs, effect manifest ref, policy refs, evidence refs, and checks.

#### Scenario: Artifact DTO binds canonical metadata
- GIVEN an artifact payload with declared schemas, dependencies, effects, policies, and evidence
- WHEN the artifact DTO is emitted
- THEN it binds kind, domain separator, payload ref, schema refs, dependency refs, effect manifest ref, policy refs, evidence refs, and checks.

### Requirement: Artifact refs MUST use domain-separated canonical hashing
r[molten.artifacts.domain_hashing] Molten MUST add domain-separated Blake3 hashing over canonical artifact representations.
r[molten.local_artifacts.domain_hashing] Molten MUST compute artifact refs from domain-separated canonical artifact envelopes rather than mutable names, paths, or raw source text.

#### Scenario: Same bytes under different kinds have different artifact refs
- GIVEN identical payload bytes installed under two artifact kinds
- WHEN artifact refs are computed
- THEN the refs differ because the artifact kind domain separator is bound into canonical identity.

### Requirement: Names, aliases, tags, and channels MUST be mutable metadata over immutable artifact refs
r[molten.artifacts.names_metadata] Molten MUST model names, aliases, tags, and version channels as metadata assertions that point to immutable artifact ids.
r[molten.local_artifacts.name_pointer_dto] Molten MUST define canonical name/alias/tag/channel pointer DTOs that point to immutable artifact refs and carry previous refs plus receipt refs.

#### Scenario: Name move does not mutate artifact content
- GIVEN a name pointing at one artifact ref
- WHEN the name is moved to another artifact ref
- THEN Molten emits a pointer receipt while both artifact payloads remain unchanged and addressable.

### Requirement: Unison/UCM MUST remain non-normative prior art
r[molten.artifacts.no_unison_compat] Molten MUST document that Unison/UCM are non-normative prior art and are not compatibility targets.
r[molten.local_artifacts.no_ucm_compat] Molten MUST document that Unison/UCM are non-normative prior art and not compatibility targets for Molten registry identity or CLI workflows.

#### Scenario: Registry identity is not UCM compatibility
- GIVEN an artifact installed in Molten
- WHEN its registry identity is rendered or queried
- THEN Molten exposes Molten content refs and does not claim UCM, Unison syntax, or Unison hash compatibility.

### Requirement: Local registry indexes MUST be Redb-backed, semantic, and rebuildable
r[molten.artifacts.redb_index] Molten MUST add a Redb-backed local index for artifact metadata, name metadata, dependency edges, and reverse dependencies.
r[molten.local_artifacts.redb_index] Molten MUST add a Redb-backed local registry for artifact envelopes, summaries, metadata pointers, dependency edges, reverse dependencies, schema/effect indexes, and receipt refs.
r[molten.local_artifacts.index_rebuild] Molten MUST make the Redb index rebuildable from canonical artifact and pointer records without trusting stale derived tables.

#### Scenario: Rebuild preserves registry query results
- GIVEN a local registry with artifacts, pointers, receipts, and dependency edges
- WHEN derived Redb tables are rebuilt from canonical records
- THEN artifact, pointer, dependency, reverse dependency, schema, effect, policy, evidence, and receipt lookups are reconstructed.

### Requirement: Dependency closures and impact sets MUST derive from explicit artifact edges
r[molten.artifacts.dependency_closure] Molten MUST compute dependency closures and closure hashes from explicit artifact dependency edges.
r[molten.local_artifacts.dependency_closure] Molten MUST compute deterministic dependency closures with ordered refs, missing-dependency diagnostics, and closure hashes.
r[molten.local_artifacts.reverse_impact] Molten MUST compute impact sets from reverse-dependency edges and prove monotonicity as dependents are installed.

#### Scenario: Missing dependency denies closure admission
- GIVEN an artifact whose dependency edge names an unavailable artifact ref
- WHEN closure admission is computed
- THEN Molten reports the missing ref and emits denial evidence without pretending the closure is complete.

### Requirement: Large artifact payloads SHOULD use content/chunk refs suitable for remote blob transport
r[molten.artifacts.iroh_payloads] Molten MUST store large immutable artifact payloads through content references suitable for Iroh blobs.
r[molten.local_artifacts.large_payload_refs] Molten MUST support large artifact payloads through chunk/content refs and verify manifests before installation or viewing.

#### Scenario: Large payload installs through manifest ref
- GIVEN an artifact payload exceeding the inline bound
- WHEN the artifact is installed
- THEN Molten stores a content/chunk manifest ref, verifies the manifest on view, and keeps artifact identity bound to canonical payload metadata.

### Requirement: Semantic queries MUST cover kind, schema, effects, policies, dependencies, provenance, docs, and transcripts
r[molten.artifacts.semantic_queries] Molten MUST add query APIs by artifact kind, schema/type refs, effects, capabilities, dependencies, and evidence refs.
r[molten.artifacts.provenance_refs] Molten MUST attach Octet/Valence provenance and review evidence refs to artifact metadata where available.
r[molten.artifacts.docs_transcripts] Molten MUST represent docs and executable transcripts as registry artifacts referencing exact artifact ids and expected receipt/trace artifacts.
r[molten.local_artifacts.semantic_indexes] Molten MUST index artifacts by kind, schema refs, effect manifest refs, dependency refs, policy refs, and evidence refs for later catalog/MCP use.

#### Scenario: Semantic query finds exact evidence-bound artifacts
- GIVEN artifacts with schema, effect, dependency, policy, and evidence refs
- WHEN semantic registry queries run
- THEN matching artifacts are returned by exact refs and docs/transcripts can reference concrete artifact ids and expected evidence.

### Requirement: Artifact installation MUST be policy/capability admitted and receipted
r[molten.artifacts.installation_admission] Molten MUST gate artifact installation through Nickel/Basalt/Trellis policy before registry mutation.
r[molten.artifacts.installation_receipts] Molten MUST emit and validate Cairn receipts for artifact installation, name changes, and dependency closure admission.
r[molten.local_artifacts.install_receipts] Molten MUST emit and parse receipts for install pass/deny, dependency-closure admission, index mutation, and missing-dependency denial.
r[molten.local_artifacts.name_move_receipts] Molten MUST emit and parse receipts for name/alias/tag/channel pointer changes that bind old/new refs without mutating artifact content.

#### Scenario: Install receipt binds admission evidence
- GIVEN an artifact install request with policy, capability, dependency, schema, and evidence refs
- WHEN installation is admitted or denied
- THEN Molten emits a receipt binding the artifact ref, installer, admission refs, decision, diagnostics, and checks.

### Requirement: Registry CLI MUST expose install, view, names, dependency, closure, and impact operations
r[molten.local_artifacts.cli_install_view] Molten MUST add `molten test artifact install`, `list`, and `view` commands that always print full artifact refs.
r[molten.local_artifacts.cli_names_deps_impact] Molten MUST add `name set/show`, `deps`, `closure`, and `impact` CLI commands over the local registry.

#### Scenario: CLI prints full refs
- GIVEN a local artifact registry command
- WHEN install, list, view, name, deps, closure, or impact runs
- THEN command output includes full canonical artifact refs and receipt refs rather than relying on mutable names as identity.

### Requirement: Upgrade sessions and cleanup checks MUST consult registry impact evidence
r[molten.local_artifacts.upgrade_impact_hook] Molten MUST wire upgrade sessions to use registry-backed impact queries when a registry root is provided, with the current ledger scan as fallback.
r[molten.local_artifacts.cleanup_safety_hook] Molten MUST extend upgrade cleanup checks to consult registry pointers, reverse dependencies, receipts, and dependency closures before admitting deletion.

#### Scenario: Upgrade impact uses reverse dependencies
- GIVEN a registry root with reverse dependency edges for an affected artifact
- WHEN an upgrade session computes impact or cleanup safety
- THEN Molten includes registry-backed impacted refs and denies cleanup while pointers or dependents still require the artifact.

### Requirement: Artifact registry tests MUST cover identity, names, closures, and properties
r[molten.artifacts.identity_tests] Molten MUST add tests showing artifact ids are stable across names and unstable across canonical payload changes.
r[molten.artifacts.name_move_tests] Molten MUST add tests showing name/alias moves emit metadata receipts without changing artifact identity.
r[molten.artifacts.closure_tests] Molten MUST add tests for dependency closure computation, missing dependency detection, and reverse dependency impact queries.
r[molten.artifacts.property_tests] Molten MUST add Hegel property tests for canonical hash determinism and dependency graph invariants.
r[molten.local_artifacts.identity_tests] Molten MUST add tests proving artifact refs are stable across names and change when payload, kind, domain, or dependencies change.
r[molten.local_artifacts.name_move_tests] Molten MUST add tests proving name moves emit receipts and do not mutate artifact content or dependency edges.
r[molten.local_artifacts.closure_impact_tests] Molten MUST add tests for closure computation, missing dependency denial, reverse-dependency impact, and upgrade integration.
r[molten.local_artifacts.property_tests] Molten MUST add Hegel properties for canonical hash determinism, closure idempotence, reverse-edge consistency, impact monotonicity, and no-name-identity.

#### Scenario: Artifact identity ignores names
- GIVEN two names pointing at the same artifact payload and metadata
- WHEN registry identity and dependency properties are checked
- THEN artifact refs remain stable across names and change only when canonical payload, kind, domain, or dependencies change.
