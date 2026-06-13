## Context

The broader `unison-artifact-catalog-mcp` change describes catalog and MCP ideas inspired by Unison Share/UCM. This local slice narrows the first implementation to a local, read-only inspection core for Molten's current artifact registry and evidence ledger. Unison remains prior art only; Molten does not adopt Unison hash formats, UCM names, typechecker semantics, or Share protocol behavior.

Molten already has local subsystems that produce canonical Preserves artifacts and receipts:

- `artifacts`: immutable artifact envelopes, names as metadata, dependency/reverse indexes.
- `schema_identity`: schema identities, aliases, compatibility receipts, structural fingerprints.
- `typed_storage`: typed refs, migration recipes/receipts.
- `eval_cache`: cache keys/values/receipts and invalidation state.
- `transcripts`: transcript artifacts and run receipts.
- `rewrites`: query/match/diff/plan/apply receipts.
- `upgrades`: upgrade plans, task receipts, name pointers.
- `chunk_store`: manifests, chunk lineage, chunk-store receipts.
- `ledger`: local immutable content store and artifact-kind classification.

## Goals

- Provide a shared local query core for artifact and evidence inspection.
- Return deterministic canonical catalog records and receipts for every query.
- Preserve identity discipline: names, aliases, tags, channels, paths, and short ids are display handles only, never artifact identity.
- Apply visibility filtering and redaction hooks before summaries, rendered views, or search results leave the catalog boundary.
- Support short-id resolution only when unambiguous and always expand to full refs before downstream operations.
- Give CLI and future MCP tools the same read-only query substrate.

## Non-Goals

- No mutating catalog operations in this slice.
- No networked catalog, remote index, or MCP server yet.
- No search over plaintext secrets or confidential payloads without redaction policy.
- No full-text indexing daemon; bounded deterministic scans are enough for the first local implementation.
- No name-based identity or path/mtime-based cache keys.
- No global consistency claims across peers.

## Catalog records

Introduce canonical Preserves records:

```preserves
<catalog-summary-v1 "molten.catalog.summary.v1"
  <artifact <ref> <kind> <payload-ref>>
  <names [<name-pointer-ref> ...]>
  <schemas [<schema-ref> ...]>
  <dependencies [<dependency-ref> ...]>
  <dependents [<dependent-ref> ...]>
  <effects <none>|<some <effect-manifest-ref>>>
  <policy [<policy-ref> ...]>
  <evidence [<evidence-ref> ...]>
  <classifications ["artifact" "schema" ...]>
  <visibility <decision> <policy-ref-or-none>>
  <checks [<check "full-ref-identity" "pass"> ...]>>
```

```preserves
<catalog-query-v1 "molten.catalog.query.v1"
  <operation "list"|"view"|"search"|"deps"|"dependents"|"short-id">
  <scope [<root-ref> ...] <include-dependencies?> <include-dependents?>>
  <filters [<filter <kind> <value>> ...]>
  <visibility [<policy-ref> ...] [<capability-ref> ...] [<hidden-ref> ...]>
  <render <mode> <include-payload?> <redaction-profile-ref-or-none>>
  <checks [<check "no-name-identity" "pass"> ...]>>
```

```preserves
<catalog-result-v1 "molten.catalog.result.v1"
  <query <query-ref>>
  <decision "pass"|"deny">
  <results [<catalog-summary-v1 ...> | <catalog-view-v1 ...> | <short-id-resolution-v1 ...> ...]>
  <diagnostics ["..."]>
  <checks [<check "visibility-filtered" "pass"> ...]>>
```

```preserves
<catalog-receipt-v1 "molten.catalog.receipt.v1"
  <operation ...>
  <decision "pass"|"deny">
  <query <query-ref>>
  <result <result-ref-or-none>>
  <refs [<artifact-ref> <receipt-ref> ...]>
  <diagnostics ["..."]>
  <checks [<check "canonical-result-ref" "pass"> ...]>>
```

## Query core

The local query core should derive all data from existing canonical stores:

- Artifact summaries from the artifact registry index and payload refs.
- Dependency/dependent sets from registry dependency/reverse indexes.
- Receipt and auxiliary artifact classifications from the ledger classifier.
- Transcript, rewrite, upgrade, cache, schema, typed-storage, chunk, and harness views by parsing known canonical records where available.

Search filters are conjunctive by default and bounded. Supported initial filters:

- full ref or short-id candidate,
- artifact kind,
- ledger artifact kind,
- schema ref,
- structural fingerprint ref,
- effect manifest ref,
- policy/capability/evidence ref,
- dependency/dependent ref,
- receipt operation/decision where the receipt shape is known,
- bounded text term over redacted/rendered public text.

## Visibility and redaction

The catalog must filter before rendering. Inputs include policy refs, capability refs, and hidden refs. The first local slice may use explicit hidden refs and public redaction hooks, but records must reserve fields for policy/capability decisions so later Basalt/Nickel/secret-redaction integration does not change the catalog contract.

Confidential/secret/protected markers should render as redaction markers rather than raw payload text, reusing the redaction profile/ref style established by repro-bundle redaction and future secrets work.

## Short ids

Short ids are UI/CLI conveniences:

- Prefix matches shorter than a configured minimum are denied.
- Ambiguous prefixes return a denial result listing candidate refs only if visible.
- Unambiguous prefixes return a `short-id-resolution-v1` record binding short id, full ref, candidate count, and checks.
- Downstream operations receive only the full ref.

## CLI

Add local inspection commands:

- `molten test catalog list --registry <path> [--ledger <path>] [--kind <kind>]`
- `molten test catalog view <ref-or-short> --registry <path> [--payload] [--redacted]`
- `molten test catalog search --registry <path> [filters...]`
- `molten test catalog deps <ref-or-short> --registry <path>`
- `molten test catalog dependents <ref-or-short> --registry <path>`
- `molten test catalog short-id <prefix> --registry <path> [--ledger <path>]`

Each command emits or can write a canonical catalog receipt.

## Future MCP boundary

Read-only MCP tools should wrap this query core without adding new authority. Mutating MCP tools, if added later, must produce policy-gated plans only, not ambient side effects.
