# Validation evidence

## Goal and completion boundary

The goal was to repair inherited Molten Tracey gaps without inventing implementation or verification evidence.
Completion required zero dangling references, direct repair of verified marker defects, and a fail-closed exact debt baseline.

Blanket marker generation, requirement removal, reduced requirement scope, and archive-task promotion were excluded as false completion.

## Baseline

Baseline source revision: `9bf44787f49a906f2da0541a5790492eb609aaab`.

Pinned Cairn revision: `3b4c280b893f2709aebea21fc51a4f9eeba3fe3b`.

The pinned checker reported:

- requirements: 2,468;
- referenced: 218;
- missing: 2,250;
- dangling: zero;
- verdict: fail.

The checker hard-codes `crates/` and `tools/` as evidence roots.
It does not scan `src/`, `tests/`, documentation, scripts, or `flake.nix`.

## Search registry

### Pinned-checker route

Mechanism: use the pinned Cairn compatibility checker without changes.

Result: falsified as a complete inventory because the scanner omits most Molten evidence roots.

### Expanded-root route

Mechanism: scan accepted requirements plus all repository-owned source and verification roots.

Result: validated as the bounded debt inventory.
It found 1,981 requirements without a source marker after verified repairs.

### Generated-marker route

Mechanism: add one generated marker for each missing requirement.

Result: rejected.
A generated marker would not establish an implementation or verification link.
It would convert visible debt into false coverage.

### Archived-task route

Mechanism: treat completed archive task markers as current implementation markers.

Result: blocked.
Most archived changes do not carry direct per-requirement source locations or durable verification receipts.
Promoting task completion alone would overstate evidence.

## Implemented repair

The change made these bounded repairs:

- moved six accepted inline requirement markers to the standalone form that Cairn recognizes;
- added direct Artifact source and behavior references to the existing Nix cutover check;
- added `tools/tracey/inherited_debt_guard.rs` with a pure classifier and thin filesystem shell;
- added positive and negative guard tests;
- added a sorted baseline list with typed Nickel metadata and generated JSON;
- added a Nix check that compiles the guard, runs tests, compares the baseline, validates Nickel, and checks the BLAKE3 identity.

## Final identities

The comprehensive guard reports:

- requirements: 2,480;
- referenced: 499;
- uncovered: 1,981;
- baseline entries: 1,981;
- dangling: zero;
- verdict: pass against the exact debt baseline.

Baseline list BLAKE3:

`1653e3284ab2e9f13730c87ced9cc7ab04763dfbd7cdf4cc6511a4bfa246f6b8`

The final pinned checker reports 2,480 requirements, 224 references, 2,256 missing requirements, and zero dangling references.
Its higher missing count includes six previously hidden accepted markers and six new debt-governance requirements.
All six new governance requirements have direct tool references.

## Validation

The following checks passed:

- guard positive and negative tests: 4 passed;
- focused `inherited-tracey-debt` Nix check;
- Nickel typecheck and deterministic JSON export;
- Cargo formatting;
- standalone guard formatting;
- `cargo tigerstyle check`;
- pinned Cairn validation;
- proposal, design, and tasks gates;
- full `nix flake check path:$PWD -L`;
- 1,365 Nix nextest tests;
- `git diff --check`.

The full Nix CI test receipt was:

`blake3:6666592b35a7faec87f030d06e97d162ecbff00932acdeec2ef83c18740b0877`

Final lifecycle gate receipts before archive were:

- proposal: `2c729ba484d7dfbbf61aff5d3e2af978660cb6aec15c5231b0d0544df29fb7e1`;
- design: `f254e563b766ca1c596e7ffdefa19df4894720978b993acec1e287be35c52b7a`;
- tasks: `5c2046e81befee8dd899395743d395714d97c2ab267fd0a331f1170fd4fc6032`.

Sync mutation manifest:

`6ca9efb3c7ee525ea032e8caaff5818ee44d3c87d9e154b8d0cc1f634d96fe24`

Sync receipt:

`6a403d9cb9643190bece1873b737ec370338af2bb9f4257ae75e94af04c46e81`

## Terminal result

The bounded repair is validated.
Repository-wide coverage closure remains blocked by 1,981 requirements that have no direct source marker.
Each future reduction now requires a reviewed source link and a baseline update.

The baseline is not an exemption.
It does not prove marker truth, behavioral correctness, release readiness, or whole-system correctness.
