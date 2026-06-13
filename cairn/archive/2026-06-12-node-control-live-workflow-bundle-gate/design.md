# Design: Node Control Live Workflow Bundle Gate UX

## Gate command

`molten node live-workflow-bundle-gate` reads a live workflow bundle, re-runs the same offline verification as `live-workflow-bundle-verify`, and emits a `node-control-live-workflow-bundle-gate-receipt-v1` to stdout or `--receipt-out`.

The command accepts the same expected node/topic/endpoint/peer/operation/scope/freshness options as verify/import. `--verify-receipt` supplies a previous verify receipt; `--require-verify-receipt` fails closed when it is absent. When supplied, the gate parses the verify receipt and compares its canonical ref to the freshly recomputed verify receipt ref. CLI output reports the decision and a deterministic next step: import a passing bundle, re-run verification for stale/missing verify receipts, fix malformed bundles, or import missing ticket/grant evidence.

## Receipt semantics

Gate receipts bind the bundle ref, optional supplied verify receipt ref, recomputed verify receipt ref, expected bindings, member refs, diagnostics, and checks. A passing gate is an operator-review admission step only. It never materializes bundle members and never satisfies live-send or receiver-ingress authority/provenance gates.
