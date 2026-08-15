# Design: nixos-vm-multinode-executable-shards

## Overview

Promote the pure shard receipt model into executable Nix check attributes. The Nix shell remains the imperative executor for VM startup and artifact copying; the Rust receipt builders remain the functional core for shard and aggregate decisions.

## Shard checks

Required executable shards:

- `nixos-vm-smoke`: service startup, health, control-loop, heartbeat, shutdown, and node evidence.
- `nixos-vm-live-control`: live ticket, peer admission, authority grant, send/receive, ingress, queue, dispatch, reconcile, ack, and protocol gate.
- `nixos-vm-service-job`: remote dataspace/service exchange, blob-ref job execution, and coordination apply.
- `nixos-vm-restart`: queued control request recovery across service restart.
- `nixos-vm-fault`: executable or unavailable VM fault descriptors, receipts, validation, and support matrix.
- `nixos-vm-multinode`: aggregate over required child shards.

## Evidence flow

Each shard must preserve its own `vm-evidence/` output and a shard receipt. The aggregate must read child shard receipts, topology/package refs, manifests, and validation receipts from the realized child outputs rather than reusing log strings.

## Boundaries

Shard evidence is platform-scoped. The aggregate is an index over child evidence and cannot promote unavailable, skipped, denied, stale, or log-only children to pass.
