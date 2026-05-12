## Phase 1: Inventory and Classification

- [ ] [serial] Regenerate the residual `external_body` inventory and classify every marker in `tuple_spec.rs`, `chain_verify_spec.rs`, and `mac_spec.rs` as structural, wrapper-over-axiom, or irreducible trusted boundary.
- [ ] [depends:inventory] Add or update local comments so every retained trusted body names the exact crypto/encoding assumption and backing runtime/library surface.

## Phase 2: Tuple Boundary Reduction

- [ ] [depends:inventory] Reduce `crates/aspen-core/verus/tuple_spec.rs` by proving structural recursive helpers where feasible while leaving only explicit `pack`/`unpack` encoding assumptions trusted.
- [ ] [depends:tuple] Verify tuple slice with `nix develop -c verus --triggers-mode silent --crate-type=lib crates/aspen-core/verus/lib.rs` and focused core/tuple runtime tests if executable contracts or runtime-evidence comments change.

## Phase 3: Chain Verification Boundary Reduction

- [ ] [depends:inventory] Narrow `crates/aspen-raft/verus/chain_verify_spec.rs` so structural chain-map/linking facts are proved or explicitly separated from Blake3 collision-resistance assumptions.
- [ ] [depends:chain] Verify chain slice with `nix develop -c verus --triggers-mode silent --crate-type=lib crates/aspen-raft/verus/lib.rs` and focused chain/hash/integrity tests if executable contracts change.

## Phase 4: MAC Boundary Reduction

- [ ] [depends:inventory] Narrow `crates/aspen-secrets/verus/mac_spec.rs` by keeping named HMAC axioms trusted while proving wrapper/output/sensitivity facts that can delegate to those axioms.
- [ ] [depends:mac] Verify MAC slice with `nix develop -c verus --triggers-mode silent --crate-type=lib crates/aspen-secrets/verus/lib.rs` and focused MAC/SOPS runtime tests if executable contracts change.

## Phase 5: Closure

- [ ] [depends:tuple] [depends:chain] [depends:mac] Regenerate the repo-wide `external_body` inventory and confirm remaining markers are only documented intentional crypto/encoding boundaries.
- [ ] [depends:closure] Run `openspec validate --all --strict`, `git diff --check`, then sync/archive this change once all tasks are complete.
