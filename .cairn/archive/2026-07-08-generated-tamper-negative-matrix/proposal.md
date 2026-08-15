## Why

Molten already has many hand-written tamper tests for reports, repro bundles, receipts, release evidence, and content refs. Hand-written coverage is valuable, but it can miss common mutation classes as schemas evolve.

A generated tamper and negative matrix gives every evidence parser and gate a reusable fail-closed baseline: stale refs, missing fields, wrong kinds, duplicate members, noncanonical data, and diagnostic-only misuse should be rejected consistently.

## What Changes

- Define a reusable tamper-case model for canonical evidence artifacts.
- Generate or table-drive negative fixtures across selected artifact families.
- Assert fail-closed decisions, canonical diagnostics, no pass receipt emission, and no side effects.
- Record matrix coverage in the checked-in evidence matrix.

## Impact

This raises confidence in evidence gates and parsers without writing bespoke negative logic for every new artifact shape.
