## Why

The current multinode evidence surface is strong but too much scenario shape lives in imperative NixOS test-driver script fragments. That makes it hard to review which topology, profile, commands, receipts, unavailable policy, and variance rules a scenario actually claims before the VM starts.

## What Changes

- Add repository-owned typed Nickel fixtures for multinode scenarios.
- Validate fixtures before any VM, local multiprocess, or CI command surface treats them as evidence.
- Derive distributed metadata fields from reviewed fixtures instead of duplicating profile ids, command strings, artifact kinds, and variance declarations by hand.
- Add positive and negative fixture coverage for valid scenarios, missing required fields, mismatched profile commands, undeclared variance, unsupported pass claims, and stale receipt refs.

## Impact

Scenario review moves from reading one large imperative script to inspecting small typed fixtures and their canonical refs. Later VM and local harness work can consume the same fixture contract without changing the evidence boundary.