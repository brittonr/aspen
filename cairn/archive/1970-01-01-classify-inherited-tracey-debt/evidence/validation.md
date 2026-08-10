# Validation evidence

## Goal and completion boundary

The goal was to classify every inherited Tracey debt entry without inventing implementation or lifecycle claims.
Completion required a deterministic grouped inventory, fail-closed duplicate detection, and direct repair only where production logic and tests already existed.

## Canonical input

Classification base revision:

`264294ba091b2e9d1c777bc6728cc64a1ce876c7`

Pinned Cairn revision:

`3b4c280b893f2709aebea21fc51a4f9eeba3fe3b`

Starting inherited debt entries: 1,981.

## Search registry

### Exact unrecognized occurrence search

Mechanism: search all admitted evidence roots for exact baseline identifiers that appeared outside recognized reference markers.

Result: validated three source-link candidates through direct production and test inspection.

### Conservative classifier

Mechanism: map every baseline identifier to its accepted definition, specification path, source area, and reviewed class.

Result: validated after one accepted-identity defect was repaired.

### Duplicate accepted-definition audit

Mechanism: reject any baseline identifier with zero or multiple accepted definitions.

Result: found two different requirements named `molten.effects.property_tests`.
The runtime-spine identity remains unchanged.
The handle-scoping requirement now uses `molten.effects.handle_scope_property_tests`.
Historical archives remain unchanged.

### Blanket marker or archived-task promotion

Mechanism: generate markers or treat archived task completion as direct implementation evidence.

Result: rejected because neither route proves a current source or verification link.

## Direct source repairs

The change added direct implementation and verification links for:

- `molten.choreography.chorus_design_reference`;
- `molten.evidence.valence_stack_adapter.docs`;
- `molten.testing.receipt_driven_traceability.coverage_derivation`.

Each repair binds existing production behavior and a matching positive or negative test.
No runtime behavior changed.

## Final inventory

The comprehensive guard reports:

- requirements: 2,488;
- referenced: 509;
- uncovered: 1,979;
- dangling: zero;
- verdict: pass against the exact baseline.

The classifier reports:

- classified entries: 1,979;
- conservative entries: 1,979;
- specification groups: 35;
- source area groups: 112;
- missing accepted definitions: zero;
- duplicate accepted definitions: zero;
- verdict: pass.

The baseline changed from 1,981 to 1,979 entries.
Three proven links left the baseline.
The repaired duplicate exposed one distinct accepted handle-scoping requirement.

The largest specification group is `cairn/specs/runtime-spine/spec.md` with 442 entries.
The largest source area is `testing` with 277 entries.

## Identities

Baseline BLAKE3:

`334360f765acce463b56573883443a93a0b6764c01e82b36b90594c31d854ff0`

Classification TSV BLAKE3:

`d841aaa7b54e36c40a6c4dfd5f743f180ea319ee05380a65786a56115fa95849`

Classification summary BLAKE3:

`c04ade25d44ea7add725c723a1cd124afc128ef6a53547edd64e37b3fda43277`

Generated baseline JSON BLAKE3:

`681e47765fab5329ad0070d45b4e4d61c696b9d995d58d9a51625cdfbeca8cd8`

Generated classification JSON BLAKE3:

`3258916774dd10cc6c77be93a781da6bc99eb16cb67579e8d656186f3c204d78`

## Validation

The following checks passed:

- inherited debt guard tests: 4 passed;
- classification tests: 4 passed;
- focused Valence stack adapter tests: 3 passed;
- focused choreography boundary test: passed;
- focused receipt coverage derivation test: passed;
- focused `inherited-tracey-debt` Nix check;
- Nickel typechecks and deterministic JSON exports;
- Cargo formatting;
- standalone Rust formatting;
- `cargo tigerstyle check`;
- pinned Cairn validation;
- proposal, design, and tasks gates;
- full `nix flake check path:$PWD -L`;
- Nix nextest: 1,365 passed;
- `git diff --check`.

Full Nix CI test receipt:

`blake3:710bdec3dc24275d38e875b57af66c6926c2d6c3985aa076400eb00570e18532`

Lifecycle gate receipts before archive:

- proposal: `f5a1476306f4400b38791db538a0dcedabf7ef6ca5560b29f809898fcfe6c863`;
- design: `f44f4440e88862a73211b82402e49816d0f837b5bc0dac6aa573245d8cd8daea`;
- tasks: `509aa4ea9e0ca7e1987dae1a0ed6722adc5aaf80d29cc8bf6243e41f07d3d45a`.

Final sync mutation manifest:

`1d6d2876d52c6b02a849bbedbbe5a84abe6410328a2a0f57ae62f0a7e14da5fe`

Final sync receipt:

`70c847adc7e738a413985c652ef96b6a585a8fa8b3082be13c89a48de275c5e1`

## Compatibility checker boundary

The pinned compatibility checker reports 2,488 requirements, 231 references, 2,257 missing requirements, and zero dangling references.
It still fails because it scans only `crates/` and `tools/`.
The repository-owned comprehensive guard is the bounded classification authority for this change.

## Search budget and terminal result

The search used four distinct mechanisms and direct adversarial review of the surviving classifier design.
No subagents were used.

The grouped inventory is complete and validated.
Global implementation coverage remains unproven for 1,979 accepted requirements.
Future batches can now start with the 442-entry runtime-spine group and reduce the baseline only through reviewed direct evidence.

The classification report is routing evidence only.
It does not establish implementation, replacement, obsolescence, invalidity, behavioral correctness, or release readiness.
