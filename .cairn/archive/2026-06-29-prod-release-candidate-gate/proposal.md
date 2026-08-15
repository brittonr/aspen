## Why

Molten has enough implemented evidence rails to start internal dogfood and narrow pilot work, but a production decision should not be based on stale local validation notes. The active source-remediated-zero work is still changing CLI/source shape, and recent split slices intentionally did not rerun every full release dogfood check.

A release-candidate gate gives operators one explicit place to bind the current source gate, hermetic Rust/Nix checks, dogfood output, release evidence bundle verification, and pilot go/no-go decision before treating a build as production-ready.

## What Changes

- Add a production release-candidate gate that depends on current source-gate evidence rather than older Octet receipts.
- Require the full validation matrix after the active source-remediated-zero change is complete or explicitly deferred.
- Bind dogfood-local-node output, release evidence bundle verification, promotion summary, export verification, and Nix nextest evidence into one canonical production-readiness receipt.
- Require an explicit pilot-scope decision that names allowed workloads, denied workloads, rollback conditions, and evidence-only caveats.

## Impact

This change does not make dogfood receipts authority. It turns the existing release evidence machinery into an auditable production-readiness checkpoint so a future operator can distinguish “tests passed at some point” from “this exact candidate is ready for a constrained production pilot.”
