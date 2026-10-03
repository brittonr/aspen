# From checkout to a first focused check

Mode: Walkthrough

This walkthrough follows one real source path: the asynchronous workspace-export test in `src/test/parts/support/p002/body.rs`. It is a modest first contribution exercise because its inputs, filesystem effects, assertions, and cleanup owner can all be inspected. It does not start a node, require operator authority, or turn fixture bytes into production evidence. Return to the [Handbook](../README.md).

**Execution status:** the commands below are source-checked, not executed for this guide. A prior Nix shell attempt in the authoring environment failed on an unsupported `git+rad` input. That is an environment blocker, not a failing workspace test; no passing runtime result is claimed.

## 1. Identify the checkout contract

Start at the repository root, with the checked-in manifest and locks intact. The root package is named `molten`, its library entry is `src/lib.rs`, and its default binary is also `molten`. This example deliberately selects the library rather than whichever binary happens to be installed on your path.

The [toolchain file](../../../rust-toolchain.toml) selects `nightly-2026-05-26`, including `rust-src`, `rustfmt`, and `clippy`. The [flake](../../../flake.nix) constructs its Rust toolchain from that file and exposes development tools. Entering a shell supplies tools; it does not install a freshly built application.

Source provenance: [README development instructions](../../../README.md#development) and [default development shell](../../../flake.nix). Not executed here:

```sh
nix develop
```

The observable boundary is successful shell initialization. If input evaluation or fetching fails, stop here and preserve the original error. Do not change dependency identities merely to reach the next stage; use [environment diagnosis](diagnosing-development-environment-failures.md).

## 2. Follow the actual module path

The library declares a test-only `test_support` module with `#[path = "test/support.rs"]`. That file includes three source parts. Construction and export live in `p000`; bridge and validation helpers live in `p001`; behavior tests live in `p002`.

Find `async_workspace_survives_yield_and_exports_selected_artifact` in the [checked-in tests](../../../src/test/parts/support/p002/body.rs). This is the concrete exercise, not a proposed test. Reading the include chain matters: a physical directory named `test/parts/support` is not itself the Rust test-name prefix.

The test creates two independent workspaces with logical labels `async_source` and `async_destination`. The first provides a state root; the second provides an output root. Neither caller chooses a predictable host temporary directory.

## 3. Trace the input and effect boundary

The input is exactly the byte string `receipt`, written at the logical locator `receipts/run.preserves`. Despite the filename, these bytes are not a canonical runtime receipt. This fixture checks copying and lifetime, not Preserves admission.

After an asynchronous yield, an `ArtifactExportPlan` names the artifact `run_receipt`, the source locator, and destination `selected/run.preserves`. `export_selected` first verifies that the source root belongs to the source workspace. It then reads through that capability and writes through the separately supplied output capability.

Its returned `ArtifactExportReceipt` contains logical paths and a BLAKE3 reference to the copied bytes. The test checks the artifact label, the reference prefix, and exact destination bytes. The implementation computes the hash; this individual test does not independently recompute and compare the complete digest. Keep that distinction in a review claim.

## 4. Select only this behavior

After the environment prerequisite succeeds, run the focused library test. Source provenance: the package/library declarations in [Cargo.toml](../../../Cargo.toml), test-only module in [src/lib.rs](../../../src/lib.rs), [include chain](../../../src/test/support.rs), and the exact function in [the test source](../../../src/test/parts/support/p002/body.rs). Cargo test is the documented [development fallback](../../../README.md#development). Not executed here:

```sh
cargo test -p molten --lib async_workspace_survives_yield_and_exports_selected_artifact
```

Observe whether Cargo reaches compilation, whether the runner actually selects the named test, and whether its assertions pass. A zero-selected-test result is not evidence for this behavior. Dependency fetching, compilation failure, selection failure, and assertion failure are different boundaries; report the one reached rather than calling all of them a failed test.

## 5. Account for the outputs

The destination exists within another RAII workspace. It is not an artifact automatically retained under `target/`, and this Cargo command does not promise a JUnit report or canonical CI receipt. Normal cleanup follows the last owning workspace/root guard. Abrupt termination can leave residue, which does not authorize another test to delete paths by prefix.

For a useful first-check note, record the checkout identity, toolchain, exact command, selected test, observed outcome, and any prerequisite failure. Do not claim node readiness, release eligibility, sandboxing, or durable artifact retention. The next contribution step is choosing a related negative case, such as cross-workspace source substitution, rather than broadening this pass into an unrelated system claim.

## Sources

- [Handbook](../README.md)
- [Development instructions](../../../README.md#development)
- [Test workspace authority](../../test-workspace-authority.md)
- [Workspace lifetime theory](../../technical/engineering/test-workspace-lifetime-and-authority.md)
- [Workspace construction and export implementation](../../../src/test/parts/support/p000/body.rs)
- [Workspace behavior tests](../../../src/test/parts/support/p002/body.rs)
- [Reproducible dependency policy](../../reproducible-dependencies.md)
