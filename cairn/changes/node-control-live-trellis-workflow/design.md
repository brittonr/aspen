# Design: Node Control Live Trellis Workflow Gate

## Protocol shape

The helper installs an in-memory finite protocol manifest with two roles, `sender` and `receiver`, and three ordered messages:

1. `sender -> receiver bundle-handoff workflow-bundle`
2. `sender -> receiver apply-evidence apply-receipt`
3. `receiver -> sender ack-evidence workflow-ack`

The lifecycle starts both roles with the bundle authority grant ref as protocol authority evidence and the bundle ref as protocol resource evidence. Message evidence binds the bundle gate receipt, apply receipt, reconcile receipt, and ack ref. The resulting protocol session gate receipt is the standard `protocol-session-gate-receipt-v1` used by the Trellis protocol runtime.

## Workflow evidence checks

Before emitting the final gate receipt, the helper parses and cross-checks the node-control evidence:

- the gate receipt must pass and bind the supplied bundle;
- the apply receipt must pass, bind the bundle, and name the supplied gate receipt;
- the reconcile receipt must pass, bind the apply receipt, and bind the bundle;
- the ack must parse, bind the apply/reconcile/bundle refs, preserve expected envelope/operation/request refs when supplied, and record a passing receiver decision.

Malformed or mismatched workflow evidence is appended as extra protocol gate diagnostics, producing a denying `protocol-session-gate-receipt-v1` while still preserving the Trellis replay diagnostics.

## CLI UX

`molten node live-workflow-bundle-protocol-gate` accepts the bundle path plus `--gate-receipt`, `--apply-receipt`, `--reconcile-receipt`, `--ack`, optional expected envelope/operation/request guards, and `--receipt-out`. It prints deterministic next-step guidance: passing gates can be archived; denying gates should be inspected with `molten node show <protocol-gate-receipt>`.

## Receipt semantics

The protocol gate receipt is review/replay evidence only. It confirms finite workflow order and receipt bindings, but it is not a grant, peer admission, policy/resource token, provenance record, sender import receipt, receiver ingress receipt, or transport proof.
