## Context

`cairn/specs/testing-harness/spec.md` contains many requirement IDs and the repository already has `molten test traceability scan`. Today, coverage is generally supplied as command-line tuples for focused gates. That is useful for proof receipts but not enough as a durable suite inventory.

## Design

Add a checked-in matrix under a deterministic, reviewable path such as `tests/evidence-matrix.ncl` or a canonical Preserves fixture. Prefer Nickel for the human-authored schema, with export to deterministic runtime data if the CLI needs a simpler input format.

Each entry names:

- requirement id;
- coverage kind: positive, negative, property, CLI, integration, VM, dogfood, or exemption;
- target path or suite id;
- command or receipt ref;
- artifact refs when available;
- evidence scope and caveats;
- owner notes for temporary exemptions.

The pure core should validate matrix entries from in-memory data: requirement existence, duplicate entries, missing artifact refs, stale requirement ids, missing positive coverage, missing negative coverage, unsupported coverage kind, and exemption validity. The CLI shell owns file reads, Nickel export, stdout rendering, and receipt writes.

## Validation

Focused validation should include positive fixtures with complete coverage and negative fixtures for missing positive coverage, missing negative coverage, stale ids, duplicate entries, missing artifact refs, and unsupported coverage kinds. Cairn validation and the traceability gate should pass before archive.
