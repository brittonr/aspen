# Runtime service contract

Aspen's runtime service contract is the portable boundary between declarative service intent and host-specific execution backends. The contract lives in `aspen-runtime-core` so admission, simulation, and operator-facing receipts can use the same vocabulary without depending on a concrete host runtime.

## Canonical fields

`RuntimeServiceSpec` remains the declarative input. `canonical_runtime_service_contract` validates that spec and produces a `RuntimeServiceContract` containing:

- service identity and generation;
- host-loading reference for the chosen runtime host boundary;
- backend kind (`native-built-in`, `wasm`, `hyperlight`, `micro-vm`, `hermit-uhyve`, `external-process`, or `deploy-action`);
- artifact identities derived from the service artifact;
- declared capability handles and resource policy;
- receipt policy and declared route IDs;
- `validated` contract state.

A validated contract is not an activation claim. Route activation and health are separate observations.

## Route and health boundary

Routes progress through `declared`, `pending`, `active`, `withdrawn`, or `failed` states. `runtime_route_state_for_health` only marks a route `active` when the service instance is both `running` and `healthy`. Starting, unknown, or degraded health stays pending; stopped/unhealthy routes withdraw; failed instances fail the route.

This prevents docs, receipts, and tests from treating a declared route as a live serving endpoint.

## Receipt correlation

`runtime_receipt_correlation` links receipts back to the canonical service generation, optional instance ID, optional backend execution ID, artifact identities, and route IDs. Receipts may summarize execution state, but they should not invent host-specific fields that are absent from the runtime-core contract.
