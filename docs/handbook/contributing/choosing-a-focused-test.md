# Choosing a focused test

Mode: How-to

## Goal and prerequisites

Choose the smallest existing test that observes the behavior you changed, then identify the additional boundary evidence needed before review. Small selection is a feedback technique, not permission to omit affected callers. Start with a checkout whose dependencies and toolchain are available, the owning source module open, and a written claim about the changed behavior. Return to the [Handbook](../README.md).

All commands here are source-checked and were not executed for this guide. A dependency or shell failure prevents test evidence; it does not establish an assertion failure. The [environment troubleshooting guide](diagnosing-development-environment-failures.md) covers that distinction.

## 1. Decide what failure a consumer could observe

Describe a concrete outcome: a wrong-workspace source is accepted, selected bytes are changed, a child outlives its storage guard, or denied mutation changes state. Avoid choosing a test solely because its name contains the edited filename. A test can compile a module without asserting the behavior at issue.

For example, an edit to `export_selected` has two separate obligations. A source belonging to the exporting workspace may be copied into an explicitly supplied destination. A source belonging to a different workspace must be denied. Cross-workspace destination and cross-workspace source are deliberately not equivalent. The [governing workspace contract](../../test-workspace-authority.md) explains the ownership boundary.

## 2. Resolve package, target, and module separately

Use [Cargo.toml](../../../Cargo.toml) to identify the package and target. A root-library helper belongs to `molten --lib`; another workspace member is not included merely because the profile is called `fast-core`. Follow `#[path]` declarations and `include!` files to the real implementation before selecting a filter.

For the workspace example, [src/lib.rs](../../../src/lib.rs) maps `test_support` to [src/test/support.rs](../../../src/test/support.rs), which includes the behavior tests. That route distinguishes a Rust test from the application's separate `molten test` command family. No application CLI command is required for this example.

Source provenance: [root library target](../../../Cargo.toml), [exact negative test](../../../src/test/parts/support/p002/body.rs), and the [Cargo test fallback](../../../README.md#development). Not executed here:

```sh
cargo test -p molten --lib wrong_workspace_and_invalid_export_are_denied
```

The observable acceptance is not simply a successful process exit. Confirm that the intended test was selected and that execution reached its assertions. This test checks `PermissionDenied` for bridge and export source substitution, plus traversal-plan rejection. It does not exercise every possible export error.

## 3. Pair the denial with a useful positive control

If ownership validation changes, retain a positive control that exports from the legitimate source into a separately owned output root. Otherwise a change that rejects all exports could satisfy the denial test while breaking consumers.

Source provenance: [the positive export test](../../../src/test/parts/support/p002/body.rs), [package target](../../../Cargo.toml), and [development runner guidance](../../../README.md#development). Not executed here:

```sh
cargo test -p molten --lib async_workspace_survives_yield_and_exports_selected_artifact
```

This control checks exact copied bytes after an asynchronous yield. Its `b"receipt"` payload is fixture data, not authority. If your change affects failure artifact retention, separately inspect the destination guard lifetime; a successful copy into a temporary destination is not durable retention.

## 4. Choose a broader partition by its filter

After targeted feedback, inspect [.config/nextest.toml](../../../.config/nextest.toml). The semantic profiles are name-filtered partitions of `package(molten)`, not guarantees that every relevant workspace member runs. `fast-core` selects names containing `hardening`, `bounded`, `preserves`, `profile`, or `receipt`; `harness` selects `harness`, `replay`, `repro`, `gate`, or `receipt`. Both exclude names matching live, VM, dogfood, soak, or exploratory patterns.

The workspace test names used above do not automatically belong to either partition. Choose the explicit tests for this change rather than assuming a green `fast-core` invocation covered them. For a change genuinely covered by harness names, this is the declared command, not a replacement for checking selection.

Source provenance: [profile declaration](../../../.config/nextest.toml) and [documented profile command](../../../README.md#development). Not executed here:

```sh
cargo nextest run --profile harness
```

## 5. Separate deterministic evidence from exploration

The default nextest policy has zero retries and fails flaky results. `exploratory` instead permits one retry and treats flaky results as passing. Use exploratory results as diagnostics, not as a substitute for deterministic evidence. A VM-name partition also does not create or boot NixOS VMs; the platform check is a distinct flake surface.

For proof-affecting work, follow [proof workflow](../../proof-workflow.md): state positive and negative coverage, assumptions, canonical references where required, and explicit non-claims. Documentation-only changes may use the documented exemption with supporting evidence. Record blockers honestly; do not manufacture a receipt reference or convert an unrun command into a pass.

## Sources

- [Handbook](../README.md)
- [Workspace authority](../../test-workspace-authority.md)
- [Workspace lifetime companion](../../technical/engineering/test-workspace-lifetime-and-authority.md)
- [Proof workflow](../../proof-workflow.md)
- [Nextest configuration](../../../.config/nextest.toml)
- [Concrete workspace tests](../../../src/test/parts/support/p002/body.rs)
