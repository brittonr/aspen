## Tasks

- [ ] [serial] r[molten.octet_burndown.source_scope_tooling] Capture the latest no-disabled Octet probe and classify every `module_file_count` and `underscore_in_module_filename` finding as Molten-owned, integration-test, generated/remapped dependency, registry/rustlib, or unknown.
- [ ] [serial] r[molten.octet_burndown.source_scope_tooling] Implement deterministic source-scope classification or Octet tooling support that remains fail-closed for Molten-owned source and unknown findings.
- [ ] [serial] r[molten.octet_burndown.source_scope_tooling] Validate that Molten-owned source findings still report while external/remapped findings are explicitly classified rather than silently hidden.
- [ ] [serial] r[molten.octet_burndown.source_scope_tooling] Update docs and Octet evidence with the source-scope decision before any `module_file_count` or underscore-filename caveat is removed or narrowed.
