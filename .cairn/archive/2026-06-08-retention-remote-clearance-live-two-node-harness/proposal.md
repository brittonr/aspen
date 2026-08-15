# retention-remote-clearance-live-two-node-harness

## Summary

Add a real local two-node happy-path test harness for the retention remote-clearance live multi-host UX.

## Motivation

The multi-host UX now has request-send, response-send, and import-workflow commands, but current coverage combines denied/offline sends with explicit or synthetic receive evidence. We need a local two-node live listener path that proves a bound live ticket, node-control live send, receive, ingress, response send, and final retention import workflow all compose without elevating live transport evidence into deletion authority.

## Scope

- Exercise request-send against a real local peer live listener and receive/ingress receipt.
- Exercise response-send against a real local requester live listener and receive/ingress receipt.
- Import the response with the real send/receive/ingress evidence and verify destructive admission still depends on imported peer clearance.

## Non-goals

- No new deletion authority, policy, or transport trust semantics.
- No new live network protocol; the harness remains local and deterministic enough for CI.
- No replacement for existing denied-transport diagnostics coverage.
