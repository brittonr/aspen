## Context

The standalone source is `ssh://git@github.com/OnixResearch/artifact-auth.git` at revision `799459346d5416fbd7b9f55840a7371441b55afa`. Its reviewed Molten profile freezes source baseline `70590459a218dc8e66948ab6f305a7c54142b710`, shared fields, compatibility extensions, and retained authority. Current Molten code may have advanced, so adoption begins with a fresh consumer baseline.

## Goals and non-goals

Goals are exact-source admission, a pure translation/evaluation boundary, positive and negative compatibility evidence, and reversible cutover. This change does not move entropy, key generation/storage/signing, opaque handles, rotation writes, capability/federation authority, Preserves/Iroh transport, runtime policy, or evidence composition into standalone packages.

## Decisions

### Exact source and profile identity precede implementation

Cargo and Nix SHALL resolve one immutable revision. The consumer SHALL bind the typed Molten profile and checked JSON from that revision and reject floating refs, sibling paths, duplicate packages, source mismatch, or incompatible licensing.

### Currentness remains an explicit Molten observation

A pure adapter SHALL map signature domain/version, purpose, profile, payload ref, signer public ref, verifier context, key generation, and `Current`/`Overlap`/`Superseded`/`Revoked` facts. Verification overlap does not imply signing admission. Opaque handle, backend, entropy, and rotation-transition metadata remain Molten extensions.

### Cutover is evidence-gated and reversible

Legacy and standalone paths SHALL run over identical observations. Equal or intentionally mapped differences may be admitted only without claim widening. Unexplained differences, unknown currentness, unrelated failures, or weaker non-claims block cutover. Legacy authority remains until explicit admission and through a bounded rollback release.

## Risks

- Verification overlap could be promoted into signing permission.
- Public-key identity could be confused with capability or federation identity.
- Transport success could be mistaken for authentication or runtime authority.
- The frozen baseline could be stale relative to current code.

The change addresses these with explicit currentness, type/claim separation, product-owned composition, a fresh baseline, negative fixtures, and rollback.
