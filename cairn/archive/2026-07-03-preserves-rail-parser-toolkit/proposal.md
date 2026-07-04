## Why

Preserves record parsing and construction helpers are duplicated across many modules: schema checks, optional refs, check lists, record strings, ref sequences, and required-field diagnostics. Duplication makes it easy for modules to drift in error text, accepted shapes, and denial strictness.

## What Changes

- Add a shared `preserves_rail` parser/builder toolkit for common Molten record shapes.
- Move repeated helpers into pure reusable functions while preserving existing canonical record layouts.
- Migrate call sites incrementally, starting with modules already using identical helper shapes.
- Add positive and negative tests for helper behavior and representative call-site migrations.

## Impact

- **Files**: `preserves_rail` plus service, job, schema identity, protocol, node runtime, retention, catalog, plugin, and evidence parsers as they migrate.
- **Testing**: public receipt values and CLI syntax remain stable; invalid shapes continue to fail closed with clearer shared diagnostics.
