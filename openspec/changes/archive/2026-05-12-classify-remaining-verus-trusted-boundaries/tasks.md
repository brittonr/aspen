## Phase 1: Inventory and Classification

- [x] [serial] Regenerated the residual `external_body` inventory and classified every marker in `tuple_spec.rs`, `chain_verify_spec.rs`, and `mac_spec.rs` as structural, wrapper-over-axiom, or irreducible trusted boundary. ✅ evidence: exact retained marker counts are tuple=12, chain_verify=9, mac=5 after reduction (started: 2026-05-12T04:04:18Z → completed: 2026-05-12T04:12:57Z)
- [x] [depends:inventory] Added or updated local comments so every retained trusted body names the exact crypto/encoding/structural assumption and backing runtime/library surface. ✅ evidence: comments now distinguish tuple `pack`/`unpack` encoding, tuple lexicographic structural lemmas, Blake3 collision-resistance wrappers, and HMAC library axioms (started: 2026-05-12T04:04:18Z → completed: 2026-05-12T04:12:57Z)

## Phase 2: Tuple Boundary Reduction

- [x] [depends:inventory] Reduced `crates/aspen-core/verus/tuple_spec.rs` by proving structural recursive helpers where feasible while leaving only explicit `pack`/`unpack` encoding assumptions trusted. ✅ evidence: proved `tuple_size`, `elements_size`, `element_size`, and three backwards-compatible comparison aliases; retained 12 documented encoding/lexicographic boundaries (started: 2026-05-12T04:06:00Z → completed: 2026-05-12T04:12:57Z)
- [x] [depends:tuple] Verified tuple slice with `nix develop -c verus --triggers-mode silent --crate-type=lib crates/aspen-core/verus/lib.rs` and focused core/tuple runtime tests if executable contracts or runtime-evidence comments change. ✅ evidence: Verus core root `57 verified, 0 errors`; `cargo test -p aspen-core --lib -- --nocapture` `131 passed`; tuple filter had no matching runtime tests (started: 2026-05-12T04:08:00Z → completed: 2026-05-12T04:12:57Z)

## Phase 3: Chain Verification Boundary Reduction

- [x] [depends:inventory] Narrowed `crates/aspen-raft/verus/chain_verify_spec.rs` so structural chain-map/linking facts are proved or explicitly separated from Blake3 collision-resistance assumptions. ✅ evidence: removed trusted body from `verified_range_implies_valid`; retained 9 documented Blake3/chain-map trust boundaries (started: 2026-05-12T04:06:00Z → completed: 2026-05-12T04:12:57Z)
- [x] [depends:chain] Verified chain slice with `nix develop -c verus --triggers-mode silent --crate-type=lib crates/aspen-raft/verus/lib.rs` and focused chain/hash/integrity tests if executable contracts change. ✅ evidence: Verus raft root `124 verified, 0 errors` with pre-existing ambiguous `should_flush` glob warning; `cargo test -p aspen-raft spec::chain_hash -- --nocapture` `1 passed`; `cargo test -p aspen-raft verified::integrity -- --nocapture` `18 passed` (started: 2026-05-12T04:08:00Z → completed: 2026-05-12T04:12:57Z)

## Phase 4: MAC Boundary Reduction

- [x] [depends:inventory] Narrowed `crates/aspen-secrets/verus/mac_spec.rs` by keeping named HMAC axioms trusted while proving wrapper/output/sensitivity facts that can delegate to those axioms. ✅ evidence: proved `mac_key_sensitivity` and `mac_output_length` by delegating to named HMAC axioms; retained 5 documented HMAC/concatenation-injectivity trust boundaries (started: 2026-05-12T04:06:00Z → completed: 2026-05-12T04:12:57Z)
- [x] [depends:mac] Verified MAC slice with `nix develop -c verus --triggers-mode silent --crate-type=lib crates/aspen-secrets/verus/lib.rs` and focused MAC/SOPS runtime tests if executable contracts change. ✅ evidence: Verus secrets root `12 verified, 0 errors`; `cargo test -p aspen-secrets --lib -- --nocapture` `57 passed`; `cargo test -p aspen-secrets mac -- --nocapture` had no matching tests (started: 2026-05-12T04:08:00Z → completed: 2026-05-12T04:12:57Z)

## Phase 5: Closure

- [x] [depends:tuple] [depends:chain] [depends:mac] Regenerated the repo-wide `external_body` inventory and confirmed remaining markers are only documented intentional crypto/encoding/structural boundaries. ✅ evidence: retained exact scoped inventory is tuple=12, chain_verify=9, mac=5; broader repo contains other existing trusted-boundary files outside this change scope (started: 2026-05-12T04:12:00Z → completed: 2026-05-12T04:12:57Z)
- [x] [depends:closure] Ran `openspec validate --all --strict`, `git diff --check`, then sync/archive this change once all tasks are complete. ✅ evidence: `openspec validate classify-remaining-verus-trusted-boundaries --strict` valid; `openspec validate --all --strict` 220 passed, 0 failed; `git diff --check` clean before archive (started: 2026-05-12T04:12:00Z → completed: 2026-05-12T04:12:57Z).
