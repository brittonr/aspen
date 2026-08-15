# Design: nixos-vm-multinode-shards

## Overview

Introduce a VM shard plan as the functional core and keep NixOS test execution as the imperative shell. The plan names the shard id, scenario fixture ref, required node set, required child receipt kinds, expected output paths, unavailable behavior, and caveats. The shell maps a validated plan to one NixOS test derivation or a full aggregation derivation.

## Shard families

- `vm-smoke`: node init/start/status/control-loop/stop under systemd.
- `vm-live-control`: cross-node node-control live workflow receipts and live transport gate evidence.
- `vm-service-job`: remote dataspace delivery, chunk/job execution, and coordination apply evidence.
- `vm-restart`: queued request recovery and duplicate/idempotent restart evidence.
- `vm-fault`: executable or unavailable VM fault descriptors, receipts, support matrix, and validation.
- `vm-full`: aggregate manifest that binds child shard refs without replacing their evidence.

## Evidence model

Each shard writes a canonical `nixos-vm-shard-run-v1` receipt with the scenario fixture ref, topology ref, node evidence refs, child refs, diagnostic log refs, unavailable status, and caveats. The aggregate profile writes a `nixos-vm-multinode-aggregate-v1` receipt that binds the child shard refs and records missing or denied shards explicitly.

## Boundaries

Shard receipts are platform integration evidence only. Logs remain diagnostic-only. The aggregate receipt is a review index and cannot convert unavailable, skipped, denied, or log-only child evidence into pass evidence.
