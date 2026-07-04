## Why

The current distributed evidence matrix names profile cost and scope, but topology coverage is still concentrated around the existing pairwise path. Release reviewers need explicit evidence that different node roles and membership shapes are covered, especially for control-plane quorum, restart and rejoin behavior, subscriber peers, and negative membership boundaries.

## What Changes

- Add an explicit multinode topology profile matrix derived from declarative fixtures.
- Cover pairwise transport, control-plane quorum, restart/rejoin, subscriber peer, and wrong-membership negative scenarios.
- Bind topology profile ids into distributed metadata and gate diagnostics.
- Add positive and negative tests proving topology roles, member bounds, and evidence scopes are not inferred from ad hoc test names.

## Impact

Multinode review becomes clearer: each scenario states which topology role shape it covers and which claims it cannot satisfy. This reduces ambiguity between simple transport smoke evidence and stronger control-plane or restart/rejoin evidence.