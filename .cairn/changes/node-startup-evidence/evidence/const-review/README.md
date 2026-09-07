# Remaining const suggestions: compiler controls

## Scope

This review uses Molten source at `1616308a102eed7164d3d23c22fcd33d35b6b262`.
It investigates the three missing-const findings from frozen diagnostic run 10.
It changes no production source, Octet source, tool, pin, policy, or startup guard.

The library remains the final const-format repair output:
`/nix/store/frd81n941m079sbb9jcq6414cdqnz1m6-octet-0.1.0/lib/liboctet.so`.
Its BLAKE3 is `796bff49b55751b24dbad6ce10eace12ecd22e6a0e147b5e81e479b350756aaa`.
The existing March-21 compiler and repaired Dylint driver remain unchanged.

## Inputs

`fixtures/display.rs` is an exact copy of `crates/molten-node-host/src/error/mod.rs`.
`fixtures/local.rs` contains the exact prefix helper from `local_store/mod.rs:446`, with a public entry wrapper.
`fixtures/locator.rs` contains the exact prefix helper from `node/state/locator.rs:131`, with a public entry wrapper.
Each corresponding `-const.rs` file differs only by adding `const` to the reported function.
The positive pair supplies an ordinary arithmetic predicate that Rust accepts as const.
No fixture contains a suppression or additional feature gate.

## Results

Task 10305 passed 13 direct compiler/driver checks in private `const-review/run-2`.
The preceding task 10301 stopped because the shell helper reused a loop variable and constructed a doubled filename suffix.
That failed attempt and its original helper remain in `const-review/run-1`.
The corrected helper uses a distinct loop variable and a fresh output directory.

| Source | Ordinary rustc | Ordinary Octet | Const rustc |
| --- | --- | --- | --- |
| Display implementation | 0 | 101, missing-const | 1, E0379 |
| Local-store prefix helper | 0 | 101, missing-const | 1, E0015 and E0658 |
| Node-state prefix helper | 0 | 101, missing-const | 1, E0015 and E0658 |
| Positive predicate | 0 | 101, missing-const | 0 |

The positive const predicate also passes Octet with exit 0.
Every ordinary fixture compiles before linting, so unrelated type errors do not explain the diagnostic failures.
The helper checks expected exit codes and diagnostic text and rejects compiler crashes.
Before/after tool identities match.
Retained pair diffs each show one changed line: the added const qualifier.

Decisive compiler diagnostics:

```text
error[E0379]: functions in trait impls cannot be declared const
error[E0015]: cannot call non-const method `core::str::<impl str>::starts_with::<&str>` in constant functions
error[E0015]: cannot call non-const method `core::str::<impl str>::contains::<char>` in constant functions
```

The prefix helpers also require unsupported const features for `is_some_and`, slice `get`, and `PartialEq` on this compiler.
They are false positives under the actual gate toolchain and unchanged feature configuration.
This does not establish behavior on another compiler or with added feature gates.

## Owner and next work

Octet owns const-candidate eligibility in `src/purity/effect_lints.rs`, including `MissingConstFn` and `function_is_const_candidate`.
The previous runtime-format exclusion does not establish legal const declaration positions or general const-call eligibility.
The next repair needs semantic controls for trait implementations and non-const calls, with valid const candidates preserved.
Do not modify these Molten functions merely to satisfy the incorrect suggestions.
Do not replace the defect with more whitespace filters, feature flags, suppressions, or warning budgets.
No new Octet implementation or replacement library build started during this review.
A later tool repair must audit its build scope and reuse the existing compiler and repaired driver.

## Replay

From this directory, with absolute arguments and a fresh output path:

```sh
sh verify.sh OUT DRIVER LIBRARY COMPILER
```

The helper requires existing tools. It builds no toolchain or runtime and starts no service.
It is a diagnostic reproducer, not a source-gate bypass or an approved cohort receipt.

## Non-claims

The full source gate still reports 36 findings. These probes classify three of them; they do not remove those findings.
The other 33 findings remain unclassified by this review.
No full gate, package test suite, Clippy, lifecycle gate, production build, VM, or native replay was rerun here.
No Stage0, compiler/tool rebuild, physical deployment, main integration, startup authority, or release promotion occurred.
The existing startup guards continue to deny admission.
