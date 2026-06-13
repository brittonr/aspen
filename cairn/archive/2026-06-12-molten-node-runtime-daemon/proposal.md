## Why

Molten has many verified local slices, but no durable node process tying them together as an Aspen 2.0 runtime. Operators need one executable node with explicit state roots, canonical startup evidence, local control commands, adapter lifecycle, and pass/fail receipts before remote peers, jobs, services, and control-plane state can be exercised as a system.

## What Changes

- Add a `molten node` runtime mode with explicit config, data roots, node identity, adapter profiles, and policy refs.
- Expose a local Preserves control surface for status, install, run, gate, and shutdown requests.
- Emit canonical node startup, adapter-start, control-command, health, and shutdown receipts.
- Mount existing ledger, artifact registry, chunk store, typed storage, eval cache, remote dataspace, and job DAG slices under the node state root.
- Keep production side effects deny-by-default unless admitted by policy, capability, resource, and effect-handle evidence.

## Impact

This turns the current library/CLI prototype into an operator-visible node boundary. It does not make the system distributed by itself; it creates the stable process, config, state, control, and receipt surface needed for distributed slices to compose safely.
