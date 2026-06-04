## Why

Local job DAG planning can now explain stage order, profile estimates, and fusion previews. The next distributed step is moving the admitted job artifact and stage dependency closure to another registry before any remote execution is allowed. Remote execution must remain fail-closed until artifact availability, hash verification, capability policy, and no-mobile-closure checks are explicit.

## What Changes

- Add transport-neutral job sync request, plan, and receipt records.
- Compute a job/stage dependency closure from the source artifact registry.
- Compare the closure against a target registry and report the missing set.
- Add loopback sync that installs missing artifacts into a target registry by canonical refs, verifying hashes before and after install.
- Keep sync separate from execution; synced artifacts do not grant authority to run.
- Add CLI commands: `molten test job sync-plan` and `molten test job sync-loopback`.

## Impact

This provides the artifact movement substrate needed for future peer-local job execution and Iroh transport. It exercises the same registry/chunk/content-ref invariants locally without adding network execution risk.
