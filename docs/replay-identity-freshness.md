# Replay identity freshness

Replay freshness compares the expected subject identity for a release, dogfood output, subsystem, or coverage row against the identity carried by replay evidence. The pure validator binds artifact, dependency closure, initial state, schema, policy, capability, revocation, handler profile, seed-or-effect-log, runtime, tool, and replay profile fields before accepting replay evidence as fresh.

Freshness receipts are canonical evidence. They emit the expected and supplied identity refs, a pass/deny decision, stale-component diagnostics, and evidence-only checks. A pass means the replay evidence applies to the named deterministic identity; it does not grant authority, policy, provenance, source-gate, release, resource, transport, retention, or execution trust.

Replay rollups and indexes preserve member identity refs so catalog search and release readback can find stale or matching replay evidence without inspecting payload contents. Catalog classifications include `replay-identity-ref:*`, `replay-rollup-identity:*`, and `replay-index-identity:*` terms, plus freshness receipt terms from the pure catalog helper.

## Node-control run context

Node-control duplicate provenance and current lifecycle state are separate replay
inputs. Preserve the original `node-control-run-v1` context (state-root, startup and
previous-run refs) and the exact `node-control-run-shutdown-v1` completion linkage.
Successive runs can share startup bytes without sharing a run ref. A passing control
receipt from an earlier run remains historical evidence; replay must not reinterpret
it as completion of a later run or execute an old shutdown again to obtain that state.

Legacy control/shutdown receipts remain readable without gaining missing run facts.
Current stopped readback follows the completion's immutable control reference even
after a later denied stop writes the mutable last-control file. Missing, unreadable
or conflicting observations reject or remain unresolved without changing lifecycle
state. Service denial for pending bounded work is independent of an already observed
current shutdown. These local bindings do not prove process termination, crash
durability, remote shutdown authority or whole-system liveness.

See the [node artifact reference](handbook/node-workflows/node-artifact-and-state-root-reference.md)
and [canonical run records](../src/node/parts/runtime/p009/body.rs).
