# Diagnostic CLI recovery and source gate 12

## Result

Task 10351 ran the canonical source check and returned exit 2 (Cargo 101).
It reported 33 node-host errors, zero warnings, and no autofixable findings.
Run 10 reported 36 errors. Only the three `missing_const_fn` findings disappeared.
All other category counts stayed unchanged. This is not complete workspace coverage.
No startup authority was granted, and no normal-node VM was launched.

The source remained `ab3efbe9788643caddd7f22bb0a13eca0d90426f`.
Its archive matched run 10 byte-for-byte, with BLAKE3
`5dc01614cbd7d3e3f4d7ad74b6673c62ede13bb7c04a75f45acc70f175fcbc5a`.
The command, metadata package selection, all-targets scope, configuration, and profile stayed unchanged.
Before/after recorded identities matched, tracked source stayed unchanged, and no ICE markers appeared.
Invocation: `d1a21525066d409785792ae54e3c22e7`; elapsed 146.367 seconds; peak memory 1.2G.

## Explicit replacement, not exact restoration

The missing CLI and runtime from run 11 were not recovered.
An audited replacement uses the existing March-21 compiler, repaired driver, and const-call library.
The recovered hook still matches its historical digest.
No startup pin, source guard, admission policy, or production source changed.

The standard runtime plan (10340) required 17 derivations, including acquisition and packaging of Rust 1.94 and production Verus.
That plan was rejected without realization.
The accepted plan (10342) contained only the CLI package, Cargo shim, and check-runtime join.
Build 10343 succeeded in 97.597 seconds; it built the package's three CLI executables.
It reused the existing compiler and dependency artifacts. The CLI build emitted 15 dead-code warnings; it was not a warning-free build or a CLI unit-test run.
No compiler, driver, lint library, Verus, Mantle, Darkhttpd, or Stage0 build ran.

Octet commit `21379f9` adds `verification/const-calls/check-runtime.nix`.
The CLI derivation came from Octet source `f42bc855d496db43cee49dbe7198f98f2bdd09ef`.
Its `cargo-octet/` subtree diff against the startup pin was empty; this does not prove binary equivalence.
The check-runtime contains Cargo/Rustup shims, the retained compiler, and the existing linker.
It does not supply Verus or establish formal-proof support. Standard Octet packaging was not changed.

## Tool identities

| Input | Store output / BLAKE3 |
|---|---|
| CLI | `/nix/store/vr0w4sf0l19rgjk3xgxk44zzjkd7sg4w-cargo-octet-0.1.0/bin/cargo-octet` |
| CLI digest | `d5d23275c81c9c5a157515fc3ca6f32e79b043d02ddf6185bc113c3b9f37c066` |
| Runtime | `/nix/store/4jfn77sinrbpda0gy4amrnjmyjaz3nl4-cargo-octet-check-runtime` |
| Cargo shim digest | `c702e0dc7dbae9ada794a5d7827c89a9bab2fbed82fa6ec7b7985baf4751a551` |
| Rustup shim digest | `bfef0666a32cec9822f8e1881bef7b57febfee348fc1db754258a9d4144dc0aa` |

CLI derivation: `/nix/store/ksla5mcjcwjjfgzpryadk5n40c2izi01-cargo-octet-0.1.0.drv`.
Runtime derivation: `/nix/store/pab4m28w5dq1h1zxgz7cck5vfj9fbb85-cargo-octet-check-runtime.drv`.
The compiler, driver, library, and hook identities remain documented in `../const-calls/`.
Task 10352 retained both outputs with GC roots and passed `nix store verify --no-trust`.
This establishes local content verification, not signatures or reproducibility.
Runtime shim digests were collected after the gate, not in its before/after identity pair.

## Controls and retained evidence

Task 10347 passed the existing driver-repair replay with the replacement CLI/runtime:
three clean workspace selections and six direct-driver controls.
These preserve bare-todo denial, unknown-lint denial, and missing-library failure.
Controls do not authorize production startup.

Private evidence is under `~/.local/state/onix/molten-node-vm/cli-recovery/` and `lifecycle-source-gate-12/`.
The partial run 11 remains untouched.
Initial Nix inspection failures (10336: string rather than derivation; 10337: absent `postBuild`) are retained.
Inspection then used string contexts and `buildCommand`; neither failure started a build.
Full Pueue JSON logs are retained separately from initial tail captures.
`private-digests.txt` binds the retained logs and source-gate observations.

Next: review the remaining 33 reported node-host findings without suppressions, arbitrary assertions, public API renames, or enum catch-alls.
Tasks 4–5 remain open. No May-26 runtime build, source-to-binary binding, approved cohort, normal-node VM, or replay is established.
