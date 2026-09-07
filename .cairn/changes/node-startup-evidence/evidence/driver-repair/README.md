# Repaired driver: dependency gate advances to source findings

The user approved a driver-only repair/build. That work is complete within its focused scope.
The production compiler, Mantle, Darkhttpd, and Stage0 were not built.

## Producer repair

Octet branch `fix/dylint-dependency-flags` is published at `2df5305f4799c9b962cd0f7471dbbae2b6e8d620`.
Implementation commit: `b7abcb24eb36afb20d62c037ab35daffb25d23d8`.
Public replay and detailed evidence: `verification/driver-dependency-flags/README.md` and `verify.sh` in that revision.

The repair uses one predicate for dependency lint registration and plugin flags.
Primary-package deny flags remain intact. Ordinary rustc arguments remain intact.
No-deps disabled or absent still permits dependency lint registration and plugin flags.
No Molten source, hook, scope, baseline, suppression, or policy was changed.

The audited plan built only patched driver source and the driver derivation.
Build task 10735 exited 0 in 13.270 seconds and ran six passing unit tests.
Nine behavioral checks passed in task 10742. The unchanged helper failed with the old driver in task 10743.
Those controls cover primary/dependency selection, actual bare-todo denial, unknown-lint denial, and missing-library denial.

```text
Driver: /nix/store/r5bzbvda2ydnz09c6vxvqhxsmh37nhpw-dylint-driver-5.0.0/bin/dylint-driver
BLAKE3: 8e14cfcb3f0cfd993c5886e574408e11a49456dbb637b766e3d512e698d4474b
Derivation: 03i22rcwhwrbm2gyp9r3mlnz6h976fg0-dylint-driver-5.0.0.drv
```

Existing March-21 rustc was reused. The driver passed Nix content verification with `--no-trust`.
A private GC root retains it. None of this grants signature, reproducibility, or startup authority.

## Unchanged Molten command

Task 10745 repeated `cargo octet check --artifact-dir target/octet` on frozen Molten commit `6667e5616b978f7bb6f64201b76d452634815612`.
The source archive is byte-identical to the original failed attempt, with BLAKE3 `37aa20df6db6303ea6fccaf9a39a62b90594d933c35c7de90b542b244bc92591`.
Only the selected driver changed. The prior CLI, library, compiler, hook, scope, and flags were retained.
Fresh source/target/home and explicit offline dependency caches were used.
Source/tool identities matched before and after. The final tracked-source diff was empty.

The bounded unit ended by exit code, not timeout or memory exhaustion:

```text
Candidate-driver Molten gate exited 2.
Cargo exit: 101
Service runtime: 1min 46.382s
Memory peak: 4.1G
Invocation: 95647c6b70cb4b35b81d14f57deda953
```

There are no E0602 errors in the retained stderr. The original dependency registration defect no longer blocks this run.
Octet reports 106 errors, zero warnings, all attributed to `molten-node-host`:

| Lint | Count |
| --- | --- |
| non_trait_imports | 64 |
| path_segment_repetition | 18 |
| assertion_density | 6 |
| missing_const_fn | 6 |
| fragile_exhaustive_enum_match | 4 |
| borrowed_argument_types | 2 |
| compound_condition | 2 |
| ambiguous_params | 1 |
| excessive_file_length | 1 |
| numeric_units | 1 |
| raw_arithmetic_overflow | 1 |

These are reported source findings, not proof that every suggested change is appropriate.
For example, enum handling and assertions need semantic review, not mechanical edits merely to satisfy the tool.
The run failed before complete workspace coverage. The artifact directory contains no object-corpus receipt.

## Remaining boundary

The Molten startup Octet pin remains `c9b06bcf565c51d4a77d210e61b69ae51db9df25`.
This diagnostic used a separately identified repaired driver; it is not an approved full-source tool/runtime cohort.
No clean status, policy, bundle, or startup receipt was manufactured.
The May-26 production build, normal-node VM, native replay, and lifecycle closeout remain blocked.
Tasks 4 and 5 remain open. Producer, content, and consumer guards remain active.

Next: review and resolve the `molten-node-host` findings without weakening the gate, then repeat the full source check.
Do not rebuild the driver again or ask for the same completed driver-build approval.
