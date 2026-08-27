## Why

World commits identify code artifacts, but Molten does not have a reviewed contract for page-aligned immutable executable extents, direct mapping, structural sharing, or W^X-safe admission.

Ordinary content availability does not prove that bytes are correctly aligned, sealed, target-compatible, executable, or authorized for mapping.

## What Changes

- Consume a pinned `executable-extent` mechanism after its independent project and conformance suite exist.
- Keep semantic code identity, build artifact identity, extent-manifest identity, and live mapping identity as separate domains.
- Add an optional world code-root profile that binds an exact Mantle extent bundle and runtime cohort.
- Remeasure every extent and validate layout, alignment, target, format, ABI, source artifact, permissions, and complete closure before mapping.
- Use the shared pure mapping-admission core and keep file, mapping, protection, and execution effects in Molten adapters.
- Enforce write-or-execute transitions and reject simultaneous writable and executable mappings.
- Recheck current artifact, runtime, resource, policy, and execution authority before mapping or activation.
- Preserve ordinary artifact profiles as an explicit weaker alternative without claiming extent conformance.

## Dependencies

- `introduce-world-commit-core`.
- The workspace `establish-executable-extent-project` coordination change and private Radicle source `rad://z37R1bP1kHcELs89RNbQRaqbCVKxB` at archived revision `025d9636f0161777710dac37b3c210ca0ad9483f`.
- Mantle `publish-executable-extent-bundles` at private Radicle revision `2c636b1b25353a1b0befa5af48dc68615cd686dd`.
- Cap Root, Durable File Publication, Artifact Auth, Artifact Binding, and the reviewed executable-extent contract.
- Existing Molten artifact registry and runtime admission mechanisms.

## Non-Goals

- Build correctness, compiler correctness, semantic code equivalence, signing authority, or provenance ownership.
- A general JIT, dynamic linker, loader, package store, or sandbox.
- Writable-and-executable memory, self-modifying code, or silent target fallback.
- Retention, deletion, release, deployment, or execution authority from extent validity.

## Impact

- **Core**: extent-root profile, manifest admission facts, mapping plans, compatibility decisions, and diagnostics.
- **Shell**: bundle loading, remeasurement, capability-relative materialization, sealed mapping, protection changes, unmapping, and read-back.
- **Schemas**: world extent references, admission receipts, mapping observations, and runtime activation receipts.
- **Testing**: valid shared mappings plus negative misalignment, overlap, truncation, digest mismatch, target mismatch, writable-executable request, stale bundle, path substitution, missing authority, partial closure, ordinary-artifact fallback, and mapping-overclaim cases.
