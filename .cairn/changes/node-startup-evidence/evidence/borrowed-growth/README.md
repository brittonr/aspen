# Vector-growth repair: source gate 14

## Result

Task 10397 ran the unchanged canonical command on frozen product source `ab3efbe9788643caddd7f22bb0a13eca0d90426f`.
It reported **30 node-host errors, zero warnings**, exit 2 (Cargo 101).
Only the two `borrowed_argument_types` findings disappeared from run 13's 32 findings.
Other category counts stayed unchanged. This is not complete workspace coverage.
No startup authority was granted, and no normal-node VM launched.

The source archive matched run 13 byte-for-byte:
`5dc01614cbd7d3e3f4d7ad74b6673c62ede13bb7c04a75f45acc70f175fcbc5a`.
Only the lint library changed. The compiler, driver, CLI/runtime, hook, command, package selection, all-targets scope, configuration, and profile stayed unchanged.
Recorded before/after identities matched; tracked source stayed unchanged; no ICE markers appeared.
Invocation `1c3cb515ea444f968a468313583e48a7`; elapsed 55.983 seconds; peak memory 1.9G.

## Diagnosis and owner repair

Task 10391 confirmed that the vector fixture compiles and the suggested slice fixture fails with E0599: slices have no `push` method.
The old library failed the corrected growth control.
These are standalone fixtures, not exact full copies of both Molten functions; the entry fixture substitutes `u64` and a standalone capacity guard.

Octet implementation `9e0e455` in `src/structure/borrowed_argument_types.rs` recognizes resolved inherent `alloc::Vec::push` calls on the exact parameter binding.
It compares the resolved implementation's ADT with the candidate vector definition.
Method syntax, direct associated-function syntax, and inherent methods are covered.
Other locals, shadowed bindings, and other parameters do not gain the exemption.

This does not exempt all mutable vectors or perform general ownership analysis.
Aliases, reborrow expressions, closures, destructuring, arbitrary helper calls, and other container-specific operations retain existing behavior.
No Molten signature, capacity guard, effect order, public API, policy, suppression, or fallback changed.

## Build and controls

Task 10391 audited one lint-library derivation. Task 10392 built it in 24.537 seconds with the existing March-21 compiler and dependency artifacts.
Seven existing purity-classifier unit tests passed; these are regression tests, not new borrowed-interface unit tests.
Task 10394 passed four new probes, retaining seven valid candidate diagnostics, plus all 36 prior arithmetic/const/workspace/driver probes and controls.
Bare-todo, unknown-lint, missing-library, runtime-arithmetic, and invalid-constant denial remain covered.
No compiler, driver, CLI, runtime, Verus, Mantle, Darkhttpd, or Stage0 build ran.
Full Octet UI/workspace/flake/Clippy acceptance is not established.

- Library: `/nix/store/r5aw3lz2sa6h2cg9lphy881w0w1ra2id-octet-0.1.0/lib/liboctet.so`
- BLAKE3: `e465f12461fb498e21a531d2c48a36c6fb4338abd60429f6cfba7622ecc9d194`
- Derivation: `/nix/store/1j1vzz58d13h485jf9g6wlk6mgv2gnjy-octet-0.1.0.drv`
- Focused build entry: `verification/const-allocation/check.nix` in the new Octet worktree.

A private GC root retains the output; local content verification passed.
This does not establish signatures, reproducibility, or an approved runtime cohort.
Owner replay: Octet `verification/borrowed-growth/verify.sh OUT DRIVER LIBRARY COMPILER`, with absolute inputs and fresh output.
Private evidence: `~/.local/state/onix/molten-node-vm/borrowed-growth/` and `lifecycle-source-gate-14/`.
Earlier attempts remain intact.

## Remaining boundary

Thirty findings remain: 18 naming, five assertion-density, four exhaustive-enum, two compound-condition, and one file-length finding.
Their validity is not established by the count. Review the enum policy and its sealed-domain contract before changing exhaustive matches or adding markers.
Do not add catch-alls, arbitrary assertions, public renames, or suppressions merely to pass.
Startup guards, startup pin, and lifecycle tasks 4–5 remain unchanged.
No May-26 production build, source-to-runtime binding, normal-node VM, or replay is established.
