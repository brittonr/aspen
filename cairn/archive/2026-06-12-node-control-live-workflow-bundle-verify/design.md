# Design: Node Control Live Workflow Bundle Verify UX

## Verify command

`molten node live-workflow-bundle-verify` reads a bundle file and emits a canonical `node-control-live-workflow-bundle-verify-receipt-v1` to stdout or `--receipt-out`. The command accepts the same expected node/topic/endpoint/peer/operation/scope/freshness arguments as bundle import, but it never writes bundle members to a state root.

## Receipt shape

The verify receipt binds the bundle ref, optional parsed ticket/admission/grant refs, supporting receipt refs, expected binding arguments, diagnostics, and checks. Malformed bundles still get a deny receipt with parse diagnostics when the outer value can be hashed.

## Validation boundary

Verification shares the same pure binding diagnostics as import. Passing verification is operational evidence only; live-send preflight and receiver ingress still require original admitted peer admission, authority grant, policy/resource evidence, delivery-idempotency evidence, and provenance where applicable.
