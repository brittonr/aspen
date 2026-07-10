# Design: executable-vm-fault-expansion

## Overview

Broaden VM fault evidence by separating executable faults, simulated diagnostics, and unavailable host support. Each fault case should bind preflight, injection, observation, cleanup, and decision evidence.

## Functional core and shell boundary

The pure core validates a VM fault support matrix and individual fault receipts from explicit refs. It checks capability support, expected outcome, decision, replay status, child refs, diagnostics, and caveats.

The VM shell owns network-control probes, fault injection commands, systemd restarts, permission fixtures, packet or link manipulation, cleanup, and artifact collection.

## Fault classes

Initial executable or probed classes:

- network delay, drop, partition, rejoin, and asymmetric latency when network-control support exists;
- restart during dispatch and duplicate operation after restart;
- stale ticket, wrong authority, conflicting operation id, and corrupted receipt denial before side effects;
- permission-denied state-root mutation denial;
- unavailable host support with diagnostic-only receipts.

## Cleanup and support policy

Fault support must be probed before pass claims. Cleanup evidence must show the VM returned to a known state or the fault remains non-pass diagnostic evidence.

## Boundaries

Executable VM faults are topology-scoped platform observations. Simulated fault cases stay diagnostic unless a separate gate explicitly accepts their scope.
