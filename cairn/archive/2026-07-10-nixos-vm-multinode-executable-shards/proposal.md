## Why

The specs and docs describe shard receipts, but the platform check remains a large monolithic `nixos-vm-multinode` script. A single failure still forces reviewers to inspect node startup, live control, service/job coordination, restart recovery, fault validation, and final export together.

## What Changes

- Expose executable NixOS VM shard checks for smoke, live-control, service/job, restart, fault, and aggregate profiles.
- Have each shard emit or preserve a `nixos-vm-shard-run-v1` receipt with declared scenario, child refs, diagnostics, and caveats.
- Make the aggregate check consume child shard outputs and reject missing, denied, unavailable-as-pass, stale, or log-only child evidence.
- Keep the existing full evidence bundle available as an aggregate review surface.

## Impact

Developers can run the smallest VM layer relevant to a change. Release review still has a full aggregate, but failures localize to an executable shard instead of a monolithic driver script.
