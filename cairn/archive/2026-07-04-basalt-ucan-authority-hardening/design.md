# Design: Basalt/UCAN authority hardening

## Context

Aspen already emits Basalt contract-envelope receipts for policy, budget, and capability gates. The capability gate intentionally embeds `<ucan-proofset-v1 ... []>` and rejects non-empty proofsets until full Basalt/UCAN validation exists. Separately, `src/capability/tokens.rs` models local capability tokens and `src/runtime/spine/mod.rs` accepts a `BasaltRequest` whose `ucan_ref` only has to be a canonical content ref.

The next boundary should make the authority path explicit:

```text
compact UCAN token/proof refs
        │
        ▼
UCAN verification shell
        │  resolves bytes, keys, revocation, replay inputs
        ▼
verified grants + verification receipt refs
        │
        ▼
Basalt enforcement core
        │  policy + contract + resource + ability + verified grants
        ▼
Basalt enforcement receipt
        │
        ▼
Molten admission receipt / harness report / runtime trace
```

## Functional core and imperative shell

The pure core receives normalized values only:

- Basalt policy/export ref and parsed contract policy.
- Contract id, resource, ability, holder/session/context refs, and requested operation.
- Verified UCAN grants derived by the shell.
- Verification receipt refs for token structure, proof traversal, revocation, caveats, and replay.

The shell owns all I/O:

- Loading compact token/proof bytes from the artifact ledger, bundle, or runtime input.
- Resolving verification keys and proof store entries.
- Supplying logical verification time, revocation facts, caveat policies, and replay admission facts.
- Translating UCAN `VerifiedToken` / invocation results into stable grant DTOs.

The core denies if any required receipt ref or derived grant binding is absent, stale, malformed, or inconsistent with the request.

## Receipt shape

Add a canonical Basalt/UCAN authority receipt to the harness and runtime evidence graph. The receipt binds:

- schema id and version,
- decision and diagnostics,
- contract id,
- resource and ability,
- holder/session/context where applicable,
- Basalt policy/source/export refs,
- UCAN proofset ref,
- UCAN verification receipt refs,
- derived grant refs,
- Basalt enforcement result and reason,
- request ref.

The receipt is evidence for one request only. It does not grant future authority, replace subsystem-specific gates, or prove current authority during replay.

## Harness compatibility

The existing local static grant fixture remains available for deterministic harness tests, but it must be classified as a fixture-derived proof source. When a non-empty UCAN proofset is present, validation must require UCAN verification receipts and Basalt enforcement receipts. Empty proofsets remain valid only for suites that explicitly use local fixture authority.

## Runtime spine

`evaluate_basalt_request` should stop treating a bare `ucan_ref` as sufficient. It should become either:

- a pure function that evaluates a Basalt/UCAN authority input containing verified grants and receipt refs, or
- a thin wrapper around that pure function after the shell has resolved and verified the UCAN proof material.

The runtime trace must record deny receipts for missing verification, wrong resource, wrong ability, wrong holder/session, revoked proof, replay denial, or Basalt policy denial before any side effect commits.

## Testing

Coverage must include both positive and negative cases. Positive cases prove a valid token/proofset admits exactly the requested operation. Negative cases must cover invalid signature, unknown key, wrong holder/audience/session/context, expired or not-yet-valid token, missing proof, revoked issuer/proof/delegation, missing caveat evidence, stale replay, mismatched Basalt contract policy, local fixture fallback attempt, and tampered receipt refs.
