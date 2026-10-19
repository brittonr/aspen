# Preflight Prolly snapshot profiles

## Problem

`load_prolly_snapshot` currently trusts the caller's `max_graph_facts` while collecting blocks. A caller can provide a self-consistent profile with `max_graph_facts` above core's `MAX_GRAPH_FACTS`; the core rejects it only after the service has traversed and accumulated the snapshot. A missing block can also hide this profile error behind a port observation.

## Change

Run the existing core `validate_profile` over the actual supplied profile before allocating traversal state or calling the block-store port. Return its domain issues when invalid. Keep the existing per-graph bound and final snapshot validation unchanged.

## Verification

An exact-core-maximum profile still loads real encoded blocks through `LocalProllyBlockStore`. A self-consistent profile at the maximum plus one returns the core's `ProfileBoundInvalid("max-graph-facts")` even when the root block is absent.
