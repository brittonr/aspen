## Why

Molten will evolve protocols, schemas, policies, effect manifests, and artifact aliases while nodes and durable records are live. A naive upgrade model would mutate names or policies in place, causing broken sessions, ambiguous storage loads, and misleading cascades of errors.

Unison's structured refactoring sessions are useful prior art: immutable definitions remain valid while name mappings and dependencies are moved deliberately. Molten should adopt a receipt-backed upgrade-session model for runtime artifacts and metadata.

## What Changes

- Add structured upgrade sessions for changing names, aliases, protocols, schemas, policies, effect manifests, handlers, and migration plans.
- Keep old and new artifacts valid concurrently until explicit cutover and cleanup tasks are admitted.
- Represent upgrade work as a content-addressed plan with ordered tasks, compatibility checks, policy gates, and receipts.
- Track impacted artifacts, durable records, active sessions, capabilities, docs, and transcripts from registry dependency edges.
- Require explicit migration recipes for typed storage and compatibility bridges for protocols or handler profiles.
- Emit Cairn receipts for session creation, task admission, task completion, cutover, rollback, and cleanup.
- Make CLIs and tools show upgrade todo lists instead of turning the whole runtime into a broken state.

## Impact

This provides a safe evolution path for Molten's content-addressed runtime. The first implementation can model an upgrade plan that moves a name from one artifact id to another, computes impacted dependencies, requires receipts for cutover, and leaves both artifacts usable until cleanup.
