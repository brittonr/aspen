## Context

The canary exercised landed commit `19549c83f9cb046f0f6f4adebd4fa02f2b936deb` with artifact-auth commit `799459346d5416fbd7b9f55840a7371441b55afa`. It generated an OS-CSPRNG Ed25519 evidence key through Molten's capability-file adapter, wrote the operational receipt through the `Receipts` namespace, reopened node state and replayed in a fresh process, then rotated generation 1 to generation 2 and observed fail-closed denial of the old receipt.

## Decisions

### Decision: Archive a product-owned, self-contained public subset

**Choice:** Store the exact Molten harness and public run artifacts inside this Cairn package with a typed manifest and BLAKE3 inventory.

**Rationale:** A product-owned archive preserves the evidence needed to review Molten's claim boundary without duplicating Mantle or Valence payloads. Exact revision and cross-consumer review links remain in the manifest; those links are not a joint signature or cross-consumer attestation.

### Decision: Keep secret node state outside the archive

**Choice:** Exclude the capability-file private key and all mutable node state. Preserve only public keys, signatures, refs, receipts, summaries, and expected-denial logs.

**Rationale:** The later admission review needs public observations of generation/currentness behavior, not credential custody. A secret scan and bounded hash inventory guard the archive boundary.

### Decision: Treat the result as rollout evidence only

**Choice:** Keep `legacy_authoritative = true`, `standalone_authority_admitted = false`, and `rollback_available = true` in the manifest and accepted requirement.

**Rationale:** Local node-state restart and rotation demonstrate mechanism behavior but do not establish network revocation authority, membership, capability, federation, transport, storage, lifecycle, signing-policy, or release authority.

## Risks / Trade-offs

- The archive proves only the captured run and exact revisions; it does not recreate deleted private state.
- BLAKE3 inventory integrity does not prove semantic correctness, production rollout, or authority.
- Generation rotation evidence does not establish a globally distributed revocation service.
