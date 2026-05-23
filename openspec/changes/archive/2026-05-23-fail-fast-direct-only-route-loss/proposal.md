## Why

The current VM-CI dogfood retry proved that the previous host-health blocker is fixed: the run reaches node health, cluster initialization, ticket emission, Forge source push, and CI pipeline discovery. The remaining stall happens later while waiting for CI and repeatedly logs `Address Lookup failed: No address lookup configured` for the same Iroh peer. In relay-disabled/direct-only dogfood this is not a slow-CI condition; it means the client has no route source for the peer and should fail quickly with a route diagnostic instead of waiting for the generic CI timeout.

## What Changes

- Add a direct-only route preflight for dogfood/client paths that use a ticket without relays.
- Register or preserve ticket-derived direct addresses so later dogfood CI polling clients can reuse the same peer route that initial health used.
- Classify missing route/address-lookup failures separately from pipeline-pending, worker-readiness, and post-registration CI execution stalls.
- Emit bounded, redacted evidence that names the affected peer and missing route source without printing tickets or secrets.

## Capabilities

### Modified Capabilities

- `dogfood-evidence`: VM-CI dogfood evidence distinguishes direct-only route loss from slow CI or worker execution stalls.
- `dogfood-local-connectivity`: relay-disabled local clients fail fast when no direct route/address lookup source exists, and ticket direct addresses remain available across later RPC clients.

## Impact

- **Files**: likely `crates/aspen-dogfood/src/**`, Aspen client/ticket connection helpers, dogfood CI wait logic, and focused tests around ticket-derived direct addresses.
- **APIs**: no public protocol break expected; may add internal route-preflight/diagnostic helpers and structured dogfood failure categories.
- **Behavior**: relay-disabled/direct-only dogfood fails in roughly 10–30 seconds for missing route configuration instead of consuming the full CI wait timeout.
- **Testing**: unit tests for route preflight and ticket address retention, negative test for missing direct addrs with relay disabled, and a live `nix run .#dogfood-local-vmci -- full` retry or classified receipt/log evidence.
