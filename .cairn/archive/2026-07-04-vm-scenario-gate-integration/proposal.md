## Why

Multinode scenario fixtures, topology profiles, reconciliation gates, and live transport gates already exist as reviewable models, but the VM check can still look like an imperative script that happens to produce the right files. Reviewers should see that a VM run consumed the declared scenario fixture and passed the same multinode gates that local and simulation layers use.

## What Changes

- Make VM shard execution bind the checked Nickel scenario fixture metadata before running the shard.
- Emit multinode scenario metadata, topology membership gate, reconciliation gate, and live transport VM gate receipts from VM evidence where applicable.
- Add negative VM gate fixtures for wrong scenario fixture, mismatched topology profile, missing receive receipt, divergent queue or ledger refs, and log-only reconciliation.
- Include these gate receipts in VM manifests and aggregate receipts without making them authority or deployment evidence.

## Impact

VM evidence becomes easier to audit because the declared scenario, the observed per-node evidence, and the pass gate all line up in canonical receipts. This reduces drift between docs, fixtures, Rust validation, and NixOS VM scripts.
