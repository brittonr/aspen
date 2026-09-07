# Constant-arithmetic repair: source gate 13

## Measured result

Task 10371 ran the unchanged canonical command on frozen product source `ab3efbe9788643caddd7f22bb0a13eca0d90426f`.
It reported **32 node-host errors, zero warnings**, exit 2 (Cargo 101).
Only `raw_arithmetic_overflow` disappeared from run 12's 33 findings; other category counts stayed unchanged.
This is not complete workspace coverage. Startup remains unauthorized; no normal-node VM launched.

The source archive matched run 12 byte-for-byte:
`5dc01614cbd7d3e3f4d7ad74b6673c62ede13bb7c04a75f45acc70f175fcbc5a`.
The compiler, driver, recovered CLI/runtime, hook, command, metadata selection, all-targets scope, configuration, and profile stayed unchanged.
Only the lint library changed. Before/after recorded identities matched; tracked source stayed unchanged; no ICE markers appeared.
Invocation `0b5ecc6fdf9c4934ae0bcbe464973a3e`; elapsed 51.354 seconds; peak memory 1.9G.

## Owner repair

Octet implementation `743f5be` in `src/safety/raw_arithmetic_overflow.rs` extends the existing literal exemption to literal-only addition, subtraction, and multiplication trees.
It bounds recursion to 32 levels per inspected operand and uses checked budget subtraction.
Unsupported and deeper expressions remain candidates. It does not exempt parameter-dependent arithmetic inside `const fn`.
The source remains readable: no replacement literals, saturation, suppressions, public renames, or policy changes in Molten.

Task 10364 audited one library derivation; task 10365 built it in 23.927 seconds using the existing March-21 compiler and dependency artifacts.
Seven existing purity-classifier unit tests passed. Task 10366 passed eight arithmetic probes and 19 prior const controls.
Task 10368 passed nine workspace/driver controls and confirmed the old library fails the corrected constant control.
Genuine runtime candidates, unknown-lint denial, missing-library denial, bare-todo denial, and compiler E0080 rejection remain covered.
No compiler, driver, CLI, runtime, Verus, Mantle, Darkhttpd, or Stage0 build ran.
Full Octet UI/workspace/flake/Clippy acceptance remains unestablished.

New library:

- `/nix/store/a6zhf51jr25sgy0w4drm3ixabjilcgn3-octet-0.1.0/lib/liboctet.so`
- BLAKE3 `b4d36f3e3d6cb8c30cbf92b596cf9944d92ec2435f0c72c13f7d9db9b7c3965a`
- Derivation `/nix/store/07c5gzsyairgk0mv61dxcpkjxr3p7rxr-octet-0.1.0.drv`
- Focused build entry `verification/const-allocation/check.nix` in the new Octet worktree.

Private GC root and successful local content verification are retained. This does not establish signatures or reproducibility.
Replay helpers and fixtures live in Octet `verification/constant-arithmetic/`.
Private logs and observations live under `~/.local/state/onix/molten-node-vm/constant-arithmetic/` and `lifecycle-source-gate-13/`.
Earlier attempts remain intact.

## Next review

The two `borrowed_argument_types` findings at `local_store/mod.rs:477,483` suggest slices for helpers that call `Vec::push`.
Their container capacity and growth are required. Review this at the Octet owner before changing signatures; do not apply the suggested slices mechanically.
This observation is not yet a retained compiler-pair probe or a repaired lint.
Other remaining findings also need semantic review.

The production source, startup Octet pin, admission guards, and lifecycle tasks 4–5 are unchanged.
No May-26 production build, source-to-runtime binding, approved complete cohort, normal-node VM, or replay is established.
