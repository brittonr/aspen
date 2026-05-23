## Phase 1: Route invariant and diagnostics

- [x] [serial] Inventory dogfood/client construction paths used for initial health, Forge push, CI polling, receipt publication, and diagnosis; identify where ticket direct addresses are lost or not registered.
- [x] [depends:inventory] Add a small route-summary/preflight helper for relay-disabled direct-only clients that reports peer id, direct-address count, and route-source availability without exposing tickets or secrets.
- [x] [depends:preflight] Add targeted direct-route-loss error/category plumbing for dogfood stages and receipt/diagnosis evidence.

## Phase 2: Retain ticket-derived routes

- [x] [depends:inventory] Preserve or register ticket-derived direct addresses for later dogfood RPC clients, especially the CI wait path reached after pipeline discovery.
- [x] [depends:route-retention] Ensure route retention does not broaden VM guest ticket bridge filtering or re-enable relay/discovery in direct-only proofs.

## Phase 3: Verification

- [x] [depends:preflight] Add positive and negative tests for direct-only route preflight, including the missing-direct-address fast failure case.
- [x] [depends:route-retention] Add a regression test proving the CI polling client has ticket-derived direct route state after initial health succeeds.
- [x] [depends:evidence] Add or update dogfood evidence/classifier tests for “pipeline discovered then host-client direct route loss.”
- [ ] [depends:tests] Run focused dogfood/client tests, formatting, OpenSpec validation, and a live `nix run .#dogfood-local-vmci -- full` retry or save a classified failure bundle.
