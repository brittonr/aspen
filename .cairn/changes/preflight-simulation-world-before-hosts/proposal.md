# Preflight simulation world admission before reference host creation

## Why

`prepare_world_for_manifest` currently constructs and activates reference extension hosts for every caller-provided simulated node before calling `canonical_admit_simulated_world`. The existing core admission rejects more than 64 nodes or 4,096 workload steps; the reference runner reaches host construction/activation and its errors first for forged oversized manifests.

## What changes

- Move the exact existing `canonical_admit_simulated_world(manifest)?` to the start of `prepare_world_for_manifest`, retaining the admitted world in `PreparedReferenceWorld` instead of calling admission after host construction.
- Keep the same profile, tier, service selection, host loop and activation order for manifests admitted by the core. No new counts, duplicate validation, policy changes or new authority.
- Add failing-before/passing-after public `run_reference_world` regressions for 65 nodes and 4,097 steps with a recognizable host-selection error sentinel, and confirm admitted-manifest service errors retain priority. Exercise an actual simulation CLI and external public caller.

## Scope

A new clean canonical owner `change/preflight-simulation-world-before-hosts-20260929` changes only split simulation composition/test source, its Octet handoff and this change-local Cairn delta. Its disjoint source/test/docs/Cairn change is also applied atop 68 packages in the detached composition. No accepted spec, waiver, baseline, pin or full known-failing strict Octet retry changes.
