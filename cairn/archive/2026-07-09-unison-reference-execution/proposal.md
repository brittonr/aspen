## Why

Unison's remote computation model is useful because a caller can send a reference to code and let the receiver fetch missing dependencies. Molten should adapt that shape for admitted artifacts, not mobile heap closures: remote execution requests carry artifact refs, closure descriptors, arguments, effect manifests, capabilities, and policy/evidence refs.

The receiver remains autonomous. It computes missing dependencies, verifies bytes, applies local admission gates, binds handlers, and only then runs the artifact.

## What Changes

- Add remote execution envelopes whose executable identity is an exact artifact ref plus dependency closure descriptor.
- Make closure fetching receiver-driven and policy-gated before install or execution.
- Bind presented capabilities, effect manifests, handler profiles, resource policy, provenance, source-gate evidence, and reply routes into execution admission receipts.
- Deny arbitrary closure serialization, sender-pushed extras, incomplete closures, and handler-profile mismatches before execution.

## Impact

- **Files**: remote artifact sync, job worker execution, node-control live workflow, effect handling, provenance gates, fixtures.
- **Testing**: positive fixtures for verified closure execution; negative fixtures for missing deps, wrong hashes, sender-pushed extras, mobile closure payloads, missing capabilities, and local policy denial.
- **Security**: transport delivery and sender identity do not grant execution. Receiver policy and evidence gates decide.