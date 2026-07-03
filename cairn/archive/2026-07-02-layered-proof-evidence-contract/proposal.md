## Why

Molten proof evidence spans pure cores, gate receipts, replay receipts, release receipts, and operator readbacks. Review is clearer when each layer has a contract and no layer is mistaken for authority outside its scope.

## What Changes

- Define a layered proof evidence contract.
- Separate pure-core, gate, replay, release, and operator-readback evidence roles.
- Require each layer to bind the previous layer by canonical ref and preserve evidence-only caveats.
- Add Hegel RS properties for cross-layer binding and boundary failures.

## Impact

- **Files**: evidence-gate specs, release/readback docs, tests.
- **Testing**: positive layered proof fixture, negative stale-layer fixture, Hegel RS generated layer graph tests.
