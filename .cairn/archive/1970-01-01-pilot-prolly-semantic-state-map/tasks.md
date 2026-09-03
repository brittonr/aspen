## Phase 1: Canonical profile and node core

- [x] [depends:introduce-world-commit-core] Record baseline semantic-state identity, diff, retention, crash, and benchmark outcomes. r[molten.prolly_map.verification]
- [x] [serial] Define the typed Nickel profile for codecs, ordering, format, boundary domain, size accounting, chunk bounds, fanout bounds, and BLAKE3 domains. r[molten.prolly_map.profile]
- [x] [depends:prolly-map-profile] Define canonical leaf, internal node, root, child range, edit, diff, closure, reachability, and GC-plan DTOs. r[molten.prolly_map.canonical_nodes]
- [x] [parallel] Implement pure profile, node, ordering, range, bound, and canonical encoding validation. r[molten.prolly_map.canonical_nodes]
- [x] [parallel] Record Trellis proof obligations and explicit BLAKE3 assumptions for ordering, search, boundaries, edits, diffs, and reachability. r[molten.prolly_map.proof_boundary]

## Phase 2: Map operations and shell

- [x] [depends:prolly-map-node-core] Implement pure point read, range read, full canonical build, and history-independent root derivation. r[molten.prolly_map.history_independence]
- [x] [depends:prolly-map-node-core] Implement bounded key-derived, size-aware split decisions with forced maximum encoded size. r[molten.prolly_map.boundaries]
- [x] [depends:prolly-map-read-core] Implement pure incremental insert, update, delete, and batch plans that preserve canonical structure. r[molten.prolly_map.history_independence]
- [x] [parallel] Implement complete bounded added, removed, and modified diff records without merge-policy decisions. r[molten.prolly_map.diff]
- [x] [parallel] Implement pure closure, reachability, and incomplete-safe GC planning from supplied roots, pins, and graph facts. r[molten.prolly_map.retention]
- [x] [serial] Add narrow immutable block-read and staged-publication ports, then implement one local shell adapter with transactional publication and unknown-outcome reconciliation. r[molten.prolly_map.storage_boundary]

## Phase 3: Evidence, benchmark, and extraction gate

- [x] [depends:prolly-map-operations] Add positive fixtures for reads, edits, equal-state histories, sharing, localized diff, restart, closure, and retained pins. r[molten.prolly_map.verification]
- [x] [parallel] Add negative malformed node, wrong profile, duplicate key, range overlap, missing child, oversized value, chosen-key pressure, tamper, stale publication, incomplete graph, active pin, and merge-overreach fixtures. r[molten.prolly_map.verification]
- [x] [depends:pilot-doltlite-semantic-state-oracle] Compare normalized bounded behavior with the pinned DoltLite oracle without requiring cross-format root equality. r[molten.prolly_map.differential]
- [x] [serial] Run sharing, changed-region diff, retained bytes, GC, restart, and adversarial-distribution benchmarks under named profiles. r[molten.prolly_map.benchmark]
- [x] [depends:prolly-map-benchmark] Apply the existing benchmark classifier and keep the implementation local unless two credible consumers satisfy the extraction gate. r[molten.prolly_map.extraction]
- [x] [serial] Document format, profile identity, architecture boundaries, migration, evidence assumptions, extraction gate, and non-claims. r[molten.prolly_map.proof_boundary]
- [x] [depends:prolly-map-verification] Run focused tests, property tests, Trellis rails, Octet, Clippy with warnings denied, Cairn validation and gates, and relevant Nix checks. r[molten.prolly_map.verification]
