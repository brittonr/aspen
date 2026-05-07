# Runtime service model types

- Change: `define-runtime-service-core`
- Task: portable runtime service model types
- Started: `2026-05-07T00:16:30Z`
- Completed: `2026-05-07T00:18:59Z`

## Implemented

`crates/aspen-runtime-core/src/lib.rs` now includes pure serializable model types for:

- `RuntimeApplicationRef`: minimal application/service/generation/route namespace/receipt-owner reference.
- `RuntimeServiceSpec`: service-owned host kind, artifact, desired replicas, singleton bit, placement hints, resources, capabilities, routes, health policy, restart policy, upgrade policy, and receipt policy.
- `RuntimeServiceInstance`: runtime instance identity, assigned node, lifecycle status, health state, lease epoch, heartbeat, active routes, and last receipt.
- `RuntimePlacementHints`, `RuntimeHealthPolicy`, `RuntimeRestartPolicy`, `RuntimeUpgradePolicy`, `RuntimeReceiptPolicy`, and `RuntimeHealthState`.

The new service spec projects into the existing `RuntimeUnitDeclaration` through `RuntimeServiceSpec::as_unit_declaration()` so admission and host/artifact boundary checks can remain shared.

## Boundary notes

- No process, filesystem, network, VM, WASM, Hyperlight, OCI, cryptographic verification, or runtime shell dependency was added.
- The crate remains a portable data-contract/pure-helper crate with `serde` only at runtime.
- Existing redacted receipt types and secret-shape rejection remain the receipt boundary for this slice.

## Verification

```console
$ rustfmt crates/aspen-runtime-core/src/lib.rs
$ CARGO_TARGET_DIR=target/agent cargo check -p aspen-runtime-core --all-targets
Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.31s
```
