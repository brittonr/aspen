## Context

CI TriggerService fires on gossip RefUpdate. Federation sync never emits gossip. Fix: emit from update_mirror_refs, auto-watch mirrors, gate behind opt-in flag.

## Goals / Non-Goals

**Goals:** Federation sync triggers CI on receiving cluster. Opt-in. VM tested.
**Non-Goals:** Periodic sync scheduling, cross-cluster CI delegation.

## Decisions

1. Emit gossip from `update_mirror_refs()` for each updated ref
2. `ForgeNode::announce_ref_update()` delegates to gossip (no-op if absent)
3. Periodic KV scan (`_fed:mirror:*`) auto-watches mirrors
4. `federation_ci_enabled` defaults false — operators opt in
