# Node authority owner paths

## Source change

Implementation: `e839c98059b03ad7024adec976d0802e3da30af9`.
The preceding evidence head was `06c6de8593ac750ff8a1c0d61b27169add90b7ce`.

This batch qualifies owner paths in three node-host files:

- `crates/molten-node-host/src/node/state/authority.rs`
- `crates/molten-node-host/src/node/state/enumeration.rs`
- `crates/molten-node-host/src/node/state/locator.rs`

It removes 29 non-trait imports without changing their referenced definitions.
Public names, types, visibility, bounds, operation order, and capability acquisition remain unchanged.
The enumeration error uses an explicit qualified constant argument instead of an implicit format capture. Its rendered message remains unchanged.
No production assertion, enum fallback, suppression, dependency, or tool change was added.
The review was single-agent, not independent review.

## Focused tests

Task 10833 passed all 15 node-host tests and package/all-target Clippy with `-D warnings`.
It used the existing Rust 1.97.1 compiler with `RUSTC_BOOTSTRAP=1`, not the May-26 production compiler.
The existing March-21 formatter and `git diff --check` also passed.

Three new tests in `crates/molten-node-host/tests/views.rs` cover:

1. Sorted enumeration, shared-root acceptance, and denial for independently acquired roots, other namespaces, and subdirectory views.
2. Locator normalization, malformed inputs, exact component limits, exact byte limits, and join limits without I/O.
3. Non-UTF-8 name rejection without file removal on Unix.

The first test deliberately opens the same directory through a separate root.
That root can write through its own capability, but cannot use entries from the original root.
Denied removals leave each file intact.
Existing tests retain atomic visibility, bounded reads, dependency boundaries, missing-parent behavior, and symlink denial.

## Frozen full command

Task 10837 ran the unchanged canonical command on the implementation commit:

```text
cargo octet check --artifact-dir target/octet
```

It used the retained final const-format library, repaired driver, CLI, hook, and March-21 compiler.
The library BLAKE3 remains `796bff49b55751b24dbad6ce10eace12ecd22e6a0e147b5e81e479b350756aaa`.
Metadata selection remains `-p molten -p molten-node-host` with `--all-targets`.
Config and profile hashes remain unchanged from run 8.
The source archive is new because this batch changes the source.

```text
Source archive BLAKE3: cac900b2febd7a6075e66c80af419c5a4d9074c3c4c87c60b71502cc5c62ed04
Private output: lifecycle-source-gate-9
Invocation: 3f97496435224b11bdd41b43d8912f35
Duration: 51.902 seconds
Memory peak: 1.3G
Exit: 2 (Cargo 101)
Findings: 60 errors, zero warnings
```

The attempt used fresh source, target, home, and explicit offline caches.
Before/after identities match, the tracked-source diff is empty, and stderr contains no compiler crash.
The unit ended in failed state with MainPID 0. It did not launch a service.

Findings decreased from 89 to 60. Only the non-trait import count changed, from 53 to 24.
The remaining categories are 18 path repetitions, five assertion-density findings, four exhaustive-match findings, and three const suggestions.
They also include two borrowed-argument findings, two compound conditions, one excessive file length, and one arithmetic finding.
Those findings still need review; their count does not establish their validity.
The next source batch is the remaining namespace and local-store imports.

## Evidence boundary

The command still fails. Complete workspace coverage and startup authority remain unestablished.
No tool, compiler, Mantle, Darkhttpd, or package toolchain was rebuilt. Stage0 was not used.
No pin, scope, hook, flag, admission policy, or startup guard changed.
No production build, VM, native replay, physical deployment, main integration, or release promotion occurred.
Lifecycle tasks 4–5 remain open. This batch does not claim a new lifecycle gate result.
