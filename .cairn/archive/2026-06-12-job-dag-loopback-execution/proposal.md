## Why

Molten can now sync a job DAG artifact closure to a target registry and perform target-side admission backed by concrete authority contexts. The next safe step is target execution in loopback only, gated by a passing admission receipt. Execution must remain fail-closed unless the target can verify the admitted closure, authority evidence, resource evidence, and target peer binding at execution time.

## What Changes

- Add canonical job execution request and receipt records.
- Add a loopback execution command that runs from the target registry only after verifying a passing admission receipt.
- Re-verify admitted target closure refs immediately before execution.
- Re-bind authority admission refs, sync evidence, resource refs, stage receipts, and output refs in the execution receipt.
- Deny missing, denied, stale, mismatched, or tampered admission evidence.
- Keep real network transport and remote worker lifecycle out of scope.

## Impact

This creates the first remote-shaped job execution path while preserving local loopback safety. It establishes the exact evidence contract future Iroh/peer execution must satisfy before a target executor starts.
