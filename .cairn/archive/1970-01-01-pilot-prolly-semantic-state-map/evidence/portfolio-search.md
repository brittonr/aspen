# Prolly map portfolio search

## Success contract

Goal: select and validate the weakest Prolly mechanism that gives Molten bounded, history-independent keyed semantic state.

Completion needs pure canonical structure, bounded edits and diffs, a storage shell, fail-closed recovery and GC, positive and negative tests, and bounded evidence.

False completion includes:

- a normal B-tree whose root depends on mutation history;
- a vendor adapter that moves DoltLite or SQLite types into the core;
- hash equality claims across Molten and DoltLite formats;
- reachability evidence that authorizes deletion;
- a proof label without a production linkage;
- benchmark counts presented as correctness or extraction authority.

Risks are chosen-key split pressure, variable-size values, malformed node bytes, incomplete closure facts, stale publication, unknown outcomes, hash assumptions, and premature extraction.

The search used three serial, correlated lenses. Subagent consent was not granted. The budget was three mechanism families, two repository searches, one adversarial audit, and deterministic repository validation.

## Registry

### Published component reuse

- **Mechanism:** Reuse Content Identity Core framing and Schema Migration Core planning. Search sibling projects for an existing published Prolly map.
- **Claim:** Shared framing and migration stay pinned while Molten retains map semantics.
- **Artifact:** `content-identity-core` revision `7f55597b5dc879b7601856e8d7fd0dbacaa2a498`; Schema Migration Core revision `4fe90e130f2871cf69a6febcdc70785adca98aea`.
- **Evidence:** No compatible Onix Prolly or history-independent ordered-map component was found. Bounded Tree owns filesystem-tree admission, not ordered map semantics.
- **Gap strength:** simpler for framing; unknown for a complete map.
- **Blocker:** no published component has the same map, authority, and evidence contract.
- **State:** validated for framing reuse; blocked for full map reuse.

### Vendor-backed map

- **Mechanism:** Use DoltLite directly as the map implementation.
- **Claim:** Obtain Prolly behavior without a new Rust core.
- **Artifact:** pinned DoltLite oracle cohort `10170ed82c1b12414db8d1b29d2fe9ea2a72fd88`.
- **Evidence:** The accepted oracle contract keeps DoltLite optional, test-only, and identity-separated.
- **Gap strength:** stronger vendor and format coupling than the goal.
- **Known failure:** It would collapse the independent oracle boundary and import SQLite process and format state.
- **State:** falsified.

### Path-local content-defined update

- **Mechanism:** Traverse one search path, rebuild affected chunks, and continue until boundaries converge.
- **Claim:** Reduce edit work while preserving canonical roots.
- **Artifact:** design sketch only.
- **Evidence:** The mechanism adds neighbor-window, convergence, partial-closure, and crash recovery obligations before the pilot has a measured need.
- **Gap strength:** stronger than the minimum goal.
- **Blocker:** no requirement sets a path-local work target.
- **State:** blocked as premature complexity.

### Rebuild-first canonical map

- **Mechanism:** Apply edits to a complete validated snapshot, sort canonical entries, rebuild with key-derived size-aware boundaries, and stage only new block identities.
- **Claim:** Equal state yields equal roots, while unchanged blocks remain shared.
- **Artifact:** `crates/molten-core/src/prolly_map/` and `src/prolly_map/`.
- **Evidence:** permutation, compaction, sharing, diff, restart, tamper, stale, unknown, retention, and adversarial-bound tests.
- **Gap strength:** simpler than a path-local update and equivalent to the pilot contract.
- **Known limitation:** edit planning reads the complete supplied snapshot.
- **Next check:** named structural benchmark and extraction classifier.
- **State:** validated candidate.

## Adversarial audit

The audit tried duplicate keys, wrong profiles, overlapping ranges, extra and missing blocks, tampered bytes, oversized values, chosen keys, forced bounds, stale CAS, block collisions, unknown publication before and after apply, incomplete graphs, active pins, crossed GC candidates, merge overreach, timing overclaims, and extraction overclaims.

All tested cases fail closed. Unknown publication performs one durable readback and no blind retry. GC needs exact revalidated candidates and separate deletion authority.

## Result

The rebuild-first canonical map survives. It is the weakest checked mechanism that satisfies the pilot contract.

The terminal state is validated for this bounded pilot. Formal Trellis refinement and a path-local optimization remain open. Neither is claimed as complete.
