## Context

The live VM-CI retry after the socket-family advertising fix reached the next boundary: host health and cluster initialization succeeded, the cluster ticket was emitted, the Forge repo was available, source push completed, and dogfood found CI pipeline `4e7daaba-4860-44e2-9654-bcb3552f1cc4`. The run then spent minutes in CI wait while Iroh repeatedly logged `Address Lookup failed: No address lookup configured` for peer `44699d83e7`.

In relay-disabled dogfood, that warning is actionable: if the client has no address lookup source and no direct address registered, additional CI polling retries cannot succeed through ambient discovery. The failure should be classified as missing direct route state and bounded by a short preflight/retry window.

## Goals / Non-Goals

**Goals:**

- Preserve ticket-derived direct addresses across later dogfood client/RPC paths.
- Add direct-only preflight/fail-fast behavior before long health/push/CI waits.
- Emit a targeted diagnostic and receipt/log classification for route loss after pipeline discovery.
- Keep the VM-CI classifier boundaries distinct: node health, host-client route, worker readiness, post-registration workspace/blob/executor progress.

**Non-Goals:**

- Re-enable relays, mDNS, DNS, or pkarr for direct-only VM-CI acceptance.
- Hide or suppress Iroh route warnings without fixing the route source.
- Broaden VM guest ticket bridge filtering beyond the existing VM workspace scope.
- Change the public ticket wire format unless implementation proves it is necessary.

## Decisions

### 1. Direct-only route preflight before long waits

**Choice:** Add a dogfood/client route preflight used before long waits. In relay-disabled/direct-only mode it checks whether the selected ticket peer has at least one usable direct address registered or available to the endpoint. If not, it returns a targeted direct-route-loss error quickly.

**Rationale:** A generic CI timeout wastes minutes and points at the wrong subsystem. Missing route configuration is deterministic once relay/discovery are disabled.

**Alternative:** Keep waiting for the existing CI timeout. Rejected because it misclassifies a routing invariant violation as slow CI.

### 2. Ticket-derived route retention

**Choice:** Ensure the initial ticket `EndpointAddr` direct addresses are inserted into the route source/address book used by any later dogfood client created for CI polling, receipt publication, or diagnosis.

**Rationale:** The successful initial health path proves the ticket can carry a usable direct route. Later clients should not need external discovery to rediscover the same peer.

**Alternative:** Reuse one client instance for the whole run. This may be acceptable as an implementation detail, but the contract should require route availability rather than force a specific object lifetime.

### 3. Evidence classification owns the operator boundary

**Choice:** Add a dogfood evidence classification for host-client direct route loss, including the highest reached boundary and redacted route policy summary.

**Rationale:** VM-CI now has multiple adjacent failure classes. Receipts/diagnosis must make the next check obvious without log archaeology.

**Alternative:** Treat `No address lookup configured` as an unstructured log warning. Rejected because the message is the key proof that route state is missing under a direct-only policy.

## Risks / Trade-offs

**False positive route checks.** A route may be discovered just after preflight in non-direct-only modes. Mitigation: scope the fail-fast invariant to relay-disabled/direct-only dogfood paths and keep non-direct modes on existing retry behavior.

**Iroh API surface differences.** The available address-book APIs may differ from the desired abstraction. Mitigation: wrap route registration/preflight behind small Aspen helpers and test pure ticket/address summaries separately from Iroh integration.

**Secret leakage in diagnostics.** Route diagnostics touch tickets and endpoint identities. Mitigation: record only peer IDs, direct-address counts/summaries, route policy flags, and bounded messages; never print full tickets or keys.

## Validation Plan

- Focused unit tests for route preflight positive and negative cases.
- Regression test that CI wait client construction receives ticket-derived direct addresses after initial health succeeds.
- Dogfood evidence/classifier test for “pipeline discovered then direct route loss.”
- Formatting and focused package tests.
- Live `nix run .#dogfood-local-vmci -- full` retry or, if still failing, a saved classified receipt/log bundle naming the new boundary.
