# Validation evidence

## Baseline

Baseline revision: `993e329f6e4237b7995aaa5ef1db11aed859b24f`.

Pinned Cairn revision: `3b4c280b893f2709aebea21fc51a4f9eeba3fe3b`.

Tracey returned nonzero for inherited repository-wide coverage debt.
Its exact scoped result was:

- dangling: `molten.testing.multinode.three_node_membership_negatives`
- missing: `molten.testing.multinode.three_node_vm_membership_negatives`

The focused negative test passed before the marker repair:

```text
three_node_quorum_gate_denies_missing_quorum_duplicate_and_log_only_claims ... ok
```

This baseline proves existing denial behavior only.
It does not prove executable VM coverage or repository-wide traceability closure.

## Final validation

The marker-only repair changed three comments and no executable statement.
It retargeted implementation and verification evidence to `molten.testing.multinode.three_node_vm_membership_negatives`.

The focused negative test passed after the repair.
Cargo formatting passed for the `molten` package.

Pinned Cairn validation returned valid state with no change or specification issues.
The lifecycle gate receipts were:

- proposal: `74ed30d6d91b19e5cc23d0e07603e8601ac3800d27fc8bb73212627b3d9b3789`
- design: `f5b7ac098c55c810245f6e015481b97f981cc3c6cb11ea8c02e5bfac721c2d85`
- tasks: `548cd89774356471de725eace9938ca5f3f87a520de01781737ae143d9764b25`

The delta restated the existing accepted requirement without changing it.
Sync preserved the accepted specification byte-for-byte at BLAKE3 `f7d3ab6f262d082485ef9e86f661fad93fd587f723d3ad26a25a73d6b212ef56`.
The sync mutation manifest was `f51478cdbdeabdd1422ae5f16aa31c6f05c8870d506047887d2268db6c42f52c`.
The sync receipt was `2df0953256ac20e3c0396581b5b83181022e9d4149a1d03cd337a0bea46a42ff`.

Post-repair Tracey reports an empty dangling set.
Neither the obsolete identity nor the accepted successor appears in the missing or dangling sets.
Tracey still returns nonzero for inherited missing coverage outside this change.
This evidence does not claim repository-wide coverage closure.

The final tasks gate passed with receipt `eb9c483f2a203085806bee451e70cbe79879fdeb6f3d24c2d4c806e6c326d86a`.
The change archived under `cairn/archive/1970-01-01-repair-three-node-membership-tracey/`.
The archive mutation manifest was `418eb3017d2b01ddc57c97e74a1e410a4605e460320bc1554b5c1a97abf446b1`.
The archive receipt was `bf6ffe0658aebe2fe6aefe7556d83c1700bd0e4d1928c1ac5a3882274ec7701d`.

Post-archive repository validation passed.
Post-archive Tracey kept an empty dangling set and the same bounded successor coverage.
Final Nix flake evaluation passed without building derivations.
