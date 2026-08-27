# Executable-extent consumer

Molten can consume one closed Mantle executable-extent profile. The consumer is independent from the Mantle producer.

The optional Cargo feature is `executable-extents`. Default builds keep this pilot outside release roots until a release decision selects the stronger profile.

## Published inputs

The consumer pins these private Radicle sources:

- executable-extent: `rad://z37R1bP1kHcELs89RNbQRaqbCVKxB`
- Mantle producer: `rad://z3DJe8tEdQuXpzTkfqCYQq6ZUqqkb`

The archived executable-extent revision is `025d9636f0161777710dac37b3c210ca0ad9483f`.

The Mantle producer revision is `2c636b1b25353a1b0befa5af48dc68615cd686dd`.

## Closed profile

The first consumer accepts only these facts:

- format: `mantle-flat-page-v1`
- target: `x86_64-linux-gnu`
- byte order: little endian
- page size: 4096 bytes
- source size: 4096 bytes
- extent count: one
- permission: executable and read-only
- relocation model: none

The consumer rejects unknown fields, unknown profiles, incomplete publication, changed producer links, changed conformance receipts, and changed identity bytes.

## Admission sequence

The shell performs these steps in order:

1. Read the manifest and receipt from an authorized directory capability.
2. Recompute the Mantle bundle and producer receipt identities.
3. Check the Mantle producer links and publication observations.
4. Run the shared positive and hostile layout and W^X corpora.
5. Prepare detached Artifact Auth and pinned Artifact Binding review values.
6. Read every declared extent through the same directory capability.
7. Remeasure each extent and compare its exact length and BLAKE3 identity.
8. Run pure Molten compatibility and W^X admission.
9. Ask the application-owned port for current artifact, runtime, resource, policy, and execution facts.
10. Keep the extent inert if any current fact denies activation.
11. Materialize, seal, map, and protect the extent through executable-extent-linux.
12. Explicitly unmap it and emit a detached Molten consumer receipt.

The consumer does not execute the mapped bytes.

The dedicated Nix Octet gate compiles only this profile and its real pure-core source. It uses the full catalog and denies all warnings. The final gate reported zero findings, warnings, and errors.

## World code-root profile

`ExtentCodeRootProfile` keeps these identities distinct:

- semantic code
- built artifact bytes
- executable extent manifest
- Mantle producer receipt
- Molten runtime cohort
- Molten policy

A world commit can bind this profile through its existing artifact root. The profile does not move mutable mappings, current authority, or effect outcomes into immutable world content.

## Authority boundary

A valid extent does not grant execution authority. `CurrentAdmissionPort` supplies current Molten-owned observations. An unavailable observation fails closed.

A denied extent produces an `inert` receipt with no mapping observations. The ordinary artifact path remains a separate, weaker profile. It cannot satisfy a policy that requires executable extents.

## Receipt boundary

The consumer receipt records exact producer, source, layout, runtime, policy, mapping, teardown, and source-pin facts. It is detached from the immutable world commit.

The receipt does not prove:

- compiler correctness
- executable code semantics
- sandbox or host integrity
- external authority freshness
- storage or retention authority
- release eligibility

## Rollback

Rollback removes the executable-extent code-root profile and keeps the ordinary artifact path. It does not reinterpret extent identities as ordinary artifact proof. It also does not preserve executable-extent release claims.
