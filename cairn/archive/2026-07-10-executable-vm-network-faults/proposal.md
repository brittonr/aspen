## Why

The current VM fault surface records network partition support as unavailable when the VM image lacks network-control tooling. That is safe, but it means delay, drop, partition, rejoin, and asymmetric latency faults are not executable evidence on hosts that can support them.

## What Changes

- Add a network-control capability probe that identifies supported backends and target links before fault execution.
- Execute bounded delay, drop, partition, rejoin, and asymmetric-latency faults when the backend is available.
- Bind preflight, injection, child workflow, cleanup, post-fault, diagnostics, and caveats into canonical fault receipts.
- Preserve unavailable and denied outcomes as non-pass evidence when host support is missing or cleanup fails.

## Impact

Cluster testing gains real network-fault evidence on capable hosts while still failing closed in unsupported environments. The evidence remains VM-topology scoped and does not claim WAN reliability or production network safety.
