## Why

`ControlPlaneConsensusEngine` currently combines descriptor/readback, proposal, read, snapshot, and recovery operations, and its proposal method mutates runtime state. The mixed interface makes it harder to test the consensus transition core without standing up mutable runtime state and obscures which engines support which capabilities.

## What Changes

- Split consensus engine behavior into smaller capability traits for descriptor/readback, proposal transitions, reads, snapshots, and recovery.
- Extract a pure proposal transition core that accepts immutable input state and command evidence and returns a deterministic transition result.
- Keep runtime mutation in a thin imperative shell that persists logs, snapshots, and receipts only after pure transition admission.
- Add conformance tests proving unsupported capabilities deny explicitly rather than becoming no-op defaults.

## Impact

- **Files**: consensus control-plane engine traits, Raft control registry proposal flow, tests, and receipt builders where needed.
- **Testing**: pure transition tests, capability-denial tests, and existing consensus integration/property tests continue to pass.
