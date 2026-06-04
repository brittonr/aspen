## Context

Molten already has several immutable stores and evidence surfaces:

- `ledger` stores canonical artifacts by content hash but has no typed dependency index.
- `chunk_store` stores large byte payloads by chunk manifests and receipts.
- `typed_storage` stores schema-tagged durable values and migration recipes.
- `upgrades` creates structured plans and receipts, but currently computes impact sets with a conservative ledger text scan.

The next step is a local artifact registry: a canonical artifact model plus metadata indexes. This is an implementation slice of the broader `unison-artifact-registry` design, but the framing should remain Molten-native. Unison/UCM are prior art only; Molten does not adopt their formats, hash algorithm, CLI workflow, typechecker, or codebase model.

## Goals

- Define immutable artifact identity from a canonical Preserves artifact envelope and explicit domain separator.
- Keep names, aliases, tags, and channels as metadata pointers, never as artifact identity.
- Make dependencies explicit, indexed, and queryable in both directions.
- Compute deterministic dependency closures and impact sets from explicit edges.
- Preserve large payload support through content/chunk refs.
- Bind installation and metadata changes to policy/capability/evidence refs and receipts.
- Give upgrade sessions a registry-backed impact-analysis path.
- Provide a small local CLI for inspection and tests before MCP/catalog work.

## Non-Goals

- Do not replace Git, Cargo, Nix, or the source repository workflow.
- Do not adopt Unison, UCM, Unison hashes, Unison syntax, or Unison Share compatibility.
- Do not infer semantic dependencies solely by parsing arbitrary source text.
- Do not treat content addressing as trust or authorization.
- Do not execute installed artifacts merely because they are present in the registry.
- Do not introduce a distributed registry consensus protocol in this slice.
- Do not make cleanup eligible based only on absence of a human-readable name.

## Canonical artifact model

The local registry should introduce an artifact envelope shaped like:

```preserves
<artifact-v1 "molten.artifacts.artifact.v1"
  <kind "wasm" | "steel" | "nickel" | "schema" | "migration-recipe" | "doc" | "transcript" | "native-descriptor" | ...>
  <domain "molten.artifacts.domain.v1:<kind>">
  <payload <inline <canonical-bytes-ref> <length>> | <content-ref <manifest-ref> <length>>>
  <schemas [<schema-ref> ...]>
  <dependencies [<artifact-ref> ...]>
  <effects <none> | <some <effect-manifest-ref>>>
  <policy [<policy-ref> ...]>
  <evidence [<receipt-ref> ...]>
  <checks [<check "domain-separated-identity" "pass"> ...]>>
```

The artifact ref is the canonical hash of the artifact envelope, not the hash of a mutable name or filesystem path. The payload ref is part of the envelope. If the payload is large, a chunk-store manifest ref identifies the immutable bytes; the artifact envelope remains the identity root.

## Metadata pointers

Names and aliases are metadata records:

```preserves
<artifact-name-pointer-v1 "molten.artifacts.name-pointer.v1"
  <kind "name" | "alias" | "tag" | "channel">
  <name "project/main">
  <artifact <artifact-ref>>
  <previous <none> | <some <artifact-ref>>>
  <policy [<policy-ref> ...]>
  <receipt <receipt-ref>>
  <checks [<check "names-are-metadata" "pass"> ...]>>
```

Changing a pointer emits a receipt and updates metadata indexes. It must not rewrite an artifact, dependency edge, payload, or historical receipt. Tools may resolve names for convenience, but all trust-boundary operations must expand to artifact refs before use.

## Redb index

The first local registry can use Redb tables for:

- artifact ref -> canonical artifact envelope bytes,
- artifact ref -> summary fields: kind, payload ref, effect manifest ref, policy/evidence refs,
- name/alias/tag/channel -> artifact ref and pointer receipt ref,
- artifact ref -> dependency refs,
- dependency ref -> reverse-dependent artifact refs,
- artifact ref -> schema refs,
- schema ref -> artifact refs,
- effect manifest ref -> executable artifact refs,
- receipt refs by operation and subject.

The index is derived from canonical records and should be rebuildable. A later remote/synchronized registry may mirror these records through Iroh docs or a Raft-backed control-plane registry, but the local semantics should not depend on that transport.

## Dependency closure and impact

Dependency closure is a deterministic graph walk from one or more artifact refs over explicit dependency edges. The closure result should include:

- root refs,
- transitive dependency refs,
- missing dependency refs,
- closure hash over ordered refs and edge proofs,
- receipt refs proving the computation.

Impact analysis walks reverse-dependency edges. Upgrade sessions use impact analysis to identify affected artifacts, docs, transcripts, typed-storage migration recipes, schema dependents, and future protocol/session dependents. If a dependency ref is missing, installation or cutover must fail closed unless an explicit compatibility/placeholder policy admits it.

## Installation admission

Installing an artifact is a trust-boundary action. The first local implementation can require explicit capability/policy/evidence refs and emit local pass/deny receipts. Future work can replace or strengthen those refs with Nickel/Basalt/Trellis/Octet evidence without changing the artifact identity model.

Install receipts should bind:

- artifact ref and kind,
- payload ref and payload length,
- direct dependency refs,
- dependency closure hash or missing-dependency denial,
- installer/initiator ref,
- policy/capability/evidence refs,
- Redb index mutation receipt refs,
- checks for immutable content and names-as-metadata separation.

Name-move receipts should bind old/new refs and prove artifact content did not change.

## Query and CLI surface

The first CLI can live under `molten test artifact` and include:

- `install` from a Preserves payload or existing ledger artifact,
- `list` by kind/name/schema/effect,
- `view` by artifact ref,
- `name set/show`,
- `deps` and `closure`,
- `impact`,
- `receipt-list` and `receipt-show` if receipt storage is local.

CLI output should always show full artifact refs. Short ids, fuzzy name resolution, and richer catalog views belong to the later catalog/MCP slice.

## Upgrade integration

Upgrade sessions should prefer registry impact queries when given a registry root. The existing ledger scan remains a fallback for compatibility and for tests that have not installed artifacts into the registry. Name-move upgrade tasks should eventually delegate metadata pointer changes to the artifact registry instead of maintaining a separate upgrade-only pointer store.

Cleanup checks should consult registry name pointers, reverse dependencies, receipts, durable typed-storage refs, upgrade plans, and chunk pins. Absence from the artifact registry alone is not deletion proof.

## Tests and properties

Required tests:

- artifact refs are stable across names and unstable across payload/kind/dependency changes,
- name moves emit receipts and do not mutate artifact content,
- closure walks dependencies deterministically and reports missing dependencies,
- impact walks reverse dependencies and is monotonic as new dependents are installed,
- large payload artifact envelopes verify chunk/content refs before installation or view,
- Hegel properties for canonical hash determinism, closure idempotence, reverse-edge consistency, and no-name-identity.

## Open Questions

- Which artifact kinds need stronger normalized IR before payload hashing?
- Should native Rust implementation artifacts be registry descriptors only, or should build outputs/provenance become first-class payloads?
- How much of provenance policy should be enforced during install versus during execution/use?
- Should registry metadata receipts be linked into scoped evidence chains immediately or after catalog/MCP work?
- When Raft/control-plane storage lands, which metadata pointers require consensus and which remain local policy decisions?
