# Design: repair three-node membership traceability

## Context

An older archived change used `molten.testing.multinode.three_node_membership_negatives`.
Later accepted coverage uses `molten.testing.multinode.three_node_vm_membership_negatives` for the same bounded denial family.
The source still references the older identity.

## Options

1. Restore the old requirement to the accepted specification.
2. Remove the old markers without replacement.
3. Retarget the old markers to the accepted successor requirement.

## Decision

Use option 3.

Restoring the old requirement would create duplicate accepted semantics and preserve an obsolete identity.
Removing the markers would leave the accepted successor requirement without direct coverage.
Retargeting changes only traceability metadata and binds existing behavior to the current accepted requirement.

## Boundaries

- Do not change the accepted requirement text.
- Do not change the three-node gate implementation or fixture semantics.
- Do not claim repository-wide Tracey closure.
- Keep `molten.testing.three_node_quorum_vm.negatives` markers intact because they cover a separate executable-shard requirement.
