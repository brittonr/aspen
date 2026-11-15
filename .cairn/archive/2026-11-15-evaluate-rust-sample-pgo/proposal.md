## Why

Issue #32 asks whether sample PGO merits a maintained profile, not to assume a gain. Existing flux-profiler `.fxt` observations are not LLVM samples. An opt-in experiment must preserve the current compiler, Octet, authority, canonical replay, and profiler-artifact roles.

## What Changes

Add one bounded x86-64 Linux Nix experiment using a separately pinned Rust 1.100 nightly cohort (needed for the existing register_tool source contract), its exact LLVM converter source, fixed native harness training and held-out datasets, paired raw measurements, and predeclared decision rules. Compare canonical harness reports and canonical failure artifacts for successful, denied, bounded-exhaustion, and error paths; replay successful/denied reports through both binaries. No default profile or release claim is introduced.

## Impact

Slice-owned Nix modules, tools, configuration, consumer tests, and development profiling documentation. Root wiring is integration-owned. Captures and results remain local development observations. Cohort compatibility and performance remain unmeasured until real execution; unavailable prerequisites are blockers, not a stop decision.
