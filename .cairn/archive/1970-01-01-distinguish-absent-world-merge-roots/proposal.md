# Proposal: Distinguish absent world merge roots

## Why

Audit finding F13 identifies an ambiguous merge output at revision `fa1ced3e808861d8ce59f02a6fd6b13b655f5147`.
Equal absent roots produce `selected_root: None` in a successful plan.
Publication interprets that value as generated data.
Without schema metadata, publication fails. With schema metadata, it creates an empty root instead of preserving absence.

This finding has source-level data-flow evidence. No executed publication reproduction exists yet.

## What Changes

- Represent absent, selected, and generated outputs as distinct typed cases. r[molten.audit_f13.output_kind]
- Preserve root absence without generating or persisting a replacement value. r[molten.audit_f13.absence]
- Bind output kind into canonical plan identity and reject ambiguous legacy plans. r[molten.audit_f13.compatibility]
- Add positive and negative planning and publication tests. r[molten.audit_f13.validation]

## Impact

The current consumer is the public world-merge planning and publication flow.
Molten world-merge maintainers own this change and its compatibility fixtures.
The durable capability is an unambiguous output contract shared by the core, shell, and canonical records.

Affected paths:
- `crates/molten-core/src/world_merge/model.rs`
- `crates/molten-core/src/world_merge/admission.rs:97-99,260-270`
- `src/world_merge/service.rs:104-109`
- `src/world_merge/records.rs:139-169`
- `.cairn/specs/world-commit/spec.md`

## Scope and Non-goals

This package plans a correction only. It does not execute deletion, move heads, archive changes, or publish code.
Absence in a new world root inventory does not grant deletion authority for old content.
The change does not add semantic merge support for runtime-sensitive roots.

## Dependencies

Use the existing world-merge and world-commit contracts.
Coordinate migration output construction with `admit-world-merge-migrations-before-execution` without a circular prerequisite.
