## Why

Node control requests can be submitted to a durable inbox and dispatched one at a time, but a node-like operator loop should consume that inbox under the same canonical Preserves evidence boundary. Without a bounded loop, the node daemon still behaves like a collection of manual one-shot commands rather than a minimal Aspen 2.0 control-plane process.

## What Changes

- Add a bounded `run-loop` command that drains queued control requests in deterministic path order.
- Emit canonical heartbeat and loop receipts tied to the active startup lock.
- Preserve side-effect safety by reusing the existing dispatch gate for `status`, `shutdown`, `install`, `run`, and `gate`.
- Make duplicate request refs idempotent by returning the prior control receipt instead of re-running operation side effects.
- Stop the loop after a passing shutdown request removes the active lock.

## Impact

The node control surface now supports a durable, bounded local loop while remaining file-backed and deterministic. This still does not add a network socket, background daemon supervisor, or distributed admission protocol; it only provides the local control-loop semantics needed before wiring a long-running node process around it.
