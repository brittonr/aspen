# Validation evidence

## Goal and completion boundary

The goal was to reduce the remaining runtime-spine debt through a bounded review of the twelve blob-ref job requirements.
Completion required direct production and test markers, typed batch evidence, exact baseline regeneration, zero dangling references, and full repository validation.

## Canonical input

Base revision:

`53f8f700f6fbd8facac379fcf50a7d4fe42b4a6c`

Pinned Cairn revision:

`3b4c280b893f2709aebea21fc51a4f9eeba3fe3b`

Starting inherited debt: 1,965 requirements.
Starting runtime-spine debt: 428 requirements.
Starting blob-ref job debt: 12 requirements.

## Search registry

### Submission model and denial

Mechanism: inspect the canonical submission builder, parser, content-ref DTO, and inline-token denial.

Result: validated the payload model and no-inline-large-bytes requirements.

### Worker execution and evidence lifecycle

Mechanism: inspect preflight, fetch, verification, handler execution, pinning, cleanup, canonical receipts, ledger import, and focused pass and deny tests.

Result: validated local worker, content verification, provenance/policy inputs, retention pins, receipts, and local tests.

### Property coverage

Mechanism: inspect the bounded Hegel test over inline-token selection, verification-before-run, pin evidence, and cleanup evidence.

Result: validated the property-test requirement.

### Broad replay, status, and DAG claims

Mechanism: adversarially compare current code and tests with the full wording of replay identity, status transitions, and job-DAG integration.

Result: rejected all three candidates.
The canonical receipt does not bind a complete replay identity, tests do not cover every declared status transition, and a check label alone does not establish true DAG integration.

## Direct repairs

The typed manifest records nine repaired requirements:

- canonical content-ref-only submission payloads;
- inline large-byte denial;
- deterministic local worker execution;
- content verification before handler execution;
- explicit provenance, policy, and effect inputs;
- active pins and cleanup evidence;
- canonical pass and deny receipts;
- focused local pass and denial tests;
- bounded property coverage.

The patch adds marker comments and evidence metadata only.
Runtime behavior and existing test assertions did not change.

## Rejected candidates

These requirements remain explicit inherited debt:

- `molten.blob_ref_jobs.job_dag_integration`;
- `molten.blob_ref_jobs.replay_integration`;
- `molten.blob_ref_jobs.status_assertions`.

The Nix gate requires all three identifiers to remain in the exact baseline.

## Final inventory

The comprehensive guard reports:

- requirements: 2,496;
- referenced: 540;
- uncovered: 1,956;
- dangling: zero;
- verdict: pass against the exact baseline.

The grouped classifier reports:

- classified entries: 1,956;
- specification groups: 35;
- source area groups: 111;
- runtime-spine entries: 419;
- blob-ref job entries: 3;
- verdict: pass.

The inherited baseline decreased by nine entries.
The runtime-spine queue decreased from 428 to 419 entries.
The blob-ref job queue decreased from 12 to 3 entries.

## Identities

Baseline BLAKE3:

`4c94630b1513724d666a0d43a02156a2d125d23e942c8388bc3eab032b5633e4`

Classification TSV BLAKE3:

`9b1b0497dfc7e03a151f613cefe49e041d848fdd457168d39e69e8b4973e1099`

Classification summary BLAKE3:

`ce3b90d95ab16b5ee177e93d4dc39917a23abd07ceab03d87c11c8b11bbdb8e5`

Generated baseline JSON BLAKE3:

`45e18e35554050c695fc10e9ad834460a1bbe376dd765e4886f10f3c81a2a58b`

Generated classification JSON BLAKE3:

`6be700a7e70278b9573126c965769f24ef68eef7c27c336625b98eb67c265dff`

Generated blob-ref job repair JSON BLAKE3:

`cb8c6f0b42481644489b0cc4d33bde27e4ba5525f6fc5344cd2dd3a81e970784`

## Validation

The following checks passed:

- pre-change focused blob-ref job tests: 4 passed;
- pre-change bounded Hegel property test: 1 passed;
- post-change focused blob-ref job tests: 4 passed;
- post-change bounded Hegel property test: 1 passed;
- inherited debt guard tests: 4 passed;
- classification tests: 4 passed;
- typed Nickel manifest checks and deterministic JSON exports;
- exact marker and baseline checks for all nine repairs;
- exact retained-baseline checks for all three rejected candidates;
- focused `inherited-tracey-debt` Nix check;
- Cargo formatting;
- `cargo tigerstyle check`;
- pinned Cairn validation;
- proposal, design, and tasks gates;
- full `nix flake check path:$PWD -L`;
- Nix nextest: 1,365 passed;
- `git diff --check`.

The first full Nix attempt correctly failed after specification sync because the four new lifecycle evidence comments were outside admitted scanner roots.
The flake freshness gate now carries direct markers for those requirements.
The focused gate and full Nix check then passed.

Full Nix CI test receipt:

`blake3:3f2fb53fb876ab2a347a00cc8fb4753cb03fc351e0b9668c1a7048fad11a78a0`

Lifecycle gate receipts before archive:

- proposal: `afff572c68454630a409168d6fad41c5b92fbeda55c34147dc605e241df7130c`;
- design: `68f3ebd3c09ab6d41c81586c46dfab35250ddcc3509447b1feab1c9356954200`;
- tasks: `0e335659b18aa0c3a6b2e99e6445fdc0b9e68ccba870074b9c58f8402e7c37ce`.

Sync mutation manifest:

`d9fb0fccc877a9489dbd6a6815f203973e8c92c1f8a530bb597cb68eacf0dc07`

Sync receipt:

`c474c82074e4ba1ebfcfc9f6d9f8814644fe5fe17a1e40820cdc77752565c8d4`

## Compatibility checker boundary

The pinned compatibility checker reports 2,496 requirements, 231 references, 2,265 missing requirements, and zero dangling references.
It still fails because it scans only `crates/` and `tools/`.
The comprehensive repository guard scans the admitted source, test, tool, documentation, script, and flake roots.

## Search budget and terminal result

The review used four serial mechanisms and an adversarial audit.
No subagents were used.

Nine direct repairs are validated.
Three broader blob-ref job requirements remain explicit debt.
The remaining 419 runtime-spine entries require further source-area review.

This batch does not prove blob-ref replay identity, full status-transition coverage, true job-DAG integration, complete runtime-spine coverage, release readiness, or whole-system correctness.
