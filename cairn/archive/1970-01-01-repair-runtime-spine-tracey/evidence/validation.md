# Validation evidence

## Goal and completion boundary

The goal was to reduce the 442-entry runtime-spine Tracey queue through direct source and test evidence.
Completion required exact per-requirement implementation markers, focused verification markers, typed repair metadata, and a fail-closed freshness check.

## Canonical input

Base revision:

`4dd9081da8137c10abcd2fac225d982db1d5d0a8`

Pinned Cairn revision:

`3b4c280b893f2709aebea21fc51a4f9eeba3fe3b`

Starting inherited debt: 1,979 requirements.
Starting runtime-spine debt: 442 requirements.

## Search registry

### Grouped source-area review

Mechanism: group the runtime-spine inventory by requirement namespace and inspect source areas with cohesive production modules.

Result: the shared Preserves rail had the strongest direct implementation and test correspondence.

### Pure-core and focused-test mapping

Mechanism: bind each candidate to a specific pure function or type and a focused positive or negative test.

Result: validated fourteen requirements across canonical decoding, parser helpers, schema validation, field contracts, and structural inspection.

### Broad trust-boundary promotion

Mechanism: mark every strict-decoding or high-risk admission requirement from the shared codec implementation.

Result: rejected.
The local codec and tests do not prove adoption at every named external trust boundary or semantic admission path.

### Archived-task promotion

Mechanism: use completed archive tasks as direct implementation evidence.

Result: rejected because task completion does not identify a current source and verification location.

## Direct repairs

The typed manifest records fourteen repaired requirements:

- two canonical-byte requirements;
- three shared parser-toolkit requirements;
- three schema-boundary requirements;
- two boundary-field-contract requirements;
- four structural value-inspection requirements.

Implementation markers bind existing code in `src/preserves/parts/rail/p001/body.rs`.
Verification markers bind focused codec tests and the job ambient-token denial test.
No runtime behavior changed.

## Final inventory

The comprehensive guard reports:

- requirements: 2,492;
- referenced: 527;
- uncovered: 1,965;
- dangling: zero;
- verdict: pass against the exact baseline.

The grouped classifier reports:

- classified entries: 1,965;
- specification groups: 35;
- source area groups: 111;
- runtime-spine entries: 428;
- verdict: pass.

The inherited baseline decreased by fourteen entries.
The runtime-spine queue decreased from 442 to 428 entries.

## Identities

Baseline BLAKE3:

`8683b2bbb290700478d6729709b76a1c462e39e8617f03b978625ce458384b76`

Classification TSV BLAKE3:

`c4bd27dc9f2db14c0d7ebd3f4de2737181594842f7ffa82b7ebe8b521878d8a7`

Classification summary BLAKE3:

`d2645c091da454748bbd849ebf195ef46437117e2d05489425dd6d112e4eeade`

Generated baseline JSON BLAKE3:

`60641e0458c591dbcc3b770848bd5e237b1411a90d101d4009a2688baad39e36`

Generated classification JSON BLAKE3:

`f2a22502dabcfc66002d9fbd16ac216b1410143a4d86cb976abda0f365342fea`

Generated runtime-spine repair JSON BLAKE3:

`439b46abb2238399d0a9228c17cc6dc191092855c47dd378a8343d266d5d24f0`

## Validation

The following checks passed:

- inherited debt guard tests: 4 passed;
- classification tests: 4 passed;
- focused production tests: 11 passed;
- runtime-spine repair manifest typecheck and export freshness;
- exact implementation and verification marker checks for all fourteen repairs;
- focused `inherited-tracey-debt` Nix check;
- Cargo formatting;
- standalone Rust formatting;
- `cargo tigerstyle check`;
- pinned Cairn validation;
- proposal, design, and tasks gates;
- full `nix flake check path:$PWD -L`;
- Nix nextest: 1,365 passed;
- `git diff --check`.

Full Nix CI test receipt:

`blake3:3bfcad189de2360b74607ea63c657485bbd0505851809ae5870d767e55c2d08b`

Lifecycle gate receipts before archive:

- proposal: `f82246435ed47aa7886581d758e53de5a9718dc54c75f170ddc22cce6d7a4e04`;
- design: `134c31c2b5d2bd209c2adead1cad5293344ae730431249369588c3fb7a71fb07`;
- tasks: `1e52f90f6e1af4b85a46b12cf1aeddcf44cfe193383f8016e4dbc547132a949e`.

Sync mutation manifest:

`d6cd27f24571ead75d177c6275f714de5e3b798e8a85f1e0278bfce6bc2c6bca`

Sync receipt:

`4702609b209e234c49ee6f925287a66748eb9b7233b61459975b2cf56e9f7122`

## Compatibility checker boundary

The pinned compatibility checker reports 2,492 requirements, 231 references, 2,261 missing requirements, and zero dangling references.
It still fails because it scans only `crates/` and `tools/`.
The comprehensive repository guard scans the admitted source, test, tool, documentation, script, and flake roots.

## Search budget and terminal result

The review used four distinct mechanisms and adversarially rejected two overbroad promotion routes.
No subagents were used.

The fourteen-entry direct repair is validated.
The remaining 428 runtime-spine entries require further source-area review.

This batch does not prove complete runtime-spine coverage, adoption at every trust boundary, release readiness, or whole-system correctness.
