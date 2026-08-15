# Change: basalt-ucan-authority-hardening

## Why

Molten currently records Basalt contract-envelope preflight evidence and preserves an explicit empty UCAN proofset seam, but runtime authority is still partly represented by local grant fixtures and a `ucan_ref` content-ref check. That keeps evidence fail-closed, but it does not yet exercise Basalt's enforcement API or UCAN's verified token/proof/revocation/replay surfaces as the authority path for side effects.

## What

- Promote Basalt enforcement receipts from marker/preflight evidence to required admission evidence for Basalt-governed runtime and harness requests.
- Accept non-empty UCAN proofsets only when UCAN verification receipts bind compact token refs, proof refs, revocation/replay facts, derived grants, and the exact request.
- Replace bare `ucan_ref` authority checks with a functional-core authority decision over verified grants and Basalt policy, leaving token/proof lookup in a thin shell.
- Clarify that local capability fixtures remain deterministic test inputs and evidence candidates, not a parallel production authority model.
- Add positive and negative fixtures for valid UCAN delegation, invalid signatures, wrong holder/audience, expired tokens, revoked proofs, stale replay, mismatched Basalt policy, and tampered receipt bindings.

## Impact

Basalt and UCAN become the normal authority boundary for capability-bearing operations instead of future seams. Reports, gate receipts, and runtime traces will bind the same request across UCAN verification, Basalt enforcement, local admission decisions, and side-effect evidence, while preserving fail-closed behavior for missing or stale proof material.
