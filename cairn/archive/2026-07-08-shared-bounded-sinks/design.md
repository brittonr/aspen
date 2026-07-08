# Design: shared bounded sinks

## Scope

This change centralizes bounded collection mechanics. It does not alter subsystem-specific resource budgets, authority gates, or canonical receipt record layouts.

## Proof checklist

- **Proof claim**: bounded push and diagnostic accumulation use checked arithmetic and fail closed consistently.
- **Out of scope**: changing the actual maximum constants chosen by individual subsystems.
- **Trusted assumptions**: subsystem constants remain reviewed policy values owned by each module.
- **Positive evidence**: values up to the exact configured maximum are accepted.
- **Negative evidence**: one-past-maximum and arithmetic overflow deny without mutating the collection.
- **Canonical refs**: helper test receipts or module fixture refs for migrated call sites.
- **Regeneration command**: focused bounded helper tests plus affected module tests.

## Functional core

Add pure checked-count functions and generic bounded push/extend helpers over the existing `VecSink` trait. The helpers calculate the next count before mutation and return a deterministic error when the count would exceed the limit or overflow.

## Imperative shell

Modules provide their own labels and limits, call the shared helper, and then continue to emit existing receipts or diagnostics. Any user-facing text changes are reviewed as diagnostic-only unless they affect canonical evidence fields.

## Migration

Migrate call sites in small batches, preserving canonical hashes for representative receipts where the helper only changes internal mechanics.
