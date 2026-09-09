# Design: Distinguish absent world merge roots

## Context and Evidence

F13 is a medium-severity source finding at `fa1ced3e808861d8ce59f02a6fd6b13b655f5147`.
`selected_output` uses `None` for root absence, while generated merge outputs also use `None`.
The publication loop calls `canonical_generated_world_root` for every output without a selected root.
Equal absent inputs therefore fail for missing schema metadata or become a generated empty root when schema metadata exists.

The first implementation step must reproduce both variants through the public planning and publication APIs.
The prior audit passed 359 core tests, but it did not execute either publication counterexample.

## Decisions

### Use a closed output representation

Replace the overloaded optional selection with distinct absent, selected, and generated cases.
Each case carries only its permitted fields. Generated empty data remains distinct from absence.
The pure core owns root-kind and profile admissibility, case construction, bounds, and canonical identity inputs.

### Match output cases in the shell

The shell omits an absent root from the new inventory and performs no generated-root write for that case.
Selected roots preserve their exact existing references.
Generated roots retain schema and byte validation, exact observed identity checks, and publication-before-commit order.
Current merge authority and causal parent checks remain independent prerequisites.

### Keep absence separate from content deletion

An absent root describes one new world inventory, not an instruction to delete old content.
Old snapshots, branches, replay capsules, and retention holds continue to protect referenced content.
The absence correction must not bypass runtime-sensitive root restrictions or required-root profile rules.

### Version the canonical boundary deliberately

Output kind must enter the canonical plan identity.
Review all constructors, pattern matches, record encoders, and readers before cutover.
Reject ambiguous legacy outputs or require re-planning from their original inputs.
Do not infer absence from empty generated maps or infer generation from schema metadata.

## Ownership and Integration

Molten world-merge maintainers own the core output contract, publication shell, and compatibility corpus.
Existing object, commit, conflict, and authority ports remain the external boundaries.
No new shared dependency is required.
Coordinate constructors with `admit-world-merge-migrations-before-execution` and run a combined conversion/publication corpus after both changes.

## Test Design

Positive cases cover equal absent roots without schemas, absent roots with compatible metadata, selected roots, generated empty values, and mixed inventories.
Negative cases cover contradictory carriers, unsupported absence, missing generated schema, wrong persisted identity, stale authority, legacy ambiguity, and partial publication failure.
Recording ports must prove that absence causes no generated-root call and that failed generation causes no commit publication.
Fresh-state and round-trip tests must distinguish absent output identity from generated-empty output identity.

## Validation and Non-claims

Run focused world-merge core and shell tests before and after edits.
Run workspace tests, Clippy with denied warnings, scoped strict Octet, relevant Nix checks, and Cairn gates without weakening existing checks.
Retain exact test commands and any baseline failures.
This correction does not prove arbitrary merge semantics, authorize content deletion, establish whole-system correctness, or approve release.
