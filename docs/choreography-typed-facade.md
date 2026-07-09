# ChoRus-inspired typed choreography facade

Molten treats ChoRus as prior art for Rust choreography ergonomics only. The authoritative protocol path remains Molten protocol manifests lowered to Trellis, admitted by projectability, projected into endpoint states, and replayed through canonical Preserves receipts.

## Boundary

The facade core added for `r[molten.choreography.chorus_design_reference]` and related requirements is pure and Sans-IO:

- `generate_protocol_facade_receipt` accepts a protocol install receipt, generator ref, and artifact ref, then emits a deterministic facade-generation receipt.
- `evaluate_protocol_facade_transition` evaluates send, receive, branch, and offer operations against projected endpoint state and returns descriptors, next state, receipt inputs, diagnostics, and non-effect transition evidence.
- `evaluate_protocol_facade_payload_access` models role-scoped located payload access and denies wrong role, wrong tag, or missing evidence before any actor delivery.
- `protocol_facade_dependency_boundary_diagnostics` is the pure dependency-drift check for forbidden ChoRus dependency markers.

The facade never grants authority, policy admission, resource rights, provenance approval, transport trust, ChoRus compatibility, or serde_json protocol identity. Runtime shells must still run the normal authority, policy, resource, replay, and receipt gates before side effects.

## Fixtures

Focused protocol-session tests cover:

- projectable manifests producing auditable facade generation receipts;
- non-projectable installs denying facade generation without endpoint refs;
- Sans-IO send parity with the projected runtime message and next-state refs;
- wrong label, missing authority evidence, and duplicate receive replay denial;
- role-scoped payload access pass/deny cases;
- ChoRus dependency drift rejection.

Run focused evidence with:

```sh
nix develop -c cargo test conversation::tests --lib
```
