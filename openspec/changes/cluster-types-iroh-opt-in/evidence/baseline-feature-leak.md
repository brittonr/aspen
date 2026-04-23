Evidence-ID: cluster-types-iroh-opt-in.i1-baseline
Task-ID: I1
Artifact-Type: baseline-note
Covers: architecture.modularity.feature-bundles-are-explicit-and-bounded.cluster-types-iroh-opt-in-is-explicit

# Baseline feature leak

## Root workspace stanza
```
258:aspen-cluster-types = { path = "crates/aspen-cluster-types", default-features = false, features = ["iroh"] }
```

## Failure mode summary

The pre-fix problem was not the package-local bare tree for `cargo tree -p aspen-cluster-types`; that tree stayed alloc-safe. The leak came from the root workspace stanza enabling `features = ["iroh"]`, which let `workspace = true` consumers inherit `iroh` implicitly unless they overrode the dependency locally.
