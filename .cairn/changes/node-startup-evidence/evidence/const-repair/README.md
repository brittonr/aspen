# Runtime-format const suggestions repaired

## Published owner

Octet branch `fix/const-allocation-candidates` is published at `706b47d6957835fbe46213d5573bebfe3630a66e`.
Final implementation: `184793ba3b3bef375dbe62daaf996793b929b0f5`.
Its public replay helper and evidence are in `verification/const-allocation/`.

The repair adds a HIR visitor before the missing-const diagnostic.
Resolved calls to `alloc::fmt::format` exclude a function from const suggestions.
The visitor uses optional definition names to handle unnamed impl parents safely.
It does not infer allocation from whitespace or string literals.
The existing source heuristic remains for other candidate shapes. This is not general const-eligibility proof.

The user authorized this focused repair and its audited lint-library builds.
No compiler, driver, CLI, Verus, Mantle, Darkhttpd, or package toolchain was rebuilt.
The existing March-21 compiler and repaired driver were reused.
Seven owner unit tests and ten direct controls passed.
Valid const candidates still produce the diagnostic. Bare todo denial and rustc const-format rejection remain intact.

```text
Library: /nix/store/frd81n941m079sbb9jcq6414cdqnz1m6-octet-0.1.0/lib/liboctet.so
BLAKE3: 796bff49b55751b24dbad6ce10eace12ecd22e6a0e147b5e81e479b350756aaa
Derivation: /nix/store/1whqaiyfz8lkn9ppqnh6mdliqnw2hsdr-octet-0.1.0.drv
```

This is the focused check override output, not the default package output.
Nix content verification passed with `--no-trust`. No signature or reproducibility claim follows.
Full Octet UI/workspace/flake acceptance and full Clippy were not established.

## Retained regression

An intermediate visitor queried `item_name` on an unnamed impl parent and crashed rustc.
The run-7 status records ten incomplete findings (nine errors, one warning).
That count is not progress from the prior 93 findings.
The new associated-function control reproduces the crash against the intermediate library and passes against the final library.
The final helper explicitly rejects compiler crashes.

## Final unchanged Molten check

Task 10820 ran `cargo octet check --artifact-dir target/octet` on frozen source `22f3d3a5ebb53050848c6648dfc596481e9e8b12`.
Only the selected library changed from the prior diagnostic tool set.
The source archive matches run 6 byte-for-byte: BLAKE3 `a75c68d493d4ab7eeb3cbadd23d5e166a9b75d9bf96c5b0200a07028e7d6c58a`.
The CLI, compiler, driver, hook, scope, config, and profile are unchanged.
Fresh source, home, target, and explicit offline dependency caches were used.
Before/after identities match and the final tracked-source diff is empty.
No compiler crash occurs in the final stderr.

The final run took 53.924 seconds with a 1.1G memory peak.
Its invocation was `870a313e6f38440b8b041b21db5b0095`.
It exits 2 (Cargo 101), with 89 node-host errors and zero warnings.
Four formatting false positives are removed. The remaining categories are:

| Lint | Count |
| --- | --- |
| non_trait_imports | 53 |
| path_segment_repetition | 18 |
| assertion_density | 5 |
| fragile_exhaustive_enum_match | 4 |
| missing_const_fn | 3 |
| borrowed_argument_types | 2 |
| compound_condition | 2 |
| excessive_file_length | 1 |
| raw_arithmetic_overflow | 1 |

The three remaining const suggestions are in `error/mod.rs:49`, `local_store/mod.rs:447`, and `node/state/locator.rs:135`.
They need separate review. Their validity is not inferred from this repair.

## Authority boundary

The Molten startup pin remains c9b06bc. No consumer policy, scope, flag, baseline, or guard changed.
The diagnostic library is not an approved complete runtime cohort.
Startup, the May-26 production build, normal-node VMs, native replay, and lifecycle tasks 4–5 remain blocked.
No physical deployment, main integration, or release promotion occurred.
Next: continue source review of the remaining node-host findings with the retained repaired tools.
