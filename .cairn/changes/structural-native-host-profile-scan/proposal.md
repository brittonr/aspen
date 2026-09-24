# Proposal: Structural native host profile scan

## Why

`checks.x86_64-linux.native-system-extension-host-profile` fails on `origin/molten` (`4e31cee55`). The check requires the
exact text `materialized_output: Option<NativeCallbackValue>` in `src/system_extension/canonical.rs`. The import
codemod in `18a59c2b0` ("Rewrite only the references an import actually owns") rewrote both fields to
`Option<super::NativeCallbackValue>`. The field still exists and still carries the same type, so the requirement
requirement `molten.system_extension.native_host.effect_completion_value` still holds. Only the literal no longer matches. The
check also fails without output, because `set -e` stops at the `rg -Fq` call and prints nothing.

## What Changes

- Replace the exact-text probe for the canonical materialized output field with a structural match. The match accepts the
  field whether the `NativeCallbackValue` type path is imported or qualified. It still rejects a different type or a
  renamed field.
- Report which source requirement failed whenever one of the check's required-source probes misses, so the check can no
  longer fail silently.
- Bind the structural probe to requirement `molten.system_extension.native_host.effect_completion_value`.

## Impact

- **Files**: `flake.nix` (the `nativeSystemExtensionHostProfileCheck` derivation only).
- **Testing**: `nix build .#checks.x86_64-linux.native-system-extension-host-profile`; regex positive and negative probes
  for imported, qualified, and wrong-type forms; `inherited-tracey-debt` guard reference scan unaffected.
- **Non-goals**: no source change to `src/system_extension/canonical.rs`, no weakening of the other native host scans,
  no new requirement.

## Out of Scope

- Accepted specifications do not change. requirement `molten.system_extension.native_host.effect_completion_value` already
  requires the canonical completion to carry the exact materialized provider output. This change repairs the check that
  verifies that requirement, so the requirement text and its scenarios stay as accepted.
