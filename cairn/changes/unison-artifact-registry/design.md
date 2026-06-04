## Context

Molten's envelope spine already requires Preserves canonical representations for communication, storage, policy, and evidence boundaries. Runtime artifacts need the same treatment. A runtime that installs modules by filename or package name alone cannot give strong receipts, repeatable remote execution, durable typed storage, or reliable upgrade planning.

Unison demonstrates the leverage of content-addressed code: definitions are immutable by hash, names are pointers, dependency conflicts caused by competing names disappear, and tooling can operate over a structured code database. Molten should adapt those ideas to its artifact set: Wasm components, Steel scripts and predicates, Nickel policy/config artifacts, Preserves schemas, Trellis choreographies and projections, migration recipes, docs, transcripts, and native-adapter descriptors.

## Goals

- Give every installed Molten artifact a stable content id derived from canonical bytes.
- Make names and versions metadata, not identity.
- Preserve dependency graph information for closure sync, impact analysis, upgrade planning, and semantic search.
- Bind artifact installation to policy decisions, capabilities, provenance evidence, and Cairn receipts.
- Support semantic tools that can answer questions such as "which artifacts require BlobPut?" or "which protocols depend on schema X?".
- Let docs and transcripts reference exact artifact hashes so examples are reproducible.

## Non-Goals

- Do not adopt Unison, UCM, Unison syntax, or Unison Cloud compatibility.
- Do not replace Cargo, Nix, or Git for Molten's Rust implementation repository.
- Do not claim that content addressing proves artifact safety; policy and evidence gates still admit use.
- Do not hash raw source text when a canonical IR, component, Preserves schema, or normalized manifest is available.
- Do not allow artifact installation to bypass Basalt, Nickel, Steel, Trellis, Cairn, or Octet/Valence evidence boundaries.

## Artifact model

A registry artifact should contain or reference:

- `artifact_id`: Blake3 over the canonical artifact payload and declared artifact-kind domain separator.
- `kind`: Wasm component, Steel script, Steel predicate, Nickel contract, Preserves schema, Trellis choreography, projected endpoint, migration recipe, docs, transcript, native descriptor, or future extension.
- `canonical_payload`: canonical Preserves bytes or a content reference to large canonical bytes.
- `schema_refs`: schemas for artifact metadata, inputs, outputs, and persisted values.
- `dependency_refs`: direct artifact ids required to inspect, install, execute, migrate, or validate the artifact.
- `effect_manifest_ref`: declared runtime effects and capabilities required by the artifact, if executable.
- `policy_refs`: Nickel/Basalt/Steel/Trellis policy artifacts that admit installation or use.
- `evidence_refs`: Octet/Valence provenance, builder attestations, review records, and Cairn receipts.
- `created_by` / `installed_by`: identity and capability references for audit.

Artifact ids should include an explicit domain separator so a Wasm module, schema, and policy file with identical bytes cannot collide semantically.

## Names and metadata

Human-readable names are mutable registry assertions:

```text
name -> artifact_id
alias -> artifact_id
tag -> artifact_id
project/version channel -> artifact_id set
```

Changing a name emits metadata receipts but does not change the target artifact. Multiple names may point at the same artifact, and multiple artifacts may intentionally have similar names. Tools should surface ambiguity rather than treating it as a dependency conflict.

## Dependency graph

The registry maintains direct dependency edges and computed closures. Dependency edges should be canonical metadata, not inferred only by scanning opaque bytes. For example:

- Wasm component -> WIT/schema artifacts, host effect manifest, imported component artifacts.
- Steel predicate -> normalized source artifact, input/output schemas, reviewed callable capabilities.
- Choreography manifest -> role/label/payload registry, payload schemas, projected endpoint artifacts.
- Migration recipe -> source schema, target schema, executable artifact, policy artifact.
- Transcript -> executable artifact, handler profile, expected trace/receipt artifacts.

Reverse dependencies and impact queries are tooling features built on top of these edges.

## Registry adapters

The first local registry can use Redb for metadata indexes and Iroh blobs for large immutable artifact payloads. Iroh docs may replicate mutable metadata surfaces such as aliases, review states, or project channels.

Adapters are not semantics. Registry identity and dependency edges are defined by canonical artifact DTOs and hashes. Remote stores may cache, mirror, or garbage-collect content subject to policy, but cannot rewrite artifact identity.

## Policy and evidence

Artifact installation and use are trust-boundary actions. They must record:

- artifact id and kind,
- canonical hash algorithm and domain separator,
- dependency closure hash or install set,
- installer's capability and delegation chain,
- static Nickel contract decision,
- dynamic Steel predicate decision if applicable,
- Trellis bounded predicate results for dependency closure and integrity,
- Cairn installation receipt,
- Octet/Valence provenance references where available.

Execution adapters must use the artifact id, not a mutable name, in evidence and trace records.

## Semantic docs and transcripts

Documentation should be data in the registry. A doc artifact may reference artifact ids for code examples, schemas, policies, and expected receipts. Transcript artifacts should run examples under declared effect handlers and compare canonical trace/receipt output. This allows docs to become reproducible regression tests without depending on fragile textual paths.

## Open Questions

- Should artifact ids use raw Blake3, keyed Blake3, or a multihash-style envelope with algorithm agility?
- Which artifact kinds need a normalized IR before hashing, and which can hash canonical Preserves wrappers around raw bytes?
- Should mutable metadata live primarily in Redb, Iroh docs, or a Raft-backed control-plane registry once consensus lands?
- How should garbage collection prove that no durable storage record or receipt still references an artifact?
