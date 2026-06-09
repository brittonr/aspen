# Change: nix-dogfood-release-evidence-binding

## Why

The Nix dogfood check already preserves the dogfood report, release-gate receipt, human summary, and nextest marker, but release review still has to infer that those files belong to the realized Nix output path. Molten needs canonical evidence and verification receipts that bind the Nix check output path to the release-gate ref and nextest dependency marker without granting any subsystem authority.

## What

- Add canonical Nix dogfood release evidence that records the Nix output path, dogfood report ref, release-gate ref, summary ref, nextest marker ref, and preserved file refs.
- Add a verification receipt that recomputes those refs from an output path and fails closed on mismatch.
- Have the `dogfood-local-node` Nix check emit both evidence and verification receipts.
- Document that these receipts remain release-review evidence only.

## Impact

Release reviewers can verify that a specific Nix check output contains the expected passing dogfood report and release gate after nextest. The evidence does not replace authority, policy, provenance, resource, transport, source-gate, retention, or destructive-operation gates.
