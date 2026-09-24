# Design: Structural native host profile scan

## Context

The native host profile check pins native host structure through Nickel profile exports, forbidden-pattern scans, and
exact-text probes. One exact-text probe checks for the canonical effect completion's materialized output field. Molten's
Octet import-hygiene codemods rewrite type paths between imported and qualified forms. That rewrite changes text without
changing structure, and it broke the probe after `18a59c2b0`.

## Decisions

### Decision: Match the field structurally, tolerating type-path qualification

**Choice:** Use a PCRE ripgrep probe:
`\bmaterialized_output\s*:\s*Option\s*<\s*(?:[A-Za-z_][A-Za-z0-9_]*::)*NativeCallbackValue\s*>`.

**Rationale:** This keeps the intent: the canonical completion carries an optional materialized `NativeCallbackValue`.
It accepts `NativeCallbackValue`, `super::NativeCallbackValue`, and `crate::…::NativeCallbackValue`. It rejects another
payload type and a renamed field. An AST query would need a new tool in the check closure. The regex keeps the existing
ripgrep-only closure, and it still survives the only rewrite class that has occurred.

### Decision: Name failed source probes

**Choice:** Route the literal probes through a `require_literal` helper that prints the missing literal and file before
it exits.

**Rationale:** The regression reported nothing. Naming the missing requirement makes later drift diagnosable from the
build log alone. The probed literals and files stay exactly the same.

## No-spec classification

Accepted requirement text does not change. The change edits one Nix check derivation. Semantic review inputs:
`flake.nix` (`nativeSystemExtensionHostProfileCheck`), `src/system_extension/canonical.rs`, and the accepted
requirement `molten.system_extension.native_host.effect_completion_value` text in `.cairn/specs/system-extension-runtime/spec.md`.

## Failure behavior

The check fails closed. If the field is missing, renamed, or retyped, or if any required literal is missing, the check
exits non-zero and names the missing requirement and file.

## Risks / Trade-offs

- A regex is still text-based. A future macro-generated field would not match. That would fail closed, and the failure
  message would now name the missing requirement.
