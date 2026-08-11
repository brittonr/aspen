# Validation evidence

## Goal and completion boundary

The goal was to reduce inherited runtime-spine debt through a bounded review of thirteen SAM service requirements.
Completion required direct production and test markers, typed batch evidence, exact baseline regeneration, zero dangling references, and full repository validation.

## Canonical input

Base revision:

`80fbe0a87ef7b8248b90ef4357eed152fe2cc037`

Pinned Cairn revision:

`3b4c280b893f2709aebea21fc51a4f9eeba3fe3b`

Starting inherited debt: 1,956 requirements.
Starting runtime-spine debt: 419 requirements.
Starting SAM service debt: 13 requirements.

## Search registry

### Canonical service records

Mechanism: inspect service record builders, parsers, BLAKE3 identities, ledger classification, safe summaries, and malformed-record tests.

Result: validated canonical records, explicit manifest boundaries, and catalog redaction.

### Demand runtime

Mechanism: inspect suite admission, bounded dependency evaluation, evidence gates, readiness and status ownership, replay identities, and focused pass and deny tests.

Result: validated admitted demand startup, dependency resolution, owned assertion replay, and the earlier demand-start requirement.

### Logical supervision

Mechanism: inspect links, monitors, failure propagation, restart decisions, scheduled demands, lifecycle receipts, report replay, and process-parentage denial.

Result: validated deterministic supervision and logical supervision.

### Cleanup and retention

Mechanism: inspect owned-state indexes, bounded restart policy, retractions, cleanup receipts, retention inputs, replay gates, and adversarial foreign-state tests.

Result: validated bounded restart, owned cleanup, cleanup replay and retention, and the earlier cleanup requirement.

## Direct repairs

The typed manifest records thirteen repaired requirements across four source areas:

- canonical service records, stable BLAKE3 identities, explicit manifest boundaries, ledger classification, and safe summaries;
- canonical demand startup after explicit evidence admission;
- bounded dependency waits and cycle denial;
- owned readiness and status artifacts with replay identity;
- canonical logical links, monitors, failures, restart decisions, and deterministic replay;
- bounded restart policy under authority and resource evidence;
- owned-state cleanup with foreign-state denial;
- cleanup receipts and retention inputs bound into replay.

All thirteen reviewed candidates have direct implementation and verification evidence.
The patch adds marker comments and evidence metadata only.
Runtime behavior and existing test assertions did not change.

## Final inventory

The comprehensive guard reports:

- requirements: 2,500;
- referenced: 557;
- uncovered: 1,943;
- dangling: zero;
- verdict: pass against the exact baseline.

The grouped classifier reports:

- classified entries: 1,943;
- specification groups: 35;
- source area groups: 107;
- runtime-spine entries: 406;
- SAM service entries in the four reviewed areas: zero;
- verdict: pass.

The inherited baseline decreased by thirteen entries.
The runtime-spine queue decreased from 419 to 406 entries.

## Identities

Baseline BLAKE3:

`ae2115f20d7b1c21c8b5c451570b739b458ca981ef8f454aa04bced8b55deb56`

Classification TSV BLAKE3:

`1fa4a3ad215952b2c91105e5f5515c33f39492dfa2a15425a40b0841327ed285`

Classification summary BLAKE3:

`bf54482eae8d6aedda47052fec017400416011b6a4a168d3572eab858aa53f00`

Generated baseline JSON BLAKE3:

`9843edbc21716a5f4402c8f75e6c0e35a01cd02e31ac14121cc5f0e3fd08027f`

Generated classification JSON BLAKE3:

`842be2ce4c8b440077918d2c3498211906a655d0eaee6df4b1847b9a8b8cc7ad`

Generated SAM repair JSON BLAKE3:

`1a46bcbde633c403d52ee5d94cea1c16f79f91df4d735e12b2e3ebfa1b96c97e`

## Validation

The following checks passed:

- pre-change focused service record tests: 6 passed;
- pre-change focused demand runtime tests: 9 passed;
- pre-change focused supervision tests: 9 passed;
- post-change focused service record tests: 6 passed;
- post-change focused demand runtime tests: 9 passed;
- post-change focused supervision tests: 9 passed;
- inherited debt guard tests: 4 passed;
- classification tests: 4 passed;
- typed Nickel manifest checks and deterministic JSON exports;
- exact marker and baseline checks for all thirteen repairs;
- focused `inherited-tracey-debt` Nix check;
- Cargo formatting;
- `cargo tigerstyle check`;
- pinned Cairn validation;
- proposal, design, and tasks gates;
- full `nix flake check path:$PWD -L`;
- Nix nextest: 1,365 passed;
- `git diff --check`.

Full Nix CI test receipt:

`blake3:9af2e293d2028eccb247e29bcb56ba04747110521036f0bbab0a1a232f38db33`

Lifecycle gate receipts before archive:

- proposal: `70de7186d3a6369e3980dff58490680d8e5d08a787906cdfd36a7e7c6cad81b8`;
- design: `902a8a2d32a7adc3bf34472b4710b90672fa87bbb2425440a46b6848f0cd7690`;
- tasks: `35bb16ed9cb51e8559449e161c83a2a6e0c56c7d313909ad2676eb62e73e11b1`.

Sync mutation manifest:

`2652824b3b8afc0fad97c9da873cd8ada5b4bb696a3f268e1dc78a332f2263c9`

Sync receipt:

`7202a76714137b57139e53f25cc8669c4696a33f81d9b4522098034d02ade3da`

## Compatibility checker boundary

The pinned compatibility checker reports 2,500 requirements, 231 references, 2,269 missing requirements, and zero dangling references.
It still fails because it scans only `crates/` and `tools/`.
The comprehensive repository guard scans the admitted source, test, tool, documentation, script, and flake roots.

## Search budget and terminal result

The review used four serial mechanisms and an adversarial boundary audit.
No subagents were used.

All thirteen reviewed SAM service requirements have direct evidence.
The remaining 406 runtime-spine entries require further source-area review.

This batch does not grant ambient service authority, accept process parentage as supervision, bypass retention policy, establish complete runtime-spine coverage, establish release readiness, or prove whole-system correctness.
