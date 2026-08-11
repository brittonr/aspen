# Validation evidence

## Goal and completion boundary

The goal was to reduce inherited runtime-spine debt through a bounded review of twelve canonical content-ref requirements.
Completion required direct production and test markers, typed candidate evidence, exact baseline regeneration, zero dangling references, and full repository validation.

## Canonical input

Base revision:

`e275a9520795b81c797f5013357d48d5d419477b`

Pinned Cairn revision:

`3b4c280b893f2709aebea21fc51a4f9eeba3fe3b`

Starting inherited debt: 1,943 requirements.
Starting runtime-spine debt: 406 requirements.
Starting canonical content-ref debt: 12 requirements.

## Approach registry

### Shared parser and typed identity

Mechanism: inspect `ContentRef`, canonical shape validation, raw-byte, hash, and hex helpers, canonical Preserves hashing, and malformed input tests.

Result: validated shared parsing, canonical shape, and rejection of unsupported aliases as runtime identity.

### Materialized storage and filename readback

Mechanism: inspect ledger and chunk-store filename conversion, local read paths, ingress materialization, canonical decoding, and byte-ref recomputation.

Result: validated filename readback, missing-content denial, and tampered-content denial.

### Node-control and trust separation

Mechanism: inspect request, payload, ingress envelope, transport receipt, and subreceipt parsing with authority and resource denial tests.

Result: validated canonical node-control refs without promoting content identity into trust.

### Runtime and migration surfaces

Mechanism: inspect runtime values, messages, assertions, observations, events, turns, snapshots, and the ten listed migrated validators.

Result: validated canonical runtime identities, replay stability, and the bounded migration requirement.

### Adversarial formatting audit

Mechanism: search production source for direct `blake3:` formatting and prefix manipulation outside the Preserves rail.

Result: falsified two broad candidates.
`src/fabric_transport/adapters.rs` still contains a subsystem-local BLAKE3 formatting helper.
`src/fabric_transport/cross_process/iroh_shell.rs` still contains direct prefix formatting and manipulation.

The serial search passes are correlated.
No subagents were used.

## Accepted repairs

The typed manifest records ten direct repairs:

- `molten.runtime_spine.canonical_content_refs.cleanup_tests`;
- `molten.runtime_spine.canonical_content_refs.filename_readback`;
- `molten.runtime_spine.canonical_content_refs.materialized_readback`;
- `molten.runtime_spine.canonical_content_refs.migration`;
- `molten.runtime_spine.canonical_content_refs.negative_tests`;
- `molten.runtime_spine.canonical_content_refs.node_control`;
- `molten.runtime_spine.canonical_content_refs.not_trust`;
- `molten.runtime_spine.canonical_content_refs.runtime_values`;
- `molten.runtime_spine.canonical_content_refs.scoped_aliases`;
- `molten.runtime_spine.canonical_content_refs.shape`.

The patch adds direct markers, deterministic evidence, and focused parser/readback assertions.
Production runtime behavior did not change.

## Rejected candidates

Two candidates remain `accepted-implementation-unestablished`:

- `molten.runtime_spine.canonical_content_refs.helper_construction`;
- `molten.runtime_spine.canonical_content_refs.no_ad_hoc_formatting`.

Current subsystem-local formatting prevents either broad claim.
Archived task completion does not override current production counterexamples.

## Final inventory

The comprehensive guard reports:

- requirements: 2,504;
- referenced: 571;
- uncovered: 1,933;
- dangling: zero;
- verdict: pass against the exact baseline.

The grouped classifier reports:

- classified entries: 1,933;
- specification groups: 35;
- source area groups: 107;
- runtime-spine entries: 396;
- canonical content-ref entries: 2;
- verdict: pass.

The inherited baseline decreased by ten entries.
The runtime-spine queue decreased from 406 to 396 entries.

## Identities

Baseline BLAKE3:

`9e0ffae8e1c4727c33e16f34ac87d94b8ee54e2312c20b02a00429475d8170a0`

Classification TSV BLAKE3:

`bfe7676b1427ecca0af847a5876c6ebbdcb63820b27c97c6a117233349ee5016`

Classification summary BLAKE3:

`6c0951e187149e65051afa3b44056898203f1e229dc8254986b1f4b86bdb9c9b`

Generated baseline JSON BLAKE3:

`ae02fe0416ad450d766beba11b84c28bcca3641f941d45cd50276f8d86865f78`

Generated classification JSON BLAKE3:

`f82201d82bb465f8f0ffe77ed149ba1de8fc715f93d193653f38b1196c603d34`

Generated canonical content-ref repair JSON BLAKE3:

`e5b5237a53a9c3b37d715e17a63e49849d92a0b873ae17d7afae3d2699c1d458`

## Validation

The following checks passed:

- pre-change focused content-ref tests: 12 passed;
- post-change focused content-ref tests: 12 passed;
- inherited debt guard tests: 4 passed;
- classification tests: 4 passed;
- typed Nickel manifest checks and deterministic JSON exports;
- exact accepted and rejected candidate checks;
- focused `inherited-tracey-debt` Nix check;
- Cargo formatting;
- `cargo tigerstyle check` with the repository baseline;
- pinned Cairn validation;
- proposal, design, and tasks gates;
- full `nix flake check path:$PWD -L`;
- Nix nextest: 1,365 passed;
- `git diff --check`.

Full Nix CI test receipt:

`blake3:60786b1c3d6f77133f1c9c0f96daafed9265c65765bb4f7bd91745ed7df5bf37`

Lifecycle gate receipts before archive:

- proposal: `30692aa19a3acc71673e9c2cf6c2f4f19b04a3b56a89c1970c8da9551eb3a97b`;
- design: `016430e3063c72df6bf547a5e1f8fbaddee09d67dcadee771945622c96428807`;
- tasks: `ad0c179b765e4fb891101681a2ce719705df08e40fb390ad6aad58e94cfca889`.

Sync mutation manifest:

`c45c5d52eae013e7e980d2cbb67fb4c2dcd66f838a59568eb6763f2df1a8249c`

Sync receipt:

`843b8cc1025127c04045208c77280ed814549352ca99f3afd2e78886a446a268`

## Compatibility checker boundary

The pinned compatibility checker reports 2,504 requirements, 231 references, 2,273 missing requirements, and zero dangling references.
It still fails because it scans only `crates/` and `tools/`.
The comprehensive repository guard scans the admitted source, test, tool, documentation, script, and flake roots.

## Terminal result and non-claims

The search budget ended after five serial mechanisms, focused tests, adversarial audit, and full deterministic validation.
Ten candidates are validated and two remain explicit debt.

This batch does not prove universal helper-only construction, removal of all ad hoc formatting, content-ref trust, complete runtime-spine coverage, release readiness, or whole-system correctness.
