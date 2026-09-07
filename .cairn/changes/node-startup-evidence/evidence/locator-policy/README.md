# Shared locator recognition: source gate 17

## Implementation

Product `cadf6beb0220b078d8be329dfaf48eb798ec92fe` gives the duplicated lexical recognition rule one private pure owner: `crates/molten-node-host/src/locator.rs::is_remote`.
The existing node-state and local-store validators call it at their original validation stage.
The predicate checks contains("://") first, then the same ordered prefixes: iroh:, http:, https:, blake3:.
It allocates no collection and performs no I/O. This is the existing case-sensitive recognition rule, not a general URI parser or authority grant.

Empty-input, length, and platform-prefix checks retain their ordering. Boundary-specific errors and subsequent path admission remain at their existing call sites.
No public names, limits, recognized syntax, permissions, effects, assertions, lint levels, or compatibility documentation were changed to silence diagnostics.

## Verification

Task10369: all25 package tests and all-target Clippy -D warnings passed (12.653s,1.1G peak).
The three new tests cover 162 finite generated comparisons against the previous explicit guard, both public boundary error texts, accepted ordinary/unknown-scheme strings, and competing empty/platform/oversize rejection precedence.
This finite corpus is not an exhaustive proof over all strings. The refactor also preserves the Boolean operation order directly.
The retained focused tools/environment are those recorded in focused-tools.txt; no compiler/tool acquisition or bootstrap ran.

Task10376 ran the unchanged canonical command on a fresh frozen source with the same diagnostic cohort as run16.
Result: integration-failure, exit2/Cargo101, **22 node-host errors, zero warnings**,50.221s/1.6G.
Both compound conditions disappeared. The two validators also fell below the density threshold after their duplicated recognition logic moved to its own owner; no assertion or whitespace workaround was added.
Remaining: naming18, assertion-density3, file-length1.
The naming findings are unchanged. Root configuration/profile hashes remain unchanged.
Archive BLAKE3: `ebe7c9051a356b1947ad6023a85323cd6cd7dbdb143a6e90c7a0be426ef6d0a4`.

## Remaining review

The three density sites are list_entries, create_dir_components, and observe_file.
They handle fallible external facts and already return errors; arbitrary input assertions would weaken that contract.
Any further decomposition must separate genuine responsibilities while preserving bounds, error precedence, no-follow behavior, acquired-handle metadata, and operation order.
The 496-line local-store module also needs responsibility review, not formatting changes or path-based exclusion.

The naming-owner false-negative controls are retained in ../naming-review/. They require an owner repair before trusting future clean coverage.
No naming exception was adopted, no clean workspace/cohort is claimed, and startup remains denied.
No normal-node VM, runtime build/binding, Stage0, physical deployment, host listener, or historical-example substitution ran.
Private evidence is in `~/.local/state/onix/molten-node-vm/locator-review/` and `lifecycle-source-gate-17/`.
Closed-domain lifecycle closeout remains separate and open.
