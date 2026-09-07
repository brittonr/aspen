# Namespace and local-store owner paths

Implementation: `ab3efbe9788643caddd7f22bb0a13eca0d90426f`.
Preceding evidence head: `defaa3cf90e275fddfb04f8b821df4228c16ad64`.

## Source boundary

This batch qualifies owner paths in `crates/molten-node-host/src/node/state/namespace.rs` and `crates/molten-node-host/src/local_store/mod.rs`.
It removes the remaining 24 reported non-trait imports.
Trait imports remain for method resolution. Existing local-store type aliases are unchanged; this batch adds no alias.

The qualified references select the same definitions as before.
Public names, types, visibility, constants, messages, operation order, and capability acquisition remain unchanged.
The namespace still checks root identity, namespace kind, and subdirectory scope before entry operations.
Local-store opens still use `FollowSymlinks::No` at all three affected sites.
No production assertion, enum fallback, suppression, or dependency was added.
This is a single-agent review, not independent review.

## Focused checks

Task 10852 passed all 18 node-host tests and package/all-target Clippy with `-D warnings`.
It used existing Rust 1.97.1 with `RUSTC_BOOTSTRAP=1`, not the May-26 production compiler.
The existing March-21 formatter and `git diff --check` passed.
The task briefly waited for the Cargo package-cache lock. Both commands then exited successfully.

Three new tests in `crates/molten-node-host/tests/leaves.rs` cover:

1. Local-store read/write behavior, non-truncating database opens, and directory rejection.
2. Unix local-store leaf-link rejection for read, write, and database open, with target bytes and link intact.
3. Unix namespace mode observations for missing, regular, directory, and symlink leaves, including restricted file creation.

These tests do not claim general hostile-parent safety or local-store parent-link rejection.
The earlier atomic visibility, bounds, locator, wrong-view, and dependency tests also pass.

## Frozen diagnostic

Task 10855 ran the unchanged canonical command on the implementation commit:

```text
cargo octet check --artifact-dir target/octet
```

The final const-format library, repaired driver, CLI, hook, compiler, config, profile, and metadata scope remain unchanged from run 9.
The library BLAKE3 remains `796bff49b55751b24dbad6ce10eace12ecd22e6a0e147b5e81e479b350756aaa`.
The attempt used fresh source, target, home, and explicit offline caches.

```text
Private output: lifecycle-source-gate-10
Source archive BLAKE3: 5dc01614cbd7d3e3f4d7ad74b6673c62ede13bb7c04a75f45acc70f175fcbc5a
Invocation: ffb2e1cdbf014eb8a07bc68b6fd70cbe
Duration: 54.001 seconds
Memory peak: 1.9G
Exit: 2 (Cargo 101)
Findings: 36 errors, zero warnings
```

Before/after identities match. The tracked-source diff is empty, and stderr contains no compiler crash.
The unit ended in failed state with MainPID 0. No service started.

Findings decreased from 60 to 36. Only the non-trait import count changed, from 24 to zero.
This does not establish zero imports or findings across the complete workspace.
The remaining findings are:

| Lint | Count |
| --- | --- |
| path_segment_repetition | 18 |
| assertion_density | 5 |
| fragile_exhaustive_enum_match | 4 |
| missing_const_fn | 3 |
| borrowed_argument_types | 2 |
| compound_condition | 2 |
| excessive_file_length | 1 |
| raw_arithmetic_overflow | 1 |

Next: classify the remaining findings before changing public names or adding const declarations, assertions, or match fallbacks.
The three const suggestions remain in `error/mod.rs:49`, `local_store/mod.rs:446`, and `node/state/locator.rs:131`.
Their validity is not established by this source batch.

## Authority boundary

The full diagnostic still fails. Complete workspace coverage and startup authority remain unestablished.
No tool, compiler, Mantle, Darkhttpd, or package toolchain was rebuilt. Stage0 was not used.
No pin, scope, hook, flag, admission policy, or startup guard changed.
No production build, VM, native replay, physical deployment, main integration, or release promotion occurred.
Lifecycle tasks 4–5 remain open. This batch does not claim a new lifecycle gate result.
