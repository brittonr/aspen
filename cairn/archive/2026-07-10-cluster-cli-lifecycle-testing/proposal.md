## Why

The cluster convenience wrapper has safety coverage for `cluster init`, but it does not yet have receipt-backed CLI coverage for the full `init → start → status → stop` lifecycle. That leaves regressions in manifest readback, node ordering, already-running detection, stopped-node status, and stop ordering visible only through manual use or the larger VM path.

## What Changes

- Add focused CLI lifecycle tests for two-node cluster init/start/status/stop using isolated state roots.
- Assert canonical node receipts and cluster manifest semantics instead of relying on rendered stdout.
- Add negative CLI fixtures for missing or malformed manifests, unsafe node names, lifecycle collisions, stale or mismatched node roots, and non-forced reinitialization.
- Keep cluster planning and manifest parsing as pure core; keep filesystem mutation and process execution in CLI/test shells.

## Impact

Developers get fast local feedback before NixOS VM checks. The tests cover cluster wrapper behavior only; they do not claim VM networking, live transport, production readiness, authority, policy, provenance, retention, or source-gate trust.
