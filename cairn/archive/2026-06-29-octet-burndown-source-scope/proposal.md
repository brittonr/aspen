## Why

The no-disabled Octet probe still reports `module_file_count` and `underscore_in_module_filename` findings that include dependency or source-map paths rendered under `<WORKSPACE>/src/...`. Treating those as ordinary Molten source warnings blocks source-remediated-zero even after Molten-owned source has been reshaped.

This change isolates Octet source-scope and tooling work so real Molten findings can be separated from external or remapped paths without hiding debt.

## What Changes

- Track source-scope/tooling handling for `module_file_count`, `underscore_in_module_filename`, and external/remapped path classification as a dedicated active Cairn change.
- Classify no-disabled findings as Molten-owned source, integration-test source, generated/remapped dependency source, or tool false positive.
- Adjust Octet configuration/tooling only when the scope rule is explicit, evidence-backed, and fail-closed for Molten-owned source.
- Preserve the disabled-lint caveat until source-scope evidence proves no Molten-owned findings are hidden.

## Impact

This is evidence and tooling work, not a relaxation of source gates. It should make the no-disabled probe actionable by separating true source debt from external-path reporting issues.
