## Why

Aspen #63 asks whether a bounded trace from the live Raft implementation can be checked against an independent specification. The archived `RaftReadIndex` model describes the read-index rule, but combines request, delivery and response steps that the implementation can observe separately. Existing receipt references and local durable replay do not supply a complete model trace.

## What Changes

- Select one explicit, bounded, fixed-membership three-replica loopback scenario on the existing live Raft service. Retain real applied transition facts, including an earlier write acknowledgement, a leader-change read retry, a new commit and a completed quorum read.
- Adapt the archived model into an Aspen-owned post-election read-index subset with a state assignment for every modeled variable and no invented intermediate state.
- Retain the pins and actual checker evidence, including negative and unsupported dispositions. Keep the interchange mapper and checker driver with Trace Lab #6 and the reusable checker package with Octet #34.

## Impact

- **Files**: the existing live Raft fixture and ports, `verification/tla`, and the formal-model scope and change evidence.
- **Testing**: opt-in local suite through the existing service executor and transport, independently checked model sample, guarded negative control, incomplete-mapping denial and Cairn review.

## Out of Scope

No default behavior, production mutant, protocol, runtime dependency, new transport, or change to ConsensusFastPath or Quint. No release, deployment, exhaustive proof, implementation-authority, or ChaosControl replay claim.
